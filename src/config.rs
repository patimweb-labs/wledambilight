// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Patrick Weppelmann

//! Persistent configuration: devices, zones and global settings.
//!
//! Stored as pretty JSON in the platform config directory:
//! - Linux:   `$XDG_CONFIG_HOME/wled-ambilight/config.json` (usually `~/.config/…`)
//! - macOS:   `~/Library/Application Support/wled-ambilight/config.json`
//! - Windows: `%APPDATA%\wled-ambilight\config.json`
//!
//! `--config <file>` overrides the location.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Protocol {
    /// DDP on UDP 4048 - best for many LEDs, what WLED prefers for realtime.
    #[default]
    Ddp,
    /// WLED UDP realtime protocol (DRGB) on UDP 21324, limited to 490 LEDs.
    Drgb,
}

/// What kind of device a [`DeviceCfg`] talks to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceKind {
    /// WLED over DDP or DRGB
    #[default]
    Wled,
    /// one controller of an OpenRGB SDK server (PC peripherals, RAM, fans, …)
    OpenRgb,
    /// Razer Chroma via the local REST API of Razer Synapse (Windows)
    Razer,
    /// Govee LAN device in realtime ("razer"/DreamView) mode
    Govee,
    /// Nanoleaf panels via External Control v2
    Nanoleaf,
}

impl DeviceKind {
    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Wled => "WLED",
            DeviceKind::OpenRgb => "OpenRGB",
            DeviceKind::Razer => "Razer Chroma",
            DeviceKind::Govee => "Govee",
            DeviceKind::Nanoleaf => "Nanoleaf",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceCfg {
    pub id: String,
    pub name: String,
    pub host: String,
    /// LEDs, segments (Govee), panels (Nanoleaf) or columns (Razer)
    pub led_count: u16,
    pub enabled: bool,
    /// WLED only
    pub protocol: Protocol,
    #[serde(default)]
    pub kind: DeviceKind,
    /// OpenRGB: TCP port of the SDK server (0 = 6742)
    #[serde(default)]
    pub port: u16,
    /// OpenRGB: "controller name@location"; Govee: model (SKU)
    #[serde(default)]
    pub target: String,
    /// Nanoleaf: auth token from pairing
    #[serde(default)]
    pub token: Option<String>,
    /// 0..1, scales everything sent to this device
    pub brightness: f32,
    /// physical strip is wired the other way round
    pub reverse: bool,
    /// BGR instead of RGB (some clones)
    pub bgr: bool,
    /// cap the update rate of this device, 0 = use the global rate
    #[serde(default)]
    pub max_fps: u32,
}

impl DeviceCfg {
    pub fn new(host: impl Into<String>, name: impl Into<String>, leds: u16) -> Self {
        let host = host.into();
        Self {
            id: host.clone(),
            name: name.into(),
            host,
            led_count: leds.max(1),
            enabled: true,
            protocol: Protocol::Ddp,
            brightness: 1.0,
            reverse: false,
            bgr: false,
            max_fps: 0,
            kind: DeviceKind::Wled,
            port: 0,
            target: String::new(),
            token: None,
        }
    }

    /// A device of another kind. `id` must be unique across all devices.
    pub fn of_kind(
        kind: DeviceKind,
        id: impl Into<String>,
        host: impl Into<String>,
        name: impl Into<String>,
        leds: u16,
    ) -> Self {
        let mut d = Self::new(host, name, leds);
        d.id = id.into();
        d.kind = kind;
        d
    }

    /// Everything that requires a new connection when it changes.
    pub fn connection_key(&self) -> String {
        format!(
            "{:?}|{}|{}|{:?}|{}|{}|{:?}",
            self.kind, self.host, self.port, self.protocol, self.target, self.led_count, self.token
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    /// DeviceCfg::id
    pub device: String,
    /// normalised screen coordinates 0..1 - independent of any resolution
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    /// how many LEDs this zone drives
    pub count: u16,
    /// true: LEDs run top -> bottom, false: left -> right
    pub vertical: bool,
    /// per zone gain
    pub gain: f32,
    pub enabled: bool,
    /// start index inside the device; None = packed after the previous zone of that device
    pub offset: Option<u16>,
    pub color: [u8; 3],
}

impl Zone {
    pub fn preset(kind: ZonePreset, device: String, count: u16, color: [u8; 3]) -> Self {
        let (name, x, y, w, h, vertical) = match kind {
            ZonePreset::Top => ("Top", 0.0, 0.0, 1.0, 0.10, false),
            ZonePreset::Bottom => ("Bottom", 0.0, 0.90, 1.0, 0.10, false),
            ZonePreset::Left => ("Left", 0.0, 0.0, 0.10, 1.0, true),
            ZonePreset::Right => ("Right", 0.90, 0.0, 0.10, 1.0, true),
            ZonePreset::Full => ("Full", 0.0, 0.0, 1.0, 1.0, false),
            ZonePreset::Custom => ("Zone", 0.25, 0.25, 0.5, 0.25, false),
        };
        Self {
            name: name.to_string(),
            device,
            x,
            y,
            w,
            h,
            count: count.max(1),
            vertical,
            gain: 1.0,
            enabled: true,
            offset: None,
            color,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZonePreset {
    Top,
    Bottom,
    Left,
    Right,
    /// whole picture (used by tests and the sampler checks)
    #[allow(dead_code)]
    Full,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// frames per second we sample and send
    pub target_fps: u32,
    /// width of the capture buffer, height follows the screen aspect
    pub capture_width: u32,
    /// temporal smoothing in ms, 0 = off
    pub smoothing_ms: f32,
    /// 0..1 blur along the strip, 0 = every LED on its own
    pub spatial: f32,
    /// max change per second (0..255 scale), 0 = unlimited
    pub slew: f32,
    /// freeze the LEDs while this window is focused (it would capture itself)
    pub pause_when_focused: bool,
    pub gamma: f32,
    pub saturation: f32,
    pub brightness: f32,
    /// Linux only: portal restore token so the capture starts without a dialog
    pub restore_token: Option<String>,
    /// show the built-in test picture instead of the screen: no capture
    /// permission needed, and a good way to check the strip and the zones
    pub test_image: bool,
    pub snap: bool,
    /// Windows/macOS: index of the captured monitor (0 = primary)
    pub monitor: usize,
    /// "auto", "de" or "en"
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            target_fps: 50,
            capture_width: 320,
            smoothing_ms: 180.0,
            spatial: 0.6,
            slew: 900.0,
            pause_when_focused: false,
            gamma: 1.0,
            saturation: 1.0,
            brightness: 1.0,
            restore_token: None,
            test_image: false,
            snap: true,
            monitor: 0,
            language: "auto".into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub devices: Vec<DeviceCfg>,
    pub zones: Vec<Zone>,
    pub settings: Settings,
}

static PATH_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// Use a different config file (`--config <file>`). Call before anything loads.
pub fn set_config_path(path: PathBuf) {
    let _ = PATH_OVERRIDE.set(path);
}

pub fn config_path() -> PathBuf {
    if let Some(p) = PATH_OVERRIDE.get() {
        return p.clone();
    }
    // dirs::config_dir() is $XDG_CONFIG_HOME or ~/.config on Linux, so existing
    // installations keep their file.
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("wled-ambilight")
        .join("config.json")
}

impl Config {
    pub fn load() -> Self {
        Self::load_path(&config_path())
    }

    /// Load from an explicit file, falling back to the backup next to it.
    pub fn load_path(path: &std::path::Path) -> Self {
        let backup = path.with_extension("json.bak");
        match Self::read(path) {
            Ok(Some(cfg)) => cfg,
            Ok(None) => Config::default(),
            Err(e) => {
                // Never silently start over: keep the broken file, then fall
                // back to the copy written before the last save.
                eprintln!("config {} is broken ({e})", path.display());
                let broken = path.with_extension("json.broken");
                if std::fs::rename(path, &broken).is_ok() {
                    eprintln!("kept it as {}", broken.display());
                }
                match Self::read(&backup) {
                    Ok(Some(cfg)) => {
                        eprintln!("using the backup {}", backup.display());
                        cfg
                    }
                    _ => Config::default(),
                }
            }
        }
    }

    /// `Ok(None)` = no file (first start), `Err` = file exists but is unreadable.
    fn read(path: &std::path::Path) -> Result<Option<Self>, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_json(&text).map(Some).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let mut v: serde_json::Value = serde_json::from_str(text)?;
        // Zones used to carry flip_h/flip_v. A reversed strip is a property of the
        // device, so fold the old flags into it (safe for the one-zone case).
        let mut flip_devices: Vec<String> = Vec::new();
        if let Some(zones) = v.get_mut("zones").and_then(|z| z.as_array_mut()) {
            for z in zones.iter_mut() {
                let flipped = z.get("flip_h").and_then(|b| b.as_bool()).unwrap_or(false)
                    || z.get("flip_v").and_then(|b| b.as_bool()).unwrap_or(false)
                    || z.get("reverse").and_then(|b| b.as_bool()).unwrap_or(false);
                if flipped {
                    if let Some(dev) = z.get("device").and_then(|d| d.as_str()) {
                        flip_devices.push(dev.to_string());
                    }
                }
                if let Some(o) = z.as_object_mut() {
                    o.remove("flip_h");
                    o.remove("flip_v");
                    o.remove("reverse");
                }
            }
        }
        if let Some(devices) = v.get_mut("devices").and_then(|d| d.as_array_mut()) {
            for d in devices {
                let id = d
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();
                if flip_devices.contains(&id) {
                    d["reverse"] = serde_json::Value::Bool(true);
                }
            }
        }
        serde_json::from_value(v)
    }

    /// Write the config atomically (temp file + rename), so a crash or a full
    /// disk can never leave a half written file behind.
    pub fn save(&self) {
        let path = config_path();
        if let Err(e) = self.save_to(&path) {
            eprintln!("could not write {}: {e}", path.display());
        }
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        {
            use std::io::Write;
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?; // survive a power cut, not just a crash of the app
        }
        // keep the last good file, so a broken one can be replaced by it
        if path.exists() {
            let _ = std::fs::copy(path, path.with_extension("json.bak"));
        }
        std::fs::rename(&tmp, path)
    }

    pub fn device(&self, id: &str) -> Option<&DeviceCfg> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Start index of every zone inside its device, one entry per zone in
    /// config order. A zone without an explicit `offset` is packed after the
    /// previous enabled zone of the same device.
    ///
    /// This is the only place that defines the packing: the sampler and every
    /// LED number in the interface come from here, so what the window shows is
    /// exactly what goes out on the wire.
    pub fn zone_starts(&self) -> Vec<u32> {
        let mut used: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        let mut out = Vec::with_capacity(self.zones.len());
        for z in &self.zones {
            let start = z
                .offset
                .map(u32::from)
                .unwrap_or_else(|| *used.get(z.device.as_str()).unwrap_or(&0));
            if z.enabled {
                used.insert(z.device.as_str(), start + z.count as u32);
            }
            out.push(start);
        }
        out
    }

    /// Which LEDs of which device every zone drives, in config order.
    pub fn zone_ranges(&self) -> Vec<(u32, u32)> {
        self.zone_starts()
            .into_iter()
            .zip(&self.zones)
            .map(|(start, z)| (start, start + z.count as u32))
            .collect()
    }

    /// LEDs of `device` that an enabled zone drives. LEDs claimed twice count
    /// once, so this is the number of LEDs that actually light up.
    pub fn used_leds(&self, device_id: &str, skip: Option<usize>) -> u16 {
        let Some(dev) = self.device(device_id) else {
            return 0;
        };
        let n = dev.led_count as usize;
        let mut taken = vec![false; n];
        for (i, (start, end)) in self.zone_ranges().into_iter().enumerate() {
            let z = &self.zones[i];
            if !z.enabled || z.device != device_id || Some(i) == skip {
                continue;
            }
            for led in start..end.min(n as u32) {
                taken[led as usize] = true;
            }
        }
        taken.iter().filter(|t| **t).count() as u16
    }

    /// First start index at which `count` LEDs of `device` are all free.
    pub fn free_start(&self, device_id: &str, count: u16) -> Option<u16> {
        let dev = self.device(device_id)?;
        let n = dev.led_count as usize;
        let mut taken = vec![false; n];
        for (i, (start, end)) in self.zone_ranges().into_iter().enumerate() {
            let z = &self.zones[i];
            if !z.enabled || z.device != device_id {
                continue;
            }
            for led in start..end.min(n as u32) {
                taken[led as usize] = true;
            }
        }
        let want = count.max(1) as usize;
        for start in 0..n {
            if start + want > n {
                break;
            }
            if taken[start..start + want].iter().all(|t| !t) {
                return Some(start as u16);
            }
        }
        None
    }

    /// Replace zone `i` by four edge zones (top, right, bottom, left) and
    /// share its LEDs out over them by edge length. `aspect` is
    /// height/width of the screen. Returns the four LED counts, or `None` when
    /// the zone has fewer than four LEDs (four edges need at least one each).
    ///
    /// Only this zone is replaced: other zones of the device keep their block,
    /// and a zone with an explicit start hands the block on to the four edges.
    pub fn split_zone_into_edges(&mut self, i: usize, aspect: f32) -> Option<[u16; 4]> {
        let zone = self.zones.get(i)?.clone();
        if zone.count < 4 {
            return None;
        }
        let counts = split_counts(zone.count, aspect);
        let kinds = [
            ZonePreset::Top,
            ZonePreset::Right,
            ZonePreset::Bottom,
            ZonePreset::Left,
        ];
        let mut next = zone.offset.map(u32::from);
        let mut zones: Vec<Zone> = Vec::with_capacity(4);
        for (k, kind) in kinds.into_iter().enumerate() {
            let mut z = Zone::preset(kind, zone.device.clone(), counts[k], zone.color);
            z.gain = zone.gain;
            z.enabled = zone.enabled;
            z.vertical = kind == ZonePreset::Left || kind == ZonePreset::Right;
            if let Some(start) = next.as_mut() {
                z.offset = Some(*start as u16);
                *start += counts[k] as u32;
            }
            zones.push(z);
        }
        self.zones.splice(i..=i, zones);
        Some(counts)
    }

    /// Where a zone would start if its LED block were not pinned (`offset`).
    pub fn zone_start_if_auto(&self, i: usize) -> u32 {
        let Some(z) = self.zones.get(i) else {
            return 0;
        };
        let mut used = 0u32;
        for prev in self.zones.iter().take(i) {
            if prev.device != z.device || !prev.enabled {
                continue;
            }
            used = prev.offset.map(u32::from).unwrap_or(used) + prev.count as u32;
        }
        used
    }

    /// LEDs of `device` that light up, and whether the layout is broken
    /// (zones overlapping each other or reaching past the end of the device).
    pub fn device_usage(&self, device_id: &str) -> DeviceUsage {
        let leds = self
            .device(device_id)
            .map(|d| d.led_count as u32)
            .unwrap_or(0) as usize;
        let mut taken = vec![false; leds];
        let mut claimed = 0i64;
        let mut overrun = false;
        for (i, (start, end)) in self.zone_ranges().into_iter().enumerate() {
            let z = &self.zones[i];
            if !z.enabled || z.device != device_id {
                continue;
            }
            claimed += (end as i64) - (start as i64);
            if end > leds as u32 {
                overrun = true;
            }
            for led in start..end.min(leds as u32) {
                taken[led as usize] = true;
            }
        }
        let used = taken.iter().filter(|t| **t).count() as u16;
        DeviceUsage {
            used,
            problem: overrun || claimed != used as i64,
        }
    }

    /// What a zone drives and where it clashes - everything the interface
    /// needs to show a zone's LED block and mark it when it does not fit.
    pub fn zone_report(&self, i: usize) -> Option<ZoneReport> {
        let z = self.zones.get(i)?;
        let ranges = self.zone_ranges();
        let (start, end) = *ranges.get(i)?;
        let leds = self
            .device(&z.device)
            .map(|d| d.led_count as u32)
            .unwrap_or(0);
        let mut overlap = None;
        if z.enabled {
            for (j, (o_start, o_end)) in ranges.iter().enumerate() {
                let other = &self.zones[j];
                if j == i || !other.enabled || other.device != z.device {
                    continue;
                }
                let lo = start.max(*o_start);
                let hi = end.min(*o_end);
                if lo < hi {
                    overlap = Some((j, lo, hi));
                    break;
                }
            }
        }
        Some(ZoneReport {
            start,
            end,
            over: end > leds,
            overlap,
        })
    }
}

/// How much of a device is lit and whether the zone layout is broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeviceUsage {
    /// LEDs that actually light up (overlaps counted once)
    pub used: u16,
    /// two zones claim the same LEDs, or a zone reaches past the last LED
    pub problem: bool,
}

/// Where one zone sits on its strip, and whether that clashes with the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneReport {
    /// first LED index the zone drives (0-based)
    pub start: u32,
    /// one past the last LED index
    pub end: u32,
    /// the zone reaches past the end of the device: those LEDs stay dark
    pub over: bool,
    /// another enabled zone of the same device claims the same LEDs
    /// (index of that zone, first and last LED index it shares)
    pub overlap: Option<(usize, u32, u32)>,
}

/// Share `total` LEDs over top, right, bottom, left by edge length.
///
/// Every edge gets at least one LED, the rest is split by edge length with the
/// largest remainder, so the four counts always add up to exactly `total`
/// (for `total >= 4`; below that one LED per edge is returned).
pub fn split_counts(total: u16, ratio: f32) -> [u16; 4] {
    let total = total.max(1) as i32;
    let ratio = if ratio.is_finite() && ratio > 0.0 {
        ratio.clamp(0.1, 10.0)
    } else {
        9.0 / 16.0
    };
    let lens = [1.0f32, ratio, 1.0, ratio];
    let sum: f32 = lens.iter().sum();
    if total < 4 {
        return [1; 4];
    }
    let free = total - 4;
    let mut counts = [1i32; 4];
    let mut exact = [0f32; 4];
    let mut rest = free;
    for (i, l) in lens.iter().enumerate() {
        exact[i] = l / sum * free as f32;
        let take = exact[i].floor() as i32;
        counts[i] += take;
        rest -= take;
    }
    while rest > 0 {
        let mut best = 0;
        for i in 1..4 {
            if exact[i] - (counts[i] - 1) as f32 > exact[best] - (counts[best] - 1) as f32 {
                best = i;
            }
        }
        counts[best] += 1;
        rest -= 1;
    }
    counts.map(|c| c.max(1) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_zone_flip_flags_move_to_the_device() {
        let json = r#"{
            "devices": [{"id":"a","name":"A","host":"a","led_count":10,"enabled":true,
                         "protocol":"Ddp","brightness":1.0,"reverse":false,"bgr":false}],
            "zones": [{"name":"Z","device":"a","x":0,"y":0,"w":1,"h":0.1,"count":10,
                       "vertical":false,"gain":1,"enabled":true,"offset":null,
                       "color":[1,2,3],"flip_h":true}]
        }"#;
        let cfg = Config::from_json(json).unwrap();
        assert!(cfg.devices[0].reverse);
        assert_eq!(cfg.devices[0].max_fps, 0);
        assert_eq!(cfg.settings.language, "auto");
        // configs from 0.2.0 have no kind: they are WLED devices
        assert_eq!(cfg.devices[0].kind, DeviceKind::Wled);
        assert_eq!(cfg.devices[0].token, None);
    }

    #[test]
    fn zones_pack_after_each_other() {
        let mut cfg = Config::default();
        cfg.devices.push(DeviceCfg::new("h", "d", 100));
        for kind in [ZonePreset::Top, ZonePreset::Right] {
            cfg.zones
                .push(Zone::preset(kind, "h".into(), 30, [0, 0, 0]));
        }
        assert_eq!(cfg.zone_ranges(), vec![(0, 30), (30, 60)]);
        assert_eq!(cfg.used_leds("h", None), 60);
        assert_eq!(cfg.used_leds("h", Some(0)), 30);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("wled-ambilight-test-{}", std::process::id()));
        let path = dir.join("config.json");
        let mut cfg = Config::default();
        cfg.devices.push(DeviceCfg::new("10.0.0.2", "Desk", 42));
        cfg.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(Config::from_json(&text).unwrap(), cfg);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn cfg_120() -> Config {
        let mut cfg = Config::default();
        cfg.devices.push(DeviceCfg::new("127.0.0.1", "Strip", 120));
        cfg
    }

    #[test]
    fn an_explicit_offset_pins_the_zone_and_the_next_one_continues_after_it() {
        let mut cfg = cfg_120();
        cfg.zones.push(Zone::preset(
            ZonePreset::Top,
            "127.0.0.1".into(),
            10,
            [0; 3],
        ));
        let mut mid = Zone::preset(ZonePreset::Custom, "127.0.0.1".into(), 20, [0; 3]);
        mid.offset = Some(50);
        cfg.zones.push(mid);
        cfg.zones.push(Zone::preset(
            ZonePreset::Bottom,
            "127.0.0.1".into(),
            5,
            [0; 3],
        ));
        // packed 0..10, pinned 50..70, then packed after the pinned block
        assert_eq!(cfg.zone_ranges(), vec![(0, 10), (50, 70), (70, 75)]);
        assert_eq!(cfg.zone_starts(), vec![0, 50, 70]);
    }

    #[test]
    fn a_disabled_zone_does_not_move_the_others() {
        let mut cfg = cfg_120();
        cfg.zones.push(Zone::preset(
            ZonePreset::Top,
            "127.0.0.1".into(),
            10,
            [0; 3],
        ));
        let mut off = Zone::preset(ZonePreset::Custom, "127.0.0.1".into(), 30, [0; 3]);
        off.enabled = false;
        cfg.zones.push(off);
        cfg.zones.push(Zone::preset(
            ZonePreset::Bottom,
            "127.0.0.1".into(),
            5,
            [0; 3],
        ));
        assert_eq!(cfg.zone_ranges(), vec![(0, 10), (10, 40), (10, 15)]);
        assert_eq!(cfg.used_leds("127.0.0.1", None), 15);
    }

    #[test]
    fn used_leds_counts_overlaps_once_and_respects_offsets() {
        let mut cfg = cfg_120();
        let mut a = Zone::preset(ZonePreset::Top, "127.0.0.1".into(), 30, [0; 3]);
        a.offset = Some(10);
        cfg.zones.push(a);
        let mut b = Zone::preset(ZonePreset::Bottom, "127.0.0.1".into(), 30, [0; 3]);
        b.offset = Some(20); // 20..50 shares 20..40 with the zone above
        cfg.zones.push(b);
        assert_eq!(cfg.used_leds("127.0.0.1", None), 40);
        assert_eq!(cfg.used_leds("127.0.0.1", Some(0)), 30);
    }

    #[test]
    fn free_start_finds_the_first_gap_big_enough() {
        let mut cfg = cfg_120();
        let mut a = Zone::preset(ZonePreset::Top, "127.0.0.1".into(), 20, [0; 3]);
        a.offset = Some(40);
        cfg.zones.push(a);
        assert_eq!(cfg.free_start("127.0.0.1", 20), Some(0));
        assert_eq!(cfg.free_start("127.0.0.1", 40), Some(0));
        // after the zone there are exactly 60 LEDs left
        assert_eq!(cfg.free_start("127.0.0.1", 60), Some(60));
        assert_eq!(cfg.free_start("127.0.0.1", 61), None);
        assert_eq!(cfg.free_start("127.0.0.1", 101), None);
    }

    #[test]
    fn zone_report_flags_overlaps_and_overruns() {
        let mut cfg = cfg_120();
        let mut a = Zone::preset(ZonePreset::Top, "127.0.0.1".into(), 50, [0; 3]);
        a.offset = Some(90); // 90..140 on a 120 LED strip
        cfg.zones.push(a);
        let mut b = Zone::preset(ZonePreset::Bottom, "127.0.0.1".into(), 20, [0; 3]);
        b.offset = Some(100); // 100..120 overlaps a
        cfg.zones.push(b);
        let ra = cfg.zone_report(0).unwrap();
        assert!(ra.over, "zone reaches past the end of the device");
        assert_eq!(ra.overlap, Some((1, 100, 120)));
        let rb = cfg.zone_report(1).unwrap();
        assert!(!rb.over);
        assert_eq!(rb.overlap, Some((0, 100, 120)));
        // the same layout without offsets is clean
        let mut clean = cfg_120();
        clean.zones.push(Zone::preset(
            ZonePreset::Top,
            "127.0.0.1".into(),
            50,
            [0; 3],
        ));
        clean.zones.push(Zone::preset(
            ZonePreset::Bottom,
            "127.0.0.1".into(),
            70,
            [0; 3],
        ));
        assert_eq!(clean.zone_report(0).unwrap().overlap, None);
        assert!(!clean.zone_report(1).unwrap().over);
    }

    #[test]
    fn a_broken_file_is_replaced_by_the_backup_and_kept() {
        let dir =
            std::env::temp_dir().join(format!("wled-ambilight-broken-{}", std::process::id()));
        let path = dir.join("config.json");
        let mut good = Config::default();
        good.devices.push(DeviceCfg::new("10.0.0.9", "Backup", 77));
        good.zones
            .push(Zone::preset(ZonePreset::Top, "10.0.0.9".into(), 77, [0; 3]));
        // the first save writes no backup, the second one backs the first up
        good.save_to(&path).unwrap();
        good.save_to(&path).unwrap();
        std::fs::write(&path, "{ this is not json").unwrap();
        let loaded = Config::load_path(&path);
        assert_eq!(loaded, good, "the backup must come back");
        assert!(
            dir.join("config.json.broken").exists(),
            "the broken file is kept for a post mortem"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!("wled-ambilight-empty-{}", std::process::id()));
        let path = dir.join("config.json");
        assert_eq!(Config::load_path(&path), Config::default());
        assert!(!path.exists(), "loading must not create a file");
    }

    #[test]
    fn zone_start_if_auto_matches_the_packed_position() {
        let mut cfg = cfg_120();
        cfg.zones.push(Zone::preset(
            ZonePreset::Top,
            "127.0.0.1".into(),
            10,
            [0; 3],
        ));
        let mut pinned = Zone::preset(ZonePreset::Custom, "127.0.0.1".into(), 20, [0; 3]);
        pinned.offset = Some(80);
        cfg.zones.push(pinned);
        cfg.zones.push(Zone::preset(
            ZonePreset::Bottom,
            "127.0.0.1".into(),
            5,
            [0; 3],
        ));
        assert_eq!(cfg.zone_start_if_auto(0), 0);
        // without its offset the middle zone would sit right after the first
        assert_eq!(cfg.zone_start_if_auto(1), 10);
        assert_eq!(cfg.zone_start_if_auto(2), 100);
    }

    #[test]
    fn device_usage_reports_overlaps_and_overruns() {
        let mut cfg = cfg_120();
        cfg.zones.push(Zone::preset(
            ZonePreset::Top,
            "127.0.0.1".into(),
            60,
            [0; 3],
        ));
        cfg.zones.push(Zone::preset(
            ZonePreset::Bottom,
            "127.0.0.1".into(),
            60,
            [0; 3],
        ));
        assert_eq!(
            cfg.device_usage("127.0.0.1"),
            DeviceUsage {
                used: 120,
                problem: false
            }
        );

        let mut overlapping = cfg_120();
        let mut a = Zone::preset(ZonePreset::Top, "127.0.0.1".into(), 10, [0; 3]);
        a.offset = Some(0);
        let mut b = Zone::preset(ZonePreset::Bottom, "127.0.0.1".into(), 10, [0; 3]);
        b.offset = Some(5);
        overlapping.zones.push(a);
        overlapping.zones.push(b);
        let u = overlapping.device_usage("127.0.0.1");
        assert_eq!(u.used, 15, "LEDs 0..15 light up, 5..10 twice");
        assert!(u.problem);

        let mut over = cfg_120();
        let mut c = Zone::preset(ZonePreset::Top, "127.0.0.1".into(), 10, [0; 3]);
        c.offset = Some(115);
        over.zones.push(c);
        let u = over.device_usage("127.0.0.1");
        assert_eq!(u.used, 5, "only LEDs 115..119 exist");
        assert!(u.problem);
    }
}
