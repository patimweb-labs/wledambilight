// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

// No console window next to the GUI on Windows (release builds).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! WLED Ambilight — screen capture to WLED, on Linux, Windows and macOS.
//!
//! The window is a web application (Vue). This file starts the pipeline, the tray
//! and the command layer; the user interface lives in `ui/`.

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
    let cfg = Config::load();
    i18n::init(&cfg.settings.language);
    if cfg.devices.is_empty() {
        eprintln!(
            "no devices configured in {} - start the GUI once to set them up",
            config::config_path().display()
        );
    }
    let signals = quit_signals::block();
    // The pipeline writes new portal tokens to disk itself.
    let pipeline = worker::Pipeline::start(&cfg);
    quit_signals::wait(signals);
    // Ctrl+C / systemd stop: hand the devices back before leaving
    pipeline.shutdown(Duration::from_secs(2));
    std::process::exit(0);
}

/// SIGINT/SIGTERM for `--headless`, so a service stop closes the outputs.
#[cfg(target_os = "linux")]
mod quit_signals {
    pub struct Set(libc::sigset_t);

    /// Block the signals in this thread *before* other threads start, so they
    /// inherit the mask and only `wait` receives them.
    pub fn block() -> Option<Set> {
        // SAFETY: plain libc calls on a zeroed, locally owned sigset_t.
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGINT);
            libc::sigaddset(&mut set, libc::SIGTERM);
            (libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut()) == 0)
                .then_some(Set(set))
        }
    }

    pub fn wait(set: Option<Set>) {
        let Some(set) = set else {
            loop {
                std::thread::park();
            }
        };
        let mut sig = 0;
        // SAFETY: `set` was initialised by `block`.
        unsafe {
            libc::sigwait(&set.0, &mut sig);
        }
    }
}

/// Elsewhere the default handlers end the process (no device hand-back).
#[cfg(not(target_os = "linux"))]
mod quit_signals {
    pub struct Set;
    pub fn block() -> Option<Set> {
        None
    }
    pub fn wait(_: Option<Set>) {
        loop {
            std::thread::park();
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
        .build(tauri::generate_context!())
        .expect("error while building WLED Ambilight")
        .run(|app, event| {
            // Beenden über Fenster, Tray oder System: erst die Geräte sauber
            // freigeben (Razer-Sitzung schließen, Govee/Nanoleaf zurückgeben).
            if let tauri::RunEvent::Exit = event {
                app.state::<AppState>().shutdown();
            }
        });
}
