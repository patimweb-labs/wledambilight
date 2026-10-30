// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Patrick Weppelmann

//! Look and feel.
//!
//! The app lives in a dark room next to a glowing screen, so the chrome is a
//! quiet graphite "wall" and the only loud thing is light: the warm-white LED
//! amber accent and the live light spill around the preview. Everything else
//! stays out of the way.

use egui::{
    epaint::{Mesh, Vertex},
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Pos2, Rect,
    Response, RichText, Sense, Stroke, TextStyle, Ui, Vec2,
};

// ------------------------------------------------------------------ palettes
//
// Every colour of the window comes from one palette. A theme is a palette, and
// switching themes only swaps the palette - the widgets read the values through
// the accessors below, so nothing has to be rebuilt and no restart is needed.
//
// The look is borrowed from Crosspoint (the PipeWire mixer): blue-black
// blueprint, one signal green, monospace numbers.

/// All colours of the interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// window background - the wall behind the screen
    pub wall: Color32,
    /// side panels
    pub panel: Color32,
    /// cards and inputs
    pub surface: Color32,
    /// hover / selected surface
    pub raised: Color32,
    /// the matte the preview sits in
    pub matte: Color32,
    pub line: Color32,
    pub line_hi: Color32,
    pub ink: Color32,
    pub ink_dim: Color32,
    pub ink_faint: Color32,
    /// the one accent: buttons, active controls, marks
    pub accent: Color32,
    /// darker accent for fills and rails
    pub accent_dark: Color32,
    /// text on accent
    pub accent_ink: Color32,
    /// "everything is fine": live capture, connected device
    pub ok: Color32,
    /// something needs attention but keeps running
    pub warn: Color32,
    /// a real error
    pub error: Color32,
}

impl Default for Palette {
    /// The built-in `Crosspoint` theme.
    fn default() -> Self {
        Self {
            wall: rgb(0x060A12),
            panel: rgb(0x0E1624),
            surface: rgb(0x101B2C),
            raised: rgb(0x18344E),
            matte: rgb(0x04070C),
            line: rgb(0x1B3550),
            line_hi: rgb(0x2A5A7E),
            ink: rgb(0xD6E2F0),
            ink_dim: rgb(0x9FB3C6),
            ink_faint: rgb(0x6C86A0),
            accent: rgb(0x39DD93),
            accent_dark: rgb(0x1C7D54),
            accent_ink: rgb(0x04140C),
            ok: rgb(0x39DD93),
            warn: rgb(0xF3C24B),
            error: rgb(0xFF6B6B),
        }
    }
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// `#rgb`, `#rrggbb` or `#rrggbbaa`.
pub fn parse_color(text: &str) -> Option<Color32> {
    let t = text.trim();
    let rest = t.strip_prefix('#')?;
    if !rest.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = |s: &str| u8::from_str_radix(s, 16).ok();
    match rest.len() {
        3 => {
            let d: Vec<u8> = rest.chars().filter_map(|c| v(&c.to_string())).collect();
            if d.len() != 3 {
                return None;
            }
            Some(Color32::from_rgb(d[0] * 17, d[1] * 17, d[2] * 17))
        }
        6 | 8 => {
            let r = v(&rest[0..2])?;
            let g = v(&rest[2..4])?;
            let b = v(&rest[4..6])?;
            let a = if rest.len() == 8 {
                v(&rest[6..8])?
            } else {
                255
            };
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

/// A theme as it stands in a file: every field optional, applied over the
/// default palette. A typo therefore changes one colour, not the whole window.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ThemeFile {
    /// name shown in the interface; the file name is used when missing
    pub name: Option<String>,
    pub wall: Option<String>,
    pub panel: Option<String>,
    pub surface: Option<String>,
    pub raised: Option<String>,
    pub matte: Option<String>,
    pub line: Option<String>,
    pub line_hi: Option<String>,
    pub ink: Option<String>,
    pub ink_dim: Option<String>,
    pub ink_faint: Option<String>,
    pub accent: Option<String>,
    pub accent_dark: Option<String>,
    pub accent_ink: Option<String>,
    pub ok: Option<String>,
    pub warn: Option<String>,
    pub error: Option<String>,
}

impl ThemeFile {
    /// Lay this file over `base`. Unreadable colours are skipped and reported.
    pub fn apply(&self, base: Palette) -> (Palette, Vec<String>) {
        let mut p = base;
        let mut problems = Vec::new();
        macro_rules! farbe {
            ($feld:ident) => {
                if let Some(v) = self.$feld.as_deref() {
                    match parse_color(v) {
                        Some(c) => p.$feld = c,
                        None => problems.push(format!(
                            "{}: '{}' ist keine Farbe (#rgb, #rrggbb, #rrggbbaa)",
                            stringify!($feld),
                            v
                        )),
                    }
                }
            };
        }
        farbe!(wall);
        farbe!(panel);
        farbe!(surface);
        farbe!(raised);
        farbe!(matte);
        farbe!(line);
        farbe!(line_hi);
        farbe!(ink);
        farbe!(ink_dim);
        farbe!(ink_faint);
        farbe!(accent);
        farbe!(accent_dark);
        farbe!(accent_ink);
        farbe!(ok);
        farbe!(warn);
        farbe!(error);
        (p, problems)
    }
}

/// The built-in themes, in the order they are offered.
pub fn builtin_themes() -> Vec<(&'static str, Palette)> {
    let base = Palette::default();
    // light: same accent, dark text, no colour on the accent surfaces
    let light = Palette {
        wall: rgb(0xE9EEF5),
        panel: rgb(0xF7FAFD),
        surface: rgb(0xFFFFFF),
        raised: rgb(0xDCE4EE),
        matte: rgb(0xDFE6EF),
        line: rgb(0xC3CFDD),
        line_hi: rgb(0x9FB0C2),
        ink: rgb(0x101821),
        ink_dim: rgb(0x44566A),
        ink_faint: rgb(0x6C7E92),
        accent: rgb(0x18A96B),
        accent_dark: rgb(0x0E7B4C),
        accent_ink: rgb(0xFFFFFF),
        ok: rgb(0x18A96B),
        warn: rgb(0xB8791A),
        error: rgb(0xC23636),
        ..base
    };
    // high contrast, colour carries no meaning
    let mono = Palette {
        wall: rgb(0x000000),
        panel: rgb(0x111111),
        surface: rgb(0x1A1A1A),
        raised: rgb(0x2B2B2B),
        matte: rgb(0x000000),
        line: rgb(0x3A3A3A),
        line_hi: rgb(0x6A6A6A),
        ink: rgb(0xFFFFFF),
        ink_dim: rgb(0xB4B4B4),
        ink_faint: rgb(0x8C8C8C),
        accent: rgb(0xEDEDED),
        accent_dark: rgb(0x8C8C8C),
        accent_ink: rgb(0x000000),
        ok: rgb(0xEDEDED),
        warn: rgb(0xD9D9D9),
        error: rgb(0xD9D9D9),
        ..base
    };
    // the original WLED Ambilight amber, for people who liked it
    let amber = Palette {
        wall: rgb(0x121419),
        panel: rgb(0x15171D),
        surface: rgb(0x1B1E26),
        raised: rgb(0x242832),
        matte: rgb(0x0C0D11),
        line: rgb(0x2B303C),
        line_hi: rgb(0x404656),
        accent: rgb(0xFFC46B),
        accent_dark: rgb(0xC9993E),
        accent_ink: rgb(0x2A1D05),
        ..base
    };
    vec![
        ("Crosspoint", base),
        ("Crosspoint Hell", light),
        ("Mono", mono),
        ("Amber", amber),
    ]
}

/// Folder for themes of your own: `…/wled-ambilight/themes/*.json`.
pub fn theme_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("wled-ambilight")
        .join("themes")
}

/// Every theme: the built-ins plus the files in [`theme_dir`].
/// The default palette is the base every file is laid over.
pub fn load_themes() -> (Vec<(String, Palette)>, Vec<String>) {
    load_themes_in(&theme_dir())
}

/// Same as [`load_themes`], for an explicit folder (tests, `--themes-dir`).
pub fn load_themes_in(dir: &std::path::Path) -> (Vec<(String, Palette)>, Vec<String>) {
    let mut problems = Vec::new();
    let mut themes = builtin_themes()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p))
        .collect::<Vec<_>>();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (themes, problems);
    };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    for path in files {
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                problems.push(format!("{stem}: {e}"));
                continue;
            }
        };
        match serde_json::from_str::<ThemeFile>(&text) {
            Ok(file) => {
                let (palette, warn) = file.apply(Palette::default());
                problems.extend(warn.into_iter().map(|w| format!("{stem}: {w}")));
                let name = file.name.filter(|n| !n.trim().is_empty()).unwrap_or(stem);
                if themes.iter().any(|(n, _)| *n == name) {
                    problems.push(format!("{name}: Theme gibt es schon, Datei übersprungen"));
                    continue;
                }
                themes.push((name, palette));
            }
            Err(e) => problems.push(format!("{stem}: {e}")),
        }
    }
    (themes, problems)
}

// ------------------------------------------------------------------ active palette

static CURRENT: std::sync::OnceLock<std::sync::RwLock<Palette>> = std::sync::OnceLock::new();

fn cell() -> &'static std::sync::RwLock<Palette> {
    CURRENT.get_or_init(|| std::sync::RwLock::new(Palette::default()))
}

/// The palette the window is drawn with right now.
pub fn current() -> Palette {
    *cell().read().unwrap()
}

/// Use `palette` from now on (the next frame picks it up).
pub fn set_palette(palette: Palette) {
    *cell().write().unwrap() = palette;
}

macro_rules! token {
    ($name:ident) => {
        /// Current colour of this token.
        pub fn $name() -> Color32 {
            current().$name
        }
    };
}
token!(wall);
token!(panel);
token!(surface);
token!(raised);
token!(matte);
token!(line);
token!(line_hi);
token!(ink);
token!(ink_dim);
token!(ink_faint);
token!(accent);
token!(accent_dark);
token!(accent_ink);
token!(ok);
token!(error);

/// Current colour of the warning token (the helper `warn` draws text).
pub fn warning() -> Color32 {
    current().warn
}

pub const R_SM: u8 = 6;
pub const R_MD: u8 = 9;
pub const R_LG: u8 = 14;

// ------------------------------------------------------------------ fonts

const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");

fn semibold_family() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Font for headings and emphasised numbers.
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, semibold_family())
}

pub fn regular(size: f32) -> FontId {
    FontId::proportional(size)
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        std::sync::Arc::new(FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        "inter-semibold".into(),
        std::sync::Arc::new(FontData::from_static(INTER_SEMIBOLD)),
    );
    // keep egui's own fonts as fallback for symbols and emoji
    let fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut prop = vec!["inter".to_owned()];
    prop.extend(fallback.iter().cloned());
    fonts.families.insert(FontFamily::Proportional, prop);
    let mut bold = vec!["inter-semibold".to_owned(), "inter".to_owned()];
    bold.extend(fallback);
    fonts.families.insert(semibold_family(), bold);
    ctx.set_fonts(fonts);
}

/// First setup: fonts plus the style built from the current palette.
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    apply_style(ctx);
}

/// Rebuild the egui style from the current palette. Call after a theme change;
/// fonts stay as they are.
pub fn apply_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);

    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    let mut v = egui::Visuals::dark();
    v.panel_fill = wall();
    v.window_fill = panel();
    v.extreme_bg_color = matte();
    v.faint_bg_color = surface();
    v.code_bg_color = surface();
    v.override_text_color = Some(ink());
    v.hyperlink_color = accent();
    v.weak_text_color = Some(ink_faint());
    // also the filled part of sliders
    v.selection.bg_fill = accent_dark();
    v.selection.stroke = Stroke::new(1.0_f32, accent());
    v.text_cursor.stroke = Stroke::new(2.0_f32, accent());
    v.window_corner_radius = CornerRadius::same(R_LG);
    v.menu_corner_radius = CornerRadius::same(R_MD);
    v.window_stroke = Stroke::new(1.0_f32, line());
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 12],
        blur: 32,
        spread: 0,
        color: Color32::from_black_alpha(150),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: [0, 8],
        blur: 22,
        spread: 0,
        color: Color32::from_black_alpha(160),
    };
    v.slider_trailing_fill = true;
    v.handle_shape = egui::style::HandleShape::Circle;
    v.striped = false;
    v.indent_has_left_vline = false;

    let r = CornerRadius::same(R_SM);
    v.widgets.noninteractive = egui::style::WidgetVisuals {
        bg_fill: panel(),
        weak_bg_fill: panel(),
        bg_stroke: Stroke::new(1.0_f32, line()),
        corner_radius: r,
        fg_stroke: Stroke::new(1.0_f32, ink_dim()),
        expansion: 0.0,
    };
    v.widgets.inactive = egui::style::WidgetVisuals {
        bg_fill: raised(),
        weak_bg_fill: surface(),
        bg_stroke: Stroke::new(1.0_f32, line()),
        corner_radius: r,
        fg_stroke: Stroke::new(1.0_f32, ink()),
        expansion: 0.0,
    };
    v.widgets.hovered = egui::style::WidgetVisuals {
        bg_fill: raised().gamma_multiply(1.15),
        weak_bg_fill: raised(),
        bg_stroke: Stroke::new(1.0_f32, line_hi()),
        corner_radius: r,
        fg_stroke: Stroke::new(1.5_f32, Color32::WHITE),
        expansion: 0.0,
    };
    v.widgets.active = egui::style::WidgetVisuals {
        bg_fill: accent(),
        weak_bg_fill: raised(),
        bg_stroke: Stroke::new(1.0_f32, accent()),
        corner_radius: r,
        fg_stroke: Stroke::new(1.5_f32, Color32::WHITE),
        expansion: 0.0,
    };
    v.widgets.open = v.widgets.hovered;
    style.visuals = v;

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.interact_size = Vec2::new(34.0, 28.0);
    style.spacing.slider_width = 180.0;
    style.spacing.slider_rail_height = 4.0;
    style.spacing.combo_width = 120.0;
    style.spacing.indent = 14.0;
    style.spacing.menu_margin = Margin::same(6);
    style.spacing.scroll = egui::style::ScrollStyle::floating();
    style.text_styles = [
        (TextStyle::Heading, semibold(16.0)),
        (TextStyle::Body, regular(13.0)),
        (TextStyle::Button, regular(13.0)),
        (TextStyle::Small, regular(11.5)),
        (TextStyle::Monospace, FontId::monospace(12.0)),
    ]
    .into();
    style.interaction.tooltip_delay = 0.35;
    ctx.set_style_of(egui::Theme::Dark, style);
}

// ------------------------------------------------------------------ text

/// Panel / group title, sentence case.
pub fn title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).font(semibold(13.5)).color(ink()));
}

/// Small label in front of a control.
pub fn field_label(ui: &mut Ui, text: &str) -> Response {
    ui.label(RichText::new(text).size(12.0).color(ink_dim()))
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(ink_faint()));
}

/// A hint that points at a problem: zones overlapping, LEDs that do not exist.
pub fn warn(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.5).color(error()));
}

// ------------------------------------------------------------------ containers

pub fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) -> Response {
    card_with(ui, surface(), line(), add)
}

pub fn card_with(
    ui: &mut Ui,
    fill: Color32,
    stroke: Color32,
    add: impl FnOnce(&mut Ui),
) -> Response {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, stroke))
        .corner_radius(CornerRadius::same(R_LG - 2))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        })
        .response
}

/// Six colour chips that show what a palette looks like.
pub fn swatch(ui: &mut Ui, palette: &Palette) {
    let chips = [
        palette.wall,
        palette.panel,
        palette.surface,
        palette.raised,
        palette.accent,
        palette.ink,
    ];
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 16.0), Sense::hover());
    let w = rect.width() / chips.len() as f32;
    for (i, c) in chips.iter().enumerate() {
        let x0 = rect.left() + i as f32 * w;
        let piece = Rect::from_min_max(
            Pos2::new(x0, rect.top()),
            Pos2::new(x0 + w - 2.0, rect.bottom()),
        );
        ui.painter().rect_filled(piece, CornerRadius::same(3), *c);
        ui.painter().rect_stroke(
            piece,
            CornerRadius::same(3),
            Stroke::new(1.0_f32, line()),
            egui::StrokeKind::Inside,
        );
    }
}

// ------------------------------------------------------------------ cursor

/// Hand cursor for anything clickable (egui does not do this by itself).
pub trait HandCursor {
    fn hand_cursor(self) -> Self;
}

impl HandCursor for Response {
    fn hand_cursor(self) -> Self {
        self.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

// ------------------------------------------------------------------ widgets

/// Status pill: coloured dot + text.
pub fn status_pill(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let font = regular(12.0);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), font, ink());
    let size = Vec2::new(galley.size().x + 30.0, 26.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let p = ui.painter();
    p.rect(
        rect,
        CornerRadius::same(13),
        color.gamma_multiply(0.10),
        Stroke::new(1.0_f32, color.gamma_multiply(0.35)),
        egui::StrokeKind::Inside,
    );
    let c = Pos2::new(rect.left() + 13.0, rect.center().y);
    p.circle_filled(c, 6.0, color.gamma_multiply(0.25));
    p.circle_filled(c, 3.5, color);
    p.galley(
        Pos2::new(rect.left() + 23.0, rect.center().y - galley.size().y / 2.0),
        galley,
        ink(),
    );
    resp
}

/// Quiet metric: "label value".
pub fn metric(ui: &mut Ui, label: &str, value: &str) {
    ui.label(RichText::new(value).font(semibold(12.0)).color(ink_dim()));
    ui.label(RichText::new(label).size(11.5).color(ink_faint()));
}

/// iOS-style switch with a label on the right. Returns true when toggled.
pub fn switch(ui: &mut Ui, on: &mut bool, label: &str) -> Response {
    let font = regular(12.5);
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font, ink());
    let track = Vec2::new(30.0, 17.0);
    let gap = if label.is_empty() { 0.0 } else { 8.0 };
    let size = Vec2::new(track.x + gap + galley.size().x, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let tr = Rect::from_min_size(
        Pos2::new(rect.left(), rect.center().y - track.y / 2.0),
        track,
    );
    let p = ui.painter();
    let off_fill = if resp.hovered() { line_hi() } else { line() };
    let fill = lerp_color(off_fill, accent(), t);
    p.rect_filled(tr, CornerRadius::same(9), fill);
    let knob_x = egui::lerp(tr.left() + 8.5..=tr.right() - 8.5, t);
    p.circle_filled(
        Pos2::new(knob_x, tr.center().y),
        6.5,
        lerp_color(ink_dim(), accent_ink(), t),
    );
    if !label.is_empty() {
        p.galley_with_override_text_color(
            Pos2::new(tr.right() + gap, rect.center().y - galley.size().y / 2.0),
            galley,
            if *on { ink() } else { ink_dim() },
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Segmented control, at least `width` wide. Returns true when the selection changed.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    value: &mut T,
    options: &[(T, &str)],
    width: Option<f32>,
) -> bool {
    let font = regular(12.5);
    let galleys: Vec<_> = options
        .iter()
        .map(|(_, l)| {
            ui.painter()
                .layout_no_wrap((*l).to_owned(), font.clone(), ink())
        })
        .collect();
    let natural: f32 = galleys.iter().map(|g| g.size().x + 22.0).sum::<f32>() + 4.0;
    let w = width.map_or(natural, |w| w.max(natural));
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(R_MD),
        matte(),
        Stroke::new(1.0_f32, line()),
        egui::StrokeKind::Inside,
    );
    let seg_w = (rect.width() - 4.0) / options.len().max(1) as f32;
    let mut changed = false;
    for (i, ((opt, _), galley)) in options.iter().zip(galleys).enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(rect.left() + 2.0 + i as f32 * seg_w, rect.top() + 2.0),
            Vec2::new(seg_w, rect.height() - 4.0),
        );
        let resp = ui
            .interact(
                r,
                ui.scope_id()
                    .with(("seg", i, rect.left() as i32, rect.top() as i32)),
                Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let sel = *value == *opt;
        if resp.clicked() && !sel {
            *value = *opt;
            changed = true;
        }
        let p = ui.painter();
        if sel {
            p.rect_filled(r, CornerRadius::same(R_SM + 1), raised());
            p.rect_stroke(
                r,
                CornerRadius::same(R_SM + 1),
                Stroke::new(1.0_f32, line_hi()),
                egui::StrokeKind::Inside,
            );
        } else if resp.hovered() {
            p.rect_filled(r, CornerRadius::same(R_SM + 1), surface());
        }
        p.galley_with_override_text_color(
            r.center() - galley.size() / 2.0,
            galley,
            if sel { ink() } else { ink_dim() },
        );
    }
    changed
}

/// Label + value on one line, full width slider underneath.
pub fn slider_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    fmt: impl Fn(f32) -> String,
) -> bool {
    ui.horizontal(|ui| {
        field_label(ui, label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(fmt(*value)).font(semibold(12.0)).color(ink()));
        });
    });
    let old = (ui.spacing().slider_width, ui.spacing().interact_size.y);
    ui.spacing_mut().slider_width = ui.available_width();
    // smaller knob than egui's default
    ui.spacing_mut().interact_size.y = 18.0;
    let r = ui
        .add(egui::Slider::new(value, range).show_value(false))
        .hand_cursor();
    ui.spacing_mut().slider_width = old.0;
    ui.spacing_mut().interact_size.y = old.1;
    ui.add_space(2.0);
    r.changed()
}

/// Dropdown with a fixed set of values.
pub fn combo<T: PartialEq + Copy>(
    ui: &mut Ui,
    id: impl egui::AsIdSalt,
    width: f32,
    value: &mut T,
    label: impl Fn(T) -> String,
    options: &[T],
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .width(width)
        .selected_text(label(*value))
        .show_ui(ui, |ui| {
            for opt in options {
                if ui
                    .selectable_value(value, *opt, label(*opt))
                    .hand_cursor()
                    .clicked()
                {
                    changed = true;
                }
            }
        })
        .response
        .hand_cursor();
    changed
}

/// Thin usage bar (e.g. LEDs assigned to zones / LEDs on the strip).
pub fn usage_bar(ui: &mut Ui, used: f32, total: f32) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 4.0), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(2), matte());
    if total <= 0.0 {
        return;
    }
    let frac = (used / total).clamp(0.0, 1.0);
    let over = used > total;
    let mut fill = rect;
    fill.set_width(rect.width() * frac);
    p.rect_filled(
        fill,
        CornerRadius::same(2),
        if over { error() } else { ink_dim() },
    );
}

/// The LED tape: one segment per LED, in the colour that is being sent.
pub fn led_tape(ui: &mut Ui, colors: &[[u8; 3]], height: f32, zero_right: bool) -> Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(R_SM), matte());
    // a small tag at the end where LED 0 sits: the only thing that shows the
    // wiring direction, because the colours themselves are always in picture
    // order
    let tag = 13.0_f32.min(rect.width() * 0.15);
    let tag_rect = if zero_right {
        Rect::from_min_max(
            Pos2::new(rect.right() - 2.0 - tag, rect.top() + 2.0),
            Pos2::new(rect.right() - 2.0, rect.bottom() - 2.0),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(rect.left() + 2.0, rect.top() + 2.0),
            Pos2::new(rect.left() + 2.0 + tag, rect.bottom() - 2.0),
        )
    };
    painter.rect_filled(tag_rect, CornerRadius::same(2), matte());
    painter.text(
        tag_rect.center(),
        egui::Align2::CENTER_CENTER,
        "0",
        regular(9.5),
        ink(),
    );
    if !colors.is_empty() {
        let mut inner = rect.shrink(2.0);
        if zero_right {
            inner.max.x -= tag + 2.0;
        } else {
            inner.min.x += tag + 2.0;
        }
        let n = colors.len() as f32;
        let seg = inner.width() / n;
        let gap = if seg > 3.0 { 1.0 } else { 0.0 };
        for (i, c) in colors.iter().enumerate() {
            let x0 = inner.left() + i as f32 * seg;
            let x1 = (x0 + seg - gap).min(inner.right());
            if x1 <= x0 {
                continue;
            }
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x0, inner.top()), Pos2::new(x1, inner.bottom())),
                if seg > 6.0 { 2 } else { 0 },
                Color32::from_rgb(c[0], c[1], c[2]),
            );
        }
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(R_SM),
        Stroke::new(1.0_f32, line()),
        egui::StrokeKind::Inside,
    );
    resp
}

/// Label chip used for zone names on the canvas.
pub fn chip(painter: &egui::Painter, pos: Pos2, text: &str, fg: Color32) -> Rect {
    let font = regular(11.5);
    let galley = painter.layout_no_wrap(text.to_owned(), font, fg);
    let rect = Rect::from_min_size(pos, galley.size() + Vec2::new(14.0, 7.0));
    painter.rect_filled(
        rect,
        CornerRadius::same(R_SM),
        Color32::from_black_alpha(190),
    );
    painter.galley(pos + Vec2::new(7.0, 3.5), galley, fg);
    rect
}

// ------------------------------------------------------------------ light spill

/// Which way the light spills from an edge segment.
#[derive(Clone, Copy)]
pub enum Spill {
    Up,
    Down,
    Left,
    Right,
}

/// Paint the light thrown onto the "wall" around the preview: a soft gradient
/// from the edge outwards, one slice per LED, in the colour that is sent.
pub fn light_spill(
    painter: &egui::Painter,
    edge: [Pos2; 2],
    dir: Spill,
    colors: &[Color32],
    depth: f32,
) {
    if colors.is_empty() {
        return;
    }
    let out = match dir {
        Spill::Up => Vec2::new(0.0, -depth),
        Spill::Down => Vec2::new(0.0, depth),
        Spill::Left => Vec2::new(-depth, 0.0),
        Spill::Right => Vec2::new(depth, 0.0),
    };
    let mut mesh = Mesh::default();
    let n = colors.len();
    // vertices along the edge (inner, mid, outer) per LED boundary; neighbouring
    // LEDs share vertices, so the colours blend into a continuous glow
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let p = edge[0] + (edge[1] - edge[0]) * t;
        // colour at a boundary = mean of the two neighbouring LEDs
        let a = colors[i.saturating_sub(1).min(n - 1)];
        let b = colors[i.min(n - 1)];
        let c = Color32::from_rgb(
            ((a.r() as u16 + b.r() as u16) / 2) as u8,
            ((a.g() as u16 + b.g() as u16) / 2) as u8,
            ((a.b() as u16 + b.b() as u16) / 2) as u8,
        );
        let base = mesh.vertices.len() as u32;
        mesh.vertices.push(Vertex {
            pos: p,
            uv: egui::epaint::WHITE_UV,
            color: c.gamma_multiply(0.85),
        });
        mesh.vertices.push(Vertex {
            pos: p + out * 0.3,
            uv: egui::epaint::WHITE_UV,
            color: c.gamma_multiply(0.38),
        });
        mesh.vertices.push(Vertex {
            pos: p + out,
            uv: egui::epaint::WHITE_UV,
            color: Color32::TRANSPARENT,
        });
        if i > 0 {
            let prev = base - 3;
            for k in 0..2 {
                mesh.add_triangle(prev + k, base + k, prev + k + 1);
                mesh.add_triangle(prev + k + 1, base + k, base + k + 1);
            }
        }
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("wled-ambilight-theme-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn colors_are_parsed_the_way_people_write_them() {
        assert_eq!(
            parse_color("#39DD93"),
            Some(Color32::from_rgb(0x39, 0xDD, 0x93))
        );
        assert_eq!(
            parse_color("  #39d  "),
            Some(Color32::from_rgb(0x33, 0x99, 0xDD))
        );
        assert_eq!(
            parse_color("#39DD9380"),
            Some(Color32::from_rgba_unmultiplied(0x39, 0xDD, 0x93, 0x80))
        );
        assert_eq!(parse_color("39DD93"), None, "the # is required");
        assert_eq!(parse_color("#39DD9"), None);
        assert_eq!(parse_color("#zzzzzz"), None);
        assert_eq!(parse_color(""), None);
    }

    #[test]
    fn a_theme_file_only_changes_what_it_names() {
        let file: ThemeFile =
            serde_json::from_str(r##"{"name":"Test","accent":"#FF0000"}"##).unwrap();
        let base = Palette::default();
        let (p, problems) = file.apply(base);
        assert!(problems.is_empty());
        assert_eq!(p.accent, Color32::from_rgb(255, 0, 0));
        // everything else stays on the built-in theme
        assert_eq!(p.wall, base.wall);
        assert_eq!(p.ink, base.ink);
        assert_eq!(p.error, base.error);
    }

    #[test]
    fn a_broken_colour_is_reported_and_skipped() {
        let file: ThemeFile =
            serde_json::from_str(r##"{"wall":"nope","accent":"#39DD93"}"##).unwrap();
        let (p, problems) = file.apply(Palette::default());
        assert_eq!(p.wall, Palette::default().wall);
        assert_eq!(p.accent, Color32::from_rgb(0x39, 0xDD, 0x93));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("wall"), "{problems:?}");
    }

    #[test]
    fn the_built_in_themes_are_all_different() {
        let themes = builtin_themes();
        assert!(themes.len() >= 4);
        let walls: Vec<u32> = themes
            .iter()
            .map(|(_, p)| (p.wall.r() as u32) << 16 | (p.wall.g() as u32) << 8 | p.wall.b() as u32)
            .collect();
        let mut unique = walls.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), walls.len(), "every theme needs its own look");
    }

    #[test]
    fn own_themes_are_found_next_to_the_config() {
        let dir = temp_dir("own");
        std::fs::write(
            dir.join("nacht.json"),
            r##"{"name":"Nacht","wall":"#000010","accent":"#00AAFF"}"##,
        )
        .unwrap();
        std::fs::write(dir.join("kaputt.json"), "{ das ist kein json").unwrap();
        std::fs::write(dir.join("notiz.txt"), "kein Theme").unwrap();

        let (themes, problems) = load_themes_in(&dir);
        let own = themes.iter().find(|(n, _)| n == "Nacht");
        let (_, palette) = own.expect("the own theme must be found");
        assert_eq!(palette.wall, Color32::from_rgb(0, 0, 0x10));
        assert_eq!(palette.accent, Color32::from_rgb(0, 0xAA, 0xFF));
        // the broken file is reported, the rest still loads
        assert!(
            problems.iter().any(|p| p.contains("kaputt")),
            "{problems:?}"
        );
        assert!(themes.iter().any(|(n, _)| n == "Crosspoint"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_file_without_a_name_uses_the_file_name() {
        let dir = temp_dir("noname");
        std::fs::write(dir.join("meins.json"), r##"{"ink":"#FFFFFF"}"##).unwrap();
        let (themes, _) = load_themes_in(&dir);
        assert!(themes.iter().any(|(n, _)| n == "meins"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn switching_the_palette_changes_the_colours_the_widgets_read() {
        let before = wall();
        set_palette(Palette {
            wall: Color32::from_rgb(1, 2, 3),
            ..Palette::default()
        });
        assert_eq!(wall(), Color32::from_rgb(1, 2, 3));
        set_palette(Palette::default());
        assert_eq!(wall(), before);
    }
}
