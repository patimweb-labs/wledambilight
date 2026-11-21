// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Kern der Anwendung: Aufnahme, Sampling, Ausgabe, Konfiguration.
//!
//! Das Fenster (`crates/fenster`) liest den gemeinsamen Zustand
//! ([`worker::Shared`]) und schreibt Änderungen als ganze [`config::Config`]
//! zurück. Alles hier ist ohne Fenster testbar.

pub mod capture;
pub mod config;
pub mod discovery;
pub mod engine;
pub mod i18n;
pub mod output;
pub mod sampler;
pub mod telemetry;
pub mod wled;
pub mod worker;
