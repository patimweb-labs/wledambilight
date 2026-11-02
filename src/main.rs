// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

// No console window next to the GUI on Windows (release builds).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! WLED Ambilight — screen capture to WLED, on Linux, Windows and macOS.
//!
//! The window is a web application (Vue). This file starts the pipeline, the tray
//! and the command layer; the user interface lives in `ui/`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{Emitter, Manager};

use wled_ambilight::config::{self, Config};
use wled_ambilight::state::AppState;
use wled_ambilight::{commands, i18n, tray, worker};

/// Reverse-DNS id: Wayland app id, desktop file, Flatpak, icon name.
pub const APP_ID: &str = "io.github.smotherer007.WledAmbilight";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
WLED Ambilight - screen capture to WLED

Usage: wled-ambilight [options]

Options:
  --minimized        start with the window minimised (autostart with tray)
  --headless         run without a window (servers, no tray)
  --config <file>    use this config file instead of the default
  -V, --version      print the version
  -h, --help         print this help
";

fn headless() -> ! {
    let mut cfg = Config::load();
    i18n::init(&cfg.settings.language);
    if cfg.devices.is_empty() {
        eprintln!(
            "no devices configured in {} - start the GUI once to set them up",
            config::config_path().display()
        );
    }
    let pipeline = worker::Pipeline::start(&cfg);
    loop {
        std::thread::sleep(Duration::from_secs(2));
        let token = pipeline.shared.restore_token();
        if token.is_some() && token != cfg.settings.restore_token {
            cfg.settings.restore_token = token;
            cfg.save();
        }
    }
}

/// Schickt Messwerte an die Oberfläche, solange das Fenster offen ist.
///
/// Ein Ereignis statt Polling: die Oberfläche zeichnet, was ankommt, und fragt
/// nichts selbst ab. 200 ms sind schnell genug für Zähler und die LED-Leiste.
fn start_telemetry(handle: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("telemetry".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_millis(200));
            let state = handle.state::<AppState>();
            let telemetry = commands::telemetry_now(state);
            if handle.emit("telemetry", telemetry).is_err() {
                break; // Fenster weg: Thread beenden
            }
        })
        .ok();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut run_headless = false;
    let mut minimized = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--headless" => run_headless = true,
            "--minimized" => minimized = true,
            "--config" => {
                let Some(path) = args.get(i + 1) else {
                    eprintln!("--config needs a file name");
                    std::process::exit(2);
                };
                config::set_config_path(path.into());
                i += 1;
            }
            "-V" | "--version" => {
                println!("wled-ambilight {VERSION}");
                return;
            }
            "-h" | "--help" => {
                print!("{HELP}");
                return;
            }
            // older macOS passes a process serial number when started from Finder
            other if other.starts_with("-psn_") => {}
            other => {
                eprintln!("unknown option: {other}\n\n{HELP}");
                std::process::exit(2);
            }
        }
        i += 1;
    }
    if run_headless {
        headless();
    }

    let cfg = Config::load();
    i18n::init(&cfg.settings.language);

    tauri::Builder::default()
        .manage(AppState::new(&cfg, minimized))
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::telemetry_now,
            commands::save_config,
            commands::restart_capture,
            commands::forget_permission,
            commands::set_paused,
            commands::monitors,
            commands::discover,
            commands::zone_preset,
            commands::device_template,
            commands::probe_wled,
            commands::pair_nanoleaf,
            commands::list_openrgb,
            commands::preview_frame,
        ])
        .setup(move |app| {
            tray::install(app)?;
            if minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            start_telemetry(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // Im Autostart-Modus (--minimized) legt Schliessen das Fenster nur weg,
            // damit Aufnahme und Taskleisten-Symbol weiterlaufen. In allen anderen
            // Faellen beendet Schliessen die Anwendung — wie jedes andere Programm.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let keep = window.app_handle().state::<AppState>().keep_in_tray();
                if keep {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running WLED Ambilight");
}

/// Ungenutzt, aber absichtlich vorhanden: erzwingt die Bindung von `Ordering`
/// in diesem Modul, damit der Tray-Code beim Umbau nicht still bricht.
#[allow(dead_code)]
fn _ordering_marker() -> Ordering {
    Ordering::Relaxed
}
