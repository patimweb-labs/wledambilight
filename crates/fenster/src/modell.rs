// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Das Modell der Oberfläche: alles, was das Fenster zeigt.
//!
//! Die Konfiguration ([`Config`]) steht hier als die Fassung, die das Fenster
//! bearbeitet; jede Änderung geht über [`Aktion`] und `logik::ausfuehren`
//! sofort an die Pipeline (`Shared::set_config`) und verzögert auf die Platte.
//! Die Messwerte ([`Telemetry`]) liest das Fenster jedes Bild neu. Es gibt
//! keine zweite Wahrheit – die Ansichten lesen nur, sie schreiben nichts.

use std::time::Instant;

use wled_ambilight::config::{Config, DeviceCfg, Zone, ZonePreset};
use wled_ambilight::discovery::Found;
use wled_ambilight::telemetry::Telemetry;

use crate::thema::Thema;

/// Die drei Reiter rechts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Reiter {
    #[default]
    Zonen,
    Bild,
    App,
}

/// Netzwerksuche.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Suche {
    #[default]
    Ruhe,
    Laeuft,
    /// Geräte, die noch nicht eingerichtet sind (leer: nichts Neues gefunden).
    Fertig(Vec<Found>),
}

/// Was der Gerätedialog gerade im Hintergrund tut.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Arbeit {
    #[default]
    Keine,
    /// WLED fragen (Name, LEDs, Firmware)
    Prueft,
    /// Nanoleaf paaren (Knopf am Controller gedrückt halten)
    Paart,
    /// Controller eines OpenRGB-Servers holen
    Holt,
}

/// Gerät einrichten oder bearbeiten. Der Entwurf wird erst mit
/// [`Aktion::DialogUebernehmen`] Teil der Konfiguration.
#[derive(Debug, Clone, PartialEq)]
pub struct GeraetDialog {
    pub entwurf: DeviceCfg,
    /// Index in `config.devices`; `None`: neues Gerät
    pub index: Option<usize>,
    /// OpenRGB: Controller zur Auswahl
    pub controller: Vec<Found>,
    pub notiz: String,
    pub arbeit: Arbeit,
}

/// Meldung unter der Kopfzeile.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Meldung {
    #[default]
    Keine,
    Hinweis(String),
    Fehler(String),
}

/// Laufendes Ziehen einer Zone auf der Bühne.
#[derive(Debug, Clone, PartialEq)]
pub struct Zug {
    pub zone: usize,
    pub art: Zugart,
    /// die Zone, wie sie beim Anfassen war
    pub von: Zone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zugart {
    Verschieben,
    Groesse,
}

/// Vorschau der Aufnahme: das zuletzt gezeigte Bild.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Vorschau {
    #[default]
    Leer,
    /// Nummer des gezeigten Bildes der Pipeline
    Bild { seq: u64 },
}

/// Alles, was das Fenster weiß.
#[derive(Debug, Clone)]
pub struct Oberflaeche {
    pub config: Config,
    pub telemetrie: Telemetry,
    pub reiter: Reiter,
    /// gewählte Zone (Index in `config.zones`)
    pub aktiv: usize,
    pub suche: Suche,
    /// „Per Adresse hinzufügen“
    pub adresse: String,
    pub dialog: Option<GeraetDialog>,
    pub meldung: Meldung,
    pub zug: Option<Zug>,
    /// Senden von Hand angehalten (Knopf oder Tray). Was die Pipeline
    /// wirklich tut, steht in `telemetrie.paused`.
    pub angehalten: bool,
    /// Das Fenster zeigt eingefrorene Werte (Linux, Fenster im Hintergrund:
    /// es würde sich sonst selbst aufnehmen).
    pub eingefroren: bool,
    pub vorschau: Vorschau,
    /// Vorschaubilder je Sekunde, die wirklich ankommen
    pub vorschau_fps: f32,
    /// Ab wann die Konfiguration auf die Platte darf (entprellt).
    pub speichern_ab: Option<Instant>,
}

impl Oberflaeche {
    pub fn neu(config: Config) -> Self {
        Oberflaeche {
            config,
            telemetrie: Telemetry::default(),
            reiter: Reiter::default(),
            aktiv: 0,
            suche: Suche::default(),
            adresse: String::new(),
            dialog: None,
            meldung: Meldung::default(),
            zug: None,
            angehalten: false,
            eingefroren: false,
            vorschau: Vorschau::default(),
            vorschau_fps: 0.0,
            speichern_ab: None,
        }
    }

    /// Das Theme folgt der Konfiguration (eine Wahrheit).
    pub fn thema(&self) -> Thema {
        Thema::aus_name(&self.config.settings.theme)
    }
}

/// Welche Zonen „Auf 4 Kanten“ und „Gerät pro Kante“ anlegen – in der
/// Reihenfolge, in der ein Streifen üblicherweise läuft (im Uhrzeigersinn ab
/// oben links; bei einem Streifen laufen unten und links darum rückwärts).
pub const KANTEN: [ZonePreset; 4] = [
    ZonePreset::Top,
    ZonePreset::Right,
    ZonePreset::Bottom,
    ZonePreset::Left,
];

/// Jede Bedienung wird zu einer Aktion; `logik::ausfuehren` führt sie aus.
#[derive(Debug, Clone, PartialEq)]
pub enum Aktion {
    // ------------------------------------------------------------- Kopf --
    Anhalten(bool),
    AufnahmeNeu,
    BerechtigungVerwerfen,
    // ---------------------------------------------------------- Geräte --
    Suchen,
    Hinzufuegen(Found),
    AdresseTippen(String),
    AdresseHinzufuegen,
    /// Gerät an Index durch die geänderte Fassung ersetzen
    Geraet(usize, DeviceCfg),
    GeraetEntfernen(usize),
    // ---------------------------------------------------------- Dialog --
    /// `None`: neues Gerät einrichten
    DialogOeffnen(Option<usize>),
    DialogEntwurf(DeviceCfg),
    DialogPruefen,
    DialogPaaren,
    DialogControllerHolen,
    DialogUebernehmen,
    DialogSchliessen,
    // ----------------------------------------------------------- Zonen --
    ZoneWaehlen(usize),
    /// Zone an Index durch die geänderte Fassung ersetzen
    Zone(usize, Zone),
    ZoneNeu,
    ZonenAufKanten,
    GeraetJeKante,
    /// Zone um einen Platz nach oben (-1) oder unten (+1)
    ZoneRuecken(usize, i32),
    ZoneEntfernen(usize),
    ZugBeginnen(Zug),
    ZugEnde,
    // ------------------------------------------------------ Einstellung --
    Reiter(Reiter),
    Einstellungen(Box<wled_ambilight::config::Settings>),
    BildZuruecksetzen,
    MeldungWeg,
}
