// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Linux: StatusNotifierItem über `ksni` (reines Rust, D-Bus, kein GTK).

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{CheckmarkItem, StandardItem};
use ksni::{Icon, MenuItem, ToolTip};
use wled_ambilight::i18n::tr;

use super::{argb, Befehl, TrayZustand, ICON_32, ICON_64};

struct Symbol {
    z: TrayZustand,
}

fn icon(rgba: &[u8], kante: i32) -> Icon {
    Icon {
        width: kante,
        height: kante,
        data: argb(rgba),
    }
}

impl ksni::Tray for Symbol {
    fn id(&self) -> String {
        "wled-ambilight".into()
    }

    fn title(&self) -> String {
        "WLED Ambilight".into()
    }

    fn icon_name(&self) -> String {
        // Im Flatpak liegt das Icon im Theme des Hosts; sonst ist das Bild
        // unten der verlässliche Weg (nicht jeder Host fällt zurück).
        if std::env::var_os("FLATPAK_ID").is_some() {
            crate::APP_ID.into()
        } else {
            String::new()
        }
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![icon(ICON_32, 32), icon(ICON_64, 64)]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "WLED Ambilight".into(),
            description: if self.z.angehalten() {
                tr("angehalten", "paused").into()
            } else {
                String::new()
            },
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.z.senden(Befehl::Zeigen);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: tr("Fenster zeigen", "Show window").into(),
                activate: Box::new(|s: &mut Symbol| s.z.senden(Befehl::Zeigen)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            CheckmarkItem {
                label: tr("Senden anhalten", "Pause sending").into(),
                checked: self.z.angehalten(),
                activate: Box::new(|s: &mut Symbol| s.z.anhalten_umschalten()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: tr("Beenden", "Quit").into(),
                activate: Box::new(|s: &mut Symbol| s.z.senden(Befehl::Beenden)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub struct Tray {
    handle: Handle<Symbol>,
}

impl Tray {
    /// Symbol anmelden. `None`, wenn kein StatusNotifierWatcher antwortet.
    pub fn starten(z: TrayZustand) -> Option<Self> {
        // Im Flatpak darf kein eigener Busname belegt werden (Sandbox).
        let sandkasten = std::env::var_os("FLATPAK_ID").is_some();
        match (Symbol { z }).disable_dbus_name(sandkasten).spawn() {
            Ok(handle) => Some(Tray { handle }),
            Err(e) => {
                eprintln!("wled-ambilight: kein Tray ({e})");
                None
            }
        }
    }

    /// Haken, Tooltip und Sprache neu lesen.
    pub fn auffrischen(&self) {
        self.handle.update(|_| {});
    }
}
