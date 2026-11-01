// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Patrick Weppelmann

//! Tauri-Kontext (Berechtigungen, Icons) und Windows-Versionsinformationen.
fn main() {
    println!("cargo:rerun-if-changed=assets/icons/app.ico");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=capabilities/default.json");

    // macOS: ScreenCaptureKit ist teilweise Swift. Das Build-Skript der Abhängigkeit
    // setzt den rpath auf die Swift-Laufzeit, aber Cargo reicht Link-Argumente von
    // Abhängigkeiten nicht an unser Binärziel weiter.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icons/app.ico")
                .set("ProductName", "WLED Ambilight")
                .set("FileDescription", "WLED Ambilight")
                .set("CompanyName", "Patrick Weppelmann")
                .set(
                    "LegalCopyright",
                    "Copyright (C) 2026 Patrick Weppelmann, GPL-3.0",
                );
            if let Err(e) = res.compile() {
                println!("cargo:warning=could not embed the Windows icon: {e}");
            }
        }
    }

    tauri_build::build();
}
