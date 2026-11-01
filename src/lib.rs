// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Kern der Anwendung: Aufnahme, Sampling, Ausgabe, Konfiguration.
//!
//! Die Oberfläche kennt diesen Kern nur über die Befehlsschicht (`commands`).
//! Alles hier ist ohne Fenster testbar — die Tests in `tests/` tun genau das.

pub mod capture;
pub mod commands;
pub mod config;
pub mod discovery;
pub mod engine;
pub mod i18n;
pub mod output;
pub mod sampler;
pub mod state;
pub mod tray;
pub mod wled;
pub mod worker;
