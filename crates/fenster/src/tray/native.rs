// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Windows-Infobereich und macOS-Menüleiste über `tray-icon`.
//!
//! Das Symbol muss im Hauptthread entstehen und dort bleiben; eframe baut die
//! App dort, deshalb ruft `betrieb` [`Tray::starten`] beim Anlegen des
//! Fensters. Menüklicks kommen in der Ereignisschleife an und setzen nur
//! Zustände in [`TrayZustand`].

use std::cell::Cell;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use wled_ambilight::i18n::{self, tr};

use super::{Befehl, TrayZustand, ICON_32, ICON_64};

const ZEIGEN: &str = "zeigen";
const ANHALTEN: &str = "anhalten";
const BEENDEN: &str = "beenden";

pub struct Tray {
    _icon: TrayIcon,
    zeigen: MenuItem,
    anhalten: CheckMenuItem,
    beenden: MenuItem,
    z: TrayZustand,
    sprache: Cell<i18n::Lang>,
}

impl Tray {
    pub fn starten(z: TrayZustand) -> Option<Self> {
        let zeigen = MenuItem::with_id(ZEIGEN, tr("Fenster zeigen", "Show window"), true, None);
        let anhalten = CheckMenuItem::with_id(
            ANHALTEN,
            tr("Senden anhalten", "Pause sending"),
            true,
            z.angehalten(),
            None,
        );
        let beenden = MenuItem::with_id(BEENDEN, tr("Beenden", "Quit"), true, None);
        let menu = Menu::new();
        let t1 = PredefinedMenuItem::separator();
        let t2 = PredefinedMenuItem::separator();
        for eintrag in [
            &zeigen as &dyn tray_icon::menu::IsMenuItem,
            &t1,
            &anhalten,
            &t2,
            &beenden,
        ] {
            if let Err(e) = menu.append(eintrag) {
                eprintln!("wled-ambilight: Tray-Menü: {e}");
            }
        }
        // Die macOS-Menüleiste will ein kleines Bild, Windows verkleinert selbst.
        let (rgba, kante) = if cfg!(target_os = "macos") {
            (ICON_32.to_vec(), 32)
        } else {
            (ICON_64.to_vec(), 64)
        };
        let bild = tray_icon::Icon::from_rgba(rgba, kante, kante).ok()?;
        let icon = match TrayIconBuilder::new()
            .with_id("wled-ambilight")
            .with_tooltip("WLED Ambilight")
            .with_icon(bild)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(cfg!(target_os = "macos"))
            .build()
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!("wled-ambilight: kein Tray ({e})");
                return None;
            }
        };
        let s = z.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            if e.id == ZEIGEN {
                s.senden(Befehl::Zeigen);
            } else if e.id == ANHALTEN {
                s.anhalten_umschalten();
            } else if e.id == BEENDEN {
                s.senden(Befehl::Beenden);
                // Ein minimiertes Fenster zeichnet unter Umständen kein Bild
                // mehr und sähe den Befehl nie: dann endet das Programm hier.
                std::thread::spawn(|| {
                    std::thread::sleep(std::time::Duration::from_secs(4));
                    std::process::exit(0);
                });
            }
        }));
        let s = z.clone();
        TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
            // Windows: Linksklick auf das Symbol holt das Fenster zurück.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = e
            {
                s.senden(Befehl::Zeigen);
            }
        }));
        Some(Tray {
            _icon: icon,
            zeigen,
            anhalten,
            beenden,
            z,
            sprache: Cell::new(i18n::current()),
        })
    }

    /// Haken und Sprache nachziehen. Billig; das Fenster ruft es jedes Bild.
    pub fn auffrischen(&self) {
        let an = self.z.angehalten();
        if self.anhalten.is_checked() != an {
            self.anhalten.set_checked(an);
        }
        let sprache = i18n::current();
        if self.sprache.get() != sprache {
            self.sprache.set(sprache);
            self.zeigen.set_text(tr("Fenster zeigen", "Show window"));
            self.anhalten
                .set_text(tr("Senden anhalten", "Pause sending"));
            self.beenden.set_text(tr("Beenden", "Quit"));
        }
    }
}
