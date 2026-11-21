// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Patrick Weppelmann

//! Talking to WLED: discovery (mDNS + subnet sweep), `/json/info` probing and
//! the two realtime protocols (DDP and DRGB).

use std::io::Write;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Protocol;

#[derive(Debug, Clone)]
pub struct WledInfo {
    pub host: String,
    pub name: String,
    pub leds: u16,
    pub version: String,
    pub arch: String,
}

fn http_get(host: &str, path: &str, timeout: Duration) -> Option<String> {
    let addr = format!("{host}:80");
    let sock: SocketAddr = addr.to_socket_addrs().ok()?.next()?;
    let mut stream = TcpStream::connect_timeout(&sock, timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept: */*\r\n\r\n");
    stream.write_all(req.as_bytes()).ok()?;
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        use std::io::Read;
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > 64 * 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let i = text.find("\r\n\r\n")?;
    Some(text[i + 4..].to_string())
}

/// Ask a single WLED device for its JSON info.
pub fn probe(host: &str, timeout: Duration) -> Option<WledInfo> {
    let body = http_get(host, "/json/info", timeout)?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    // a random HTTP server would not answer with WLED's json
    if v.get("ver").is_none() && v.get("leds").is_none() {
        return None;
    }
    let leds = v
        .get("leds")
        .and_then(|l| l.get("count"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as u16;
    Some(WledInfo {
        host: host.to_string(),
        name: v
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("WLED")
            .to_string(),
        leds,
        version: v
            .get("ver")
            .and_then(|n| n.as_str())
            .unwrap_or("?")
            .to_string(),
        arch: v
            .get("arch")
            .and_then(|n| n.as_str())
            .unwrap_or("?")
            .to_string(),
    })
}

/// Local IPv4 address used for the default route (no packet is sent).
pub fn local_ip() -> Option<IpAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    Some(sock.local_addr().ok()?.ip())
}

/// Probe every address of the local /24 subnet for WLED (48 at a time).
/// Finds devices that do not announce themselves via mDNS.
pub fn sweep(on_found: impl Fn(WledInfo) + Send + Sync + 'static) {
    let Some(IpAddr::V4(v4)) = local_ip() else {
        return;
    };
    let o = v4.octets();
    let hosts: Vec<String> = (1..=254)
        .filter(|last| *last != o[3])
        .map(|last| format!("{}.{}.{}.{}", o[0], o[1], o[2], last))
        .collect();
    let queue = Arc::new(Mutex::new(hosts));
    let on_found = Arc::new(on_found);
    let mut handles = Vec::new();
    for _ in 0..48 {
        let queue = queue.clone();
        let on_found = on_found.clone();
        handles.push(std::thread::spawn(move || loop {
            let Some(host) = queue.lock().unwrap().pop() else {
                break;
            };
            if let Some(info) = probe(&host, Duration::from_millis(350)) {
                on_found(info);
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
}

/// mDNS browse for several service types at once (pure Rust, every OS).
/// Returns (service type, IPv4 address, instance name) for `wait`.
pub fn mdns(services: &[&str], wait: Duration) -> Vec<(String, String, String)> {
    let Ok(daemon) = mdns_sd::ServiceDaemon::new() else {
        return Vec::new();
    };
    let receivers: Vec<_> = services
        .iter()
        .filter_map(|s| daemon.browse(s).ok().map(|rx| (s.to_string(), rx)))
        .collect();
    let deadline = Instant::now() + wait;
    let mut out: Vec<(String, String, String)> = Vec::new();
    while Instant::now() < deadline {
        let mut idle = true;
        for (service, rx) in &receivers {
            while let Ok(ev) = rx.try_recv() {
                idle = false;
                if let mdns_sd::ServiceEvent::ServiceResolved(info) = ev {
                    let instance = info
                        .fullname
                        .strip_suffix(&format!(".{service}"))
                        .unwrap_or(&info.fullname)
                        .to_string();
                    for addr in info.addresses.iter() {
                        let ip = addr.to_ip_addr();
                        if ip.is_ipv4() {
                            out.push((service.clone(), ip.to_string(), instance.clone()));
                        }
                    }
                }
            }
        }
        if idle {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    for s in services {
        let _ = daemon.stop_browse(s);
    }
    let _ = daemon.shutdown();
    out.sort();
    out.dedup();
    out
}

/// Probe one host in the background (manual "add by address").
pub fn probe_in_background(host: String) -> Arc<Mutex<Option<Result<WledInfo, String>>>> {
    let out = Arc::new(Mutex::new(None));
    let o = out.clone();
    std::thread::spawn(move || {
        let host = host
            .trim()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        let res = probe(&host, Duration::from_millis(1500)).ok_or(host);
        *o.lock().unwrap() = Some(res);
    });
    out
}

struct DdpSender {
    sock: UdpSocket,
    seq: u8,
}

/// UDP target for a device address. `host` may carry its own port
/// (`192.168.0.5:4048`), which the tests and NAT setups use.
fn udp_target(host: &str, default_port: u16) -> Option<SocketAddr> {
    if let Some(addr) = literal_target(host, default_port) {
        return Some(addr);
    }
    let host = host.trim();
    // `to_socket_addrs` needs a port; a bare name gets the default one
    let mut addr = match host.to_socket_addrs() {
        Ok(mut a) => a.next()?,
        Err(_) => (host, default_port).to_socket_addrs().ok()?.next()?,
    };
    if addr.port() == 0 {
        addr.set_port(default_port);
    }
    Some(addr)
}

/// An address that needs no name lookup (IP, optionally with port).
fn literal_target(host: &str, default_port: u16) -> Option<SocketAddr> {
    let host = host.trim();
    if let Ok(addr) = host.parse::<SocketAddr>() {
        return Some(addr);
    }
    host.parse::<IpAddr>()
        .ok()
        .map(|ip| SocketAddr::new(ip, default_port))
}

fn udp_socket(addr: SocketAddr) -> std::io::Result<UdpSocket> {
    let bind: SocketAddr = if addr.is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    };
    let sock = UdpSocket::bind(bind)?;
    sock.connect(addr)?;
    sock.set_nonblocking(true)?;
    Ok(sock)
}

impl DdpSender {
    /// Sender for an explicit address (the tests use a local port).
    fn connect_to(addr: SocketAddr) -> Option<Self> {
        Some(Self {
            sock: udp_socket(addr).ok()?,
            seq: 0,
        })
    }

    pub fn send(&mut self, colors: &[[u8; 3]]) -> std::io::Result<()> {
        // 480 LEDs per packet keeps the datagram below the ethernet MTU
        const CHUNK: usize = 480;
        let mut offset = 0usize;
        let mut result = Ok(());
        while offset < colors.len() {
            let n = (colors.len() - offset).min(CHUNK);
            let last = offset + n >= colors.len();
            let mut pkt = Vec::with_capacity(10 + n * 3);
            // version 1; PUSH only on the last packet so WLED shows the whole
            // frame at once instead of tearing at every 480-LED boundary
            pkt.push(if last { 0x41 } else { 0x40 });
            // sequence 1..=15 (0 means "unused" in DDP)
            pkt.push((self.seq % 15) + 1);
            pkt.push(0x0B); // RGB24
            pkt.push(0x01); // destination id
            pkt.extend_from_slice(&((offset as u32) * 3).to_be_bytes());
            pkt.extend_from_slice(&((n as u16) * 3).to_be_bytes());
            for c in &colors[offset..offset + n] {
                pkt.extend_from_slice(c);
            }
            if let Err(e) = self.sock.send(&pkt) {
                result = Err(e);
            }
            offset += n;
        }
        self.seq = self.seq.wrapping_add(1);
        result
    }
}

pub struct DrgbSender {
    sock: UdpSocket,
}

impl DrgbSender {
    fn connect_to(addr: SocketAddr) -> Option<Self> {
        Some(Self {
            sock: udp_socket(addr).ok()?,
        })
    }

    pub fn send(&mut self, colors: &[[u8; 3]]) -> std::io::Result<()> {
        let mut pkt = Vec::with_capacity(6 + colors.len() * 3);
        pkt.extend_from_slice(b"DRGB");
        pkt.push(2); // seconds until WLED falls back to its own effect
        for c in colors.iter().take(DRGB_MAX) {
            pkt.extend_from_slice(c);
        }
        self.sock.send(&pkt).map(|_| ())
    }
}

/// DRGB fits 490 LEDs into one datagram.
pub const DRGB_MAX: usize = 490;

enum Socket {
    Ddp(DdpSender),
    Drgb(DrgbSender),
}

/// How often an unresolvable or unreachable address is tried again.
const RETRY: Duration = Duration::from_secs(3);
/// How often a host *name* is looked up again (DHCP may move the device).
const REFRESH: Duration = Duration::from_secs(60);

/// Realtime sender for one WLED device.
///
/// `send` runs in the pipeline thread on every tick, so it never blocks:
/// IP addresses are used directly, host names are looked up in a helper
/// thread (a failing lookup can take seconds), and a broken socket (network
/// changed, interface down) is rebuilt after a short pause.
pub struct Sender {
    host: String,
    protocol: Protocol,
    sock: Option<(SocketAddr, Socket)>,
    lookup: Option<std::sync::mpsc::Receiver<Option<SocketAddr>>>,
    next_try: Instant,
    looked_up: Instant,
    error: Option<String>,
}

impl Sender {
    pub fn new(host: &str, protocol: Protocol) -> Self {
        Self {
            host: host.trim().to_string(),
            protocol,
            sock: None,
            lookup: None,
            next_try: Instant::now(),
            looked_up: Instant::now(),
            error: None,
        }
    }

    fn port(&self) -> u16 {
        match self.protocol {
            Protocol::Ddp => 4048,
            Protocol::Drgb => 21324,
        }
    }

    fn open(&mut self, addr: SocketAddr) {
        let sock = match self.protocol {
            Protocol::Ddp => DdpSender::connect_to(addr).map(Socket::Ddp),
            Protocol::Drgb => DrgbSender::connect_to(addr).map(Socket::Drgb),
        };
        match sock {
            Some(s) => {
                self.sock = Some((addr, s));
                self.error = None;
            }
            None => {
                self.sock = None;
                self.error = Some(format!("{addr}: no route"));
                self.next_try = Instant::now() + RETRY;
            }
        }
    }

    fn start_lookup(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        let host = self.host.clone();
        let port = self.port();
        let spawned = std::thread::Builder::new()
            .name("wled-lookup".into())
            .spawn(move || {
                let _ = tx.send(udp_target(&host, port));
            });
        if spawned.is_ok() {
            self.lookup = Some(rx);
        } else {
            self.next_try = Instant::now() + RETRY;
        }
    }

    /// Make sure there is a socket, without ever waiting for the network.
    fn prepare(&mut self) {
        let now = Instant::now();
        if let Some(rx) = &self.lookup {
            match rx.try_recv() {
                Ok(found) => {
                    self.lookup = None;
                    self.looked_up = now;
                    match found {
                        // same address as before: keep the socket (and the DDP sequence)
                        Some(addr) if self.sock.as_ref().map(|s| s.0) == Some(addr) => {}
                        Some(addr) => self.open(addr),
                        // a failed refresh keeps the last working address
                        None if self.sock.is_some() => {}
                        None => {
                            self.error = Some(format!(
                                "{}: {}",
                                self.host,
                                crate::i18n::tr(
                                    "Adresse nicht auflösbar",
                                    "cannot resolve address"
                                )
                            ));
                            self.next_try = now + RETRY;
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.lookup = None;
                    self.next_try = now + RETRY;
                }
            }
            return;
        }
        if self.sock.is_none() {
            if now < self.next_try {
                return;
            }
            match literal_target(&self.host, self.port()) {
                Some(addr) => self.open(addr),
                None => self.start_lookup(),
            }
        } else if literal_target(&self.host, self.port()).is_none()
            && self.looked_up.elapsed() >= REFRESH
        {
            self.start_lookup();
        }
    }

    pub fn send(&mut self, colors: &[[u8; 3]]) {
        self.prepare();
        let Some((_, sock)) = self.sock.as_mut() else {
            return;
        };
        let result = match sock {
            Socket::Ddp(s) => s.send(colors),
            Socket::Drgb(s) => s.send(colors),
        };
        if let Err(e) = result {
            use std::io::ErrorKind;
            match e.kind() {
                // full send buffer: drop this frame, the next one follows
                ErrorKind::WouldBlock => {}
                // ICMP "port unreachable" from the last packet: WLED is
                // rebooting or realtime is off; keep sending
                ErrorKind::ConnectionRefused => {}
                // network changed or went away: build a new socket shortly
                _ => {
                    self.error = Some(format!("{}: {e}", self.host));
                    self.sock = None;
                    self.next_try = Instant::now() + RETRY;
                }
            }
        }
    }

    pub fn status(&self) -> crate::output::Status {
        use crate::output::Status;
        match (&self.sock, &self.error) {
            (Some(_), _) => Status::Ok,
            (None, Some(e)) => Status::Error(e.clone()),
            (None, None) => Status::Connecting,
        }
    }
}

/// Build the DDP datagrams for one frame (exposed for tests).
#[cfg(test)]
fn ddp_packets(colors: &[[u8; 3]]) -> Vec<Vec<u8>> {
    let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let port = sock.local_addr().unwrap().port();
    sock.set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    // connect straight to the test port: macOS refuses to re-connect a
    // connected UDP socket (EISCONN), Linux allows it
    let mut tx = DdpSender::connect_to(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
    tx.send(colors).unwrap();
    let mut out = Vec::new();
    let mut buf = [0u8; 2048];
    while let Ok(n) = sock.recv(&mut buf) {
        out.push(buf[..n].to_vec());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unresolvable_name_never_blocks_the_pipeline() {
        // `.invalid` is reserved and never resolves (RFC 6761)
        let mut tx = Sender::new("wled-strip.invalid", Protocol::Ddp);
        let t0 = Instant::now();
        for _ in 0..200 {
            tx.send(&[[1, 2, 3]; 10]);
        }
        assert!(
            t0.elapsed() < Duration::from_millis(200),
            "send waited for DNS: {:?}",
            t0.elapsed()
        );
        // the lookup runs in the background and ends in an error
        let deadline = Instant::now() + Duration::from_secs(15);
        while !matches!(tx.status(), crate::output::Status::Error(_)) && Instant::now() < deadline {
            tx.send(&[[0; 3]; 1]);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(matches!(tx.status(), crate::output::Status::Error(_)));
    }

    #[test]
    fn a_host_with_port_is_sent_to_directly() {
        let rx = UdpSocket::bind("127.0.0.1:0").unwrap();
        rx.set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();
        let host = format!("127.0.0.1:{}", rx.local_addr().unwrap().port());
        let mut tx = Sender::new(&host, Protocol::Drgb);
        tx.send(&[[7, 8, 9]]);
        let mut buf = [0u8; 64];
        let n = rx.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"DRGB\x02\x07\x08\x09");
        assert_eq!(tx.status(), crate::output::Status::Ok);
        assert_eq!(udp_target("localhost", 4048).map(|a| a.port()), Some(4048));
    }

    #[test]
    fn ddp_splits_and_pushes_only_the_last_packet() {
        let colors = vec![[1u8, 2, 3]; 1000];
        let pkts = ddp_packets(&colors);
        assert_eq!(pkts.len(), 3);
        assert_eq!(pkts[0][0], 0x40);
        assert_eq!(pkts[1][0], 0x40);
        assert_eq!(pkts[2][0], 0x41);
        // offsets in bytes: 0, 1440, 2880
        let off = |p: &Vec<u8>| u32::from_be_bytes([p[4], p[5], p[6], p[7]]);
        assert_eq!(off(&pkts[0]), 0);
        assert_eq!(off(&pkts[1]), 480 * 3);
        assert_eq!(off(&pkts[2]), 960 * 3);
        assert_eq!(u16::from_be_bytes([pkts[2][8], pkts[2][9]]), 40 * 3);
        assert!(pkts.iter().all(|p| (1..=15).contains(&p[1])));
    }
}
