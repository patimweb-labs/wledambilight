// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Mitte: der Bildschirm mit den Zonen. Diese Fläche ist das Herz der App –
//! hier wird gezogen, skaliert und gesehen, was ankommt.
//!
//! Die Zonen sind normalisierte Koordinaten (0..1), dieselben Werte, die der
//! Kern speichert; die Fläche nimmt die größtmögliche Größe im
//! Seitenverhältnis der Aufnahme ein. Die Füllfarbe einer Zone ist die Farbe,
//! die zuletzt für sie hinausging – so sieht man sofort, ob Aufnahme,
//! Zuordnung und Ausgabe zusammenpassen.

use egui::{Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use wled_ambilight::i18n::tr;

use super::Umfeld;
use crate::bausteine::{self, Stufe};
use crate::logik;
use crate::modell::{Aktion, Oberflaeche, Zug, Zugart};
use crate::thema::{self, Farben};

pub fn buehne(ui: &mut Ui, f: &Farben, o: &Oberflaeche, u: &Umfeld, a: &mut Vec<Aktion>) {
    let gesamt = ui.available_rect_before_wrap();
    let fuss = thema::SIZE_CONTROL_SM;
    let platz = Vec2::new(
        gesamt.width(),
        (gesamt.height() - fuss - thema::SPACE_2).max(1.0),
    );
    let groesse = logik::einpassen(platz, logik::verhaeltnis(o.telemetrie.dims));
    let flaeche = Rect::from_center_size(
        Pos2::new(gesamt.center().x, gesamt.top() + platz.y / 2.0),
        groesse,
    );
    ui.allocate_rect(Rect::from_min_size(gesamt.min, platz), Sense::hover());

    let p = ui.painter().clone();
    p.rect(
        flaeche,
        CornerRadius::ZERO,
        f.bg_input,
        Stroke::new(thema::SIZE_BORDER, f.border_strong),
        StrokeKind::Outside,
    );
    match u.vorschau {
        Some(textur) => {
            p.image(
                textur,
                flaeche,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => bausteine::mittig(ui, flaeche, tr("Bildschirm", "Screen"), f.text_faint),
    }

    let starts = o.config.zone_starts();
    for (i, z) in o.config.zones.iter().enumerate() {
        zone(
            ui,
            f,
            o,
            flaeche,
            i,
            z,
            starts.get(i).copied().unwrap_or(0),
            a,
        );
    }

    // Fußzeile mit den Messwerten
    let t = &o.telemetrie;
    let zeile = Rect::from_min_size(
        Pos2::new(gesamt.left(), gesamt.bottom() - fuss),
        Vec2::new(gesamt.width(), fuss),
    );
    p.hline(
        zeile.x_range(),
        zeile.top(),
        Stroke::new(thema::SIZE_BORDER, f.border_subtle),
    );
    ui.scope_builder(egui::UiBuilder::new().max_rect(zeile), |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = thema::SPACE_3;
            // Schmale Bühne (kleinstes Fenster): nur, was zum Zonen setzen zählt.
            let breit = zeile.width() >= 2.0 * thema::SIZE_PANEL_INSPEKTOR;
            let teile = [
                breit.then(|| {
                    format!(
                        "{} {}×{}",
                        tr("Bildschirm", "Screen"),
                        t.screen.0,
                        t.screen.1
                    )
                }),
                Some(format!(
                    "{} {}×{}",
                    tr("Puffer", "Buffer"),
                    t.dims.0,
                    t.dims.1
                )),
                Some(format!("{} {}", o.config.zones.len(), tr("Zonen", "zones"))),
                breit.then(|| format!("{:.0} fps {}", o.vorschau_fps, tr("Vorschau", "preview"))),
            ];
            for teil in teile.into_iter().flatten() {
                ui.label(bausteine::text(f, teil, Stufe::Zahl));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(bausteine::text(
                    f,
                    format!(
                        "{}: {}",
                        tr("Aufnahme", "Capture"),
                        logik::aufnahme_text(&t.capture)
                    ),
                    Stufe::Zahl,
                ));
            });
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn zone(
    ui: &mut Ui,
    f: &Farben,
    o: &Oberflaeche,
    flaeche: Rect,
    i: usize,
    z: &wled_ambilight::config::Zone,
    start: u32,
    a: &mut Vec<Aktion>,
) {
    let rect = Rect::from_min_size(
        flaeche.min + Vec2::new(z.x * flaeche.width(), z.y * flaeche.height()),
        Vec2::new(z.w * flaeche.width(), z.h * flaeche.height()),
    );
    let id = ui.scope_id().with(("zone", i));
    let antwort = ui.interact(rect, id, Sense::click_and_drag());
    let griff = Rect::from_min_size(
        rect.max - Vec2::splat(thema::SIZE_GRIFF),
        Vec2::splat(thema::SIZE_GRIFF),
    );
    let griff_antwort = ui
        .interact(griff, id.with("griff"), Sense::drag())
        .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
    let name = format!("{} {}", tr("Zone", "Zone"), z.name);
    antwort.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    let griff_name = format!("{} {}", tr("Größe", "Size"), z.name);
    griff_antwort.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &griff_name));

    // Ziehen: Anfang, Weg, Ende
    let einrasten = o.config.settings.snap;
    if griff_antwort.drag_started() {
        a.push(Aktion::ZugBeginnen(Zug {
            zone: i,
            art: Zugart::Groesse,
            von: z.clone(),
        }));
    } else if antwort.drag_started() {
        a.push(Aktion::ZugBeginnen(Zug {
            zone: i,
            art: Zugart::Verschieben,
            von: z.clone(),
        }));
    }
    if let Some(zug) = o.zug.as_ref().filter(|zug| zug.zone == i) {
        if antwort.dragged() || griff_antwort.dragged() {
            let weg = ui.input(
                |e| match (e.pointer.press_origin(), e.pointer.latest_pos()) {
                    (Some(von), Some(bis)) => bis - von,
                    _ => Vec2::ZERO,
                },
            );
            let neu = logik::zone_ziehen(
                zug,
                weg.x / flaeche.width(),
                weg.y / flaeche.height(),
                einrasten,
            );
            if neu != *z {
                a.push(Aktion::Zone(i, neu));
            }
        }
    }
    if antwort.drag_stopped() || griff_antwort.drag_stopped() {
        a.push(Aktion::ZugEnde);
    }
    if antwort.clicked() {
        a.push(Aktion::ZoneWaehlen(i));
        antwort.request_focus();
    }
    if antwort.has_focus() {
        // Pfeiltasten gehören der Zone, nicht der Fokusnavigation von egui.
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        let (dx, dy, gross) = ui.input(|e| {
            let mut d = (0.0, 0.0);
            for (taste, schritt) in [
                (egui::Key::ArrowLeft, (-1.0, 0.0)),
                (egui::Key::ArrowRight, (1.0, 0.0)),
                (egui::Key::ArrowUp, (0.0, -1.0)),
                (egui::Key::ArrowDown, (0.0, 1.0)),
            ] {
                if e.key_pressed(taste) {
                    d.0 += schritt.0;
                    d.1 += schritt.1;
                }
            }
            (d.0, d.1, e.modifiers.shift)
        });
        if dx != 0.0 || dy != 0.0 {
            let neu = logik::zone_schubsen(z, dx, dy, gross, einrasten);
            if neu != *z {
                a.push(Aktion::Zone(i, neu));
            }
        }
    }

    // Zeichnen
    let aktiv = i == o.aktiv;
    let farbe = o
        .telemetrie
        .zone_colors
        .get(i)
        .copied()
        .unwrap_or([0, 0, 0]);
    let leuchtet = farbe.iter().map(|c| *c as u32).sum::<u32>() > 6;
    let fuellung = if leuchtet {
        Color32::from_rgb(farbe[0], farbe[1], farbe[2]).gamma_multiply(0.55)
    } else {
        f.accent_default.gamma_multiply(0.12)
    };
    let deckkraft = if z.enabled { 1.0 } else { 0.4 };
    let p = ui.painter_at(flaeche);
    p.rect(
        rect,
        CornerRadius::ZERO,
        fuellung.gamma_multiply(deckkraft),
        Stroke::new(
            if aktiv {
                thema::SIZE_FOCUS
            } else {
                thema::SIZE_BORDER
            },
            f.accent_default.gamma_multiply(deckkraft),
        ),
        StrokeKind::Inside,
    );
    if antwort.has_focus() {
        p.rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(thema::SIZE_FOCUS, f.focus_ring),
            StrokeKind::Outside,
        );
    }
    p.rect_filled(
        griff,
        CornerRadius::ZERO,
        f.accent_default.gamma_multiply(deckkraft),
    );
    // Nummer und Beschriftung
    let schrift = FontId::monospace(thema::FONT_SIZE_XS);
    let nummer = (i + 1).to_string();
    let marke = Rect::from_min_size(
        rect.min + Vec2::splat(thema::SPACE_1),
        Vec2::new(
            thema::FONT_SIZE_XS * (0.4 + 0.65 * nummer.len() as f32),
            thema::FONT_SIZE_XS + 2.0,
        ),
    );
    let innen = ui.painter_at(rect.intersect(flaeche));
    innen.rect_filled(marke, CornerRadius::ZERO, f.accent_default);
    innen.text(
        marke.center(),
        egui::Align2::CENTER_CENTER,
        &nummer,
        schrift.clone(),
        f.text_on_accent,
    );
    let beschriftung = format!("{} · {}", z.name, logik::led_text(start, z.count));
    let pos = Pos2::new(marke.right() + thema::SPACE_1, marke.center().y);
    // Schatten unter der Schrift, damit sie auf jedem Bild lesbar bleibt.
    innen.text(
        pos + Vec2::splat(1.0),
        egui::Align2::LEFT_CENTER,
        &beschriftung,
        schrift.clone(),
        f.bg_input,
    );
    innen.text(
        pos,
        egui::Align2::LEFT_CENTER,
        &beschriftung,
        schrift,
        f.text_default,
    );
}
