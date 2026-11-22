// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Die Ansichten: `render(modell) → Aktionen`.
//!
//! Gelesen wird aus [`Oberflaeche`], geschrieben nichts – jede Bedienung wird
//! zur [`Aktion`], die `logik::ausfuehren` ausführt. Der Aufbau ist der der
//! Web-Oberfläche: Kopfzeile, links Geräte, Mitte Bildschirm mit Zonen, rechts
//! Inspektor (Zonen, Bild, App), unten Ausgabe.

pub mod ausgabe;
pub mod buehne;
pub mod dialog;
pub mod geraete;
pub mod inspektor;
pub mod kopf;

use crate::modell::{Aktion, Oberflaeche};
use crate::thema;

/// Was die Ansichten außer dem Modell brauchen und nicht selbst rechnen.
#[derive(Debug, Clone, Default)]
pub struct Umfeld {
    /// Vorschaubild der Aufnahme (Textur), falls schon eins da ist
    pub vorschau: Option<egui::TextureId>,
    /// Monitore zur Auswahl (leer unter Linux: dort fragt das Portal)
    pub monitore: Vec<String>,
    /// Kann die App den Monitor selbst wählen?
    pub monitorwahl: bool,
    /// Testbild von außen erzwungen (`AMBILIGHT_DEMO=1`)
    pub testbild_erzwungen: bool,
}

/// Das ganze Fenster in einem Bild.
pub fn fenster(ui: &mut egui::Ui, o: &Oberflaeche, u: &Umfeld) -> Vec<Aktion> {
    let ctx = ui.ctx().clone();
    let f = o.thema().farben();
    let mut a = Vec::new();
    // Panels stoßen aneinander; der Abstand gehört in die Panels, nicht
    // zwischen sie (sonst scheint der Fenstergrund als Spalt durch).
    let abstand = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    let innen = |ui: &mut egui::Ui| ui.spacing_mut().item_spacing = abstand;

    // Feste Höhen: ein Panel merkt sich sonst seine Größe aus dem ersten
    // Bild, und was die Fläche nicht füllt, bleibt als Streifen stehen.
    let meldung = !matches!(o.meldung, crate::modell::Meldung::Keine);
    let kopf_hoehe = thema::SIZE_CONTROL_MD
        + 2.0 * thema::SPACE_2
        + if meldung {
            thema::SIZE_CONTROL_SM + thema::SPACE_2
        } else {
            0.0
        };
    egui::Panel::top("kopf")
        .resizable(false)
        .exact_size(kopf_hoehe)
        .frame(
            crate::bausteine::rahmen(f, f.bg_panel).inner_margin(egui::Margin::symmetric(
                thema::SPACE_3 as i8,
                thema::SPACE_2 as i8,
            )),
        )
        .show(ui, |ui| {
            innen(ui);
            kopf::kopf(ui, f, o, &mut a)
        });
    egui::Panel::bottom("ausgabe")
        .frame(
            crate::bausteine::rahmen(f, f.bg_panel).inner_margin(egui::Margin::symmetric(
                thema::SPACE_3 as i8,
                thema::SPACE_2 as i8,
            )),
        )
        .show(ui, |ui| {
            innen(ui);
            ausgabe::ausgabe(ui, f, o)
        });
    egui::Panel::left("geraete")
        .resizable(false)
        .exact_size(thema::SIZE_PANEL_GERAETE)
        .frame(crate::bausteine::rahmen(f, f.bg_panel))
        .show(ui, |ui| {
            innen(ui);
            geraete::spalte(ui, f, o, &mut a)
        });
    egui::Panel::right("inspektor")
        .resizable(false)
        .exact_size(thema::SIZE_PANEL_INSPEKTOR)
        .frame(crate::bausteine::rahmen(f, f.bg_panel))
        .show(ui, |ui| {
            innen(ui);
            inspektor::inspektor(ui, f, o, u, &mut a)
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(f.bg_canvas)
                .inner_margin(egui::Margin::same(thema::SPACE_3 as i8)),
        )
        .show(ui, |ui| {
            innen(ui);
            buehne::buehne(ui, f, o, u, &mut a)
        });
    dialog::geraet(&ctx, f, o, &mut a);
    a
}
