// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Was das Fenster aus der Pipeline liest – als reine Funktionen.
//!
//! Früher lag das in der Befehlsschicht für die Web-Oberfläche. Das Fenster
//! ist jetzt Rust und liest den gemeinsamen Zustand direkt; geblieben sind die
//! Rechnungen, in denen Fehler wehtun (Startindex, Offsets, deaktivierte
//! Zonen, Rückwärts-Verdrahtung) – ohne Fenster, ohne Gerät prüfbar.

use std::sync::atomic::Ordering;

use crate::capture::CaptureStatus;
use crate::config::Config;
use crate::discovery;
use crate::output::Status;
use crate::worker::Shared;

/// Verbindungszustand eines Geräts, in der Reihenfolge der Konfiguration.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceState {
    pub id: String,
    pub status: Status,
}

/// Messwerte der Pipeline, einmal je Bild gelesen.
#[derive(Debug, Clone, PartialEq)]
pub struct Telemetry {
    pub capture: CaptureStatus,
    pub screen: (u32, u32),
    pub dims: (u32, u32),
    pub fps_in: f32,
    pub fps_out: f32,
    pub sample_us: u64,
    pub paused: bool,
    pub devices: Vec<DeviceState>,
    /// Farben, die zuletzt je Gerät auf die Leitung gingen (höchstens 512 je Gerät)
    pub device_leds: Vec<Vec<[u8; 3]>>,
    /// Eine Farbe je Zone, in der Reihenfolge der Konfiguration. So zeigt die
    /// Vorschau nicht die eingestellte Farbe, sondern die, die wirklich ankommt.
    pub zone_colors: Vec<[u8; 3]>,
}

impl Default for Telemetry {
    fn default() -> Self {
        Telemetry {
            capture: CaptureStatus::Idle,
            screen: (0, 0),
            dims: (0, 0),
            fps_in: 0.0,
            fps_out: 0.0,
            sample_us: 0,
            paused: false,
            devices: Vec::new(),
            device_leds: Vec::new(),
            zone_colors: Vec::new(),
        }
    }
}

/// Messwerte für `cfg` aus dem gemeinsamen Zustand lesen.
///
/// `cfg` ist die Fassung, die das Fenster zeigt; Reihenfolge und Zonen folgen
/// ihr, nicht der HashMap der Pipeline.
pub fn read(shared: &Shared, cfg: &Config) -> Telemetry {
    let stats = shared.stats();
    let devices = cfg
        .devices
        .iter()
        .map(|d| DeviceState {
            id: d.id.clone(),
            status: stats.devices.get(&d.id).cloned().unwrap_or_default(),
        })
        .collect();
    let sent = shared
        .last
        .lock()
        .ok()
        .map(|v| v.clone())
        .unwrap_or_default();
    let device_leds = (0..cfg.devices.len())
        .map(|index| {
            sent.get(index)
                .map(|leds| leds.iter().take(512).copied().collect())
                .unwrap_or_default()
        })
        .collect();
    Telemetry {
        capture: stats.status.clone(),
        screen: stats.screen,
        dims: stats.dims,
        fps_in: stats.fps_in,
        fps_out: stats.fps_out,
        sample_us: stats.sample_us as u64,
        paused: shared.paused.load(Ordering::Relaxed),
        devices,
        device_leds,
        zone_colors: zone_colors(cfg, &sent),
    }
}

/// Eine Farbe je Zone: der Mittelwert der LEDs, die diese Zone gerade belegt.
///
/// Reine Funktion, damit sie ohne Fenster, ohne Pipeline und ohne Gerät prüfbar
/// ist — die Logik ist der Teil, in dem Fehler wehtun (Startindex, Offsets,
/// deaktivierte Zonen).
///
/// Die Startindizes kommen aus `Config::zone_starts`, derselben Quelle wie im
/// Sampler. `sent` ist die Leitungsreihenfolge: bei `reverse` liegt LED 0 am
/// Ende des Puffers, bei `bgr` sind Rot und Blau getauscht. Beides wird hier
/// zurückgerechnet, damit die Vorschau die Farbe des Bildes zeigt.
pub fn zone_colors(cfg: &Config, sent: &[Vec<[u8; 3]>]) -> Vec<[u8; 3]> {
    let starts = cfg.zone_starts();
    cfg.zones
        .iter()
        .zip(starts)
        .map(|(zone, start)| {
            if !zone.enabled {
                return [0, 0, 0];
            }
            let Some(device_index) = cfg.devices.iter().position(|d| d.id == zone.device) else {
                return [0, 0, 0];
            };
            let device = &cfg.devices[device_index];
            let Some(colors) = sent.get(device_index) else {
                return [0, 0, 0];
            };
            let len = colors.len();
            let start = start as usize;
            let end = (start + zone.count as usize).min(len);
            if start >= end {
                return [0, 0, 0];
            }
            let mut sum = [0u32; 3];
            for led in start..end {
                let wire = if device.reverse { len - 1 - led } else { led };
                let mut c = colors[wire];
                if device.bgr {
                    c.swap(0, 2);
                }
                for k in 0..3 {
                    sum[k] += c[k] as u32;
                }
            }
            let n = (end - start) as u32;
            [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
        })
        .collect()
}

/// Bereits eingerichtete Geräte aus der Fundliste nehmen.
///
/// Der Netzwerkscan sieht nur, was antwortet — nicht, was schon konfiguriert ist.
/// Ein Vorschlag, den man nicht mehr hinzufügen kann, ist keiner: die Liste zeigt
/// nur Geräte, die noch fehlen. Bekannt heißt: gleiche Kennung, oder gleiche
/// Adresse am gleichen Port.
pub fn without_known(
    found: Vec<discovery::Found>,
    devices: &[crate::config::DeviceCfg],
) -> Vec<discovery::Found> {
    found
        .into_iter()
        .filter(|f| {
            !devices.iter().any(|d| {
                if d.id == f.id {
                    return true;
                }
                // Hinter einer Adresse koennen mehrere Geraete haengen (OpenRGB-
                // Server mit Tastatur und Maus). Dort zaehlt nur die Kennung.
                if !f.target.is_empty() || !d.target.is_empty() {
                    return false;
                }
                d.host == f.host && d.port == f.port
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DeviceCfg, Zone};

    fn found(host: &str, port: u16, id: &str, target: &str) -> crate::discovery::Found {
        crate::discovery::Found {
            kind: crate::config::DeviceKind::Wled,
            id: id.to_string(),
            name: id.to_string(),
            host: host.to_string(),
            port,
            target: target.to_string(),
            leds: 1,
            detail: String::new(),
        }
    }

    #[test]
    fn known_devices_are_not_offered_again() {
        let wled = DeviceCfg::new("192.168.1.42", "Wand", 120);
        let mut keyboard = DeviceCfg::new("127.0.0.1", "Tastatur", 104);
        keyboard.id = "openrgb:127.0.0.1:6742:Tastatur".to_string();
        keyboard.port = 6742;
        keyboard.target = "Keyboard@0".to_string();

        let rest = without_known(
            vec![
                found("192.168.1.42", 0, "wled:192.168.1.42", ""),
                found(
                    "127.0.0.1",
                    6742,
                    "openrgb:127.0.0.1:6742:Tastatur",
                    "Keyboard@0",
                ),
                // Gleicher Server, anderer Controller: der fehlt noch.
                found("127.0.0.1", 6742, "openrgb:127.0.0.1:6742:Maus", "Mouse@1"),
                found("192.168.1.99", 0, "wled:192.168.1.99", ""),
            ],
            &[wled, keyboard],
        );

        let ids: Vec<&str> = rest.iter().map(|f| f.id.as_str()).collect();
        assert_eq!(ids, ["openrgb:127.0.0.1:6742:Maus", "wled:192.168.1.99"]);
    }

    #[test]
    fn same_address_without_target_is_the_same_device() {
        let strip = DeviceCfg::new("192.168.1.7", "Regal", 60);
        let rest = without_known(
            vec![found("192.168.1.7", 0, "wled:192.168.1.7", "")],
            &[strip],
        );
        assert!(
            rest.is_empty(),
            "gleiche Adresse ohne Ziel ist dasselbe Gerät"
        );
    }

    fn device(id: &str, leds: u16) -> DeviceCfg {
        // `DeviceCfg::new` leitet die id aus dem Host ab; für den Test brauchen
        // wir eine feste id, damit die Zonen sie finden.
        let mut device = DeviceCfg::new("127.0.0.1", id, leds);
        device.id = id.to_string();
        device
    }

    fn zone(device: &str, count: u16, offset: Option<u16>, enabled: bool) -> Zone {
        let mut z = Zone::preset(
            crate::config::ZonePreset::Custom,
            device.to_string(),
            count,
            [0, 0, 0],
        );
        z.offset = offset;
        z.enabled = enabled;
        z
    }

    #[test]
    fn without_offset_the_zones_follow_each_other() {
        // Zwei Zonen auf einem Gerät: die erste liest 0..2, die zweite 2..4.
        let cfg = Config {
            devices: vec![device("d1", 4)],
            zones: vec![zone("d1", 2, None, true), zone("d1", 2, None, true)],
            ..Config::default()
        };
        let sent = vec![vec![[255, 0, 0], [255, 0, 0], [0, 0, 255], [0, 0, 255]]];
        assert_eq!(zone_colors(&cfg, &sent), vec![[255, 0, 0], [0, 0, 255]]);
    }

    #[test]
    fn an_offset_wins_over_the_running_index() {
        let cfg = Config {
            devices: vec![device("d1", 4)],
            zones: vec![zone("d1", 2, Some(2), true)],
            ..Config::default()
        };
        let sent = vec![vec![[255, 0, 0], [255, 0, 0], [0, 255, 0], [0, 255, 0]]];
        assert_eq!(zone_colors(&cfg, &sent), vec![[0, 255, 0]]);
    }

    #[test]
    fn a_disabled_zone_does_not_move_the_cursor() {
        // Eine deaktivierte Zone verbraucht keinen Platz: die nächste beginnt
        // trotzdem bei 0. Sonst würden beim Ausschalten Farben verrutschen.
        let cfg = Config {
            devices: vec![device("d1", 4)],
            zones: vec![zone("d1", 2, None, false), zone("d1", 2, None, true)],
            ..Config::default()
        };
        let sent = vec![vec![[1, 2, 3], [1, 2, 3], [9, 9, 9], [9, 9, 9]]];
        let colors = zone_colors(&cfg, &sent);
        assert_eq!(colors[0], [0, 0, 0], "deaktiviert bleibt dunkel");
        assert_eq!(colors[1], [1, 2, 3], "die zweite Zone beginnt bei 0");
    }

    #[test]
    fn a_zone_reaching_past_the_sent_data_averages_what_is_there() {
        // Acht LEDs konfiguriert, zwei gesendet: kein Absturz, sondern der
        // Mittelwert des Vorhandenen.
        let cfg = Config {
            devices: vec![device("d1", 8)],
            zones: vec![zone("d1", 8, None, true)],
            ..Config::default()
        };
        let sent = vec![vec![[9, 9, 9], [9, 9, 9]]];
        assert_eq!(zone_colors(&cfg, &sent), vec![[9, 9, 9]]);
    }

    #[test]
    fn a_zone_starting_behind_the_sent_data_is_dark() {
        let cfg = Config {
            devices: vec![device("d1", 2)],
            zones: vec![zone("d1", 2, Some(2), true)],
            ..Config::default()
        };
        let sent = vec![vec![[9, 9, 9], [9, 9, 9]]];
        assert_eq!(zone_colors(&cfg, &sent), vec![[0, 0, 0]]);
    }

    #[test]
    fn a_packed_zone_continues_after_a_pinned_one() {
        // dieselbe Regel wie im Sampler: nach dem festen Block 2..4 folgt 4..6
        let cfg = Config {
            devices: vec![device("d1", 6)],
            zones: vec![zone("d1", 2, Some(2), true), zone("d1", 2, None, true)],
            ..Config::default()
        };
        let sent = vec![vec![
            [1, 1, 1],
            [1, 1, 1],
            [2, 2, 2],
            [2, 2, 2],
            [3, 3, 3],
            [3, 3, 3],
        ]];
        assert_eq!(cfg.zone_starts(), vec![2, 4]);
        assert_eq!(zone_colors(&cfg, &sent), vec![[2, 2, 2], [3, 3, 3]]);
    }

    #[test]
    fn reverse_and_bgr_are_undone_for_the_preview() {
        // Gerät mit vier LEDs, umgedreht verdrahtet und BGR: die Engine schickt
        // logisch [rot, rot, blau, blau] als [blau, blau, rot, rot] in BGR.
        let mut d = device("d1", 4);
        d.reverse = true;
        d.bgr = true;
        let cfg = Config {
            devices: vec![d],
            zones: vec![zone("d1", 2, None, true), zone("d1", 2, None, true)],
            ..Config::default()
        };
        let red_bgr = [0, 0, 255];
        let blue_bgr = [255, 0, 0];
        let sent = vec![vec![blue_bgr, blue_bgr, red_bgr, red_bgr]];
        assert_eq!(zone_colors(&cfg, &sent), vec![[255, 0, 0], [0, 0, 255]]);
    }

    #[test]
    fn a_zone_for_an_unknown_device_is_dark() {
        let cfg = Config {
            devices: vec![device("d1", 2)],
            zones: vec![zone("fehlt", 2, None, true)],
            ..Config::default()
        };
        assert_eq!(zone_colors(&cfg, &[]), vec![[0, 0, 0]]);
    }
}
