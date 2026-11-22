// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Gerät einrichten: dieselben Felder wie bisher, je nach Art. WLED kennt
//! ein Protokoll, OpenRGB einen Controller, Nanoleaf einen Token, Govee ein
//! Modell. Der Dialog entscheidet nichts über das Gerät – er füllt nur den
//! Entwurf, den „Übernehmen“ in die Konfiguration schreibt.

use egui::{RichText, Ui};
use wled_ambilight::config::{DeviceCfg, DeviceKind, Protocol};
use wled_ambilight::i18n::tr;

use crate::bausteine::{self, Icon, Stufe, Variante};
use crate::modell::{Aktion, Arbeit, GeraetDialog, Oberflaeche};
use crate::thema::{self, Farben};

/// Breite des Dialogs.
const BREITE: f32 = thema::SIZE_PANEL_INSPEKTOR * 1.4;

pub fn geraet(ctx: &egui::Context, f: &Farben, o: &Oberflaeche, a: &mut Vec<Aktion>) {
    let Some(d) = &o.dialog else {
        return;
    };
    let antwort = egui::Modal::new(egui::Id::unique("geraetedialog"))
        .frame(
            bausteine::rahmen(f, f.bg_panel)
                .corner_radius(egui::CornerRadius::same(thema::RADIUS_MD as u8))
                .inner_margin(egui::Margin::same(thema::SPACE_4 as i8)),
        )
        .show(ctx, |ui| {
            ui.set_width(BREITE);
            let titel = if d.index.is_some() {
                d.entwurf.name.clone()
            } else {
                tr("Gerät einrichten", "Set up device").to_string()
            };
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&titel)
                        .font(bausteine::fett(thema::FONT_SIZE_LG))
                        .color(f.text_default),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if bausteine::icon_knopf(
                        ui,
                        f,
                        Icon::Schliessen,
                        tr("Schließen", "Close"),
                        true,
                    )
                    .clicked()
                    {
                        a.push(Aktion::DialogSchliessen);
                    }
                });
            });
            ui.separator();
            let inhalt = ui.scope_builder(egui::UiBuilder::new().id_salt("dialoginhalt"), |ui| {
                formular(ui, f, d, a)
            });
            inhalt.response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Dialog, true, format!("Dialog {titel}"))
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if bausteine::knopf(
                        ui,
                        f,
                        None,
                        tr("Übernehmen", "Apply"),
                        Variante::Primaer,
                        true,
                    )
                    .clicked()
                    {
                        a.push(Aktion::DialogUebernehmen);
                    }
                    if bausteine::knopf(
                        ui,
                        f,
                        None,
                        tr("Abbrechen", "Cancel"),
                        Variante::Normal,
                        true,
                    )
                    .clicked()
                    {
                        a.push(Aktion::DialogSchliessen);
                    }
                });
            });
        });
    if antwort.should_close() && !a.contains(&Aktion::DialogUebernehmen) {
        a.push(Aktion::DialogSchliessen);
    }
}

fn zwei(ui: &mut Ui, links: impl FnOnce(&mut Ui), rechts: impl FnOnce(&mut Ui)) {
    ui.columns(2, |s| {
        links(&mut s[0]);
        rechts(&mut s[1]);
    });
}

fn formular(ui: &mut Ui, f: &Farben, d: &GeraetDialog, a: &mut Vec<Aktion>) {
    let alt = &d.entwurf;
    let mut neu: DeviceCfg = alt.clone();
    let arbeitet = d.arbeit != Arbeit::Keine;
    zwei(
        ui,
        |ui| {
            if let Some(t) = bausteine::feld(ui, f, tr("Name", "Name"), &alt.name, "") {
                neu.name = t;
            }
        },
        |ui| {
            let arten: Vec<(DeviceKind, String)> = [
                DeviceKind::Wled,
                DeviceKind::OpenRgb,
                DeviceKind::Razer,
                DeviceKind::Govee,
                DeviceKind::Nanoleaf,
            ]
            .into_iter()
            .map(|k| (k, k.label().to_string()))
            .collect();
            if let Some(k) = bausteine::auswahl(ui, f, tr("Art", "Kind"), &alt.kind, &arten) {
                neu.kind = k;
            }
        },
    );
    zwei(
        ui,
        |ui| {
            if let Some(t) =
                bausteine::feld(ui, f, tr("Adresse", "Address"), &alt.host, "192.168.1.42")
            {
                neu.host = t;
            }
        },
        |ui| {
            if let Some(n) = bausteine::zahl(
                ui,
                f,
                tr("Anzahl LEDs", "LED count"),
                alt.led_count as u32,
                1..=4096,
            ) {
                neu.led_count = n as u16;
            }
        },
    );
    match alt.kind {
        DeviceKind::Wled => {
            if let Some(p) = bausteine::auswahl(
                ui,
                f,
                tr("Protokoll", "Protocol"),
                &alt.protocol,
                &[
                    (Protocol::Ddp, tr("DDP (schnell)", "DDP (fast)").to_string()),
                    (
                        Protocol::Drgb,
                        tr("DRGB (WLED, bis 490 LEDs)", "DRGB (WLED, up to 490 LEDs)").to_string(),
                    ),
                ],
            ) {
                neu.protocol = p;
            }
            if bausteine::knopf(
                ui,
                f,
                Some(Icon::Funk),
                tr("Gerät prüfen", "Probe device"),
                Variante::Normal,
                !arbeitet && !alt.host.trim().is_empty(),
            )
            .clicked()
            {
                a.push(Aktion::DialogPruefen);
            }
        }
        DeviceKind::OpenRgb => {
            zwei(
                ui,
                |ui| {
                    if let Some(n) =
                        bausteine::zahl(ui, f, tr("Port", "Port"), alt.port as u32, 0..=65535)
                    {
                        neu.port = n as u16;
                    }
                },
                |ui| {
                    ui.add_space(thema::FONT_SIZE_SM + thema::SPACE_2);
                    if bausteine::knopf(
                        ui,
                        f,
                        None,
                        tr("Controller holen", "Load controllers"),
                        Variante::Normal,
                        !arbeitet,
                    )
                    .clicked()
                    {
                        a.push(Aktion::DialogControllerHolen);
                    }
                },
            );
            if d.controller.is_empty() {
                if let Some(t) = bausteine::feld(
                    ui,
                    f,
                    tr("Controller", "Controller"),
                    &alt.target,
                    tr("leer = erster Controller", "empty = first controller"),
                ) {
                    neu.target = t;
                }
            } else {
                let liste: Vec<(String, String)> = d
                    .controller
                    .iter()
                    .map(|c| (c.target.clone(), c.name.clone()))
                    .collect();
                if let Some(t) =
                    bausteine::auswahl(ui, f, tr("Controller", "Controller"), &alt.target, &liste)
                {
                    if let Some(c) = d.controller.iter().find(|c| c.target == t) {
                        neu.led_count = c.leds.max(1);
                    }
                    neu.target = t;
                }
            }
        }
        DeviceKind::Nanoleaf => {
            let token = alt.token.clone().unwrap_or_default();
            if let Some(t) = bausteine::feld(
                ui,
                f,
                tr("Token", "Token"),
                &token,
                tr("einmal paaren, dann bleibt er", "pair once, then it stays"),
            ) {
                neu.token = (!t.is_empty()).then_some(t);
            }
            if bausteine::knopf(
                ui,
                f,
                None,
                tr("Paaren", "Pair"),
                Variante::Normal,
                !arbeitet && !alt.host.trim().is_empty(),
            )
            .clicked()
            {
                a.push(Aktion::DialogPaaren);
            }
        }
        DeviceKind::Govee => {
            if let Some(t) = bausteine::feld(ui, f, tr("Modell", "Model"), &alt.target, "H6199") {
                neu.target = t;
            }
        }
        DeviceKind::Razer => {}
    }
    if let Some(w) = bausteine::regler(
        ui,
        f,
        tr("Helligkeit", "Brightness"),
        (alt.brightness * 100.0).round(),
        0.0..=100.0,
        1.0,
        |w| format!("{w:.0} %"),
    ) {
        neu.brightness = w / 100.0;
    }
    if let Some(an) = bausteine::schalter(ui, f, alt.enabled, tr("Aktiv", "Active")) {
        neu.enabled = an;
    }
    if let Some(an) = bausteine::schalter(
        ui,
        f,
        alt.reverse,
        tr("Streifen läuft rückwärts", "Strip runs backwards"),
    ) {
        neu.reverse = an;
    }
    if let Some(an) = bausteine::schalter(
        ui,
        f,
        alt.bgr,
        tr("Rot und Blau tauschen", "Swap red and blue"),
    ) {
        neu.bgr = an;
    }
    if arbeitet {
        ui.horizontal(|ui| {
            bausteine::warten(ui, f);
            let was = match d.arbeit {
                Arbeit::Prueft => tr("Frage das Gerät …", "Asking the device …"),
                Arbeit::Paart => tr(
                    "Halte den Knopf am Controller gedrückt …",
                    "Hold the button on the controller …",
                ),
                Arbeit::Holt => tr("Hole Controller …", "Loading controllers …"),
                Arbeit::Keine => "",
            };
            ui.label(bausteine::text(f, was, Stufe::Gedaempft));
        });
    } else if !d.notiz.is_empty() {
        ui.label(bausteine::text(f, &d.notiz, Stufe::Gedaempft));
    }
    if neu != *alt {
        a.push(Aktion::DialogEntwurf(neu));
    }
}
