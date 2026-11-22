# 0001 – Fenster in egui statt Tauri und Vue

Stand: 2026-11-21 · entschieden

## Kontext

Die Oberfläche war eine Web-Anwendung (Vue 3 + Vite in `ui/`), ausgeliefert
mit Tauri 2, gebaut auf der Komponentenbibliothek `weppelmann-ui`. Dafür
brauchte jeder Bau Node, die Bibliothek als Nachbarordner, auf Linux WebKitGTK,
GTK und libayatana-appindicator; im Flatpak lief npm im Sandkasten nicht, die
Oberfläche wurde deshalb auf dem Host gebaut und mitgeliefert. Wunsch des
Nutzers: das Projekt „zu einem reinen Rust-Projekt machen“, wie crosspoint.

## Entscheidung

1. **egui 0.37 mit eframe (glow, AccessKit)**, aufgebaut wie
   `crosspoint/crates/fenster`: `modell.rs` (Summentypen, `Aktion`),
   `logik.rs` (führt Aktionen aus, rechnet, getestet), `ansichten/`
   (`render(modell) → Aktionen`), `bausteine.rs` (Knöpfe, Schalter, Regler,
   Abzeichen, Farbleiste), `thema.rs` (drei Themes aus Tokens).
2. **Workspace mit drei Paketen:** `crates/core` (der bisherige Kern,
   unverändert als Bibliothek `wled_ambilight`), `crates/fenster`, das
   Programm im Wurzelpaket. Die Befehlsschicht für die Web-Oberfläche
   (`commands.rs`, `state.rs`) entfällt; ihre reinen Rechnungen
   (Zonenfarben, bekannte Geräte) liegen in `core::telemetry`.
3. **Aufbau der Oberfläche bleibt:** Kopfzeile, links Geräte, Mitte Bildschirm
   mit Zonen, rechts Inspektor (Zonen, Bild, App), unten Ausgabe, Gerätedialog.
4. **Tray wie vor Tauri:** ksni unter Linux, tray-icon unter Windows/macOS.
   Mit `--minimized` schließt der Fensterknopf unter Linux das Fenster wirklich
   und „Fenster zeigen“ öffnet ein neues (`run_and_return`) – winit kann unter
   Wayland weder verstecken noch aus der Minimierung zurückholen. Unter
   Windows/macOS wird minimiert, weil ihr Tray die Ereignisschleife braucht.
5. **Tests mit `egui_kittest`:** bedienen über AccessKit (fehlende Namen lassen
   sie scheitern) und erzeugen mit `WLED_BILDER` die Bildschirmfotos.
6. **Windows nimmt das Fenster aus der Aufnahme** (`SetWindowDisplayAffinity`)
   – das lag unter Tauri brach und läuft jetzt wieder über den Fenstergriff
   von eframe.

## Alternativen

| Alternative | Warum nicht |
|---|---|
| Tauri behalten | Node, WebKitGTK, Bibliothek als Nachbarordner, npm-Bruch im Flatpak |
| iced | AccessKit in egui reifer; crosspoint und sprechwerk zeigen egui im Einsatz |
| Slint | eigene Auszeichnungssprache |
| die alte egui-Oberfläche zurückholen (vor `f336273`) | 3 500 Zeilen in einer Datei, eigene Theme-Dateien, keine Trennung Modell/Ansicht |

## Kosten

- Die Komponentenbibliothek `weppelmann-ui` wird hier nicht mehr benutzt; ihr
  Aussehen ist in `bausteine.rs` nachgebaut, nicht geteilt.
- Playwright-/axe-Prüfungen der Web-Oberfläche entfallen; an ihre Stelle tritt
  der AccessKit-Check von `egui_kittest` (jedes Widget braucht einen Namen).
- Scrollleisten erscheinen als schwebende Leisten von egui, nicht mehr über
  das eigene `is-scrolling`-Verhalten.
- Die innere Crates haben eine feste Version (`0.1.0`); die Programmversion
  steht nur im Wurzelpaket, weil release-please nur dort anhebt.
