// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Taskleisten-Symbol. Auf allen drei Plattformen dasselbe Menü: zeigen, anhalten,
//! fortsetzen, beenden. Die Zustände kommen aus der Pipeline, nicht aus dem Menü.

use std::sync::atomic::Ordering;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager};

use crate::state::AppState;

pub fn install(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Fenster zeigen", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Senden anhalten", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Senden fortsetzen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("Standard-Icon fehlt".into()))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("WLED Ambilight")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "pause" | "resume" => {
                let paused = event.id.as_ref() == "pause";
                let state = app.state::<AppState>();
                state.shared().paused.store(paused, Ordering::Relaxed);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
