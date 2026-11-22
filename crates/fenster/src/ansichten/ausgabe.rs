// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Untere Leiste: je Gerät ein Streifen mit den Farben, die gerade
//! hinausgehen, oben rechts die Messwerte der Pipeline.

use egui::{RichText, Ui};
use wled_ambilight::i18n::tr;

use crate::bausteine::{self, Stufe};
use crate::modell::Oberflaeche;
use crate::thema::{self, Farben};

pub fn ausgabe(ui: &mut Ui, f: &Farben, o: &Oberflaeche) {
    let t = &o.telemetrie;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(tr("Ausgabe", "Output"))
                .font(bausteine::fett(thema::FONT_SIZE_SM))
                .color(f.text_default),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(bausteine::text(
                f,
                format!(
                    "capture {}×{} · in {:.0} fps · out {:.0} fps · sampling {} µs",
                    t.dims.0, t.dims.1, t.fps_in, t.fps_out, t.sample_us
                ),
                Stufe::Zahl,
            ));
        });
    });
    for (i, d) in o.config.devices.iter().enumerate() {
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(thema::SIZE_ZEILE, thema::FONT_SIZE_SM),
                egui::Sense::hover(),
            );
            ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                ui.add(egui::Label::new(bausteine::text(f, &d.name, Stufe::Gedaempft)).truncate())
                    .on_hover_text(&d.host);
            });
            let leds = t.device_leds.get(i).map(Vec::as_slice).unwrap_or(&[]);
            bausteine::farbleiste(
                ui,
                f,
                leds,
                &format!("{} {}", tr("Ausgabe", "Output"), d.name),
            );
        });
    }
}
