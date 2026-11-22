// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Aktionen ausführen und alles rechnen, was die Ansichten brauchen.
//!
//! Hier steht die Logik, die früher im Web-Store lag (`store.ts`,
//! `lib/zones.ts`): Konfiguration ändern und an die Pipeline geben, Geräte aus
//! der Suche übernehmen, Zonen ziehen und anlegen. Alles ohne Fenster prüfbar;
//! die Netzwerkarbeit (Suche, Prüfen, Paaren) läuft über [`Netz`], damit Tests
//! ohne Netz auskommen.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{Duration, Instant};

use wled_ambilight::capture::CaptureStatus;
use wled_ambilight::config::{Config, DeviceCfg, DeviceKind, Settings, Zone, ZonePreset};
use wled_ambilight::discovery::Found;
use wled_ambilight::i18n::tr;
use wled_ambilight::worker::Shared;

use crate::modell::{
    Aktion, Arbeit, GeraetDialog, Meldung, Oberflaeche, Suche, Zug, Zugart, KANTEN,
};

/// So lange wartet die Platte nach der letzten Änderung (Tippen, Ziehen).
pub const SPEICHERN_NACH: Duration = Duration::from_millis(300);

/// Feinster Schritt beim Ziehen mit Einrasten: 0,5 % – feiner als das Auge,
/// grob genug zum Treffen.
pub const RASTER: f32 = 0.005;
/// Kleinste Kante einer Zone.
pub const ZONE_MIN: f32 = 0.02;

// ------------------------------------------------------------------ Netz ----

/// Netzwerkarbeit, die blockiert. Läuft im Hintergrund; Tests setzen eine
/// Attrappe ein.
pub trait Netz: Send + Sync {
    /// Netzwerk durchsuchen (WLED, OpenRGB, Govee, Nanoleaf, Razer).
    fn suchen(&self) -> Vec<Found>;
    /// WLED am Host fragen: Name, Firmware, LED-Anzahl.
    fn pruefen(&self, host: &str) -> Result<Found, String>;
    /// Nanoleaf paaren: liefert den Token.
    fn paaren(&self, host: &str) -> Result<String, String>;
    /// Controller eines OpenRGB-Servers.
    fn openrgb(&self, host: &str, port: u16) -> Result<Vec<Found>, String>;
}

/// Das echte Netz.
pub struct EchtesNetz;

impl Netz for EchtesNetz {
    fn suchen(&self) -> Vec<Found> {
        let scan = wled_ambilight::discovery::search();
        let start = Instant::now();
        while !scan.done() && start.elapsed() < Duration::from_secs(12) {
            std::thread::sleep(Duration::from_millis(100));
        }
        scan.found.lock().map(|v| v.clone()).unwrap_or_default()
    }

    fn pruefen(&self, host: &str) -> Result<Found, String> {
        wled_ambilight::wled::probe(host, Duration::from_millis(900))
            .map(|info| Found::wled(&info))
            .ok_or_else(|| {
                format!(
                    "{host} {}",
                    tr(
                        "antwortet nicht auf der WLED-Schnittstelle",
                        "does not answer on the WLED interface"
                    )
                )
            })
    }

    fn paaren(&self, host: &str) -> Result<String, String> {
        wled_ambilight::output::nanoleaf::pair(host)
    }

    fn openrgb(&self, host: &str, port: u16) -> Result<Vec<Found>, String> {
        use wled_ambilight::output::openrgb;
        let port = if port == 0 {
            openrgb::DEFAULT_PORT
        } else {
            port
        };
        let liste = openrgb::list(host, port, Duration::from_millis(1500))?;
        Ok(liste
            .iter()
            .map(|c| Found::openrgb(host, port, c))
            .collect())
    }
}

/// Ergebnis einer Hintergrundarbeit.
#[derive(Debug, Clone, PartialEq)]
pub enum Antwort {
    Gefunden(Vec<Found>),
    Geprueft(Result<Found, String>),
    Gepaart(Result<String, String>),
    Controller(Result<Vec<Found>, String>),
}

/// Was das Fenster zum Ausführen braucht.
pub struct Kern {
    pub shared: Arc<Shared>,
    pub netz: Arc<dyn Netz>,
    /// Hintergrundarbeit meldet sich hier zurück.
    pub antworten: Sender<Antwort>,
    /// Weckt das Fenster, wenn eine Antwort da ist.
    pub wecker: Arc<dyn Fn() + Send + Sync>,
    /// Wohin gespeichert wird; `None` (Tests): nie auf die Platte.
    pub konfig_pfad: Option<PathBuf>,
    /// Von Hand angehalten – Fenster und Tray teilen sich den Stand, auch
    /// wenn gerade kein Fenster offen ist.
    pub von_hand: Arc<AtomicBool>,
}

impl Kern {
    fn im_hintergrund(&self, arbeit: impl FnOnce(&dyn Netz) -> Antwort + Send + 'static) {
        let netz = Arc::clone(&self.netz);
        let tx = self.antworten.clone();
        let wecker = Arc::clone(&self.wecker);
        let gestartet = std::thread::Builder::new()
            .name("netz".into())
            .spawn(move || {
                let _ = tx.send(arbeit(netz.as_ref()));
                wecker();
            });
        if let Err(e) = gestartet {
            eprintln!("wled-ambilight: Hintergrundarbeit nicht gestartet: {e}");
        }
    }
}

// ------------------------------------------------------------- Ausführen ----

/// Konfiguration hat sich geändert: an die Pipeline, Speichern vormerken.
fn geaendert(o: &mut Oberflaeche, k: &Kern, jetzt: Instant) {
    o.config.sanitize();
    k.shared.set_config(&o.config);
    wled_ambilight::i18n::init(&o.config.settings.language);
    o.speichern_ab = Some(jetzt + SPEICHERN_NACH);
    o.aktiv = o.aktiv.min(o.config.zones.len().saturating_sub(1));
}

/// Führt eine Aktion aus.
pub fn ausfuehren(a: Aktion, o: &mut Oberflaeche, k: &Kern, jetzt: Instant) {
    use std::sync::atomic::Ordering::Relaxed;
    match a {
        Aktion::Anhalten(an) => {
            o.angehalten = an;
            k.von_hand.store(an, Relaxed);
            k.shared.paused.store(an, Relaxed);
        }
        Aktion::AufnahmeNeu => k.shared.request_restart(),
        Aktion::BerechtigungVerwerfen => {
            k.shared.clear_restore_token();
            k.shared.request_restart();
            o.config.settings.restore_token = None;
        }
        Aktion::Suchen => {
            if o.suche != Suche::Laeuft {
                o.suche = Suche::Laeuft;
                k.im_hintergrund(|n| Antwort::Gefunden(n.suchen()));
            }
        }
        Aktion::Hinzufuegen(f) => {
            if o.config.devices.iter().any(|d| d.id == f.id) {
                o.meldung = Meldung::Hinweis(format!(
                    "{} {}",
                    f.name,
                    tr("ist bereits eingerichtet.", "is already set up.")
                ));
            } else {
                o.config.devices.push(geraet_aus_fund(&f));
                if let Suche::Fertig(liste) = &mut o.suche {
                    liste.retain(|x| x.id != f.id);
                }
                geaendert(o, k, jetzt);
            }
        }
        Aktion::AdresseTippen(t) => o.adresse = t,
        Aktion::AdresseHinzufuegen => {
            let adresse = o.adresse.trim().to_string();
            if adresse.is_empty() {
                return;
            }
            if o.config.devices.iter().any(|d| d.id == adresse) {
                o.meldung = Meldung::Hinweis(format!(
                    "{adresse} {}",
                    tr("ist bereits eingerichtet.", "is already set up.")
                ));
                return;
            }
            o.config
                .devices
                .push(DeviceCfg::new(adresse.clone(), adresse, 60));
            o.adresse.clear();
            geaendert(o, k, jetzt);
            let index = o.config.devices.len() - 1;
            ausfuehren(Aktion::DialogOeffnen(Some(index)), o, k, jetzt);
        }
        Aktion::Geraet(i, neu) => {
            if let Some(d) = o.config.devices.get_mut(i) {
                if *d != neu {
                    *d = neu;
                    geaendert(o, k, jetzt);
                }
            }
        }
        Aktion::GeraetEntfernen(i) => {
            if i < o.config.devices.len() {
                let weg = o.config.devices.remove(i);
                o.config.zones.retain(|z| z.device != weg.id);
                geaendert(o, k, jetzt);
            }
        }
        Aktion::DialogOeffnen(index) => {
            let entwurf = match index.and_then(|i| o.config.devices.get(i)) {
                Some(d) => d.clone(),
                None => DeviceCfg::new("", tr("Neues Gerät", "New device"), 60),
            };
            o.dialog = Some(GeraetDialog {
                entwurf,
                index: index.filter(|i| *i < o.config.devices.len()),
                controller: Vec::new(),
                notiz: String::new(),
                arbeit: Arbeit::Keine,
            });
        }
        Aktion::DialogEntwurf(d) => {
            if let Some(dialog) = &mut o.dialog {
                dialog.entwurf = d;
            }
        }
        Aktion::DialogPruefen => {
            if let Some(dialog) = &mut o.dialog {
                let host = dialog.entwurf.host.trim().to_string();
                dialog.arbeit = Arbeit::Prueft;
                k.im_hintergrund(move |n| Antwort::Geprueft(n.pruefen(&host)));
            }
        }
        Aktion::DialogPaaren => {
            if let Some(dialog) = &mut o.dialog {
                let host = dialog.entwurf.host.trim().to_string();
                dialog.arbeit = Arbeit::Paart;
                k.im_hintergrund(move |n| Antwort::Gepaart(n.paaren(&host)));
            }
        }
        Aktion::DialogControllerHolen => {
            if let Some(dialog) = &mut o.dialog {
                let host = match dialog.entwurf.host.trim() {
                    "" => "127.0.0.1".to_string(),
                    h => h.to_string(),
                };
                let port = dialog.entwurf.port;
                dialog.arbeit = Arbeit::Holt;
                k.im_hintergrund(move |n| Antwort::Controller(n.openrgb(&host, port)));
            }
        }
        Aktion::DialogUebernehmen => {
            if let Some(dialog) = o.dialog.take() {
                let mut d = dialog.entwurf;
                d.host = d.host.trim().to_string();
                if d.id.is_empty() {
                    d.id = eindeutige_id(&o.config, &d);
                }
                match dialog.index {
                    Some(i) if i < o.config.devices.len() => {
                        // Neue Kennung: Zonen ziehen mit, sonst verlören sie
                        // ihr Gerät.
                        let alt = o.config.devices[i].id.clone();
                        if alt != d.id {
                            for z in o.config.zones.iter_mut().filter(|z| z.device == alt) {
                                z.device = d.id.clone();
                            }
                        }
                        o.config.devices[i] = d;
                    }
                    _ => o.config.devices.push(d),
                }
                geaendert(o, k, jetzt);
            }
        }
        Aktion::DialogSchliessen => o.dialog = None,
        Aktion::ZoneWaehlen(i) => o.aktiv = i.min(o.config.zones.len().saturating_sub(1)),
        Aktion::Zone(i, neu) => {
            if let Some(z) = o.config.zones.get_mut(i) {
                if *z != neu {
                    *z = neu;
                    geaendert(o, k, jetzt);
                }
            }
        }
        Aktion::ZoneNeu => {
            let Some(d) = o.config.devices.first() else {
                return;
            };
            let anzahl = (d.led_count / 4).max(1);
            let mut z = Zone::preset(ZonePreset::Custom, d.id.clone(), anzahl, [0, 0, 0]);
            z.name = format!("{} {}", tr("Zone", "Zone"), o.config.zones.len() + 1);
            o.config.zones.push(z);
            o.aktiv = o.config.zones.len() - 1;
            geaendert(o, k, jetzt);
        }
        Aktion::ZonenAufKanten => {
            let Some(d) = o.config.devices.first().cloned() else {
                return;
            };
            let erste = o.config.zones.len();
            for (kante, anzahl) in KANTEN.into_iter().zip(kanten_leds(d.led_count)) {
                let mut z = Zone::preset(kante, d.id.clone(), anzahl, [0, 0, 0]);
                z.name = kanten_name(kante).to_string();
                o.config.zones.push(z);
            }
            o.aktiv = erste;
            geaendert(o, k, jetzt);
        }
        Aktion::GeraetJeKante => {
            let erste = o.config.zones.len();
            let geraete: Vec<DeviceCfg> = o.config.devices.iter().take(4).cloned().collect();
            for (kante, d) in KANTEN.into_iter().zip(geraete) {
                let mut z = Zone::preset(kante, d.id.clone(), d.led_count, [0, 0, 0]);
                z.name = kanten_name(kante).to_string();
                o.config.zones.push(z);
            }
            if o.config.zones.len() > erste {
                o.aktiv = erste;
                geaendert(o, k, jetzt);
            }
        }
        Aktion::ZoneRuecken(i, schritt) => {
            let ziel = i as i64 + schritt as i64;
            if i < o.config.zones.len() && ziel >= 0 && (ziel as usize) < o.config.zones.len() {
                let z = o.config.zones.remove(i);
                o.config.zones.insert(ziel as usize, z);
                o.aktiv = ziel as usize;
                geaendert(o, k, jetzt);
            }
        }
        Aktion::ZoneEntfernen(i) => {
            if i < o.config.zones.len() {
                o.config.zones.remove(i);
                geaendert(o, k, jetzt);
            }
        }
        Aktion::ZugBeginnen(z) => {
            o.aktiv = z.zone;
            o.zug = Some(z);
        }
        Aktion::ZugEnde => o.zug = None,
        Aktion::Reiter(r) => o.reiter = r,
        Aktion::Einstellungen(s) => {
            // Der Portal-Token gehört der Pipeline (siehe `Shared::set_config`).
            if o.config.settings != *s {
                o.config.settings = *s;
                geaendert(o, k, jetzt);
            }
        }
        Aktion::BildZuruecksetzen => {
            o.config.settings = bild_standard(&o.config.settings);
            geaendert(o, k, jetzt);
        }
        Aktion::MeldungWeg => o.meldung = Meldung::Keine,
    }
}

/// Eine Antwort aus dem Hintergrund einarbeiten.
pub fn antwort(a: Antwort, o: &mut Oberflaeche) {
    match a {
        Antwort::Gefunden(liste) => {
            o.suche = Suche::Fertig(wled_ambilight::telemetry::without_known(
                liste,
                &o.config.devices,
            ));
        }
        Antwort::Geprueft(ergebnis) => {
            if let Some(d) = &mut o.dialog {
                d.arbeit = Arbeit::Keine;
                match ergebnis {
                    Ok(f) => {
                        d.entwurf.name = f.name;
                        d.entwurf.led_count = f.leds.max(1);
                        d.notiz = f.detail;
                    }
                    Err(e) => d.notiz = e,
                }
            }
        }
        Antwort::Gepaart(ergebnis) => {
            if let Some(d) = &mut o.dialog {
                d.arbeit = Arbeit::Keine;
                match ergebnis {
                    Ok(token) => {
                        d.entwurf.token = Some(token);
                        d.notiz = tr("Token erhalten.", "Token received.").into();
                    }
                    Err(e) => {
                        d.notiz = format!(
                            "{} {e}",
                            tr(
                                "Kein Token – blinkt der Controller?",
                                "No token – is the controller flashing?"
                            )
                        )
                    }
                }
            }
        }
        Antwort::Controller(ergebnis) => {
            if let Some(d) = &mut o.dialog {
                d.arbeit = Arbeit::Keine;
                match ergebnis {
                    Ok(liste) => {
                        d.notiz = if liste.is_empty() {
                            tr("Kein Controller gefunden.", "No controller found.").into()
                        } else {
                            String::new()
                        };
                        d.controller = liste;
                    }
                    Err(e) => d.notiz = e,
                }
            }
        }
    }
}

// --------------------------------------------------------------- Rechnen ----

/// Ein Gerät aus einem Fund der Suche.
pub fn geraet_aus_fund(f: &Found) -> DeviceCfg {
    let mut d = DeviceCfg::of_kind(f.kind, f.id.clone(), f.host.clone(), f.name.clone(), f.leds);
    d.port = f.port;
    d.target = f.target.clone();
    d
}

/// Kennung für ein Gerät, das noch keine hat: die Adresse, bei Bedarf mit
/// laufender Nummer.
fn eindeutige_id(cfg: &Config, d: &DeviceCfg) -> String {
    let grund = if d.host.is_empty() {
        d.name.clone()
    } else {
        d.host.clone()
    };
    let frei = |id: &str| !cfg.devices.iter().any(|x| x.id == id);
    if frei(&grund) {
        return grund;
    }
    (2..)
        .map(|n| format!("{grund}#{n}"))
        .find(|id| frei(id))
        .unwrap_or(grund)
}

/// Die LEDs eines Streifens auf vier Kanten verteilt (Rest an die ersten).
pub fn kanten_leds(gesamt: u16) -> [u16; 4] {
    let je = gesamt / 4;
    let rest = gesamt % 4;
    std::array::from_fn(|i| (je + u16::from((i as u16) < rest)).max(1))
}

fn kanten_name(k: ZonePreset) -> &'static str {
    match k {
        ZonePreset::Top => tr("Oben", "Top"),
        ZonePreset::Right => tr("Rechts", "Right"),
        ZonePreset::Bottom => tr("Unten", "Bottom"),
        ZonePreset::Left => tr("Links", "Left"),
        ZonePreset::Full => tr("Vollbild", "Full"),
        ZonePreset::Custom => tr("Zone", "Zone"),
    }
}

/// Bildeinstellungen auf Standard, alles andere bleibt.
pub fn bild_standard(s: &Settings) -> Settings {
    let d = Settings::default();
    Settings {
        smoothing_ms: d.smoothing_ms,
        spatial: d.spatial,
        slew: d.slew,
        gamma: d.gamma,
        saturation: d.saturation,
        brightness: d.brightness,
        ..s.clone()
    }
}

fn klemmen(wert: f32, unten: f32, oben: f32) -> f32 {
    wert.max(unten).min(oben.max(unten))
}

fn rasten(wert: f32, an: bool) -> f32 {
    if an {
        (wert / RASTER).round() * RASTER
    } else {
        wert
    }
}

/// Eine Zone ziehen: `dx`, `dy` sind der Weg seit dem Anfassen, in Anteilen
/// der Bühne (0..1). Verschieben hält die Größe, Größe ändern die Ecke oben
/// links; die Zone bleibt immer im Bild.
pub fn zone_ziehen(zug: &Zug, dx: f32, dy: f32, einrasten: bool) -> Zone {
    let mut z = zug.von.clone();
    match zug.art {
        Zugart::Verschieben => {
            z.x = klemmen(rasten(z.x + dx, einrasten), 0.0, 1.0 - z.w);
            z.y = klemmen(rasten(z.y + dy, einrasten), 0.0, 1.0 - z.h);
        }
        Zugart::Groesse => {
            z.w = klemmen(rasten(z.w + dx, einrasten), ZONE_MIN, 1.0 - z.x);
            z.h = klemmen(rasten(z.h + dy, einrasten), ZONE_MIN, 1.0 - z.y);
        }
    }
    z
}

/// Pfeiltasten: Zone um einen Schritt schieben (mit Umschalt groß).
pub fn zone_schubsen(z: &Zone, dx: f32, dy: f32, gross: bool, einrasten: bool) -> Zone {
    let schritt = if gross { 0.05 } else { RASTER };
    let mut n = z.clone();
    n.x = klemmen(rasten(z.x + dx * schritt, einrasten), 0.0, 1.0 - z.w);
    n.y = klemmen(rasten(z.y + dy * schritt, einrasten), 0.0, 1.0 - z.h);
    n
}

/// Größte Fläche im Seitenverhältnis `verhaeltnis` (Breite/Höhe), die in
/// `platz` passt – damit weder Testbild noch Aufnahme verzerrt und die Zonen
/// genau auf dem Bild liegen.
pub fn einpassen(platz: egui::Vec2, verhaeltnis: f32) -> egui::Vec2 {
    let v = if verhaeltnis.is_finite() && verhaeltnis > 0.0 {
        verhaeltnis
    } else {
        16.0 / 9.0
    };
    let breite = platz.x.min(platz.y * v).max(1.0);
    egui::vec2(breite.floor(), (breite / v).floor().max(1.0))
}

/// Seitenverhältnis der Aufnahme (Puffer), Vorgabe 16:9.
pub fn verhaeltnis(dims: (u32, u32)) -> f32 {
    if dims.0 > 0 && dims.1 > 0 {
        dims.0 as f32 / dims.1 as f32
    } else {
        16.0 / 9.0
    }
}

/// „LED 0–29“ für eine Zone.
pub fn led_text(start: u32, anzahl: u16) -> String {
    if anzahl > 1 {
        format!("LED {start}–{}", start + anzahl as u32 - 1)
    } else {
        format!("LED {start}")
    }
}

/// Farben der LEDs `start … start + anzahl` eines Geräts in Bildfarben.
///
/// `leitung` ist der Puffer, wie er auf die Leitung ging: bei `reverse` liegt
/// LED 0 am Ende, bei `bgr` sind Rot und Blau getauscht. Beides wird
/// zurückgerechnet.
pub fn leds_von(leitung: &[[u8; 3]], d: &DeviceCfg, start: usize, anzahl: usize) -> Vec<[u8; 3]> {
    let ende = (start + anzahl).min(leitung.len());
    (start..ende)
        .map(|led| {
            let mut c = leitung[if d.reverse {
                leitung.len() - 1 - led
            } else {
                led
            }];
            if d.bgr {
                c.swap(0, 2);
            }
            c
        })
        .collect()
}

/// Zustand der Aufnahme als Ton für das Abzeichen in der Kopfzeile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ton {
    Neutral,
    Ok,
    Warnung,
    Fehler,
}

/// Abzeichen der Kopfzeile: Ton und Text.
pub fn live_zustand(capture: &CaptureStatus, angehalten: bool, fps: f32) -> (Ton, String) {
    if angehalten {
        return (Ton::Neutral, tr("angehalten", "paused").into());
    }
    match capture {
        CaptureStatus::Running { .. } => (Ton::Ok, format!("Live · {fps:.0} fps")),
        CaptureStatus::Error(_) => (Ton::Fehler, tr("Fehler", "error").into()),
        CaptureStatus::Starting => (Ton::Warnung, tr("startet", "starting").into()),
        CaptureStatus::Idle => (Ton::Neutral, tr("bereit", "ready").into()),
    }
}

/// Kurzname des Aufnahmezustands (Fußzeile der Bühne).
pub fn aufnahme_text(capture: &CaptureStatus) -> &'static str {
    match capture {
        CaptureStatus::Running { .. } => tr("läuft", "running"),
        CaptureStatus::Error(_) => tr("Fehler", "error"),
        CaptureStatus::Starting => tr("startet", "starting"),
        CaptureStatus::Idle => tr("bereit", "ready"),
    }
}

/// Was eine „LED“ bei diesem Gerät heißt.
pub fn einheit(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Govee => tr("Segmente", "segments"),
        DeviceKind::Nanoleaf => tr("Panels", "panels"),
        DeviceKind::Razer => tr("Spalten", "columns"),
        _ => "LEDs",
    }
}

/// Soll die Pipeline gerade senden? Von Hand angehalten, oder (Linux) das
/// Fenster hat den Fokus und würde sich selbst aufnehmen.
pub fn soll_anhalten(
    von_hand: bool,
    pause_bei_fokus: bool,
    fokus: bool,
    testbild: bool,
    nimmt_sich_aus: bool,
) -> bool {
    von_hand || (!nimmt_sich_aus && !testbild && pause_bei_fokus && fokus)
}

/// Zeigt das Fenster lebende Werte? Unter Linux nimmt die Aufnahme das eigene
/// Fenster mit; im Hintergrund würde jede neue Farbe im Fenster zurück in die
/// LEDs laufen. Deshalb friert das Fenster dann ein.
pub fn lebt(nimmt_sich_aus: bool, fokus: bool, testbild: bool) -> bool {
    nimmt_sich_aus || fokus || testbild
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    struct Attrappe {
        gefunden: Vec<Found>,
    }
    impl Netz for Attrappe {
        fn suchen(&self) -> Vec<Found> {
            self.gefunden.clone()
        }
        fn pruefen(&self, host: &str) -> Result<Found, String> {
            Ok(fund(host, "Regal", 144))
        }
        fn paaren(&self, _: &str) -> Result<String, String> {
            Ok("geheim".into())
        }
        fn openrgb(&self, _: &str, _: u16) -> Result<Vec<Found>, String> {
            Err("kein Server".into())
        }
    }

    fn fund(host: &str, name: &str, leds: u16) -> Found {
        Found {
            kind: DeviceKind::Wled,
            id: host.into(),
            name: name.into(),
            host: host.into(),
            port: 0,
            target: String::new(),
            leds,
            detail: "WLED 0.15".into(),
        }
    }

    fn kern(gefunden: Vec<Found>) -> (Kern, mpsc::Receiver<Antwort>) {
        let (tx, rx) = mpsc::channel();
        (
            Kern {
                shared: Shared::new(&Config::default()),
                netz: Arc::new(Attrappe { gefunden }),
                antworten: tx,
                wecker: Arc::new(|| {}),
                konfig_pfad: None,
                von_hand: Arc::new(AtomicBool::new(false)),
            },
            rx,
        )
    }

    fn mit_geraet() -> Oberflaeche {
        let mut cfg = Config::default();
        cfg.devices
            .push(DeviceCfg::new("192.168.1.42", "Wand", 120));
        Oberflaeche::neu(cfg)
    }

    #[test]
    fn eine_aenderung_geht_sofort_an_die_pipeline_und_spaeter_auf_die_platte() {
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        let jetzt = Instant::now();
        let mut d = o.config.devices[0].clone();
        d.brightness = 0.5;
        ausfuehren(Aktion::Geraet(0, d), &mut o, &k, jetzt);
        assert_eq!(k.shared.config().devices[0].brightness, 0.5);
        assert_eq!(o.speichern_ab, Some(jetzt + SPEICHERN_NACH));
    }

    #[test]
    fn suche_laeuft_im_hintergrund_und_bietet_nur_neues_an() {
        let (k, rx) = kern(vec![
            fund("192.168.1.42", "Wand", 120),
            fund("192.168.1.7", "Regal", 60),
        ]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::Suchen, &mut o, &k, Instant::now());
        assert_eq!(o.suche, Suche::Laeuft);
        let a = rx.recv_timeout(Duration::from_secs(2)).expect("Antwort");
        antwort(a, &mut o);
        let Suche::Fertig(liste) = &o.suche else {
            panic!("Suche nicht fertig");
        };
        assert_eq!(liste.len(), 1, "Wand ist schon eingerichtet");
        let regal = liste[0].clone();
        ausfuehren(Aktion::Hinzufuegen(regal), &mut o, &k, Instant::now());
        assert_eq!(o.config.devices.len(), 2);
        assert_eq!(o.config.devices[1].led_count, 60);
        assert_eq!(o.suche, Suche::Fertig(vec![]), "hinzugefügt verschwindet");
    }

    #[test]
    fn ein_geraet_zweimal_hinzufuegen_meldet_sich() {
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(
            Aktion::Hinzufuegen(fund("192.168.1.42", "Wand", 120)),
            &mut o,
            &k,
            Instant::now(),
        );
        assert_eq!(o.config.devices.len(), 1);
        assert!(matches!(o.meldung, Meldung::Hinweis(_)));
    }

    #[test]
    fn per_adresse_hinzufuegen_oeffnet_den_dialog() {
        let (k, _) = kern(vec![]);
        let mut o = Oberflaeche::neu(Config::default());
        ausfuehren(
            Aktion::AdresseTippen(" 10.0.0.5 ".into()),
            &mut o,
            &k,
            Instant::now(),
        );
        ausfuehren(Aktion::AdresseHinzufuegen, &mut o, &k, Instant::now());
        assert_eq!(o.config.devices[0].host, "10.0.0.5");
        assert!(o.adresse.is_empty());
        assert_eq!(o.dialog.as_ref().and_then(|d| d.index), Some(0));
    }

    #[test]
    fn pruefen_fuellt_den_entwurf() {
        let (k, rx) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::DialogOeffnen(Some(0)), &mut o, &k, Instant::now());
        ausfuehren(Aktion::DialogPruefen, &mut o, &k, Instant::now());
        assert_eq!(o.dialog.as_ref().unwrap().arbeit, Arbeit::Prueft);
        antwort(rx.recv_timeout(Duration::from_secs(2)).unwrap(), &mut o);
        let d = o.dialog.as_ref().unwrap();
        assert_eq!(d.arbeit, Arbeit::Keine);
        assert_eq!(
            (d.entwurf.name.as_str(), d.entwurf.led_count),
            ("Regal", 144)
        );
        // Erst Übernehmen schreibt in die Konfiguration.
        assert_eq!(o.config.devices[0].name, "Wand");
        ausfuehren(Aktion::DialogUebernehmen, &mut o, &k, Instant::now());
        assert_eq!(o.config.devices[0].name, "Regal");
        assert!(o.dialog.is_none());
    }

    #[test]
    fn neue_kennung_nimmt_die_zonen_mit() {
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::ZonenAufKanten, &mut o, &k, Instant::now());
        ausfuehren(Aktion::DialogOeffnen(Some(0)), &mut o, &k, Instant::now());
        let mut d = o.dialog.as_ref().unwrap().entwurf.clone();
        d.id = "wand".into();
        ausfuehren(Aktion::DialogEntwurf(d), &mut o, &k, Instant::now());
        ausfuehren(Aktion::DialogUebernehmen, &mut o, &k, Instant::now());
        assert!(o.config.zones.iter().all(|z| z.device == "wand"));
    }

    #[test]
    fn geraet_entfernen_nimmt_seine_zonen_mit() {
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::ZonenAufKanten, &mut o, &k, Instant::now());
        assert_eq!(o.config.zones.len(), 4);
        ausfuehren(Aktion::GeraetEntfernen(0), &mut o, &k, Instant::now());
        assert!(o.config.devices.is_empty() && o.config.zones.is_empty());
        assert_eq!(o.aktiv, 0);
    }

    #[test]
    fn vier_kanten_teilen_den_streifen() {
        assert_eq!(kanten_leds(120), [30, 30, 30, 30]);
        assert_eq!(kanten_leds(143), [36, 36, 36, 35]);
        assert_eq!(
            kanten_leds(2),
            [1, 1, 1, 1],
            "jede Kante mindestens eine LED"
        );
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::ZonenAufKanten, &mut o, &k, Instant::now());
        // Die Zonen folgen aufeinander: 0, 30, 60, 90.
        assert_eq!(o.config.zone_starts(), vec![0, 30, 60, 90]);
    }

    #[test]
    fn zonen_ruecken_und_auswahl_folgt() {
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::ZonenAufKanten, &mut o, &k, Instant::now());
        let zweite = o.config.zones[1].name.clone();
        ausfuehren(Aktion::ZoneRuecken(1, -1), &mut o, &k, Instant::now());
        assert_eq!(o.config.zones[0].name, zweite);
        assert_eq!(o.aktiv, 0);
        // Über den Rand hinaus passiert nichts.
        ausfuehren(Aktion::ZoneRuecken(0, -1), &mut o, &k, Instant::now());
        assert_eq!(o.config.zones[0].name, zweite);
    }

    #[test]
    fn ziehen_bleibt_im_bild_und_rastet_ein() {
        let von = Zone::preset(ZonePreset::Custom, "d".into(), 10, [0, 0, 0]);
        let zug = Zug {
            zone: 0,
            art: Zugart::Verschieben,
            von: von.clone(),
        };
        let z = zone_ziehen(&zug, 0.0123, 5.0, true);
        assert!((z.x - 0.26).abs() < 1e-6, "auf 0,5 % gerastet: {}", z.x);
        assert!((z.y - (1.0 - von.h)).abs() < 1e-6, "unten am Rand");
        assert_eq!((z.w, z.h), (von.w, von.h), "Verschieben hält die Größe");
        let gross = Zug {
            art: Zugart::Groesse,
            ..zug
        };
        let z = zone_ziehen(&gross, -1.0, 1.0, false);
        assert_eq!(z.w, ZONE_MIN);
        assert!((z.y + z.h - 1.0).abs() < 1e-6);
    }

    #[test]
    fn schubsen_klein_und_gross() {
        let z = Zone::preset(ZonePreset::Custom, "d".into(), 10, [0, 0, 0]);
        assert!((zone_schubsen(&z, 1.0, 0.0, false, true).x - 0.255).abs() < 1e-6);
        assert!((zone_schubsen(&z, 0.0, -1.0, true, true).y - 0.20).abs() < 1e-6);
        assert_eq!(zone_schubsen(&z, -100.0, 0.0, true, false).x, 0.0);
    }

    #[test]
    fn einpassen_haelt_das_seitenverhaeltnis() {
        let r = einpassen(egui::vec2(1000.0, 1000.0), 16.0 / 9.0);
        assert_eq!(r, egui::vec2(1000.0, 562.0));
        let r = einpassen(egui::vec2(1000.0, 300.0), 16.0 / 9.0);
        assert_eq!(r, egui::vec2(533.0, 300.0));
        // Ohne bekannte Größe: 16:9
        assert_eq!(verhaeltnis((0, 0)), 16.0 / 9.0);
        assert_eq!(einpassen(egui::vec2(160.0, 900.0), f32::NAN).y, 90.0);
    }

    #[test]
    fn led_text_und_leds_von_rechnen_zurueck() {
        assert_eq!(led_text(0, 30), "LED 0–29");
        assert_eq!(led_text(7, 1), "LED 7");
        let mut d = DeviceCfg::new("h", "n", 4);
        d.reverse = true;
        d.bgr = true;
        // Leitung: [blau, blau, rot, rot] in BGR → logisch [rot, rot, blau, blau]
        let leitung = [[255, 0, 0], [255, 0, 0], [0, 0, 255], [0, 0, 255]];
        assert_eq!(leds_von(&leitung, &d, 0, 2), vec![[255, 0, 0], [255, 0, 0]]);
        assert_eq!(leds_von(&leitung, &d, 3, 5), vec![[0, 0, 255]]);
        assert!(leds_von(&leitung, &d, 9, 2).is_empty());
    }

    #[test]
    fn anhalten_folgt_hand_und_fokus() {
        // Linux, Fenster vorn, Option an: anhalten
        assert!(soll_anhalten(false, true, true, false, false));
        // Testbild: kein Rückkoppeln möglich
        assert!(!soll_anhalten(false, true, true, true, false));
        // Windows/macOS nehmen das Fenster aus
        assert!(!soll_anhalten(false, true, true, false, true));
        assert!(soll_anhalten(true, false, false, false, true));
        assert!(!lebt(false, false, false));
        assert!(lebt(false, true, false) && lebt(true, false, false) && lebt(false, false, true));
    }

    #[test]
    fn bild_zuruecksetzen_laesst_den_rest() {
        let s = Settings {
            brightness: 1.7,
            target_fps: 30,
            theme: "light".into(),
            ..Settings::default()
        };
        let n = bild_standard(&s);
        assert_eq!(n.brightness, 1.0);
        assert_eq!((n.target_fps, n.theme.as_str()), (30, "light"));
    }

    #[test]
    fn dialog_ohne_gueltigen_index_legt_neu_an() {
        let (k, _) = kern(vec![]);
        let mut o = mit_geraet();
        ausfuehren(Aktion::DialogOeffnen(None), &mut o, &k, Instant::now());
        let mut d = o.dialog.as_ref().unwrap().entwurf.clone();
        d.host = "192.168.1.42".into();
        ausfuehren(Aktion::DialogEntwurf(d), &mut o, &k, Instant::now());
        ausfuehren(Aktion::DialogUebernehmen, &mut o, &k, Instant::now());
        assert_eq!(o.config.devices.len(), 2);
        assert_eq!(
            o.config.devices[1].id, "192.168.1.42#2",
            "Kennung eindeutig"
        );
    }
}
