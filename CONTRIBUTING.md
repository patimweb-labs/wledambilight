# Contributing and releasing

## Development

```sh
cargo run                       # GUI
AMBILIGHT_DEMO=1 cargo run      # test picture instead of the screen
AMBILIGHT_STATS=1 cargo run     # timing / flicker statistics on stderr
cargo test --workspace
cargo clippy --all-targets --workspace -- -D warnings
cargo fmt --all
```

Pure Rust: the window is egui (`crates/fenster`), the core a library
(`crates/core`, imported as `wled_ambilight`). The window reads the shared
state of the pipeline (`worker::Shared`) and writes changes back as a whole
`Config`; it never talks to a device directly.

### Project layout

| Path | Purpose |
| --- | --- |
| `src/main.rs` | command line, headless mode |
| `crates/fenster/src/betrieb.rs` | one process: pipeline, tray, window lifecycle |
| `crates/fenster/src/modell.rs` | what the window knows: sum types and `Aktion` |
| `crates/fenster/src/logik.rs` | runs actions, background network work, pure maths (tested) |
| `crates/fenster/src/ansichten/` | views: `render(model) → actions` (header, devices, stage, inspector, output, dialog) |
| `crates/fenster/src/bausteine.rs` | widgets: buttons, switches, sliders, badges, LED strip, icons |
| `crates/fenster/src/thema.rs` | three themes from `tokens/*.tokens.json` (`build.rs`) |
| `crates/fenster/src/tray/` | tray icon: `ksni` on Linux, `tray-icon` on Windows/macOS |
| `crates/fenster/tests/` | window tests via AccessKit (`egui_kittest`), screenshots |
| `crates/core/src/telemetry.rs` | what the window reads from the pipeline, zone colours |
| `crates/core/src/i18n.rs` | German/English switch (`tr("Deutsch", "English")`) |
| `crates/core/src/worker.rs` | pipeline thread: capture → engine on a fixed tick |
| `crates/core/src/engine.rs` | median, smoothing, grading, per-device rate limits, sending |
| `crates/core/src/sampler.rs` | zones → pixel cells → mean colours |
| `crates/core/src/capture/` | one backend per OS (`linux.rs`, `windows.rs`, `macos.rs`), `scale.rs` downscaler, `demo.rs` |
| `crates/core/src/wled.rs` | WLED: `/json/info`, DDP and DRGB senders, mDNS and subnet sweep helpers |
| `crates/core/src/output/` | one output per device: `mod.rs` (threaded driver with reconnect), `openrgb.rs`, `razer.rs`, `govee.rs`, `nanoleaf.rs` |
| `crates/core/src/discovery.rs` | “Search the network” for every device kind |
| `crates/core/src/config.rs` | config file, migration of old fields, atomic save |
| `assets/` | logo SVGs (source of all icons), generated icons, Inter font |
| `packaging/` | desktop file + AppStream, Flatpak, macOS `Info.plist`, Windows installer, `icons/erzeugen.sh` |

Windows and macOS code is only compiled on those systems; CI builds and tests
all three on every push and pull request.

### Adding a device type

1. Add a variant to `DeviceKind` (`crates/core/src/config.rs`).
2. Write a `Driver` in `crates/core/src/output/<name>.rs`: `connect`, `write` (one frame),
   optionally `keepalive` (called about once per second) and `close`. Run it
   through `Threaded::spawn` in `output::open` – reconnecting, frame dropping
   and status reporting come for free.
3. Find the devices in `crates/core/src/discovery.rs` and map them to a `Found`.
4. Device-specific settings go into the device dialog in
   `crates/fenster/src/ansichten/dialog.rs`.
5. Unit-test the packet format; a fake server in the test (see `openrgb.rs`)
   covers the whole driver.

Only add protocols whose licence fits GPL-3.0 (open protocols, documented
network APIs). Vendor SDKs that must be linked under a proprietary licence
(e.g. Corsair iCUE) cannot be included.

### Icons

`assets/wled-ambilight-logo.svg` and its variants (`-zeichen`, `-hell`,
`-mono`, `-klein`, `-symbolic`) are the source ([ADR 0002](docs/adr/0002-logo-als-svg.md)).
After changing them:

```sh
packaging/icons/erzeugen.sh     # PNG 16–1024, app.ico, raw RGBA for window and tray
```

## Commit messages

Releases are generated from the commit history, so commits on `main` follow
[Conventional Commits](https://www.conventionalcommits.org/):

| Prefix | Meaning | Version bump (before 1.0) |
| --- | --- | --- |
| `feat:` | new feature | minor (0.**x**.0) |
| `fix:` | bug fix | patch |
| `perf:`, `refactor:`, `docs:`, `build:` | listed in the changelog | patch |
| `ci:`, `test:`, `chore:` | not listed | – |
| `feat!:` or `BREAKING CHANGE:` in the body | breaking change | minor before 1.0, major after |

Examples: `feat: per-zone gamma`, `fix(windows): tray icon after explorer restart`.

## Release process

Everything after the merge is automatic
([.github/workflows/release.yml](.github/workflows/release.yml)):

1. Push commits to `main`.
2. [release-please](https://github.com/googleapis/release-please) opens or
   updates a pull request **“chore: release x.y.z”**. It bumps the version in
   `Cargo.toml`, `Cargo.lock` and the AppStream metainfo and writes
   `CHANGELOG.md`.
3. Merge that pull request when you want to ship. release-please creates the
   tag `vx.y.z` and the GitHub release with the changelog.
4. The same workflow builds and attaches:
   - Linux: `.tar.gz`, `.deb`, `.rpm` (x86_64; ARM64 can be enabled), `.flatpak`
   - Windows: installer (`-setup.exe`) and portable `.zip`
   - macOS: universal `.dmg` (Apple silicon + Intel)
   - `SHA256SUMS.txt`

To rebuild an existing release, run the workflow manually
(*Actions → Release → Run workflow*) with the tag, or push a tag `v*` by hand.

### One-time repository settings

- *Settings → Actions → General → Workflow permissions*: **Read and write
  permissions** and **Allow GitHub Actions to create and approve pull
  requests** (release-please needs both).
- Optional, for signed and notarised macOS builds – repository secrets:

  | Secret | Content |
  | --- | --- |
  | `MACOS_CERTIFICATE` | Developer ID Application certificate, `.p12`, base64 encoded |
  | `MACOS_CERTIFICATE_PASSWORD` | password of the `.p12` |
  | `MACOS_SIGNING_IDENTITY` | e.g. `Developer ID Application: Patrick Weppelmann (TEAMID)` |
  | `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` | for `notarytool` (app-specific password) |

  Without them the app is ad-hoc signed and users confirm it once.

### Flathub

See [packaging/flatpak/README.md](packaging/flatpak/README.md).

## License of contributions

The project is licensed under GPL-3.0-only. By submitting a contribution you
agree that it is published under this license.
