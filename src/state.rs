// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Gemeinsamer Zustand zwischen Oberfläche und Pipeline.
//!
//! Die Oberfläche hält keine eigene Kopie der Wahrheit: sie liest den Zustand der
//! Pipeline und schreibt Änderungen über `Shared::set_config` zurück. Damit gibt es
//! genau eine Quelle für Geräte, Zonen und Messwerte.

use std::sync::Arc;

use crate::config::Config;
use crate::worker::{Pipeline, Shared};

pub struct AppState {
    /// Hält den Pipeline-Thread am Leben (Drop stoppt ihn).
    pipeline: Pipeline,
    /// Nur im Autostart-Modus (`--minimized`) bleibt die App am Leben, wenn das
    /// Fenster geschlossen wird. Sonst beendet Schließen die Anwendung — ein
    /// unsichtbarer Prozess ohne Fenster ist keine Funktion, sondern eine Falle.
    keep_in_tray: bool,
}

impl AppState {
    pub fn new(cfg: &Config, keep_in_tray: bool) -> Self {
        Self {
            pipeline: Pipeline::start(cfg),
            keep_in_tray,
        }
    }

    pub fn keep_in_tray(&self) -> bool {
        self.keep_in_tray
    }

    pub fn shared(&self) -> Arc<Shared> {
        self.pipeline.shared.clone()
    }

    /// Konfiguration setzen und auf die Platte schreiben.
    ///
    /// Der Portal-Token gehört der Pipeline; `set_config` behält ihn, deshalb wird
    /// die Datei aus dem *gespeicherten* Zustand geschrieben und nicht aus der
    /// Fassung, die die Oberfläche geschickt hat.
    pub fn apply_config(&self, cfg: &Config) {
        let shared = self.shared();
        shared.set_config(cfg);
        shared.config().save();
    }
}
