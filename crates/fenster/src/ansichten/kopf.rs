// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Kopfzeile: Marke links, Zustand daneben, Handlungen rechts. Darunter, wenn
//! es eine gibt, die Meldung.

use egui::{RichText, Ui};
use wled_ambilight::capture::CaptureStatus;
use wled_ambilight::i18n::tr;

use crate::bausteine::{self, Icon, Stufe, Variante};
use crate::logik;
use crate::modell::{Aktion, Meldung, Oberflaeche};
use crate::thema::{self, Farben};

pub fn kopf(ui: &mut Ui, f: &Farben, o: &Oberflaeche, a: &mut Vec<Aktion>) {
    let t = &o.telemetrie;
    ui.horizontal(|ui| {
        ui.set_height(thema::SIZE_CONTROL_MD);
        ui.spacing_mut().item_spacing.x = thema::SPACE_3;
        bausteine::zeichen(ui, thema::SIZE_CONTROL_MD);
        ui.label(
            RichText::new("WLED Ambilight")
                .font(bausteine::fett(thema::FONT_SIZE_LG))
                .color(f.text_default),
        );
        let (ton, text) = logik::live_zustand(&t.capture, o.angehalten || t.paused, t.fps_in);
        bausteine::abzeichen(ui, f, ton, &text);
        if let CaptureStatus::Error(fehler) = &t.capture {
            ui.add(
                egui::Label::new(
                    RichText::new(format!("{}: {fehler}", tr("Aufnahme", "Capture")))
                        .font(egui::FontId::monospace(thema::FONT_SIZE_XS))
                        .color(f.status_error),
                )
                .truncate(),
            )
            .on_hover_text(fehler);
        }
        if o.eingefroren {
            ui.label(bausteine::text(
                f,
                tr(
                    "Anzeige ruht, solange das Fenster im Hintergrund ist",
                    "Display rests while the window is in the background",
                ),
                Stufe::Leise,
            ));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (icon, text, variante) = if o.angehalten {
                (
                    Icon::Abspielen,
                    tr("Fortsetzen", "Resume"),
                    Variante::Primaer,
                )
            } else {
                (Icon::Pause, tr("Pausieren", "Pause"), Variante::Normal)
            };
            if bausteine::knopf(ui, f, Some(icon), text, variante, true).clicked() {
                a.push(Aktion::Anhalten(!o.angehalten));
            }
            if bausteine::knopf(
                ui,
                f,
                Some(Icon::Neu),
                tr("Aufnahme neu starten", "Restart capture"),
                Variante::Normal,
                true,
            )
            .clicked()
            {
                a.push(Aktion::AufnahmeNeu);
            }
            ui.separator();
            // Testbild: ohne Bildschirmfreigabe prüfen, ob Zonen, Richtung
            // und Farben stimmen.
            if let Some(an) = bausteine::schalter(
                ui,
                f,
                o.config.settings.test_image,
                tr("Testbild", "Test picture"),
            ) {
                let mut s = o.config.settings.clone();
                s.test_image = an;
                a.push(Aktion::Einstellungen(Box::new(s)));
            }
        });
    });
    let (farbe, text) = match &o.meldung {
        Meldung::Keine => return,
        Meldung::Hinweis(t) => (f.text_muted, t),
        Meldung::Fehler(t) => (f.status_error, t),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).color(farbe).size(thema::FONT_SIZE_SM));
        if bausteine::icon_knopf(
            ui,
            f,
            Icon::Schliessen,
            tr("Meldung schließen", "Dismiss"),
            true,
        )
        .clicked()
        {
            a.push(Aktion::MeldungWeg);
        }
    });
}
