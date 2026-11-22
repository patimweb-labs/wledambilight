// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Das Fenster ohne Aufnahme, ohne Netz und ohne Geräte (AGENTS.md §4): ein
//! Beispielzustand hinein, über AccessKit bedienen (Rollen und Namen müssen
//! stimmen, sonst findet der Test nichts), den gemeinsamen Zustand prüfen.
//!
//! Mit `WLED_BILDER=<verzeichnis>` schreibt der Lauf außerdem die
//! Bildschirmfotos für README und Doku – erzeugt, nicht von Hand abgelegt.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{mpsc, Arc};

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use wled_ambilight::capture::{CaptureStatus, SharedFrame};
use wled_ambilight::config::{Config, DeviceCfg, DeviceKind, Zone, ZonePreset};
use wled_ambilight::discovery::Found;
use wled_ambilight::i18n::{self, Lang};
use wled_ambilight::output::Status;
use wled_ambilight::worker::Shared;
use wled_ambilight_fenster::app::{Fenster, Plattform};
use wled_ambilight_fenster::logik::{Kern, Netz};
use wled_ambilight_fenster::modell::{Aktion, Oberflaeche};
use wled_ambilight_fenster::thema::Thema;

struct OhneNetz;
impl Netz for OhneNetz {
    fn suchen(&self) -> Vec<Found> {
        vec![Found {
            kind: DeviceKind::Wled,
            id: "192.168.1.77".into(),
            name: "Regal".into(),
            host: "192.168.1.77".into(),
            port: 0,
            target: String::new(),
            leds: 60,
            detail: "WLED 0.15.1".into(),
        }]
    }
    fn pruefen(&self, _: &str) -> Result<Found, String> {
        Err("kein Netz im Test".into())
    }
    fn paaren(&self, _: &str) -> Result<String, String> {
        Err("kein Netz im Test".into())
    }
    fn openrgb(&self, _: &str, _: u16) -> Result<Vec<Found>, String> {
        Ok(vec![])
    }
}

/// Farbe des Testbildes an einer Stelle: ein Regenbogen von links nach
/// rechts, nach unten dunkler.
fn regenbogen(x: f32, y: f32) -> [u8; 3] {
    let h = x * 6.0;
    let c = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as u32 {
        0 => (1.0, c, 0.0),
        1 => (c, 1.0, 0.0),
        2 => (0.0, 1.0, c),
        3 => (0.0, c, 1.0),
        4 => (c, 0.0, 1.0),
        _ => (1.0, 0.0, c),
    };
    let v = 1.0 - 0.6 * y;
    [
        (r * v * 255.0) as u8,
        (g * v * 255.0) as u8,
        (b * v * 255.0) as u8,
    ]
}

/// Ein Aufbau wie im Alltag: WLED hinter dem Monitor auf vier Kanten,
/// eine Tastatur über OpenRGB, die Aufnahme läuft.
fn beispiel(thema: Thema) -> (Fenster, Arc<Shared>) {
    i18n::set(Lang::De);
    let mut cfg = Config::default();
    cfg.settings.language = "de".into();
    cfg.settings.theme = thema.name().into();
    cfg.devices
        .push(DeviceCfg::new("192.168.1.42", "Monitor hinten", 120));
    let mut tastatur = DeviceCfg::of_kind(
        DeviceKind::OpenRgb,
        "openrgb:127.0.0.1:6742:Tastatur",
        "127.0.0.1",
        "Tastatur",
        104,
    );
    tastatur.port = 6742;
    cfg.devices.push(tastatur);
    for (kante, name) in [
        (ZonePreset::Top, "Oben"),
        (ZonePreset::Right, "Rechts"),
        (ZonePreset::Bottom, "Unten"),
        (ZonePreset::Left, "Links"),
    ] {
        let mut z = Zone::preset(kante, "192.168.1.42".into(), 30, [0, 0, 0]);
        z.name = name.into();
        z.reverse = kante.reversed_in_loop();
        cfg.zones.push(z);
    }
    let mut z = Zone::preset(ZonePreset::Full, tastatur_id(), 104, [0, 0, 0]);
    z.name = "Tastatur".into();
    z.y = 0.6;
    z.h = 0.4;
    cfg.zones.push(z);

    let shared = Shared::new(&cfg);
    {
        let (w, h) = (320u32, 180u32);
        let mut daten = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            for x in 0..w {
                daten.extend_from_slice(&regenbogen(x as f32 / w as f32, y as f32 / h as f32));
            }
        }
        *shared.frame.lock().unwrap() = SharedFrame {
            w,
            h,
            data: Arc::new(daten),
            seq: 1,
        };
        let mut s = shared.stats.lock().unwrap();
        s.status = CaptureStatus::Running { w, h };
        s.screen = (2560, 1440);
        s.dims = (w, h);
        s.fps_in = 50.0;
        s.fps_out = 50.0;
        s.sample_us = 140;
        s.devices.insert("192.168.1.42".into(), Status::Ok);
        s.devices.insert(tastatur_id(), Status::Connecting);
        *shared.last.lock().unwrap() = vec![
            (0..120)
                .map(|i| regenbogen(i as f32 / 120.0, 0.0))
                .collect(),
            (0..104)
                .map(|i| regenbogen(i as f32 / 104.0, 0.8))
                .collect(),
        ];
    }
    let (tx, rx) = mpsc::channel();
    let kern = Kern {
        shared: shared.clone(),
        netz: Arc::new(OhneNetz),
        antworten: tx,
        wecker: Arc::new(|| {}),
        konfig_pfad: None,
        von_hand: Arc::new(AtomicBool::new(false)),
    };
    let plattform = Plattform {
        nimmt_sich_aus: true,
        monitorwahl: false,
        monitore: vec![],
        testbild_erzwungen: false,
    };
    (
        Fenster::neu(kern, rx, Oberflaeche::neu(cfg), plattform),
        shared,
    )
}

fn tastatur_id() -> String {
    "openrgb:127.0.0.1:6742:Tastatur".into()
}

fn harness(f: Fenster, groesse: (f32, f32)) -> Harness<'static, Fenster> {
    Harness::builder()
        .with_size(groesse)
        .with_step_dt(1.0 / 30.0)
        .with_max_steps(8)
        .with_context_setup(wled_ambilight_fenster::einrichten)
        .build_ui_state(|ui, f: &mut Fenster| f.bild(ui), f)
}

#[test]
fn pausieren_haelt_die_pipeline_an() {
    let (f, shared) = beispiel(Thema::Dunkel);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(3);
    assert!(h.query_by_label("Live · 50 fps").is_some(), "Abzeichen");
    h.get_by_label("Pausieren").click();
    h.run_steps(2);
    assert!(shared.paused.load(Relaxed));
    assert!(h.state().kern.von_hand.load(Relaxed), "Tray sieht es auch");
    h.get_by_label("Fortsetzen").click();
    h.run_steps(2);
    assert!(!shared.paused.load(Relaxed));
}

#[test]
fn geraetekarte_schreibt_in_die_pipeline() {
    let (f, shared) = beispiel(Thema::Dunkel);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(3);
    // Jede Karte hat den Schalter; der erste gehört zum ersten Gerät.
    h.get_all_by_label("Streifen läuft rückwärts")
        .next()
        .expect("Schalter")
        .click();
    h.run_steps(2);
    assert!(shared.config().devices[0].reverse);
    // Nur WLED hat die Protokollwahl.
    assert_eq!(h.query_all_by_label("DRGB 21324").count(), 1);
    h.get_by_label("DRGB 21324").click();
    h.run_steps(2);
    assert_eq!(
        shared.config().devices[0].protocol,
        wled_ambilight::config::Protocol::Drgb
    );
}

#[test]
fn geraet_entfernen_nimmt_die_zonen_mit() {
    let (f, shared) = beispiel(Thema::Dunkel);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(3);
    h.get_all_by_label("Entfernen")
        .next()
        .expect("Entfernen")
        .click();
    h.run_steps(2);
    let cfg = shared.config();
    assert_eq!(cfg.devices.len(), 1);
    assert_eq!(cfg.zones.len(), 1, "nur die Tastatur-Zone bleibt");
    assert!(h.query_by_label("Zone Oben").is_none());
}

#[test]
fn dialog_bearbeitet_einen_entwurf() {
    let (f, shared) = beispiel(Thema::Hell);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(3);
    h.get_all_by_label("Bearbeiten")
        .next()
        .expect("Bearbeiten")
        .click();
    h.run_steps(3);
    assert!(
        h.query_by_label("Dialog Monitor hinten").is_some(),
        "Dialog offen"
    );
    assert!(
        h.query_by_label("Gerät prüfen").is_some(),
        "WLED kann geprüft werden"
    );
    h.get_by_label("Abbrechen").click();
    h.run_steps(3);
    assert!(
        h.query_by_label("Dialog Monitor hinten").is_none(),
        "Dialog zu"
    );
    assert_eq!(shared.config().devices[0].name, "Monitor hinten");
}

#[test]
fn suche_bietet_neue_geraete_an() {
    let (f, shared) = beispiel(Thema::Dunkel);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(2);
    h.get_by_label("Netzwerk durchsuchen").click();
    // Die Suche läuft im Hintergrund; ein paar Bilder später ist sie da.
    for _ in 0..40 {
        h.run_steps(1);
        if h.query_by_label("Regal hinzufügen").is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    h.get_by_label("Regal hinzufügen").click();
    h.run_steps(2);
    assert_eq!(shared.config().devices.len(), 3);
}

#[test]
fn zonen_liste_und_pfeiltasten() {
    let (f, shared) = beispiel(Thema::Dunkel);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(3);
    // Reihenfolge ändern: „Rechts“ nach oben
    h.get_by_label("Rechts nach oben").click();
    h.run_steps(2);
    assert_eq!(shared.config().zones[0].name, "Rechts");
    // Eine Zone auf der Bühne anklicken und mit Pfeiltasten schieben.
    // Die Tastatur-Zone ist volle Breite und liegt unten: nur nach oben ist Platz.
    let vorher = shared.config().zones[4].y;
    h.get_by_label("Zone Tastatur").click();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    let nachher = shared.config().zones[4].y;
    assert!(
        nachher < vorher,
        "Pfeil hoch schiebt ({vorher} → {nachher})"
    );
    // Mit Umschalt in großen Schritten (5 %).
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowUp);
    h.run_steps(2);
    let gross = shared.config().zones[4].y;
    assert!((nachher - gross - 0.05).abs() < 1e-4, "{nachher} → {gross}");
}

/// Issue #32: Senkrecht und Richtung sind zwei eigene Schalter, die sich
/// nicht gegenseitig ausschalten.
#[test]
fn zonenrichtung_ist_unabhaengig_von_senkrecht() {
    let (f, shared) = beispiel(Thema::Dunkel);
    // Hoch genug, dass der Inspektor ohne Scrollen alle Schalter zeigt.
    let mut h = harness(f, (1400.0, 1300.0));
    h.run_steps(3);
    h.get_by_label("Zone Rechts").click();
    h.run_steps(2);
    h.get_by_label("Richtung umkehren (LED 0 am anderen Ende)")
        .click();
    h.run_steps(2);
    let z = &shared.config().zones[1];
    assert_eq!(z.name, "Rechts");
    assert!(z.reverse && z.vertical, "beides an");
    // Links lief schon rückwärts; Senkrecht ausschalten lässt die Richtung.
    h.get_by_label("Zone Links").click();
    h.run_steps(2);
    h.get_by_label("Senkrecht (LEDs von oben nach unten)")
        .click();
    h.run_steps(2);
    let z = &shared.config().zones[3];
    assert_eq!(z.name, "Links");
    assert!(z.reverse && !z.vertical, "Richtung bleibt");
}

#[test]
fn reiter_bild_und_app() {
    let (f, shared) = beispiel(Thema::Mono);
    let mut h = harness(f, (1400.0, 880.0));
    h.run_steps(2);
    h.get_by_label("Bild").click();
    h.run_steps(2);
    assert!(h.query_all_by_label("Zeitliche Glättung").count() > 0);
    h.get_by_label("App").click();
    h.run_steps(2);
    h.get_by_label("Hell").click();
    h.run_steps(3);
    assert_eq!(shared.config().settings.theme, "light");
    assert_eq!(h.state().o.thema(), Thema::Hell);
}

#[test]
fn bilder_in_allen_themes() {
    let ziel = std::env::var_os("WLED_BILDER").map(PathBuf::from);
    for thema in Thema::ALLE {
        for (breite, hoehe, name) in [(1400.0, 880.0, "1400"), (1040.0, 660.0, "min")] {
            let (f, _) = beispiel(thema);
            let mut h = harness(f, (breite, hoehe));
            h.run_steps(6);
            assert!(h.query_by_label("Zone Oben").is_some(), "{}", thema.name());
            if let Some(ziel) = &ziel {
                let bild = h.render().expect("zeichnen (wgpu)");
                std::fs::create_dir_all(ziel).unwrap();
                bild.save(ziel.join(format!("fenster-{}-{name}.png", thema.name())))
                    .unwrap();
            }
        }
        // Der Gerätedialog
        let (mut f, _) = beispiel(thema);
        f.ausfuehren(Aktion::DialogOeffnen(Some(0)));
        let mut h = harness(f, (1400.0, 880.0));
        h.run_steps(6);
        assert!(h.query_by_label("Dialog Monitor hinten").is_some());
        if let Some(ziel) = &ziel {
            let bild = h.render().expect("zeichnen (wgpu)");
            bild.save(ziel.join(format!("dialog-{}.png", thema.name())))
                .unwrap();
        }
    }
}
