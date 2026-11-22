// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! WLED Ambilight: das Fenster in egui, direkt auf dem laufenden Kern.
//!
//! Aufbau wie in crosspoint: [`modell`] (was das Fenster weiß), [`logik`]
//! (Aktionen ausführen, reine Rechnungen), [`ansichten`] (`render(modell) →
//! Aktionen`), [`bausteine`] (Knöpfe, Schalter, Regler), [`thema`] (drei
//! Themes aus Tokens). [`betrieb`] startet Pipeline, Tray und Fenster in einem
//! Prozess.

pub mod ansichten;
pub mod app;
pub mod bausteine;
pub mod betrieb;
pub mod logik;
pub mod modell;
pub mod thema;
pub mod tray;
pub mod vorschau;

/// Reverse-DNS-Kennung: Wayland-App-ID, Desktop-Datei, Flatpak, Icon-Name.
pub const APP_ID: &str = "io.github.smotherer007.WledAmbilight";

/// Einmal beim Start: Schriften und die Bildlader für SVG-Icons und Logo.
/// Ohne die Lader bliebe jedes Icon ein leerer Kasten.
pub fn einrichten(ctx: &egui::Context) {
    thema::schriften(ctx);
    egui_extras::install_image_loaders(ctx);
}
