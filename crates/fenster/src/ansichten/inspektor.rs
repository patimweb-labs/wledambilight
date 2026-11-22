// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Rechte Spalte mit drei Reitern: Zonen, Bild, App.

use egui::{RichText, Ui};
use wled_ambilight::config::Settings;
use wled_ambilight::i18n::tr;

use super::Umfeld;
use crate::bausteine::{self, Icon, Stufe, Variante};
use crate::logik;
use crate::modell::{Aktion, Oberflaeche, Reiter};
use crate::thema::{self, Farben, Thema};

pub fn inspektor(ui: &mut Ui, f: &Farben, o: &Oberflaeche, u: &Umfeld, a: &mut Vec<Aktion>) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(
            thema::SPACE_3 as i8,
            thema::SPACE_2 as i8,
        ))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (r, name) in [
                    (Reiter::Zonen, tr("Zonen", "Zones")),
                    (Reiter::Bild, tr("Bild", "Picture")),
                    (Reiter::App, "App"),
                ] {
                    let an = o.reiter == r;
                    let antwort = ui.add(
                        egui::Button::new(
                            RichText::new(name)
                                .font(bausteine::fett(thema::FONT_SIZE_SM))
                                .color(if an { f.accent_default } else { f.text_muted }),
                        )
                        .frame_when_inactive(an)
                        .selected(an),
                    );
                    antwort.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::Tab, true, an, name)
                    });
                    if antwort.clicked() && !an {
                        a.push(Aktion::Reiter(r));
                    }
                }
            });
        });
    ui.separator();
    egui::ScrollArea::vertical()
        .id_salt(("inspektor", o.reiter as u8))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(
                    thema::SPACE_3 as i8,
                    thema::SPACE_1 as i8,
                ))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    match o.reiter {
                        Reiter::Zonen => zonen(ui, f, o, a),
                        Reiter::Bild => bild(ui, f, &o.config.settings, a),
                        Reiter::App => app(ui, f, &o.config.settings, u, a),
                    }
                });
        });
}

// ------------------------------------------------------------------ Zonen ----

fn zonen(ui: &mut Ui, f: &Farben, o: &Oberflaeche, a: &mut Vec<Aktion>) {
    ui.label(bausteine::text(
        f,
        tr(
            "Die Reihenfolge dieser Liste ist die Reihenfolge auf dem Streifen.",
            "The order of this list is the order on the strip.",
        ),
        Stufe::Leise,
    ));
    let starts = o.config.zone_starts();
    let anzahl = o.config.zones.len();
    for (i, z) in o.config.zones.iter().enumerate() {
        let an = i == o.aktiv;
        let rahmen = egui::Frame::new()
            .fill(f.bg_raised)
            .stroke(egui::Stroke::new(
                thema::SIZE_BORDER,
                if an {
                    f.accent_default
                } else {
                    f.border_subtle
                },
            ))
            .inner_margin(egui::Margin::symmetric(
                thema::SPACE_2 as i8,
                thema::SPACE_1 as i8,
            ));
        // Die Zeile selbst ist anklickbar (Auswahl); die Knöpfe liegen darauf
        // und gehen vor, weil sie später gesetzt werden.
        let zeile = ui.scope_builder(
            egui::UiBuilder::new()
                .sense(egui::Sense::click())
                .id_salt(("zonenwahl", i)),
            |ui| {
                rahmen.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.push_id(("zonenzeile", i), |ui| {
                        ui.horizontal(|ui| {
                            bausteine::farbpunkt(
                                ui,
                                f,
                                o.telemetrie
                                    .zone_colors
                                    .get(i)
                                    .copied()
                                    .unwrap_or([60, 60, 60]),
                            );
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.label(
                                    bausteine::text(f, &z.name, Stufe::Normal)
                                        .size(thema::FONT_SIZE_SM),
                                );
                                let geraet = o
                                    .config
                                    .device(&z.device)
                                    .map(|d| d.name.as_str())
                                    .unwrap_or(&z.device);
                                ui.label(bausteine::text(
                                    f,
                                    format!(
                                        "{} · {geraet}",
                                        logik::led_text(
                                            starts.get(i).copied().unwrap_or(0),
                                            z.count
                                        )
                                    ),
                                    Stufe::Zahl,
                                ));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    if bausteine::icon_knopf(
                                        ui,
                                        f,
                                        Icon::Muell,
                                        &format!("{} {}", z.name, tr("entfernen", "remove")),
                                        true,
                                    )
                                    .clicked()
                                    {
                                        a.push(Aktion::ZoneEntfernen(i));
                                    }
                                    if bausteine::icon_knopf(
                                        ui,
                                        f,
                                        Icon::Runter,
                                        &format!("{} {}", z.name, tr("nach unten", "down")),
                                        i + 1 < anzahl,
                                    )
                                    .clicked()
                                    {
                                        a.push(Aktion::ZoneRuecken(i, 1));
                                    }
                                    if bausteine::icon_knopf(
                                        ui,
                                        f,
                                        Icon::Hoch,
                                        &format!("{} {}", z.name, tr("nach oben", "up")),
                                        i > 0,
                                    )
                                    .clicked()
                                    {
                                        a.push(Aktion::ZoneRuecken(i, -1));
                                    }
                                },
                            );
                        });
                    });
                })
            },
        );
        let name = z.name.clone();
        zeile
            .response
            .widget_info(|| egui::WidgetInfo::selected(egui::Role::ListItem, true, an, &name));
        if zeile.response.clicked() && !an {
            a.push(Aktion::ZoneWaehlen(i));
        }
    }
    if o.config.zones.is_empty() {
        ui.label(bausteine::text(
            f,
            tr("Noch keine Zonen", "No zones yet"),
            Stufe::Normal,
        ));
        ui.label(bausteine::text(
            f,
            tr(
                "Zone anlegen und auf den Bildschirm ziehen.",
                "Create a zone and drag it onto the screen.",
            ),
            Stufe::Leise,
        ));
    }
    let geraete = o.config.devices.len();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = thema::SPACE_1;
        if bausteine::knopf(
            ui,
            f,
            Some(Icon::Plus),
            tr("Zone", "Zone"),
            Variante::Normal,
            geraete > 0,
        )
        .clicked()
        {
            a.push(Aktion::ZoneNeu);
        }
        if bausteine::knopf(
            ui,
            f,
            None,
            tr("Auf 4 Kanten", "Split to 4 edges"),
            Variante::Normal,
            geraete > 0,
        )
        .clicked()
        {
            a.push(Aktion::ZonenAufKanten);
        }
        if bausteine::knopf(
            ui,
            f,
            None,
            tr("Gerät pro Kante", "Device per edge"),
            Variante::Normal,
            geraete > 1,
        )
        .clicked()
        {
            a.push(Aktion::GeraetJeKante);
        }
    });

    let Some(z) = o.config.zones.get(o.aktiv) else {
        return;
    };
    let i = o.aktiv;
    ui.separator();
    bausteine::abschnitt(ui, f, tr("Ausgewählte Zone", "Selected zone"));
    let mut neu = z.clone();
    if let Some(t) = bausteine::feld(ui, f, tr("Name", "Name"), &z.name, "") {
        neu.name = t;
    }
    let geraete: Vec<(String, String)> = o
        .config
        .devices
        .iter()
        .map(|d| (d.id.clone(), d.name.clone()))
        .collect();
    if let Some(d) = bausteine::auswahl(ui, f, tr("Gerät", "Device"), &z.device, &geraete) {
        neu.device = d;
    }
    if let Some(n) = bausteine::zahl(
        ui,
        f,
        tr("Anzahl LEDs", "LED count"),
        z.count as u32,
        1..=4096,
    ) {
        neu.count = n as u16;
    }
    let auto = logik::led_text(o.config.zone_starts().get(i).copied().unwrap_or(0), z.count);
    let offset = z.offset.map(|v| v.to_string()).unwrap_or_default();
    if let Some(t) = bausteine::feld(
        ui,
        f,
        tr("LED-Start auf dem Gerät", "LED start on the device"),
        &offset,
        &format!("{} = {auto}", tr("leer: automatisch", "empty: automatic")),
    ) {
        let t = t.trim();
        if t.is_empty() {
            neu.offset = None;
        } else if let Ok(v) = t.parse::<u16>() {
            neu.offset = Some(v);
        }
    }
    if let Some(an) = bausteine::schalter(ui, f, z.enabled, tr("Aktiv", "Active")) {
        neu.enabled = an;
    }
    if let Some(an) = bausteine::schalter(
        ui,
        f,
        z.vertical,
        tr(
            "Senkrecht (LEDs von oben nach unten)",
            "Vertical (LEDs top to bottom)",
        ),
    ) {
        neu.vertical = an;
    }
    bausteine::abschnitt(ui, f, tr("Größe im Aufnahmebild", "Size in the capture"));
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Breite", "Width"),
        (z.w * 100.0).round(),
        2.0..=100.0,
        1.0,
        prozent,
    ) {
        neu.w = (w / 100.0).min(1.0 - z.x);
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Höhe", "Height"),
        (z.h * 100.0).round(),
        2.0..=100.0,
        1.0,
        prozent,
    ) {
        neu.h = (w / 100.0).min(1.0 - z.y);
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Verstärkung", "Gain"),
        (z.gain * 100.0).round(),
        20.0..=200.0,
        1.0,
        prozent,
    ) {
        neu.gain = w / 100.0;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = thema::SPACE_1;
        if bausteine::knopf(
            ui,
            f,
            None,
            tr("Volle Breite", "Full width"),
            Variante::Normal,
            true,
        )
        .clicked()
        {
            neu.x = 0.0;
            neu.w = 1.0;
        }
        if bausteine::knopf(
            ui,
            f,
            None,
            tr("Volle Höhe", "Full height"),
            Variante::Normal,
            true,
        )
        .clicked()
        {
            neu.y = 0.0;
            neu.h = 1.0;
        }
        if bausteine::knopf(
            ui,
            f,
            None,
            tr("An Kante", "To edge"),
            Variante::Normal,
            true,
        )
        .clicked()
        {
            (neu.x, neu.y, neu.w, neu.h) = (0.0, 0.0, 1.0, 0.1);
        }
    });
    if neu != *z {
        a.push(Aktion::Zone(i, neu));
    }
}

fn prozent(w: f32) -> String {
    format!("{w:.0} %")
}

// ------------------------------------------------------------------- Bild ----

fn bild(ui: &mut Ui, f: &Farben, s: &Settings, a: &mut Vec<Aktion>) {
    let mut neu = s.clone();
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Helligkeit", "Brightness"),
        (s.brightness * 100.0).round(),
        0.0..=200.0,
        1.0,
        prozent,
    ) {
        neu.brightness = w / 100.0;
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Sättigung", "Saturation"),
        (s.saturation * 100.0).round(),
        0.0..=200.0,
        1.0,
        prozent,
    ) {
        neu.saturation = w / 100.0;
    }
    if let Some(w) = bausteine::regler(ui, f, "Gamma", s.gamma, 0.5..=3.0, 0.05, |w| {
        format!("{w:.2}")
    }) {
        neu.gamma = w;
    }
    bausteine::abschnitt(ui, f, tr("Bewegung", "Motion"));
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Zeitliche Glättung", "Temporal smoothing"),
        s.smoothing_ms,
        0.0..=600.0,
        10.0,
        |w| format!("{w:.0} ms"),
    ) {
        neu.smoothing_ms = w;
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Räumliche Glättung", "Spatial smoothing"),
        (s.spatial * 100.0).round(),
        0.0..=100.0,
        1.0,
        prozent,
    ) {
        neu.spatial = w / 100.0;
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Max. Änderung pro Sekunde", "Max. change per second"),
        s.slew,
        0.0..=2000.0,
        50.0,
        |w| {
            if w == 0.0 {
                tr("unbegrenzt", "unlimited").to_string()
            } else {
                format!("{w:.0}")
            }
        },
    ) {
        neu.slew = w;
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Bildrate", "Frame rate"),
        s.target_fps as f32,
        10.0..=120.0,
        5.0,
        |w| format!("{w:.0} fps"),
    ) {
        neu.target_fps = w as u32;
    }
    if neu != *s {
        a.push(Aktion::Einstellungen(Box::new(neu)));
    }
    ui.add_space(thema::SPACE_2);
    if bausteine::knopf(
        ui,
        f,
        Some(Icon::Neu),
        tr("Auf Standard zurücksetzen", "Reset to defaults"),
        Variante::Normal,
        true,
    )
    .clicked()
    {
        a.push(Aktion::BildZuruecksetzen);
    }
}

// -------------------------------------------------------------------- App ----

fn app(ui: &mut Ui, f: &Farben, s: &Settings, u: &Umfeld, a: &mut Vec<Aktion>) {
    let mut neu = s.clone();
    bausteine::abschnitt(ui, f, tr("Aufnahme", "Capture"));
    if let Some(an) = bausteine::schalter(
        ui,
        f,
        s.test_image || u.testbild_erzwungen,
        tr(
            "Testbild statt Bildschirm",
            "Test picture instead of screen",
        ),
    ) {
        neu.test_image = an;
    }
    if u.testbild_erzwungen {
        ui.label(bausteine::text(
            f,
            tr(
                "AMBILIGHT_DEMO=1 erzwingt das Testbild.",
                "AMBILIGHT_DEMO=1 forces the test picture.",
            ),
            Stufe::Leise,
        ));
    }
    if u.monitorwahl {
        let monitore: Vec<(usize, String)> = u
            .monitore
            .iter()
            .enumerate()
            .map(|(i, m)| (i, m.clone()))
            .collect();
        if let Some(m) = bausteine::auswahl(ui, f, tr("Monitor", "Display"), &s.monitor, &monitore)
        {
            neu.monitor = m;
        }
    }
    let breiten: Vec<(u32, String)> = std::iter::once((0, tr("nativ", "native").to_string()))
        .chain([160u32, 240, 320, 480, 640].map(|w| (w, format!("{w} px"))))
        .collect();
    if let Some(w) = bausteine::auswahl(
        ui,
        f,
        tr("Auflösung", "Resolution"),
        &s.capture_width,
        &breiten,
    ) {
        neu.capture_width = w;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = thema::SPACE_1;
        let neu_waehlen = if u.monitorwahl {
            tr("Aufnahme neu starten", "Restart capture")
        } else {
            tr("Anderen Monitor wählen", "Choose another display")
        };
        if bausteine::knopf(
            ui,
            f,
            Some(Icon::Bildschirm),
            neu_waehlen,
            Variante::Normal,
            true,
        )
        .clicked()
        {
            a.push(Aktion::AufnahmeNeu);
        }
        if !u.monitorwahl
            && bausteine::knopf(
                ui,
                f,
                None,
                tr("Berechtigung verwerfen", "Forget permission"),
                Variante::Normal,
                true,
            )
            .clicked()
        {
            a.push(Aktion::BerechtigungVerwerfen);
        }
    });

    bausteine::abschnitt(ui, f, tr("Bedienung", "Handling"));
    if let Some(an) = bausteine::schalter(
        ui,
        f,
        s.snap,
        tr("Beim Ziehen einrasten", "Snap while dragging"),
    ) {
        neu.snap = an;
    }
    if !u.monitorwahl {
        if let Some(an) = bausteine::schalter(
            ui,
            f,
            s.pause_when_focused,
            tr(
                "Anhalten, wenn dieses Fenster den Fokus hat",
                "Pause while this window is focused",
            ),
        ) {
            neu.pause_when_focused = an;
        }
    }

    bausteine::abschnitt(ui, f, tr("Design", "Design"));
    let sprachen = [
        ("auto".to_string(), tr("System", "System").to_string()),
        ("de".to_string(), "Deutsch".to_string()),
        ("en".to_string(), "English".to_string()),
    ];
    let sprache = match s.language.as_str() {
        "de" | "en" => s.language.clone(),
        _ => "auto".to_string(),
    };
    if let Some(l) = bausteine::auswahl(ui, f, tr("Sprache", "Language"), &sprache, &sprachen) {
        neu.language = l;
    }
    ui.label(bausteine::text(
        f,
        tr("Farbschema", "Theme"),
        Stufe::Gedaempft,
    ));
    let themen: Vec<(Thema, String)> = Thema::ALLE
        .iter()
        .map(|t| (*t, t.anzeige().to_string()))
        .collect();
    if let Some(t) = bausteine::umschalter(ui, f, &Thema::aus_name(&s.theme), &themen) {
        neu.theme = t.name().to_string();
    }
    if neu != *s {
        a.push(Aktion::Einstellungen(Box::new(neu)));
    }

    bausteine::abschnitt(ui, f, tr("Über", "About"));
    ui.label(bausteine::text(
        f,
        format!(
            "WLED Ambilight {} · GPL-3.0-or-later · Patrick Weppelmann",
            env!("CARGO_PKG_VERSION")
        ),
        Stufe::Leise,
    ));
}
