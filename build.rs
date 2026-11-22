// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Plattformteile des Programms: Symbol und Versionsangaben in der .exe
//! (Windows), Swift-Laufzeit für ScreenCaptureKit (macOS).
fn main() {
    println!("cargo:rerun-if-changed=assets/icons/app.ico");

    // macOS: ScreenCaptureKit ist teilweise Swift. Das Build-Skript der Abhängigkeit
    // setzt den rpath auf die Swift-Laufzeit, aber Cargo reicht Link-Argumente von
    // Abhängigkeiten nicht an unser Binärziel weiter.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }

    // Windows: das Symbol im Explorer und in der Taskleiste kommt aus der
    // Ressource, nicht aus dem Fenster. Früher schrieb das `tauri_build`.
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/app.ico");
        res.set("ProductName", "WLED Ambilight");
        res.set("FileDescription", "WLED Ambilight");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Windows-Ressource nicht geschrieben: {e}");
        }
    }
}
