// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

// No console window next to the GUI on Windows (release builds).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! WLED Ambilight — screen capture to WLED, on Linux, Windows and macOS.
//!
//! Das Fenster ist egui (`crates/fenster`), der Kern liegt in `crates/core`.
//! Hier stehen nur die Kommandozeile und der Betrieb ohne Fenster.

use std::time::Duration;

use wled_ambilight::config::{self, Config};
use wled_ambilight::{i18n, worker};

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

    wled_ambilight_fenster::betrieb::starten(minimized);
}
