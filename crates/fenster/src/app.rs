// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Die Anwendung: Messwerte und Vorschau einsammeln, zeichnen, Aktionen
//! ausführen, entprellt speichern. Jedes Bild ist `render(modell)`
//! (AGENTS.md §2.1).

use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use wled_ambilight::capture;
use wled_ambilight::telemetry;

use crate::ansichten::{self, Umfeld};
use crate::logik::{self, Antwort, Kern};
use crate::modell::{Aktion, Oberflaeche, Vorschau};
use crate::thema::{self, Thema};
use crate::vorschau;

/// Was das Fenster über die Plattform wissen muss – als Daten, nicht als
/// Plattformweiche (AGENTS.md §2.10).
#[derive(Debug, Clone)]
pub struct Plattform {
    /// Nimmt die Aufnahme das eigene Fenster aus (Windows, macOS)?
    pub nimmt_sich_aus: bool,
    /// Kann die App den Monitor wählen (sonst fragt das Portal)?
    pub monitorwahl: bool,
    pub monitore: Vec<String>,
    /// Testbild von außen erzwungen (`AMBILIGHT_DEMO=1`)
    pub testbild_erzwungen: bool,
}

impl Plattform {
    /// Die Plattform, auf der das Programm läuft.
    pub fn hier() -> Self {
        Plattform {
            nimmt_sich_aus: capture::EXCLUDES_OWN_WINDOW,
            monitorwahl: capture::HAS_MONITOR_CHOICE,
            monitore: if capture::HAS_MONITOR_CHOICE {
                capture::monitors()
            } else {
                Vec::new()
            },
            testbild_erzwungen: capture::test_picture_forced(),
        }
    }
}

pub struct Fenster {
    pub kern: Kern,
    pub o: Oberflaeche,
    antworten: Receiver<Antwort>,
    plattform: Plattform,
    textur: Option<egui::TextureHandle>,
    angewandt: Option<Thema>,
    /// Vorschaubilder seit `zaehler_seit`
    zaehler: (u32, Instant),
}

impl Fenster {
    pub fn neu(
        kern: Kern,
        antworten: Receiver<Antwort>,
        o: Oberflaeche,
        plattform: Plattform,
    ) -> Self {
        Fenster {
            kern,
            o,
            antworten,
            plattform,
            textur: None,
            angewandt: None,
            zaehler: (0, Instant::now()),
        }
    }

    /// Eine Aktion von außen ausführen (Tray, Tests).
    pub fn ausfuehren(&mut self, a: Aktion) {
        logik::ausfuehren(a, &mut self.o, &self.kern, Instant::now());
    }

    /// Ausstehende Änderungen sofort auf die Platte (Beenden).
    pub fn speichern(&mut self) {
        if self.o.speichern_ab.take().is_some() {
            if let Some(pfad) = &self.kern.konfig_pfad {
                if let Err(e) = self.kern.shared.config().save_to(pfad) {
                    eprintln!("wled-ambilight: {} nicht geschrieben: {e}", pfad.display());
                }
            }
        }
    }

    /// Das Bild der Aufnahme in die Textur, wenn ein neues da ist.
    fn vorschau(&mut self, ctx: &egui::Context, jetzt: Instant) {
        let rahmen = match self.kern.shared.frame.lock() {
            Ok(r) => r.clone(),
            Err(_) => return,
        };
        if self.o.vorschau == (Vorschau::Bild { seq: rahmen.seq }) {
            return;
        }
        if let Some(bild) = vorschau::bild(&rahmen, vorschau::MAX_BREITE) {
            match &mut self.textur {
                Some(t) => t.set(bild, egui::TextureOptions::LINEAR),
                None => {
                    self.textur =
                        Some(ctx.load_texture("vorschau", bild, egui::TextureOptions::LINEAR))
                }
            }
            self.o.vorschau = Vorschau::Bild { seq: rahmen.seq };
            self.zaehler.0 += 1;
        }
        let dauer = jetzt - self.zaehler.1;
        if dauer >= Duration::from_secs(1) {
            self.o.vorschau_fps = self.zaehler.0 as f32 / dauer.as_secs_f32();
            self.zaehler = (0, jetzt);
        }
    }

    /// Ein Bild: einsammeln, zeichnen, ausführen.
    pub fn bild(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let thema = self.o.thema();
        if self.angewandt != Some(thema) {
            thema::anwenden(&ctx, thema);
            self.angewandt = Some(thema);
        }
        let jetzt = Instant::now();
        while let Ok(a) = self.antworten.try_recv() {
            logik::antwort(a, &mut self.o);
        }
        // Tray und Fenster teilen sich „von Hand angehalten“.
        self.o.angehalten = self.kern.von_hand.load(Relaxed);

        let fokus = ctx.input(|i| i.focused);
        let p = self.plattform.clone();
        let testbild = p.testbild_erzwungen || self.o.config.settings.test_image;
        let lebt = logik::lebt(p.nimmt_sich_aus, fokus, testbild);
        self.o.eingefroren = !lebt;
        let anhalten = logik::soll_anhalten(
            self.o.angehalten,
            self.o.config.settings.pause_when_focused,
            fokus,
            testbild,
            p.nimmt_sich_aus,
        );
        self.kern.shared.paused.store(anhalten, Relaxed);
        // Ein eingefrorenes Fenster braucht nach dem Start trotzdem ein Bild.
        let nie_gelesen = self.o.vorschau == Vorschau::Leer;
        if lebt || nie_gelesen {
            self.o.telemetrie = telemetry::read(&self.kern.shared, &self.o.config);
            self.vorschau(&ctx, jetzt);
        }

        let umfeld = Umfeld {
            vorschau: self.textur.as_ref().map(|t| t.id()),
            monitore: p.monitore.clone(),
            monitorwahl: p.monitorwahl,
            testbild_erzwungen: p.testbild_erzwungen,
        };
        let aktionen = ansichten::fenster(ui, &self.o, &umfeld);
        for a in aktionen {
            logik::ausfuehren(a, &mut self.o, &self.kern, jetzt);
        }
        handzeiger(&ctx);
        if self.o.speichern_ab.is_some_and(|t| jetzt >= t) {
            self.speichern();
        }
        ctx.request_repaint_after(Duration::from_millis(match (fokus, lebt) {
            (true, _) => 16,
            (false, true) => 100,
            (false, false) => 250,
        }));
    }
}

/// Handzeiger über allem, was man anklicken kann.
///
/// egui setzt `Visuals::interact_cursor` nur bei `Button`; Schalter,
/// Aufklapplisten, Reiter, Zeilen und Zonen blieben beim Pfeil. Hier einmal je
/// Bild für alle: liegt der Zeiger über einem klickbaren Element und hat noch
/// niemand einen eigenen Zeiger gewählt (Textfeld, Größengriff), wird es die
/// Hand. Übernommen aus crosspoint.
pub fn handzeiger(ctx: &egui::Context) {
    let unter_zeiger: Vec<egui::Id> =
        ctx.viewport(|vp| vp.interact_widgets.hovered.iter().copied().collect());
    let klickbar = unter_zeiger.into_iter().any(|id| {
        ctx.read_response(id)
            .is_some_and(|r| r.enabled() && r.sense.senses_click())
    });
    if klickbar {
        ctx.output_mut(|o| {
            if o.cursor_icon == egui::CursorIcon::Default {
                o.cursor_icon = egui::CursorIcon::PointingHand;
            }
        });
    }
}
