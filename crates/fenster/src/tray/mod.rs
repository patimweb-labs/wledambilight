// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Tray-Symbol: Fenster zeigen, Senden anhalten, Beenden – auf allen drei
//! Plattformen dasselbe Menü.
//!
//! - Linux: StatusNotifierItem über `ksni` (KDE, GNOME mit AppIndicator, Xfce …)
//! - Windows / macOS: Infobereich bzw. Menüleiste über `tray-icon`
//!
//! Das Menü wirkt auf gemeinsame Zustände ([`TrayZustand`]); was das Fenster
//! betrifft (zeigen, beenden), geht als [`Befehl`] an den Lebenszyklus in
//! `betrieb`. Wie ein Fenster „weggelegt“ wird, unterscheidet sich und steht
//! als Daten in [`MINIMIEREN`]: unter Wayland lässt sich ein Fenster weder
//! verstecken noch aus der Minimierung zurückholen, Windows und macOS
//! brauchen für ihr Tray dagegen eine laufende Ereignisschleife.

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use wled_ambilight::worker::Shared;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Tray;

#[cfg(any(target_os = "windows", target_os = "macos"))]
mod native;
#[cfg(any(target_os = "windows", target_os = "macos"))]
pub use native::Tray;

/// Andere Plattformen (BSD …): kein Tray, das Fenster bleibt der einzige Weg.
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub struct Tray;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
impl Tray {
    pub fn starten(_: TrayZustand) -> Option<Self> {
        None
    }
    pub fn auffrischen(&self) {}
}

/// `true`: mit Tray wird das Fenster beim Schließen minimiert (Windows,
/// macOS). `false`: es wird geschlossen und bei Bedarf neu geöffnet (Linux).
pub const MINIMIEREN: bool = cfg!(any(target_os = "windows", target_os = "macos"));

/// Rohe RGBA-Bilder des Zeichens (`packaging/icons/erzeugen.sh`).
pub const ICON_32: &[u8] = include_bytes!("../../../../assets/icons/tray-32.rgba");
pub const ICON_64: &[u8] = include_bytes!("../../../../assets/icons/tray-64.rgba");

/// Was das Fenster tun soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Befehl {
    Zeigen,
    Beenden,
}

/// Was das Tray-Menü liest und schreibt.
#[derive(Clone)]
pub struct TrayZustand {
    pub shared: Arc<Shared>,
    pub von_hand: Arc<AtomicBool>,
    pub befehle: Sender<Befehl>,
    /// Das offene Fenster, das bei einem Befehl geweckt wird.
    pub wecker: Arc<Mutex<Option<egui::Context>>>,
}

impl TrayZustand {
    fn wecken(&self) {
        if let Ok(w) = self.wecker.lock() {
            if let Some(ctx) = w.as_ref() {
                ctx.request_repaint();
            }
        }
    }

    pub fn senden(&self, b: Befehl) {
        let _ = self.befehle.send(b);
        self.wecken();
    }

    pub fn angehalten(&self) -> bool {
        self.von_hand.load(Relaxed)
    }

    /// Senden anhalten oder fortsetzen – wirkt sofort, auch ohne Fenster.
    pub fn anhalten_umschalten(&self) {
        let an = !self.von_hand.load(Relaxed);
        self.von_hand.store(an, Relaxed);
        self.shared.paused.store(an, Relaxed);
        self.wecken();
    }
}

/// RGBA → ARGB32 in Netzwerk-Bytefolge, wie StatusNotifierItem es will.
pub fn argb(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argb_in_netzwerkfolge() {
        assert_eq!(
            argb(&[1, 2, 3, 4, 5, 6, 7, 8]),
            vec![4, 1, 2, 3, 8, 5, 6, 7]
        );
    }

    #[test]
    fn rohbilder_haben_die_richtige_groesse() {
        assert_eq!(ICON_32.len(), 32 * 32 * 4);
        assert_eq!(ICON_64.len(), 64 * 64 * 4);
    }

    #[test]
    fn anhalten_wirkt_ohne_fenster() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let z = TrayZustand {
            shared: Shared::new(&Default::default()),
            von_hand: Arc::new(AtomicBool::new(false)),
            befehle: tx,
            wecker: Arc::new(Mutex::new(None)),
        };
        z.anhalten_umschalten();
        assert!(z.angehalten() && z.shared.paused.load(Relaxed));
        z.anhalten_umschalten();
        assert!(!z.angehalten() && !z.shared.paused.load(Relaxed));
    }
}
