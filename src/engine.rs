// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Patrick Weppelmann

//! Colour pipeline: sample -> median -> smoothing -> grading -> send.
//!
//! `tick` runs on a fixed grid (see `worker`). New capture frames are sampled
//! once; ticks in between only advance the smoothing, which keeps fades fluid
//! even when the screen delivers fewer frames than the LEDs get.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::capture::SharedFrame;
use crate::config::{Config, DeviceCfg};
use crate::output::{self, Output};
use crate::sampler::{self, Plan};

pub struct Engine {
    pub plan: Plan,
    plan_w: u32,
    plan_h: u32,
    /// sequence number of the last sampled frame
    sampled_seq: Option<u64>,
    /// latest filtered sample per device / led
    raw: Vec<Vec<[f32; 3]>>,
    /// smoothed values per device / led
    smoothing: Vec<Vec<[f32; 3]>>,
    /// last three raw samples per device / led, for the median filter
    history: Vec<Vec<[[f32; 3]; 3]>>,
    history_pos: usize,
    /// last colours that went out, for the LED preview
    pub last: Vec<Vec<[u8; 3]>>,
    /// one output per device id, with the connection key it was opened for
    outputs: HashMap<String, (String, Box<dyn Output>)>,
    /// frames sent per device (index = device index)
    sent: Vec<u64>,
    /// update rate of the fastest device
    pub send_fps: f32,
    fps_window: (Instant, Vec<u64>),
    pub last_sample_us: u128,
    /// last time we actually sent to a device (index = device index)
    last_sent: Vec<Instant>,
    /// freeze the picture (own window focused, or user pressed pause)
    pub paused: bool,
    stats: Option<Stats>,
}

/// Debug output with `AMBILIGHT_STATS=1`: how much the LEDs change per frame.
struct Stats {
    sum: f64,
    n: u64,
    t: Instant,
}

fn rgb_to_hsv(c: [f32; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (c[0], c[1], c[2]);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, if max <= 0.0 { 0.0 } else { d / max }, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}

#[inline]
fn median3(a: f32, b: f32, c: f32) -> f32 {
    a.max(b).min(a.min(b).max(c))
}

impl Engine {
    pub fn new(cfg: &Config, w: u32, h: u32) -> Self {
        Self {
            plan: sampler::build(cfg, w.max(1), h.max(1)),
            plan_w: w.max(1),
            plan_h: h.max(1),
            sampled_seq: None,
            raw: Vec::new(),
            smoothing: Vec::new(),
            history: Vec::new(),
            history_pos: 0,
            last: Vec::new(),
            outputs: HashMap::new(),
            sent: Vec::new(),
            send_fps: 0.0,
            fps_window: (Instant::now(), Vec::new()),
            last_sample_us: 0,
            last_sent: Vec::new(),
            paused: false,
            stats: std::env::var("AMBILIGHT_STATS").is_ok().then(|| Stats {
                sum: 0.0,
                n: 0,
                t: Instant::now(),
            }),
        }
    }

    /// Send `colors` to `dev`, (re)opening the output when its connection
    /// settings changed.
    fn send(&mut self, di: usize, dev: &DeviceCfg, colors: &[[u8; 3]]) {
        let key = dev.connection_key();
        let stale = self
            .outputs
            .get(&dev.id)
            .map(|(k, _)| k != &key)
            .unwrap_or(true);
        if stale {
            self.outputs
                .insert(dev.id.clone(), (key, output::open(dev)));
        }
        if let Some((_, out)) = self.outputs.get_mut(&dev.id) {
            out.send(colors);
        }
        if self.sent.len() <= di {
            self.sent.resize(di + 1, 0);
        }
        self.sent[di] += 1;
    }

    /// Is device `di` due for a packet, given its own fps cap?
    fn due(&mut self, di: usize, dev: &DeviceCfg, target_fps: u32) -> bool {
        if self.last_sent.len() <= di {
            self.last_sent
                .resize(di + 1, Instant::now() - Duration::from_secs(1));
        }
        let fps = if dev.max_fps > 0 {
            dev.max_fps
        } else {
            target_fps.max(1)
        };
        let interval = 1.0 / fps as f32;
        if self.last_sent[di].elapsed().as_secs_f32() >= interval - 0.001 {
            self.last_sent[di] = Instant::now();
            true
        } else {
            false
        }
    }

    /// Sample a new frame and run it through the median filter.
    fn sample(&mut self, frame: &SharedFrame, cfg: &Config) {
        let t0 = Instant::now();
        let mut fresh = Vec::new();
        sampler::sample(&self.plan, &frame.data, cfg.settings.spatial, &mut fresh);
        self.last_sample_us = t0.elapsed().as_micros();

        // median of the last three samples per channel removes single-frame
        // spikes (cursor blink, video compression noise) without real lag
        let shape_changed = self.history.len() != fresh.len()
            || self
                .history
                .iter()
                .zip(&fresh)
                .any(|(h, f)| h.len() != f.len());
        if shape_changed {
            self.history = fresh
                .iter()
                .map(|d| d.iter().map(|c| [*c; 3]).collect())
                .collect();
        }
        self.history_pos = (self.history_pos + 1) % 3;
        let use_median = cfg.settings.smoothing_ms > 0.0;
        for (dh, df) in self.history.iter_mut().zip(fresh.iter_mut()) {
            for (h, c) in dh.iter_mut().zip(df.iter_mut()) {
                h[self.history_pos] = *c;
                if use_median {
                    for k in 0..3 {
                        c[k] = median3(h[0][k], h[1][k], h[2][k]);
                    }
                }
            }
        }
        self.raw = fresh;
    }

    /// Connection state of every open output, by device id.
    pub fn statuses(&self) -> HashMap<String, output::Status> {
        self.outputs
            .iter()
            .map(|(id, (_, o))| (id.clone(), o.status()))
            .collect()
    }

    /// Close every output and wait briefly, so sessions end cleanly (Razer
    /// session closed, Govee/Nanoleaf handed back) when the app quits.
    pub fn shutdown(&mut self) {
        for (_, out) in self.outputs.values_mut() {
            out.shutdown(Duration::from_millis(800));
        }
        self.outputs.clear();
    }

    /// Close outputs of devices that were removed or switched off (ends
    /// sessions such as Razer Chroma, gives Govee/Nanoleaf back to their app).
    fn prune(&mut self, cfg: &Config) {
        if self.outputs.len() > cfg.devices.iter().filter(|d| d.enabled).count() {
            self.outputs
                .retain(|id, _| cfg.devices.iter().any(|d| d.enabled && &d.id == id));
        }
    }

    pub fn tick(&mut self, cfg: &Config, frame: Option<&SharedFrame>, dt: f32) {
        self.prune(cfg);
        let (w, h) = frame
            .map(|f| (f.w.max(1), f.h.max(1)))
            .unwrap_or((self.plan_w, self.plan_h));
        // rebuild the cell plan whenever the frame size or the zone layout changed
        if w != self.plan_w || h != self.plan_h || self.plan.layout != sampler::layout_hash(cfg) {
            self.plan = sampler::build(cfg, w, h);
            self.plan_w = w;
            self.plan_h = h;
            self.smoothing.clear();
            self.sampled_seq = None;
        }

        if self.paused {
            // keep the current colours on the strip instead of going dark
            // (WLED would fall back to its own effect after a timeout)
            for (di, dev) in cfg.devices.iter().enumerate() {
                if !dev.enabled || !self.due(di, dev, cfg.settings.target_fps) {
                    continue;
                }
                if let Some(last) = self.last.get(di).cloned() {
                    self.send(di, dev, &last);
                }
            }
            self.update_fps();
            return;
        }

        if let Some(f) = frame {
            if self.sampled_seq != Some(f.seq) {
                self.sampled_seq = Some(f.seq);
                self.sample(f, cfg);
            }
        }

        let s = &cfg.settings;
        let tau = (s.smoothing_ms / 1000.0).max(0.0);
        let alpha = if tau <= 0.0 {
            1.0
        } else {
            (1.0 - (-dt.max(0.0) / tau).exp()).clamp(0.0, 1.0)
        };
        // slew limit: changes per second are capped, so a single noisy frame
        // cannot make an LED jump
        let max_step = if s.slew > 0.0 {
            s.slew * dt.max(0.001)
        } else {
            f32::INFINITY
        };
        let gain_global = s.brightness;

        // enabled devices are planned in order; map config index -> plan index
        let mut plan_idx = 0usize;
        for (di, dev) in cfg.devices.iter().enumerate() {
            if !dev.enabled {
                continue;
            }
            let pi = plan_idx;
            plan_idx += 1;
            let Some(plan_dev) = self.plan.devices.get(pi) else {
                continue;
            };
            let n = dev.led_count as usize;
            if self.smoothing.len() <= di {
                self.smoothing.resize(di + 1, Vec::new());
            }
            let first_fill = self.smoothing[di].len() != n;
            if first_fill {
                self.smoothing[di] = vec![[0.0; 3]; n];
            }
            let raw_dev = self.raw.get(pi);
            let mut out = vec![[0u8; 3]; n];
            for (led, px_out) in out.iter_mut().enumerate() {
                let g = plan_dev.gains.get(led).copied().unwrap_or(0.0);
                let c0 = raw_dev
                    .and_then(|d| d.get(led))
                    .copied()
                    .unwrap_or([0.0; 3]);
                let c = [c0[0] * g, c0[1] * g, c0[2] * g];
                let sm = &mut self.smoothing[di][led];
                if alpha >= 1.0 || first_fill {
                    *sm = c;
                } else {
                    for k in 0..3 {
                        let diff = (c[k] - sm[k]) * alpha;
                        let step = diff.clamp(-max_step, max_step);
                        // dead band: tiny steps far from the target are noise
                        if step.abs() > 0.01 || diff.abs() < 0.6 {
                            sm[k] += step;
                        }
                    }
                }
                let mut v = *sm;
                if s.gamma != 1.0 {
                    for x in v.iter_mut() {
                        *x = 255.0 * (*x / 255.0).clamp(0.0, 1.0).powf(s.gamma);
                    }
                }
                if s.saturation != 1.0 {
                    let (hh, ss, vv) = rgb_to_hsv([v[0] / 255.0, v[1] / 255.0, v[2] / 255.0]);
                    let g = hsv_to_rgb(hh, (ss * s.saturation).clamp(0.0, 1.0), vv);
                    v = [g[0] * 255.0, g[1] * 255.0, g[2] * 255.0];
                }
                let gain = dev.brightness * gain_global;
                let mut px = [
                    (v[0] * gain).clamp(0.0, 255.0) as u8,
                    (v[1] * gain).clamp(0.0, 255.0) as u8,
                    (v[2] * gain).clamp(0.0, 255.0) as u8,
                ];
                if dev.bgr {
                    px.swap(0, 2);
                }
                *px_out = px;
            }
            if dev.reverse {
                out.reverse();
            }
            if self.last.len() <= di {
                self.last.resize(di + 1, Vec::new());
            }
            if let Some(st) = self.stats.as_mut() {
                if let Some(prev) = self.last.get(di).filter(|p| p.len() == out.len()) {
                    let d: f64 = prev
                        .iter()
                        .zip(&out)
                        .map(|(a, b)| {
                            (0..3)
                                .map(|k| (a[k] as f64 - b[k] as f64).abs())
                                .sum::<f64>()
                        })
                        .sum();
                    st.sum += d / (out.len().max(1) * 3) as f64;
                    st.n += 1;
                }
            }
            if self.due(di, dev, s.target_fps) {
                self.send(di, dev, &out);
            }
            self.last[di] = out;
        }

        if let Some(st) = self.stats.as_mut() {
            if st.t.elapsed().as_secs_f32() >= 2.0 {
                eprintln!(
                    "dt {:.1} ms  change {:.2}/frame  out {:.0} fps  sample {} µs  devices {}",
                    dt * 1000.0,
                    st.sum / st.n.max(1) as f64,
                    self.send_fps,
                    self.last_sample_us,
                    self.plan.devices.len(),
                );
                st.sum = 0.0;
                st.n = 0;
                st.t = Instant::now();
            }
        }
        self.update_fps();
    }

    fn update_fps(&mut self) {
        let el = self.fps_window.0.elapsed().as_secs_f32();
        if el > 1.0 {
            let prev = &self.fps_window.1;
            let best = self
                .sent
                .iter()
                .enumerate()
                .map(|(i, n)| n - prev.get(i).copied().unwrap_or(0).min(*n))
                .max()
                .unwrap_or(0);
            self.send_fps = best as f32 / el;
            self.fps_window = (Instant::now(), self.sent.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DeviceCfg, Zone, ZonePreset};
    use std::net::UdpSocket;
    use std::sync::Arc;

    fn setup(leds: u16) -> Config {
        let mut cfg = Config::default();
        // TEST-NET address, nothing listens there
        cfg.devices.push(DeviceCfg::new("192.0.2.1", "t", leds));
        cfg.zones.push(Zone::preset(
            ZonePreset::Full,
            "192.0.2.1".into(),
            leds,
            [0, 0, 0],
        ));
        cfg.settings.smoothing_ms = 0.0;
        cfg
    }

    fn solid(w: u32, h: u32, rgb: [u8; 3], seq: u64) -> SharedFrame {
        let mut data = Vec::with_capacity((w * h * 3) as usize);
        for _ in 0..w * h {
            data.extend_from_slice(&rgb);
        }
        SharedFrame {
            w,
            h,
            data: Arc::new(data),
            seq,
        }
    }

    #[test]
    fn passes_colour_through_without_smoothing() {
        let cfg = setup(8);
        let mut e = Engine::new(&cfg, 32, 18);
        e.tick(&cfg, Some(&solid(32, 18, [200, 100, 50], 1)), 0.02);
        assert_eq!(e.last[0][0], [200, 100, 50]);
        assert_eq!(e.last[0].len(), 8);
    }

    #[test]
    fn median_rejects_a_single_spike() {
        let mut cfg = setup(4);
        cfg.settings.smoothing_ms = 1.0; // enables the median, nearly no lag
        cfg.settings.slew = 0.0;
        let mut e = Engine::new(&cfg, 32, 18);
        for seq in 1..=3 {
            e.tick(&cfg, Some(&solid(32, 18, [10, 10, 10], seq)), 1.0);
        }
        e.tick(&cfg, Some(&solid(32, 18, [250, 250, 250], 4)), 1.0);
        assert!(e.last[0][0][0] < 20, "spike leaked: {:?}", e.last[0][0]);
    }

    #[test]
    fn bgr_and_reverse() {
        let mut cfg = setup(2);
        cfg.devices[0].bgr = true;
        let mut e = Engine::new(&cfg, 32, 18);
        e.tick(&cfg, Some(&solid(32, 18, [1, 2, 3], 1)), 0.02);
        assert_eq!(e.last[0][0], [3, 2, 1]);
    }

    #[test]
    fn saturation_zero_is_grey() {
        let mut cfg = setup(1);
        cfg.settings.saturation = 0.0;
        let mut e = Engine::new(&cfg, 32, 18);
        e.tick(&cfg, Some(&solid(32, 18, [200, 0, 0], 1)), 0.02);
        let c = e.last[0][0];
        assert_eq!(c[0], c[1]);
        assert_eq!(c[1], c[2]);
    }

    #[test]
    fn median3_works() {
        assert_eq!(median3(1.0, 5.0, 3.0), 3.0);
        assert_eq!(median3(9.0, 1.0, 1.0), 1.0);
    }

    // ---------------------------------------------------------------- on the wire
    //
    // These tests run the whole chain - zone layout -> sampler -> smoothing ->
    // DDP - against a socket that plays WLED. They check what actually arrives
    // at a strip, not what one function in the middle thinks.

    /// A WLED stand-in: receives DDP and rebuilds the LED buffer.
    struct FakeStrip {
        sock: UdpSocket,
        len: usize,
    }

    impl FakeStrip {
        fn new(len: usize) -> Self {
            let sock = UdpSocket::bind("127.0.0.1:0").expect("bind a test socket");
            sock.set_read_timeout(Some(Duration::from_millis(300)))
                .unwrap();
            Self { sock, len }
        }

        fn host(&self) -> String {
            format!("127.0.0.1:{}", self.sock.local_addr().unwrap().port())
        }

        /// Wait for one complete DDP frame (the packet with PUSH set).
        fn frame(&mut self) -> Vec<[u8; 3]> {
            let mut leds = vec![[0u8; 3]; self.len];
            let mut buf = [0u8; 4096];
            let deadline = Instant::now() + Duration::from_secs(3);
            while Instant::now() < deadline {
                let Ok(n) = self.sock.recv(&mut buf) else {
                    break; // timeout: no more packets
                };
                if n < 10 {
                    continue;
                }
                let push = buf[0] & 0x01 != 0;
                let first = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]) as usize / 3;
                let count = u16::from_be_bytes([buf[8], buf[9]]) as usize / 3;
                for i in 0..count {
                    let o = 10 + i * 3;
                    if o + 2 < n && first + i < self.len {
                        leds[first + i] = [buf[o], buf[o + 1], buf[o + 2]];
                    }
                }
                if push {
                    break;
                }
            }
            leds
        }
    }

    /// A picture whose red channel runs left to right and whose green channel
    /// runs top to bottom, so a LED can be traced back to the pixels it read.
    /// 128x72 keeps both values inside a `u8`.
    fn ramp(w: u32, h: u32, seq: u64) -> SharedFrame {
        let mut data = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                data.push(x as u8);
                data.push(y as u8);
                data.push(0);
            }
        }
        SharedFrame {
            w,
            h,
            data: Arc::new(data),
            seq,
        }
    }

    /// Left half white, right half black.
    fn halves(w: u32, h: u32, seq: u64) -> SharedFrame {
        let mut data = Vec::with_capacity((w * h * 3) as usize);
        for _ in 0..h {
            for x in 0..w {
                let v = if x < w / 2 { 255 } else { 0 };
                data.extend_from_slice(&[v, v, v]);
            }
        }
        SharedFrame {
            w,
            h,
            data: Arc::new(data),
            seq,
        }
    }

    /// A config that passes colours through untouched and sends to `host`.
    fn net_cfg(host: &str, leds: u16) -> Config {
        let mut cfg = Config::default();
        cfg.devices.push(DeviceCfg::new(host, "Strip", leds));
        cfg.settings.smoothing_ms = 0.0;
        cfg.settings.spatial = 0.0;
        cfg.settings.slew = 0.0;
        cfg.settings.gamma = 1.0;
        cfg.settings.saturation = 1.0;
        cfg.settings.brightness = 1.0;
        cfg
    }

    fn mean(v: &[[u8; 3]], channel: usize) -> f32 {
        if v.is_empty() {
            return 0.0;
        }
        v.iter().map(|c| c[channel] as f32).sum::<f32>() / v.len() as f32
    }

    /// A full-saturation rainbow across the width: every column its own hue.
    fn rainbow(w: u32, h: u32, seq: u64) -> SharedFrame {
        let mut data = Vec::with_capacity((w * h * 3) as usize);
        for _y in 0..h {
            for x in 0..w {
                let hue = x as f32 / w as f32 * 6.0;
                let i = hue.floor() as i32;
                let f = hue - i as f32;
                let q = 255.0 * (1.0 - f);
                let t = 255.0 * f;
                let c = match i.rem_euclid(6) {
                    0 => [255.0, t, 0.0],
                    1 => [q, 255.0, 0.0],
                    2 => [0.0, 255.0, t],
                    3 => [0.0, q, 255.0],
                    4 => [t, 0.0, 255.0],
                    _ => [255.0, 0.0, q],
                };
                data.extend_from_slice(&[c[0] as u8, c[1] as u8, c[2] as u8]);
            }
        }
        SharedFrame {
            w,
            h,
            data: Arc::new(data),
            seq,
        }
    }

    /// HSV saturation of an arriving colour, 0..1.
    fn saturation(c: [u8; 3]) -> f32 {
        let max = c[0].max(c[1]).max(c[2]) as f32;
        let min = c[0].min(c[1]).min(c[2]) as f32;
        if max <= 0.0 {
            0.0
        } else {
            (max - min) / max
        }
    }

    #[test]
    fn a_rainbow_survives_the_pipeline_with_its_colours() {
        // The pipeline must not wash out what it reads: averaging, the spatial
        // blend, smoothing and the grading may shift brightness, but a fully
        // saturated rainbow has to arrive saturated.
        for spatial in [0.0f32, 0.5, 0.6] {
            let mut cfg = setup(120);
            cfg.settings.smoothing_ms = 120.0;
            cfg.settings.spatial = spatial;
            cfg.settings.slew = 900.0;
            let mut e = Engine::new(&cfg, 120, 68);
            // let the smoothing settle, then look at the frame
            for seq in 1..=12 {
                e.tick(&cfg, Some(&rainbow(120, 68, seq)), 0.02);
            }
            let out = &e.last[0];
            assert_eq!(out.len(), 120);
            let mean: f32 = out.iter().map(|c| saturation(*c)).sum::<f32>() / out.len() as f32;
            eprintln!(
                "spatial {spatial:.1}: mittlere Sättigung {mean:.3}, hellster Kanal {}",
                out.iter().map(|c| *c.iter().max().unwrap()).max().unwrap()
            );
            assert!(
                mean > 0.8,
                "spatial {spatial}: the LEDs arrive washed out (mean saturation {mean:.2})"
            );
            // and the brightness is not silently cut in half
            let brightest = out.iter().map(|c| *c.iter().max().unwrap()).max().unwrap();
            assert!(
                brightest > 230,
                "spatial {spatial}: no LED is bright ({brightest})"
            );
        }
    }

    #[test]
    fn a_pinned_led_block_lands_on_exactly_those_leds() {
        let mut strip = FakeStrip::new(120);
        let host = strip.host();
        let mut cfg = net_cfg(&host, 120);
        let mut zone = Zone::preset(ZonePreset::Full, host.clone(), 20, [0; 3]);
        zone.offset = Some(50);
        cfg.zones.push(zone);

        let mut e = Engine::new(&cfg, 128, 72);
        e.tick(&cfg, Some(&ramp(128, 72, 1)), 0.02);
        let frame = strip.frame();

        assert_eq!(frame.len(), 120, "the strip gets all its LEDs");
        assert!(
            frame[..50].iter().all(|c| *c == [0, 0, 0]),
            "LEDs before the block stay dark: {:?}",
            &frame[45..50]
        );
        assert!(
            frame[70..].iter().all(|c| *c == [0, 0, 0]),
            "LEDs after the block stay dark: {:?}",
            &frame[70..75]
        );
        // inside the block the picture is read left to right
        for k in 1..20 {
            assert!(
                frame[50 + k][0] > frame[49 + k][0],
                "LED {} must be further right than LED {}: {:?}",
                k,
                k - 1,
                &frame[50..70]
            );
        }
    }

    #[test]
    fn two_zones_share_the_strip_in_list_order() {
        let mut strip = FakeStrip::new(120);
        let host = strip.host();
        let mut cfg = net_cfg(&host, 120);
        // zone 1 reads the left half, zone 2 the right half
        let mut left = Zone::preset(ZonePreset::Full, host.clone(), 40, [0; 3]);
        left.x = 0.0;
        left.w = 0.5;
        let mut right = Zone::preset(ZonePreset::Full, host.clone(), 30, [0; 3]);
        right.x = 0.5;
        right.w = 0.5;
        cfg.zones.push(left);
        cfg.zones.push(right);
        assert_eq!(cfg.zone_ranges(), vec![(0, 40), (40, 70)]);

        let mut e = Engine::new(&cfg, 128, 72);
        e.tick(&cfg, Some(&halves(128, 72, 1)), 0.02);
        let frame = strip.frame();
        assert!(
            frame[..40].iter().all(|c| *c == [255, 255, 255]),
            "the first zone owns LED 0..39: {:?}",
            &frame[..4]
        );
        assert!(
            frame[40..70].iter().all(|c| *c == [0, 0, 0]),
            "the second zone starts at LED 40: {:?}",
            &frame[40..44]
        );
    }

    #[test]
    fn splitting_a_zone_lights_the_whole_strip_clockwise() {
        let mut strip = FakeStrip::new(120);
        let host = strip.host();
        let mut cfg = net_cfg(&host, 120);
        cfg.zones
            .push(Zone::preset(ZonePreset::Full, host.clone(), 120, [0; 3]));

        let counts = cfg.split_zone_into_edges(0, 9.0 / 16.0).unwrap();
        assert_eq!(counts.iter().map(|c| *c as u32).sum::<u32>(), 120);
        // four zones, one after the other, no gap and no overlap
        let ranges = cfg.zone_ranges();
        assert_eq!(ranges.len(), 4);
        let mut expected = 0u32;
        for (i, (start, end)) in ranges.iter().enumerate() {
            assert_eq!(*start, expected, "zone {i} must start where {i} - 1 ended");
            assert_eq!(*end, expected + counts[i] as u32);
            expected = *end;
        }
        assert_eq!(expected, 120, "the four edges cover the whole strip");
        assert!(cfg.zones[1].vertical && cfg.zones[3].vertical);

        let mut e = Engine::new(&cfg, 128, 72);
        e.tick(&cfg, Some(&ramp(128, 72, 1)), 0.02);
        let frame = strip.frame();

        let top = &frame[0..counts[0] as usize];
        let right = &frame[counts[0] as usize..(counts[0] + counts[1]) as usize];
        let bottom =
            &frame[(counts[0] + counts[1]) as usize..(counts[0] + counts[1] + counts[2]) as usize];
        let left = &frame[(120 - counts[3]) as usize..];

        // top and bottom run left -> right, top sits at the top of the picture
        assert!(top[1][0] > top[0][0] && bottom[1][0] > bottom[0][0]);
        assert!(mean(top, 1) < mean(bottom, 1), "top rows over bottom rows");
        // left and right run top -> bottom
        assert!(right[1][1] > right[0][1], "right edge runs downwards");
        assert!(left[1][1] > left[0][1], "left edge runs downwards");
        assert!(
            mean(right, 0) > mean(left, 0),
            "right columns right of left"
        );
    }

    #[test]
    fn reverse_mirrors_the_whole_strip() {
        let build = |reverse: bool| {
            let strip = FakeStrip::new(8);
            let host = strip.host();
            let mut cfg = net_cfg(&host, 8);
            cfg.devices[0].reverse = reverse;
            cfg.zones
                .push(Zone::preset(ZonePreset::Full, host.clone(), 8, [0; 3]));
            let mut e = Engine::new(&cfg, 128, 72);
            e.tick(&cfg, Some(&ramp(128, 72, 1)), 0.02);
            let mut strip = strip;
            strip.frame()
        };
        let forward = build(false);
        let mut mirrored = build(true);
        mirrored.reverse();
        assert_eq!(forward, mirrored);
        assert!(forward[0][0] < forward[7][0], "LED 0 is the left end");
    }

    #[test]
    fn a_zone_that_does_not_fit_does_not_move_the_others() {
        let mut strip = FakeStrip::new(10);
        let host = strip.host();
        let mut cfg = net_cfg(&host, 10);
        // zone 1 owns LED 0..7 and reads the left half of the picture
        let mut left = Zone::preset(ZonePreset::Full, host.clone(), 8, [0; 3]);
        left.x = 0.0;
        left.w = 0.5;
        cfg.zones.push(left);
        // zone 2 is pinned at LED 8 and claims five LEDs - two of them do not exist
        let mut right = Zone::preset(ZonePreset::Full, host.clone(), 5, [0; 3]);
        right.x = 0.5;
        right.w = 0.5;
        right.offset = Some(8);
        cfg.zones.push(right);
        assert_eq!(cfg.zone_ranges(), vec![(0, 8), (8, 13)]);
        assert!(cfg.zone_report(1).unwrap().over);

        let mut e = Engine::new(&cfg, 128, 72);
        e.tick(&cfg, Some(&halves(128, 72, 1)), 0.02);
        let frame = strip.frame();
        assert_eq!(frame.len(), 10, "the packet never grows past the device");
        assert!(
            frame[..8].iter().all(|c| *c == [255, 255, 255]),
            "the first zone keeps LED 0..7: {:?}",
            &frame[..4]
        );
        assert!(
            frame[8..].iter().all(|c| *c == [0, 0, 0]),
            "the overflowing zone still drives the LEDs that exist: {:?}",
            &frame[8..]
        );
    }
}
