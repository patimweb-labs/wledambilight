// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Ein Prozess: Pipeline, Tray und Fenster.
//!
//! 1. Konfiguration laden, Pipeline starten (eigener Thread: Aufnahme →
//!    Sampling → Geräte).
//! 2. Tray anmelden (Fenster zeigen, Senden anhalten, Beenden).
//! 3. Fenster (eframe) im Hauptthread auf demselben gemeinsamen Zustand.
//!
//! Schließen beendet das Programm – außer beim Autostart (`--minimized`):
//! dann läuft die Aufnahme im Tray weiter. Unter Linux wird das Fenster dazu
//! wirklich geschlossen und auf „Fenster zeigen“ neu geöffnet (winit kann
//! unter Wayland kein Fenster verstecken); unter Windows und macOS wird es
//! minimiert, weil ihr Tray die laufende Ereignisschleife braucht.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use wled_ambilight::config::{self, Config};
use wled_ambilight::i18n;
use wled_ambilight::worker::Pipeline;

use crate::app::{Fenster, Plattform};
use crate::logik::{EchtesNetz, Kern};
use crate::modell::Oberflaeche;
use crate::tray::{self, Befehl, Tray, TrayZustand};

/// Symbol des Fensters (aus `assets/wled-ambilight-logo.svg`, ADR 0002).
fn symbol() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!("../../../assets/icons/app-64.rgba").to_vec(),
        width: 64,
        height: 64,
    }
}

/// Startet Pipeline und Fenster. Kehrt zurück, wenn beides zu ist.
pub fn starten(minimiert: bool) {
    let cfg = Config::load();
    i18n::init(&cfg.settings.language);
    let pipeline = Pipeline::start(&cfg);
    let (tx, befehle) = mpsc::channel();
    let zustand = TrayZustand {
        shared: pipeline.shared.clone(),
        von_hand: Arc::new(AtomicBool::new(false)),
        befehle: tx,
        wecker: Arc::new(Mutex::new(None)),
    };
    let befehle = Rc::new(befehle);
    let tray: Rc<RefCell<Option<Tray>>> = Rc::new(RefCell::new(None));
    // Linux: das Symbol lebt in einem eigenen Thread und überdauert die
    // Fenster. Windows/macOS: es entsteht mit dem ersten Fenster.
    if !tray::MINIMIEREN {
        *tray.borrow_mut() = Tray::starten(zustand.clone());
    }
    let ende = Rc::new(Cell::new(false));
    // Ohne Tray kein unsichtbarer Start: das Fenster wäre unerreichbar.
    let mut zeigen = !(minimiert && tray.borrow().is_some()) || tray::MINIMIEREN;
    let mut erstes = true;
    loop {
        if zeigen {
            let start_minimiert = erstes && minimiert;
            erstes = false;
            let (z, t, b, e) = (
                zustand.clone(),
                Rc::clone(&tray),
                Rc::clone(&befehle),
                Rc::clone(&ende),
            );
            fenster_laufen(move |cc| {
                crate::einrichten(&cc.egui_ctx);
                // Windows: das eigene Fenster nicht mit aufnehmen.
                if let Ok(griff) = raw_window_handle::HasWindowHandle::window_handle(cc) {
                    wled_ambilight::capture::exclude_own_window(griff.as_raw());
                }
                if t.borrow().is_none() && tray::MINIMIEREN {
                    *t.borrow_mut() = Tray::starten(z.clone());
                }
                Box::new(Pult::neu(
                    &cc.egui_ctx,
                    z,
                    t,
                    b,
                    e,
                    minimiert,
                    start_minimiert,
                ))
            });
        }
        let behalten = minimiert && tray.borrow().is_some();
        if ende.get() || !behalten || tray::MINIMIEREN {
            break;
        }
        // Linux, Fenster zu, Tray da: warten, bis jemand es will.
        match befehle.recv() {
            Ok(Befehl::Zeigen) => zeigen = true,
            Ok(Befehl::Beenden) | Err(_) => break,
        }
    }
    pipeline.shutdown(Duration::from_secs(2));
}

/// Ein Fensterlauf (kehrt zurück, wenn das Fenster zu ist).
fn fenster_laufen(
    app: impl FnOnce(&eframe::CreationContext<'_>) -> Box<dyn eframe::App> + 'static,
) {
    let optionen = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WLED Ambilight")
            .with_app_id(crate::APP_ID)
            .with_icon(Arc::new(symbol()))
            .with_inner_size([1400.0, 880.0])
            .with_min_inner_size([1040.0, 660.0]),
        // Erlaubt einen zweiten Lauf im selben Prozess (Tray → Fenster).
        run_and_return: true,
        ..Default::default()
    };
    if let Err(e) = eframe::run_native("wled-ambilight", optionen, Box::new(move |cc| Ok(app(cc))))
    {
        eprintln!("wled-ambilight: Fenster: {e}");
    }
}

/// Das Fenster mit Lebenszyklus (Tray-Befehle, Schließen, Minimieren).
struct Pult {
    fenster: Fenster,
    tray: Rc<RefCell<Option<Tray>>>,
    befehle: Rc<Receiver<Befehl>>,
    ende: Rc<Cell<bool>>,
    wecker: Arc<Mutex<Option<egui::Context>>>,
    /// Mit Tray beim Schließen weiterlaufen (Autostart).
    behalten: bool,
    start_minimiert: bool,
}

impl Pult {
    fn neu(
        ctx: &egui::Context,
        z: TrayZustand,
        tray: Rc<RefCell<Option<Tray>>>,
        befehle: Rc<Receiver<Befehl>>,
        ende: Rc<Cell<bool>>,
        minimiert: bool,
        start_minimiert: bool,
    ) -> Pult {
        if let Ok(mut w) = z.wecker.lock() {
            *w = Some(ctx.clone());
        }
        let (tx, rx) = mpsc::channel();
        let weck = ctx.clone();
        let kern = Kern {
            shared: z.shared.clone(),
            netz: Arc::new(EchtesNetz),
            antworten: tx,
            wecker: Arc::new(move || weck.request_repaint()),
            konfig_pfad: Some(config::config_path()),
            von_hand: z.von_hand.clone(),
        };
        let o = Oberflaeche::neu(z.shared.config());
        let behalten = minimiert && tray.borrow().is_some();
        Pult {
            fenster: Fenster::neu(kern, rx, o, Plattform::hier()),
            tray,
            befehle,
            ende,
            wecker: z.wecker,
            behalten,
            start_minimiert: start_minimiert && behalten,
        }
    }
}

impl Drop for Pult {
    fn drop(&mut self) {
        self.fenster.speichern();
        if let Ok(mut w) = self.wecker.lock() {
            *w = None;
        }
    }
}

impl eframe::App for Pult {
    /// Fenstergrund in der Farbe der Fläche – sonst scheint beim Vergrößern
    /// das Grau von eframe durch.
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if std::mem::take(&mut self.start_minimiert) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        while let Ok(b) = self.befehle.try_recv() {
            match b {
                Befehl::Zeigen => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                Befehl::Beenden => {
                    self.ende.set(true);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.ende.get() {
            if !self.behalten {
                self.ende.set(true);
            } else if tray::MINIMIEREN {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            // Linux mit Tray: das Fenster schließt, `starten` wartet auf den
            // nächsten Tray-Befehl.
        }
        if let Some(t) = self.tray.borrow().as_ref() {
            t.auffrischen();
        }
        self.fenster.bild(ui);
    }
}
