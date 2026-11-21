// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Diagnose der Bildschirmaufnahme ausserhalb des Fensters.
//!
//! Startet die Aufnahme wie die Anwendung und gibt alle 500 ms den Status aus.
//! Damit laesst sich trennen, ob das Portal, GStreamer oder die App klemmt:
//!
//! ```sh
//! WAYLAND_DISPLAY=wayland-1 GDK_BACKEND=wayland XDG_RUNTIME_DIR=/run/user/1000 \
//! XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland \
//!   cargo run --release --example capture_probe -- 320 30
//! ```

use std::time::Duration;

use wled_ambilight::capture::{CaptureOptions, CaptureStatus, ScreenCapture};

fn main() {
    let mut args = std::env::args().skip(1);
    let width: u32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(320);
    let fps: u32 = args.next().and_then(|v| v.parse().ok()).unwrap_or(30);

    println!("Aufnahme starten: {width}px, {fps} fps, Bildschirm (kein Testbild)");
    let capture = ScreenCapture::start(CaptureOptions {
        width,
        fps,
        monitor: 0,
        restore_token: None,
        test_image: false,
    });

    let mut last_seq = u64::MAX;
    for step in 0..24 {
        std::thread::sleep(Duration::from_millis(500));
        let status = capture.status();
        let frame = capture.frame();
        let new_frame = frame.seq != last_seq;
        last_seq = frame.seq;
        println!(
            "{:>4} ms  Status={:?}  Bildschirm={:?}  Puffer={}x{}  Frames={}{}",
            (step + 1) * 500,
            status,
            capture.screen(),
            frame.w,
            frame.h,
            frame.seq,
            if new_frame { " (neu)" } else { "" },
        );
        if matches!(status, CaptureStatus::Running { .. }) && frame.seq > 0 {
            println!("Aufnahme laeuft.");
            break;
        }
        if let CaptureStatus::Error(message) = status {
            eprintln!("Fehler: {message}");
            break;
        }
    }

    capture.stop();
    println!("beendet.");
}
