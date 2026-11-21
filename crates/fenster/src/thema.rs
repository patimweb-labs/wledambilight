// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Drei Themes und nichts sonst (AGENTS.md §2.7). Die Werte kommen aus den
//! Token-Dateien (`build.rs`); hier wird nur zugeordnet, welcher Token welche
//! Fläche von egui färbt. Aufbau wie in crosspoint.

use egui::{self, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

/// Das gewählte Farbschema.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Thema {
    #[default]
    Dunkel,
    Hell,
    Mono,
}

impl Thema {
    pub const ALLE: [Thema; 3] = [Thema::Dunkel, Thema::Hell, Thema::Mono];

    /// Aus dem gespeicherten Namen; Unbekanntes (auch die alten Namen wie
    /// „Crosspoint“) wird `dark`, ohne die Datei anzufassen.
    pub fn aus_name(name: &str) -> Self {
        match name {
            "light" => Thema::Hell,
            "mono" => Thema::Mono,
            _ => Thema::Dunkel,
        }
    }

    /// Name in der Konfiguration.
    pub fn name(self) -> &'static str {
        match self {
            Thema::Dunkel => "dark",
            Thema::Hell => "light",
            Thema::Mono => "mono",
        }
    }

    /// Anzeigename in der Oberfläche.
    pub fn anzeige(self) -> &'static str {
        match self {
            Thema::Dunkel => wled_ambilight::i18n::tr("Dunkel", "Dark"),
            Thema::Hell => wled_ambilight::i18n::tr("Hell", "Light"),
            Thema::Mono => "Mono",
        }
    }

    pub fn farben(self) -> &'static Farben {
        match self {
            Thema::Dunkel => &DUNKEL,
            Thema::Hell => &HELL,
            Thema::Mono => &MONO,
        }
    }
}

/// Schriftfamilie für Überschriften und Knöpfe (Inter SemiBold).
pub fn fett() -> FontFamily {
    FontFamily::Name("fett".into())
}

/// Inter für Text, die Monospace von egui für Zahlen und Adressen.
pub fn schriften(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    defs.font_data.insert(
        "inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    defs.font_data.insert(
        "inter-fett".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../../../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    let rueckfall = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    defs.families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".into());
    let mut fett_liste = vec!["inter-fett".to_string()];
    fett_liste.extend(rueckfall);
    defs.families.insert(fett(), fett_liste);
    ctx.set_fonts(defs);
}

/// Wendet ein Theme auf egui an.
pub fn anwenden(ctx: &egui::Context, thema: Thema) {
    let f = thema.farben();
    let ecke = CornerRadius::same(RADIUS_MD as u8);
    let mut style = (*ctx.global_style()).clone();

    let mut v = if thema == Thema::Hell {
        egui::Visuals::light()
    } else {
        egui::Visuals::dark()
    };
    v.override_text_color = Some(f.text_default);
    v.panel_fill = f.bg_canvas;
    v.window_fill = f.bg_panel;
    v.window_stroke = Stroke::new(SIZE_BORDER, f.border_strong);
    v.window_corner_radius = ecke;
    v.menu_corner_radius = ecke;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.extreme_bg_color = f.bg_input;
    v.faint_bg_color = f.bg_raised;
    v.code_bg_color = f.bg_input;
    v.hyperlink_color = f.accent_default;
    v.warn_fg_color = f.status_warn;
    v.error_fg_color = f.status_error;
    v.selection.bg_fill = f.accent_deep;
    v.selection.stroke = Stroke::new(SIZE_BORDER, f.accent_default);
    v.text_cursor.stroke = Stroke::new(SIZE_FOCUS, f.focus_ring);
    v.interact_cursor = Some(egui::CursorIcon::PointingHand);
    v.slider_trailing_fill = true;

    let w = &mut v.widgets;
    for (zustand, flaeche, linie, schrift) in [
        (
            &mut w.noninteractive,
            f.bg_panel,
            f.border_subtle,
            f.text_default,
        ),
        (
            &mut w.inactive,
            f.bg_raised,
            f.border_subtle,
            f.text_default,
        ),
        (&mut w.hovered, f.bg_hover, f.border_strong, f.text_default),
        (&mut w.active, f.bg_selected, f.focus_ring, f.text_default),
        (&mut w.open, f.bg_selected, f.border_strong, f.text_default),
    ] {
        zustand.bg_fill = flaeche;
        zustand.weak_bg_fill = flaeche;
        zustand.bg_stroke = Stroke::new(SIZE_BORDER, linie);
        zustand.fg_stroke = Stroke::new(SIZE_BORDER, schrift);
        zustand.corner_radius = ecke;
        zustand.expansion = 0.0;
    }
    // Reglerschiene und Schalterbahn sind `inactive.bg_fill`: auf der Karte
    // (`bg_raised`) muss sie sich abheben (Test unten).
    w.inactive.bg_fill = f.border_strong;
    style.visuals = v;

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(SPACE_2, SPACE_2);
    s.button_padding = egui::vec2(SPACE_2, SPACE_1);
    s.interact_size.y = SIZE_CONTROL_SM;
    s.window_margin = egui::Margin::same(SPACE_4 as i8);
    s.menu_margin = egui::Margin::same(SPACE_1 as i8);
    // Scrollleisten nur beim Scrollen, schmal und ohne Spur (wie zuvor im Web).
    s.scroll = egui::style::ScrollStyle::floating();

    style.text_styles = [
        (TextStyle::Small, FontId::proportional(FONT_SIZE_XS)),
        (TextStyle::Body, FontId::proportional(FONT_SIZE_MD)),
        (TextStyle::Button, FontId::new(FONT_SIZE_SM, fett())),
        (TextStyle::Monospace, FontId::monospace(FONT_SIZE_SM)),
        (TextStyle::Heading, FontId::new(FONT_SIZE_LG, fett())),
    ]
    .into();
    ctx.set_global_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namen_hin_und_zurueck() {
        for t in Thema::ALLE {
            assert_eq!(Thema::aus_name(t.name()), t);
        }
        // Alte Theme-Namen aus der Konfiguration werden dark.
        assert_eq!(Thema::aus_name("Crosspoint"), Thema::Dunkel);
        assert_eq!(Thema::aus_name("rosa"), Thema::Dunkel);
    }

    #[test]
    fn themes_unterscheiden_sich() {
        assert_ne!(DUNKEL.bg_canvas, HELL.bg_canvas);
        assert_ne!(DUNKEL.accent_default, MONO.accent_default);
        for f in [&DUNKEL, &HELL, &MONO] {
            assert_ne!(f.text_default, f.bg_panel);
            assert_ne!(f.accent_default, f.signal_default);
        }
    }

    /// Relative Leuchtdichte nach WCAG 2.1.
    fn leuchtdichte(c: egui::Color32) -> f32 {
        let kanal = |wert: u8| {
            let s = wert as f32 / 255.0;
            if s <= 0.039_28 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * kanal(c.r()) + 0.7152 * kanal(c.g()) + 0.0722 * kanal(c.b())
    }

    fn kontrast(a: egui::Color32, b: egui::Color32) -> f32 {
        let (x, y) = (leuchtdichte(a), leuchtdichte(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Jedes Theme muss Text, Akzent und Signal so tragen, dass man sie liest
    /// (WCAG AA: 4,5 für kleinen Text, 3,0 für Linien und Flächen).
    #[test]
    fn kontrast_traegt_in_allen_drei_themes() {
        for (name, f) in [("dark", &DUNKEL), ("light", &HELL), ("mono", &MONO)] {
            for flaeche in [f.bg_canvas, f.bg_panel, f.bg_raised] {
                assert!(
                    kontrast(f.text_default, flaeche) >= 7.0,
                    "{name}: Text auf Fläche zu schwach"
                );
                assert!(
                    kontrast(f.text_muted, flaeche) >= 4.5,
                    "{name}: gedämpfter Text zu schwach"
                );
                assert!(
                    kontrast(f.accent_default, flaeche) >= 3.0,
                    "{name}: Akzent auf Fläche zu schwach"
                );
                assert!(
                    kontrast(f.signal_default, flaeche) >= 3.0,
                    "{name}: Signal auf Fläche zu schwach"
                );
                for status in [f.status_ok, f.status_warn, f.status_error] {
                    assert!(
                        kontrast(status, flaeche) >= 3.0,
                        "{name}: Statusfarbe auf Fläche zu schwach"
                    );
                }
            }
            for flaeche in [f.accent_default, f.signal_default] {
                assert!(
                    kontrast(f.text_on_accent, flaeche) >= 4.5,
                    "{name}: Schrift auf dem Akzent zu schwach"
                );
            }
        }
    }

    /// Die Schiene eines Reglers liegt auf der Karte: sie muss sich von der
    /// Kartenfläche abheben, und die Füllung von der Schiene.
    #[test]
    fn reglerschiene_hebt_sich_von_der_karte_ab() {
        for (name, f) in [("dark", &DUNKEL), ("light", &HELL), ("mono", &MONO)] {
            assert!(
                kontrast(f.border_strong, f.bg_raised) >= 1.5,
                "{name}: Schiene auf der Karte unsichtbar"
            );
            assert!(
                kontrast(f.accent_default, f.border_strong) >= 1.5,
                "{name}: Füllung hebt sich nicht von der Schiene ab"
            );
        }
    }

    /// Logo und Oberfläche dürfen nicht auseinanderlaufen: Akzent (Cyan oben
    /// am Bildschirm) und Signal (Glut unten am Fuß) stehen im Zeichen
    /// (ADR 0002).
    #[test]
    fn zeichen_und_tokens_tragen_dieselben_farben() {
        let zeichen = include_str!("../../../assets/wled-ambilight-logo.svg");
        let hex = |c: egui::Color32| format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
        for (was, farbe) in [
            ("Cyan", DUNKEL.accent_default),
            ("Glut", DUNKEL.signal_default),
        ] {
            assert!(
                zeichen.contains(&hex(farbe)),
                "{was}: {} steht nicht im Zeichen",
                hex(farbe)
            );
        }
    }
}
