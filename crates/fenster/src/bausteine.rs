// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Bausteine der Oberfläche: Knöpfe, Schalter, Regler, Abzeichen,
//! Fortschritt, Farbleiste, Icons. Die Rolle, die zuvor die Web-Bibliothek
//! (`@weppelmann/components`) hatte.
//!
//! Jeder Baustein liest seine Farben aus dem Theme ([`Farben`]) und seine
//! Maße aus den Tokens – kein Farbwert, keine Pixelzahl ohne Namen. Werte
//! kommen herein, eine Änderung geht als Rückgabe hinaus; gespeichert wird
//! hier nichts. Jeder Baustein trägt einen zugänglichen Namen (AccessKit):
//! Screenreader und Tests finden ihn darüber.

use egui::{
    Align2, Color32, CornerRadius, FontId, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui,
    Vec2, WidgetInfo,
};

use crate::logik::Ton;
use crate::thema::{self, Farben, Thema};

pub fn fett(groesse: f32) -> FontId {
    FontId::new(groesse, thema::fett())
}

/// Rahmen eines Bereichs (Spalte, Leiste).
pub fn rahmen(f: &Farben, flaeche: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(flaeche)
        .stroke(Stroke::new(thema::SIZE_BORDER, f.border_subtle))
}

/// Eine Karte (Gerät, Zone) auf einer Spalte.
pub fn karte(f: &Farben) -> egui::Frame {
    egui::Frame::new()
        .fill(f.bg_raised)
        .stroke(Stroke::new(thema::SIZE_BORDER, f.border_subtle))
        .corner_radius(CornerRadius::same(thema::RADIUS_MD as u8))
        .inner_margin(egui::Margin::same(thema::SPACE_2 as i8))
}

/// Text in einer der drei Stufen.
pub fn text(f: &Farben, inhalt: impl Into<String>, stufe: Stufe) -> RichText {
    let t = RichText::new(inhalt);
    match stufe {
        Stufe::Normal => t.color(f.text_default),
        Stufe::Gedaempft => t.color(f.text_muted).size(thema::FONT_SIZE_SM),
        Stufe::Leise => t.color(f.text_faint).size(thema::FONT_SIZE_XS),
        Stufe::Zahl => t
            .color(f.text_muted)
            .font(FontId::monospace(thema::FONT_SIZE_XS)),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stufe {
    Normal,
    Gedaempft,
    Leise,
    /// Zahlen, Adressen, Messwerte (Monospace)
    Zahl,
}

/// Überschrift eines Abschnitts im Inspektor.
pub fn abschnitt(ui: &mut Ui, f: &Farben, titel: &str) {
    ui.add_space(thema::SPACE_1);
    ui.label(
        RichText::new(titel)
            .font(fett(thema::FONT_SIZE_SM))
            .color(f.text_default),
    );
}

// ----------------------------------------------------------------- Icons ----

/// Die Icons der Oberfläche: eine kuratierte Auswahl aus Lucide (ISC,
/// Beilage `icons/LUCIDE-LICENSE.txt`), dieselbe Sorte wie in crosspoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Suchen,
    Plus,
    Regler,
    Muell,
    Neu,
    Abspielen,
    Pause,
    Hoch,
    Runter,
    Funk,
    Bildschirm,
    Schliessen,
    Sonne,
    Mond,
    Kontrast,
}

impl Icon {
    pub const ALLE: [Icon; 15] = [
        Icon::Suchen,
        Icon::Plus,
        Icon::Regler,
        Icon::Muell,
        Icon::Neu,
        Icon::Abspielen,
        Icon::Pause,
        Icon::Hoch,
        Icon::Runter,
        Icon::Funk,
        Icon::Bildschirm,
        Icon::Schliessen,
        Icon::Sonne,
        Icon::Mond,
        Icon::Kontrast,
    ];

    pub fn quelle(self) -> egui::ImageSource<'static> {
        match self {
            Icon::Suchen => egui::include_image!("../icons/search.svg"),
            Icon::Plus => egui::include_image!("../icons/plus.svg"),
            Icon::Regler => egui::include_image!("../icons/sliders-horizontal.svg"),
            Icon::Muell => egui::include_image!("../icons/trash-2.svg"),
            Icon::Neu => egui::include_image!("../icons/refresh-cw.svg"),
            Icon::Abspielen => egui::include_image!("../icons/play.svg"),
            Icon::Pause => egui::include_image!("../icons/pause.svg"),
            Icon::Hoch => egui::include_image!("../icons/arrow-up.svg"),
            Icon::Runter => egui::include_image!("../icons/arrow-down.svg"),
            Icon::Funk => egui::include_image!("../icons/wifi.svg"),
            Icon::Bildschirm => egui::include_image!("../icons/monitor.svg"),
            Icon::Schliessen => egui::include_image!("../icons/x.svg"),
            Icon::Sonne => egui::include_image!("../icons/sun.svg"),
            Icon::Mond => egui::include_image!("../icons/moon.svg"),
            Icon::Kontrast => egui::include_image!("../icons/contrast.svg"),
        }
    }

    /// Das Icon als Bild in Größe und Farbe (Strich, keine Fläche).
    pub fn bild(self, groesse: f32, farbe: Color32) -> egui::Image<'static> {
        egui::Image::new(self.quelle())
            .tint(farbe)
            .fit_to_exact_size(Vec2::splat(groesse))
    }
}

/// Das Icon eines Themes: Mond (dunkel), Sonne (hell), Kontrast (mono).
pub fn thema_icon(t: Thema) -> Icon {
    match t {
        Thema::Dunkel => Icon::Mond,
        Thema::Hell => Icon::Sonne,
        Thema::Mono => Icon::Kontrast,
    }
}

/// Das Zeichen der App (ADR 0002): bringt seine Farben selbst mit und ist in
/// allen Themes dasselbe Bild.
pub fn zeichen(ui: &mut Ui, groesse: f32) -> Response {
    ui.add(
        egui::Image::new(egui::include_image!(
            "../../../assets/wled-ambilight-zeichen.svg"
        ))
        .fit_to_exact_size(Vec2::splat(groesse))
        .alt_text("WLED Ambilight"),
    )
}

// ---------------------------------------------------------------- Knöpfe ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variante {
    Normal,
    Primaer,
    Gefahr,
}

/// Knopf mit Icon und Text. Der Text ist der zugängliche Name.
pub fn knopf(
    ui: &mut Ui,
    f: &Farben,
    icon: Option<Icon>,
    beschriftung: &str,
    variante: Variante,
    aktiv: bool,
) -> Response {
    let (flaeche, schrift, linie) = match variante {
        Variante::Normal => (f.bg_raised, f.text_default, f.border_strong),
        Variante::Primaer => (f.accent_default, f.text_on_accent, f.accent_default),
        Variante::Gefahr => (f.bg_raised, f.status_error, f.status_error),
    };
    let text = RichText::new(beschriftung)
        .font(fett(thema::FONT_SIZE_SM))
        .color(schrift);
    let knopf = match icon {
        Some(i) => egui::Button::image_and_text(i.bild(thema::FONT_SIZE_MD, schrift), text),
        None => egui::Button::new(text),
    }
    .fill(flaeche)
    .stroke(Stroke::new(thema::SIZE_BORDER, linie))
    .min_size(Vec2::new(0.0, thema::SIZE_CONTROL_SM));
    ui.add_enabled(aktiv, knopf)
}

/// Knopf nur mit Icon. `name` ist Tooltip **und** zugänglicher Name.
pub fn icon_knopf(ui: &mut Ui, f: &Farben, icon: Icon, name: &str, aktiv: bool) -> Response {
    let antwort = ui
        .add_enabled(
            aktiv,
            egui::Button::image(icon.bild(thema::FONT_SIZE_MD, f.text_muted))
                .frame_when_inactive(false)
                .min_size(Vec2::splat(thema::SIZE_CONTROL_SM)),
        )
        .on_hover_text(name);
    let name = name.to_string();
    antwort.widget_info(|| WidgetInfo::labeled(egui::Role::Button, aktiv, &name));
    antwort
}

// --------------------------------------------------------------- Schalter ----

/// Ein Schalter mit Beschriftung rechts. Liefert den neuen Stand, wenn er
/// umgelegt wurde.
pub fn schalter(ui: &mut Ui, f: &Farben, an: bool, beschriftung: &str) -> Option<bool> {
    schalter_mit_namen(ui, f, an, beschriftung, beschriftung)
}

/// Schalter ohne sichtbare Beschriftung; `name` liest der Screenreader vor
/// und zeigt der Tooltip.
pub fn schalter_ohne_text(ui: &mut Ui, f: &Farben, an: bool, name: &str) -> Option<bool> {
    schalter_mit_namen(ui, f, an, "", name)
}

fn schalter_mit_namen(
    ui: &mut Ui,
    f: &Farben,
    an: bool,
    beschriftung: &str,
    name: &str,
) -> Option<bool> {
    let hoehe = thema::FONT_SIZE_MD + thema::SPACE_1;
    let bahn = Vec2::new(hoehe * 1.8, hoehe);
    let galerie = ui.painter().layout_no_wrap(
        beschriftung.to_string(),
        FontId::proportional(thema::FONT_SIZE_SM),
        f.text_default,
    );
    let breite = bahn.x
        + if beschriftung.is_empty() {
            0.0
        } else {
            thema::SPACE_2 + galerie.size().x
        };
    let (rect, antwort) =
        ui.allocate_exact_size(Vec2::new(breite, thema::SIZE_CONTROL_SM), Sense::click());
    let antwort = antwort.on_hover_cursor(egui::CursorIcon::PointingHand);
    let antwort = if beschriftung.is_empty() {
        antwort.on_hover_text(name)
    } else {
        antwort
    };
    let name = name.to_string();
    antwort.widget_info(|| WidgetInfo::selected(egui::Role::Switch, true, an, &name));
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        let b = Rect::from_min_size(
            egui::pos2(rect.left(), rect.center().y - bahn.y / 2.0),
            bahn,
        );
        let (grund, knopf) = if an {
            (f.accent_default, f.text_on_accent)
        } else {
            (f.border_strong, f.text_default)
        };
        // Eckig wie alles andere: Bahn und Knopf sind Rechtecke.
        p.rect_filled(b, CornerRadius::ZERO, grund);
        let kante = bahn.y - 2.0 * thema::SIZE_FOCUS;
        let x = if an {
            b.right() - bahn.y / 2.0
        } else {
            b.left() + bahn.y / 2.0
        };
        p.rect_filled(
            Rect::from_center_size(egui::pos2(x, b.center().y), Vec2::splat(kante)),
            CornerRadius::ZERO,
            knopf,
        );
        if antwort.has_focus() {
            p.rect_stroke(
                b.expand(thema::SIZE_FOCUS),
                CornerRadius::ZERO,
                Stroke::new(thema::SIZE_FOCUS, f.focus_ring),
                StrokeKind::Outside,
            );
        }
        if !beschriftung.is_empty() {
            p.galley(
                egui::pos2(
                    b.right() + thema::SPACE_2,
                    rect.center().y - galerie.size().y / 2.0,
                ),
                galerie,
                f.text_default,
            );
        }
    }
    antwort.clicked().then_some(!an)
}

// ----------------------------------------------------------------- Regler ----

/// Regler mit Beschriftung und Wert darüber („Helligkeit … 80 %“). `zeige`
/// formatiert den Wert. Liefert den neuen Wert bei Änderung.
#[allow(clippy::too_many_arguments)]
pub fn regler(
    ui: &mut Ui,
    f: &Farben,
    beschriftung: &str,
    wert: f32,
    bereich: std::ops::RangeInclusive<f32>,
    schritt: f32,
    zeige: impl Fn(f32) -> String,
) -> Option<f32> {
    let breite = ui.available_width();
    ui.horizontal(|ui| {
        ui.label(text(f, beschriftung, Stufe::Gedaempft));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(text(f, zeige(wert), Stufe::Zahl));
        });
    });
    let mut neu = wert;
    let antwort = ui
        .scope(|ui| {
            ui.spacing_mut().slider_width = breite;
            // Die Füllung ist `selection.bg_fill`; im Theme ist das die
            // gedämpfte Auswahlfarbe für Text, hier soll es der Akzent sein.
            ui.visuals_mut().selection.bg_fill = f.accent_default;
            ui.add(
                egui::Slider::new(&mut neu, bereich)
                    .step_by(schritt as f64)
                    .show_value(false),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
        })
        .inner;
    let name = beschriftung.to_string();
    antwort.widget_info(|| WidgetInfo::slider(true, neu as f64, &name));
    (antwort.changed() && neu != wert).then_some(neu)
}

// --------------------------------------------------------------- Auswahl ----

/// Aufklappliste mit Beschriftung darüber. `optionen` sind (Wert, Text).
pub fn auswahl<T: PartialEq + Clone>(
    ui: &mut Ui,
    f: &Farben,
    beschriftung: &str,
    aktuell: &T,
    optionen: &[(T, String)],
) -> Option<T> {
    if !beschriftung.is_empty() {
        ui.label(text(f, beschriftung, Stufe::Gedaempft));
    }
    let gezeigt = optionen
        .iter()
        .find(|(w, _)| w == aktuell)
        .map(|(_, t)| t.clone())
        .unwrap_or_default();
    let mut neu = None;
    let antwort = egui::ComboBox::from_id_salt(("auswahl", beschriftung))
        .selected_text(gezeigt)
        .width(ui.available_width() - thema::SPACE_2)
        .show_ui(ui, |ui| {
            for (w, t) in optionen {
                if ui.selectable_label(w == aktuell, t).clicked() && w != aktuell {
                    neu = Some(w.clone());
                }
            }
        })
        .response;
    let name = beschriftung.to_string();
    antwort.widget_info(|| WidgetInfo::labeled(egui::Role::ComboBox, true, &name));
    neu
}

/// Umschalter aus zwei bis drei Teilen (Protokoll, Theme).
pub fn umschalter<T: PartialEq + Clone>(
    ui: &mut Ui,
    f: &Farben,
    aktuell: &T,
    optionen: &[(T, String)],
) -> Option<T> {
    let mut neu = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for (w, t) in optionen {
            let an = w == aktuell;
            let knopf = egui::Button::new(
                RichText::new(t)
                    .font(FontId::proportional(thema::FONT_SIZE_XS))
                    .color(if an { f.text_on_accent } else { f.text_muted }),
            )
            .fill(if an { f.accent_default } else { f.bg_input })
            .stroke(Stroke::new(thema::SIZE_BORDER, f.border_subtle))
            .corner_radius(CornerRadius::ZERO);
            let r = ui
                .add(knopf)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let name = t.clone();
            r.widget_info(|| WidgetInfo::selected(egui::Role::RadioButton, true, an, &name));
            if r.clicked() && !an {
                neu = Some(w.clone());
            }
        }
    });
    neu
}

// --------------------------------------------------------------- Eingabe ----

/// Einzeiliges Eingabefeld mit Beschriftung darüber. Liefert den neuen Text.
pub fn feld(
    ui: &mut Ui,
    f: &Farben,
    beschriftung: &str,
    wert: &str,
    hinweis: &str,
) -> Option<String> {
    if !beschriftung.is_empty() {
        ui.label(text(f, beschriftung, Stufe::Gedaempft));
    }
    let mut neu = wert.to_string();
    let antwort = ui.add(
        egui::TextEdit::singleline(&mut neu)
            .hint_text(hinweis)
            .desired_width(ui.available_width()),
    );
    let name = beschriftung.to_string();
    if !name.is_empty() {
        antwort.widget_info(|| WidgetInfo::text_edit(true, wert, &neu, &name));
    }
    (neu != wert).then_some(neu)
}

/// Zahl mit Beschriftung darüber.
pub fn zahl(
    ui: &mut Ui,
    f: &Farben,
    beschriftung: &str,
    wert: u32,
    bereich: std::ops::RangeInclusive<u32>,
) -> Option<u32> {
    ui.label(text(f, beschriftung, Stufe::Gedaempft));
    let mut neu = wert;
    let antwort = ui.add(egui::DragValue::new(&mut neu).range(bereich).speed(1.0));
    let name = beschriftung.to_string();
    antwort.widget_info(|| WidgetInfo::labeled(egui::Role::SpinButton, true, &name));
    (neu != wert).then_some(neu)
}

// ----------------------------------------------------------- Anzeigen ----

/// Farbe eines Tons.
pub fn ton_farbe(f: &Farben, ton: Ton) -> Color32 {
    match ton {
        Ton::Neutral => f.text_muted,
        Ton::Ok => f.status_ok,
        Ton::Warnung => f.status_warn,
        Ton::Fehler => f.status_error,
    }
}

/// Abzeichen: Punkt in der Tonfarbe und kurzer Text.
pub fn abzeichen(ui: &mut Ui, f: &Farben, ton: Ton, inhalt: &str) -> Response {
    let farbe = ton_farbe(f, ton);
    let galerie = ui.painter().layout_no_wrap(
        inhalt.to_string(),
        FontId::proportional(thema::FONT_SIZE_XS),
        farbe,
    );
    let punkt = thema::SPACE_2;
    let groesse = Vec2::new(
        punkt + thema::SPACE_1 * 3.0 + galerie.size().x + thema::SPACE_1,
        thema::FONT_SIZE_XS + thema::SPACE_2,
    );
    let (rect, antwort) = ui.allocate_exact_size(groesse, Sense::hover());
    let name = inhalt.to_string();
    antwort.widget_info(|| WidgetInfo::labeled(egui::Role::Status, true, &name));
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(thema::RADIUS_SM as u8),
        f.bg_input,
        Stroke::new(thema::SIZE_BORDER, f.border_subtle),
        StrokeKind::Inside,
    );
    p.rect_filled(
        Rect::from_center_size(
            egui::pos2(rect.left() + thema::SPACE_1 + punkt / 2.0, rect.center().y),
            Vec2::splat(punkt - 2.0 * thema::SIZE_BORDER),
        ),
        CornerRadius::ZERO,
        farbe,
    );
    p.galley(
        egui::pos2(
            rect.left() + thema::SPACE_1 * 2.0 + punkt,
            rect.center().y - galerie.size().y / 2.0,
        ),
        galerie,
        farbe,
    );
    antwort
}

/// Fortschrittsbalken (0..1). `zuviel` färbt ihn als Fehler.
pub fn fortschritt(ui: &mut Ui, f: &Farben, anteil: f32, zuviel: bool) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), thema::SPACE_1),
        Sense::hover(),
    );
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::ZERO, f.bg_input);
    let mut voll = rect;
    voll.set_width(rect.width() * anteil.clamp(0.0, 1.0));
    p.rect_filled(
        voll,
        CornerRadius::ZERO,
        if zuviel {
            f.status_error
        } else {
            f.accent_default
        },
    );
}

/// Farbleiste: die Farben, die gerade auf die Leitung gehen, eine Zelle je LED.
pub fn farbleiste(ui: &mut Ui, f: &Farben, farben: &[[u8; 3]], name: &str) {
    let (rect, antwort) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), thema::SIZE_LED * 2.0),
        Sense::hover(),
    );
    let name = name.to_string();
    antwort.widget_info(|| WidgetInfo::labeled(egui::Role::Image, true, &name));
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::ZERO, f.bg_input);
    if farben.is_empty() {
        return;
    }
    let zelle = rect.width() / farben.len() as f32;
    for (i, c) in farben.iter().enumerate() {
        let r = Rect::from_min_size(
            egui::pos2(rect.left() + i as f32 * zelle, rect.top()),
            Vec2::new(zelle.max(1.0), rect.height()),
        );
        p.rect_filled(r, CornerRadius::ZERO, Color32::from_rgb(c[0], c[1], c[2]));
    }
}

/// Ein kleines Farbquadrat (Zonenliste).
pub fn farbpunkt(ui: &mut Ui, f: &Farben, farbe: [u8; 3]) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(thema::SPACE_2), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::ZERO,
        Color32::from_rgb(farbe[0], farbe[1], farbe[2]),
        Stroke::new(thema::SIZE_BORDER, f.border_strong),
        StrokeKind::Inside,
    );
}

/// Drehendes Zeichen für laufende Arbeit.
pub fn warten(ui: &mut Ui, f: &Farben) {
    ui.add(
        egui::Spinner::new()
            .size(thema::FONT_SIZE_MD)
            .color(f.accent_default),
    );
}

/// Text mittig in ein Rechteck (Platzhalter auf der Bühne).
pub fn mittig(ui: &Ui, rect: Rect, inhalt: &str, farbe: Color32) {
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        inhalt,
        FontId::monospace(thema::FONT_SIZE_XS),
        farbe,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jedes_icon_hat_eine_datei() {
        for i in Icon::ALLE {
            let quelle = i.quelle();
            assert!(
                matches!(quelle, egui::ImageSource::Bytes { .. }),
                "{i:?} nicht eingebettet"
            );
        }
    }
}
