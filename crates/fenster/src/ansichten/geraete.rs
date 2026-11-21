// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Gerätespalte: eine Karte je Gerät, alles direkt bedienbar, dazu die
//! Netzwerksuche. Die Reihenfolge folgt der Konfiguration, weil sie auch die
//! Reihenfolge der Ausgabe ist.

use egui::{RichText, Ui};
use wled_ambilight::config::{DeviceCfg, DeviceKind, Protocol};
use wled_ambilight::i18n::tr;
use wled_ambilight::output::Status;

use crate::bausteine::{self, Icon, Stufe, Variante};
use crate::logik::{self, Ton};
use crate::modell::{Aktion, Oberflaeche, Suche};
use crate::thema::{self, Farben};

pub fn spalte(ui: &mut Ui, f: &Farben, o: &Oberflaeche, a: &mut Vec<Aktion>) {
    let rand = egui::Margin::symmetric(thema::SPACE_3 as i8, thema::SPACE_2 as i8);
    // Kopf der Spalte
    egui::Frame::new().inner_margin(rand).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(tr("Geräte", "Devices"))
                    .font(bausteine::fett(thema::FONT_SIZE_MD))
                    .color(f.text_default),
            );
            ui.label(bausteine::text(
                f,
                o.config.devices.len().to_string(),
                Stufe::Zahl,
            ));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if bausteine::icon_knopf(
                    ui,
                    f,
                    Icon::Plus,
                    tr("Gerät einrichten", "Set up device"),
                    true,
                )
                .clicked()
                {
                    a.push(Aktion::DialogOeffnen(None));
                }
                let sucht = o.suche == Suche::Laeuft;
                if bausteine::icon_knopf(
                    ui,
                    f,
                    Icon::Suchen,
                    tr("Netzwerk durchsuchen", "Search network"),
                    !sucht,
                )
                .clicked()
                {
                    a.push(Aktion::Suchen);
                }
            });
        });
        suche(ui, f, o, a);
    });

    // Unten: per Adresse hinzufügen
    egui::Panel::bottom("adresse")
        .resizable(false)
        .exact_size(thema::FONT_SIZE_XS + thema::SIZE_CONTROL_SM + 4.0 * thema::SPACE_2)
        .frame(
            egui::Frame::new()
                .inner_margin(rand)
                .stroke(egui::Stroke::new(thema::SIZE_BORDER, f.border_subtle)),
        )
        .show(ui, |ui| {
            ui.label(bausteine::text(
                f,
                tr("Per Adresse hinzufügen", "Add by address"),
                Stufe::Leise,
            ));
            ui.horizontal(|ui| {
                let mut adresse = o.adresse.clone();
                let feld = ui.add(
                    egui::TextEdit::singleline(&mut adresse)
                        .hint_text("192.168.1.42")
                        .desired_width(
                            ui.available_width() - thema::SIZE_CONTROL_SM - thema::SPACE_2,
                        ),
                );
                let name = tr("Adresse", "Address");
                feld.widget_info(|| {
                    egui::WidgetInfo::text_edit(true, o.adresse.as_str(), adresse.as_str(), name)
                });
                if adresse != o.adresse {
                    a.push(Aktion::AdresseTippen(adresse));
                }
                let eingabe = feld.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let knopf = bausteine::icon_knopf(
                    ui,
                    f,
                    Icon::Plus,
                    tr("Adresse hinzufügen", "Add address"),
                    !o.adresse.trim().is_empty(),
                );
                if knopf.clicked() || eingabe {
                    a.push(Aktion::AdresseHinzufuegen);
                }
            });
        });

    egui::ScrollArea::vertical()
        .id_salt("geraeteliste")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(thema::SPACE_2 as i8, 0))
                .show(ui, |ui| {
                    if o.config.devices.is_empty() {
                        leer(ui, f, a);
                    }
                    for (i, d) in o.config.devices.iter().enumerate() {
                        karte(ui, f, o, i, d, a);
                        ui.add_space(thema::SPACE_1);
                    }
                });
        });
}

fn suche(ui: &mut Ui, f: &Farben, o: &Oberflaeche, a: &mut Vec<Aktion>) {
    match &o.suche {
        Suche::Ruhe => {}
        Suche::Laeuft => {
            ui.horizontal(|ui| {
                bausteine::warten(ui, f);
                ui.label(bausteine::text(
                    f,
                    tr("Suche läuft …", "Searching …"),
                    Stufe::Gedaempft,
                ));
            });
        }
        Suche::Fertig(liste) if liste.is_empty() => {
            ui.label(bausteine::text(
                f,
                tr("Keine neuen Geräte gefunden.", "No new devices found."),
                Stufe::Leise,
            ));
        }
        Suche::Fertig(liste) => {
            ui.label(bausteine::text(f, tr("Gefunden", "Found"), Stufe::Leise));
            for fund in liste {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            bausteine::text(f, &fund.name, Stufe::Normal).size(thema::FONT_SIZE_SM),
                        );
                        ui.label(bausteine::text(
                            f,
                            format!("{} · {}", fund.kind.label(), fund.detail),
                            Stufe::Leise,
                        ));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let name = format!("{} {}", fund.name, tr("hinzufügen", "add"));
                        if bausteine::icon_knopf(ui, f, Icon::Plus, &name, true).clicked() {
                            a.push(Aktion::Hinzufuegen(fund.clone()));
                        }
                    });
                });
            }
        }
    }
}

fn leer(ui: &mut Ui, f: &Farben, a: &mut Vec<Aktion>) {
    bausteine::karte(f).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical_centered(|ui| {
            ui.label(bausteine::text(
                f,
                tr("Noch kein Gerät eingerichtet", "No device set up yet"),
                Stufe::Normal,
            ));
            ui.label(bausteine::text(
                f,
                tr(
                    "Per Adresse hinzufügen oder im Netzwerk suchen.",
                    "Add by address or search the network.",
                ),
                Stufe::Leise,
            ));
            if bausteine::knopf(
                ui,
                f,
                Some(Icon::Suchen),
                tr("Netzwerk durchsuchen", "Search network"),
                Variante::Primaer,
                true,
            )
            .clicked()
            {
                a.push(Aktion::Suchen);
            }
        });
    });
}

fn karte(ui: &mut Ui, f: &Farben, o: &Oberflaeche, i: usize, d: &DeviceCfg, a: &mut Vec<Aktion>) {
    let mut neu = d.clone();
    bausteine::karte(f).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.push_id(("geraet", i), |ui| {
            ui.horizontal(|ui| {
                let name = ui
                    .add(
                        egui::Label::new(
                            RichText::new(&d.name)
                                .font(bausteine::fett(thema::FONT_SIZE_MD))
                                .color(f.accent_default),
                        )
                        .truncate()
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_text(tr("Bearbeiten", "Edit"));
                if name.clicked() {
                    a.push(Aktion::DialogOeffnen(Some(i)));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let name = format!("{} {}", d.name, tr("aktiv", "active"));
                    if let Some(an) = bausteine::schalter_ohne_text(ui, f, d.enabled, &name) {
                        neu.enabled = an;
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label(bausteine::text(f, &d.host, Stufe::Zahl));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (ton, text) = zustand(o, d);
                    let r = bausteine::abzeichen(ui, f, ton, &text);
                    if let Some(Status::Error(e)) = o
                        .telemetrie
                        .devices
                        .iter()
                        .find(|s| s.id == d.id)
                        .map(|s| &s.status)
                    {
                        r.on_hover_text(e);
                    }
                });
            });
            let nutzung = o.config.device_usage(&d.id);
            ui.label(bausteine::text(
                f,
                format!(
                    "{} / {} {} {}",
                    nutzung.used,
                    d.led_count,
                    logik::einheit(d.kind),
                    tr("belegt", "used")
                ),
                Stufe::Leise,
            ));
            bausteine::fortschritt(
                ui,
                f,
                nutzung.used as f32 / d.led_count.max(1) as f32,
                nutzung.problem,
            );
            if d.kind == DeviceKind::Wled {
                if let Some(p) = bausteine::umschalter(
                    ui,
                    f,
                    &d.protocol,
                    &[
                        (Protocol::Ddp, "DDP 4048".into()),
                        (Protocol::Drgb, "DRGB 21324".into()),
                    ],
                ) {
                    neu.protocol = p;
                }
            }
            if let Some(w) = bausteine::regler(
                ui,
                f,
                tr("Helligkeit", "Brightness"),
                (d.brightness * 100.0).round(),
                0.0..=100.0,
                1.0,
                |w| format!("{w:.0} %"),
            ) {
                neu.brightness = w / 100.0;
            }
            let fps = [
                (0u32, tr("Global", "Global").to_string()),
                (24, "24".into()),
                (30, "30".into()),
                (60, "60".into()),
                (120, "120".into()),
            ];
            if let Some(w) = bausteine::auswahl(
                ui,
                f,
                tr("Max. Bildrate", "Max. frame rate"),
                &d.max_fps,
                &fps,
            ) {
                neu.max_fps = w;
            }
            if let Some(an) = bausteine::schalter(
                ui,
                f,
                d.reverse,
                tr("Streifen läuft rückwärts", "Strip runs backwards"),
            ) {
                neu.reverse = an;
            }
            if let Some(an) = bausteine::schalter(
                ui,
                f,
                d.bgr,
                tr("Rot und Blau tauschen", "Swap red and blue"),
            ) {
                neu.bgr = an;
            }
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = thema::SPACE_1;
                if bausteine::knopf(
                    ui,
                    f,
                    None,
                    tr("LEDs aus", "LEDs off"),
                    Variante::Normal,
                    d.enabled,
                )
                .clicked()
                {
                    neu.enabled = false;
                }
                if bausteine::knopf(
                    ui,
                    f,
                    Some(Icon::Regler),
                    tr("Bearbeiten", "Edit"),
                    Variante::Normal,
                    true,
                )
                .clicked()
                {
                    a.push(Aktion::DialogOeffnen(Some(i)));
                }
                if bausteine::knopf(
                    ui,
                    f,
                    Some(Icon::Muell),
                    tr("Entfernen", "Remove"),
                    Variante::Gefahr,
                    true,
                )
                .clicked()
                {
                    a.push(Aktion::GeraetEntfernen(i));
                }
            });
        });
    });
    if neu != *d {
        a.push(Aktion::Geraet(i, neu));
    }
}

/// Verbindungszustand eines Geräts als Abzeichen.
fn zustand(o: &Oberflaeche, d: &DeviceCfg) -> (Ton, String) {
    if !d.enabled {
        return (Ton::Neutral, tr("aus", "off").into());
    }
    match o
        .telemetrie
        .devices
        .iter()
        .find(|s| s.id == d.id)
        .map(|s| &s.status)
    {
        Some(Status::Ok) => (Ton::Ok, tr("verbunden", "connected").into()),
        Some(Status::Error(_)) => (Ton::Fehler, tr("Fehler", "error").into()),
        _ => (Ton::Neutral, tr("verbindet", "connecting").into()),
    }
}
