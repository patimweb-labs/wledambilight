// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Befehlsschicht zwischen Oberfläche und Pipeline.
//!
//! Jeder Befehl ist eine reine Abbildung auf den gemeinsamen Zustand: lesen,
//! schreiben, neu starten. Es gibt keine zweite Wahrheit im Frontend — was die
//! Oberfläche zeigt, kommt aus `Shared` und geht als `Config` zurück.

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;

use crate::capture::{self, CaptureStatus};
use crate::config::{Config, DeviceCfg, DeviceKind, Zone};
use crate::discovery;
use crate::state::AppState;

/// Verbindungszustand eines Geräts, wie ihn die Oberfläche anzeigt.
#[derive(Serialize, Clone)]
pub struct DeviceState {
    pub id: String,
    pub state: &'static str,
    pub error: Option<String>,
}

/// Messwerte der Pipeline. Wird zyklisch als Ereignis geschickt.
#[derive(Serialize, Clone)]
pub struct Telemetry {
    /// idle | starting | running | error
    pub capture: &'static str,
    pub capture_error: Option<String>,
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

/// Vollbild des Zustands: Konfiguration plus Messwerte. Einmal beim Start.
#[derive(Serialize, Clone)]
pub struct Snapshot {
    pub config: Config,
    pub telemetry: Telemetry,
}

fn device_key(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Wled => "wled",
        DeviceKind::OpenRgb => "openrgb",
        DeviceKind::Razer => "razer",
        DeviceKind::Govee => "govee",
        DeviceKind::Nanoleaf => "nanoleaf",
    }
}

fn telemetry(state: &AppState) -> Telemetry {
    let shared = state.shared();
    let stats = shared.stats();
    let cfg = shared.config();

    let (capture, capture_error) = match &stats.status {
        CaptureStatus::Idle => ("idle", None),
        CaptureStatus::Starting => ("starting", None),
        CaptureStatus::Running { .. } => ("running", None),
        CaptureStatus::Error(message) => ("error", Some(message.clone())),
    };

    // Reihenfolge folgt der Konfiguration, nicht der HashMap.
    let devices = cfg
        .devices
        .iter()
        .map(|d| {
            let (state, error) = match stats.devices.get(&d.id) {
                Some(crate::output::Status::Connecting) | None => ("connecting", None),
                Some(crate::output::Status::Ok) => ("ok", None),
                Some(crate::output::Status::Error(e)) => ("error", Some(e.clone())),
            };
            DeviceState {
                id: d.id.clone(),
                state,
                error,
            }
        })
        .collect();

    let sent = shared
        .last
        .lock()
        .ok()
        .map(|v| v.clone())
        .unwrap_or_default();
    // Ein Streifen je Gerät, in der Reihenfolge der Konfiguration.
    let device_leds: Vec<Vec<[u8; 3]>> = cfg
        .devices
        .iter()
        .enumerate()
        .map(|(index, _)| {
            sent.get(index)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .take(512)
                .collect()
        })
        .collect();

    let zone_colors = zone_colors(&cfg, &sent);

    Telemetry {
        capture,
        capture_error,
        screen: stats.screen,
        dims: stats.dims,
        fps_in: stats.fps_in,
        fps_out: stats.fps_out,
        sample_us: stats.sample_us as u64,
        paused: shared.paused.load(Ordering::Relaxed),
        devices,
        device_leds,
        zone_colors,
    }
}

#[tauri::command]
pub fn snapshot(state: State<'_, AppState>) -> Snapshot {
    Snapshot {
        config: state.shared().config(),
        telemetry: telemetry(&state),
    }
}

#[tauri::command]
pub fn telemetry_now(state: State<'_, AppState>) -> Telemetry {
    telemetry(&state)
}

/// Konfiguration übernehmen, speichern und die Pipeline neu rechnen lassen.
#[tauri::command]
pub fn save_config(state: State<'_, AppState>, config: Config) -> Snapshot {
    state.apply_config(&config);
    Snapshot {
        config: state.shared().config(),
        telemetry: telemetry(&state),
    }
}

/// Aufnahme neu verhandeln (nach einem Berechtigungsdialog oder Monitorwechsel).
#[tauri::command]
pub fn restart_capture(state: State<'_, AppState>) {
    state.shared().request_restart();
}

/// Portal-Token verwerfen: die nächste Aufnahme fragt wieder nach Erlaubnis.
#[tauri::command]
pub fn forget_permission(state: State<'_, AppState>) {
    state.shared().clear_restore_token();
}

#[tauri::command]
pub fn set_paused(state: State<'_, AppState>, paused: bool) {
    state.shared().paused.store(paused, Ordering::Relaxed);
}

#[tauri::command]
pub fn monitors() -> Vec<String> {
    capture::monitors()
}

/// Ein Gerät, das sich mit einem Klick hinzufügen lässt.
#[derive(Serialize, Clone)]
pub struct FoundDevice {
    pub kind: &'static str,
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub target: String,
    pub leds: u16,
    pub detail: String,
}

impl From<discovery::Found> for FoundDevice {
    fn from(f: discovery::Found) -> Self {
        Self {
            kind: device_key(f.kind),
            id: f.id,
            name: f.name,
            host: f.host,
            port: f.port,
            target: f.target,
            leds: f.leds,
            detail: f.detail,
        }
    }
}

/// Netzwerk durchsuchen. Blockiert bis alle Teilnehmer fertig sind (höchstens 12 s),
/// deshalb in einem eigenen Thread statt im Aufrufer.
#[tauri::command]
pub async fn discover() -> Result<Vec<FoundDevice>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let scan = discovery::search();
        let start = Instant::now();
        while !scan.done() && start.elapsed() < Duration::from_secs(12) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let found = scan.found.lock().map(|v| v.clone()).unwrap_or_default();
        found.into_iter().map(FoundDevice::from).collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())
}

/// Vorschlag für eine neue Zone. Die Voreinstellungen sind dieselben wie bisher:
/// vier Ränder, Vollbild oder frei.
#[tauri::command]
pub fn zone_preset(device: String, count: u16, preset: String) -> Zone {
    use crate::config::ZonePreset;
    let kind = match preset.as_str() {
        "top" => ZonePreset::Top,
        "bottom" => ZonePreset::Bottom,
        "left" => ZonePreset::Left,
        "right" => ZonePreset::Right,
        "full" => ZonePreset::Full,
        _ => ZonePreset::Custom,
    };
    Zone::preset(kind, device, count, [0, 0, 0])
}

/// Vorschlag für ein neues Gerät.
#[tauri::command]
pub fn device_template(host: String, name: String, leds: u16) -> DeviceCfg {
    DeviceCfg::new(host, name, leds)
}

/// Eine Farbe je Zone: der Mittelwert der LEDs, die diese Zone gerade belegt.
///
/// Reine Funktion, damit sie ohne Fenster, ohne Pipeline und ohne Gerät prüfbar
/// ist — die Logik ist der Teil, in dem Fehler wehtun (Startindex, Offsets,
/// deaktivierte Zonen).
pub fn zone_colors(cfg: &Config, sent: &[Vec<[u8; 3]>]) -> Vec<[u8; 3]> {
    let mut cursor: std::collections::HashMap<&str, u16> = std::collections::HashMap::new();
    cfg.zones
        .iter()
        .map(|zone| {
            let Some(device_index) = cfg.devices.iter().position(|d| d.id == zone.device) else {
                return [0, 0, 0];
            };
            let Some(colors) = sent.get(device_index) else {
                return [0, 0, 0];
            };
            if !zone.enabled {
                return [0, 0, 0];
            }
            let start = zone.offset.unwrap_or_else(|| {
                let next = cursor.entry(zone.device.as_str()).or_insert(0);
                let at = *next;
                if zone.enabled {
                    *next = at.saturating_add(zone.count);
                }
                at
            }) as usize;
            let end = (start + zone.count as usize).min(colors.len());
            if start >= end {
                return [0, 0, 0];
            }
            let slice = &colors[start..end];
            let n = slice.len() as u32;
            let sum = slice.iter().fold([0u32; 3], |acc, c| {
                [
                    acc[0] + c[0] as u32,
                    acc[1] + c[1] as u32,
                    acc[2] + c[2] as u32,
                ]
            });
            [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
        })
        .collect()
}

/// Gerät testen: WLED antwortet mit Name, Firmware und LED-Anzahl.
#[tauri::command]
pub async fn probe_wled(host: String) -> Result<FoundDevice, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::wled::probe(&host, Duration::from_millis(900))
            .map(|info| FoundDevice::from(discovery::Found::wled(&info)))
            .ok_or_else(|| format!("{host} antwortet nicht auf der WLED-Schnittstelle"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Nanoleaf paaren: der Controller muss innerhalb von 30 s blinken (Knopf gedrückt).
/// Der zurückgegebene Token gehört in die Gerätekonfiguration.
#[tauri::command]
pub async fn pair_nanoleaf(host: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || crate::output::nanoleaf::pair(&host))
        .await
        .map_err(|e| e.to_string())?
}

/// Controller eines OpenRGB-SDK-Servers auflisten (lokal oder im Netz).
#[tauri::command]
pub async fn list_openrgb(host: String, port: u16) -> Result<Vec<FoundDevice>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let port = if port == 0 {
            crate::output::openrgb::DEFAULT_PORT
        } else {
            port
        };
        let controllers = crate::output::openrgb::list(&host, port, Duration::from_millis(1500))?;
        Ok(controllers
            .iter()
            .map(|c| FoundDevice::from(discovery::Found::openrgb(&host, port, c)))
            .collect::<Vec<_>>())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Das aktuelle Bild der Aufnahme als JPEG (data-URL) für die Vorschau.
///
/// Die Oberfläche fragt das Bild getrennt von den Messwerten ab: Bilder sind
/// groß, Messwerte klein. So bleibt die Telemetrie schnell, und die Vorschau
/// kann gebremst werden, ohne dass Zähler und LED-Leiste darunter leiden.
#[tauri::command]
pub fn preview_frame(state: State<'_, AppState>) -> Option<String> {
    let frame = state.shared().frame.lock().ok()?.clone();
    if frame.w == 0 || frame.h == 0 || frame.data.len() < (frame.w * frame.h * 3) as usize {
        return None;
    }
    let image = image::RgbImage::from_raw(frame.w, frame.h, frame.data.to_vec())?;
    let mut jpeg = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 70);
    encoder.encode_image(&image).ok()?;
    Some(format!(
        "data:image/jpeg;base64,{}",
        crate::output::base64(&jpeg)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DeviceCfg, Zone};

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
    fn a_zone_for_an_unknown_device_is_dark() {
        let cfg = Config {
            devices: vec![device("d1", 2)],
            zones: vec![zone("fehlt", 2, None, true)],
            ..Config::default()
        };
        assert_eq!(zone_colors(&cfg, &[]), vec![[0, 0, 0]]);
    }
}
