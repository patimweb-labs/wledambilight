# Changelog

## [0.5.1](https://github.com/patimweb-labs/wledambilight/compare/v0.5.0...v0.5.1) (2026-11-02)


### Documentation

* Ablauf ueber Branch und Pull Request festschreiben ([#14](https://github.com/patimweb-labs/wledambilight/issues/14)) ([f228893](https://github.com/patimweb-labs/wledambilight/commit/f2288935e15c241c1c7afa46ad8cafcd1c5a61fd))

### Bug Fixes

* **release:** Linux-Job installiert die Tauri-Systempakete — der Bau brach in `pango-sys` ab
* **release:** `icons/icon.ico` ergänzt, ohne das die Windows-Ressourcendatei nicht entsteht
* **release:** Windows- und Linux-Pakete fehlten im 0.5.0-Release, nur macOS wurde gebaut
* **flatpak:** `shared-modules` als Untermodul eingebunden (im CI-Checkout fehlte es) und die
  Oberfläche wird vor dem Sandkasten gebaut, weil `npm ci` dort abbricht
* **tauri:** `tauri.conf.json` stand auf 0.3.0; sie folgt jetzt der Version aus `Cargo.toml`

## [0.5.0](https://github.com/patimweb-labs/wledambilight/compare/v0.4.0...v0.5.0) (2026-11-02)


### ⚠ BREAKING CHANGES

* anderes Toolkit, andere Abhängigkeiten.

### Features

* Oberfläche als Web-Anwendung (Vue + Tauri) ([f336273](https://github.com/patimweb-labs/wledambilight/commit/f33627356e04929c698f5a623ecb56c98ba7d4a4))

## [0.4.0](https://github.com/patimweb-labs/wledambilight/compare/v0.3.0...v0.4.0) (2026-11-02)


### Features

* test picture instead of the screen ([664e11c](https://github.com/patimweb-labs/wledambilight/commit/664e11cefefe641c5d0ab408d015f05406658b46))
* the LED tape shows the strip as it lies, with an LED 0 tag ([4dc4c84](https://github.com/patimweb-labs/wledambilight/commit/4dc4c8475e42a82ba7c10feabbbb711d821af3d6))
* themes at runtime, Crosspoint look as the default ([cd09647](https://github.com/patimweb-labs/wledambilight/commit/cd0964713c18b9e75275e08e4974fe711c3277c4))


### Bug fixes

* change the resolution without a new portal session ([5aad2ea](https://github.com/patimweb-labs/wledambilight/commit/5aad2eae13381fcb864ba4d903fed8a6ddd96e95))
* move a zone's LED block and split only that zone ([c6c62ed](https://github.com/patimweb-labs/wledambilight/commit/c6c62ed047148b5ca69fc951c81908a0351ee54f))
* switch test picture and screen back and forth instantly ([33f2110](https://github.com/patimweb-labs/wledambilight/commit/33f2110cb761ae3de9f8a63de42588b018503ef3))
* the up/down arrows of the zone list were dead ([11a9a24](https://github.com/patimweb-labs/wledambilight/commit/11a9a24e0e30cd27c0941ff1c8fadaba68ea2e19))


### Build and packaging

* bump screencapturekit from 11.0.0 to 11.1.1 ([fdef499](https://github.com/patimweb-labs/wledambilight/commit/fdef4997fae8f0819f7aa9799b8002f8afd57ed2))
* bump screencapturekit from 11.0.0 to 11.1.1 ([379efbf](https://github.com/patimweb-labs/wledambilight/commit/379efbf1b1b95725f81041f25280a8e16c59b3bb))
* bump the cargo-patch group with 2 updates ([7f351c9](https://github.com/patimweb-labs/wledambilight/commit/7f351c925085d7beb77211fcd053ed3cbf6a902b))
* bump the cargo-patch group with 2 updates ([5290a84](https://github.com/patimweb-labs/wledambilight/commit/5290a8481792a07260a004e2e61a5216cde70114))
* bump ureq from 3.4.2 to 3.5.0 ([45e58fd](https://github.com/patimweb-labs/wledambilight/commit/45e58fd659c21816660da9e1e09fcbb5c35b18b0))
* bump ureq from 3.4.2 to 3.5.0 ([37219c4](https://github.com/patimweb-labs/wledambilight/commit/37219c4c4969372d8e2a7abb13b0d09580bdb5af))

## [0.3.0](https://github.com/Smotherer007/wledambilight/compare/v0.2.0...v0.3.0) (2026-10-20)


### Features

* OpenRGB, Razer Chroma, Govee and Nanoleaf as further device types ([0f89d59](https://github.com/Smotherer007/wledambilight/commit/0f89d595458e286490e2011214824d30eb369b26))

## [0.2.0](https://github.com/Smotherer007/wledambilight/compare/v0.1.0...v0.2.0) (2026-10-20)


### Features

* cross-platform support, new interface and release pipeline ([0cb446d](https://github.com/Smotherer007/wledambilight/commit/0cb446dc9fb86a4364305a3ad990ac8de6c9fb00))


### Bug fixes

* DDP test on macOS (connected UDP socket cannot be re-connected) ([be3a98d](https://github.com/Smotherer007/wledambilight/commit/be3a98d1196f2448a61ffbe2545f9227a50c0891))


### Refactoring

* adapt to the egui 0.37 API ([9f06516](https://github.com/Smotherer007/wledambilight/commit/9f06516c498798cf3463a4e62ddceec47bd8b34b))


### Build and packaging

* bump the cargo group with 3 updates ([4bfa354](https://github.com/Smotherer007/wledambilight/commit/4bfa3547e04e367563bcf1df1795d59d377603d0))
* bump the cargo group with 3 updates ([4f88bfa](https://github.com/Smotherer007/wledambilight/commit/4f88bfa5c223edac071f3f9641676a44da33baeb))
* link the Swift runtime rpath on macOS ([8ddfcb6](https://github.com/Smotherer007/wledambilight/commit/8ddfcb60730a2cef9d3eb033ca90e703c886353a))

## Changelog

All notable changes are listed here. New entries are added above by
[release-please](https://github.com/googleapis/release-please) from the
commit messages (see [CONTRIBUTING.md](CONTRIBUTING.md)).

## Before the first release (on top of 0.1.0)

### Features

- Five device types in one setup: WLED, OpenRGB, Razer Chroma, Govee and Nanoleaf, each with its own output driver and reconnect handling
- One search finds every device kind (mDNS, subnet sweep, Govee LAN scan, local OpenRGB and Synapse); Nanoleaf pairing included
- Windows (Windows Graphics Capture) and macOS (ScreenCaptureKit) support next to Linux; on both the app's own window is left out of the capture
- Native tray icon on Windows and macOS, monitor selection on both
- Redesigned interface: light spill around the preview, inspector with tabs, switches and segmented controls, empty states, embedded Inter font, new app icon
- German and English interface, following the system language
- Device discovery via mDNS on every OS (no `avahi-browse` needed); add devices by IP or host name
- `--minimized`, `--version`, `--help`; `--config` is honoured now
- `AMBILIGHT_DEMO=1` test picture
- Release pipeline with packages for all platforms, Flatpak manifest, GPL-3.0 license

### Bug fixes

- Zone edits now apply immediately (the sampling plan was only rebuilt when the capture size changed)
- A disabled device no longer shifts the colours of the devices after it
- Saturation scales the existing saturation instead of replacing it (grey areas no longer turn red)
- DDP: only the last packet of a frame sets the PUSH flag, so strips with more than 480 LEDs no longer tear; sequence numbers stay in 1–15
- LED rulers in the preview show the correct colours for reversed and BGR devices
- The incoming frame rate is measured (was always 0)
- The portal restore token is no longer overwritten by the UI, so the share dialog does not come back
- Arrow keys no longer move the zone while typing in a text field
- Pause respects the per-device frame rate limit
- The configuration is written atomically
- Tests compile again

### Performance

- Frames are shared instead of copied between capture, pipeline and preview
- New frames are sampled once; ticks in between only advance the smoothing
