# Architektur

Stand 2026-11-02. Diese Datei beschreibt den Aufbau und die Grenzen; die Details
stehen in den Modulköpfen des Quelltexts.

## Zuschnitt

Ein Crate (`wled-ambilight`) mit Kern **und** Tauri-Host, ein Frontend (`ui/`,
Vue 3 + Vite). Der Kern ist eine Bibliothek (`src/lib.rs`), damit Tests und
Oberfläche dieselbe Logik benutzen; das Fenster ist nur die Hülle.

```mermaid
flowchart TD
    UI[ui/<br/>Vue 3 + Vite<br/>baut nach ui/dist]
    CMD[src/commands.rs<br/>Befehle der Oberfläche]
    STATE[src/state.rs<br/>AppState]
    WORKER[src/worker.rs<br/>Aufnahme-Thread]
    ENGINE[src/engine.rs<br/>Zustand, Rückmeldung]
    SAMPLER[src/sampler.rs<br/>Zonen, Glättung]
    CAP[capture/<br/>Bildschirm via Portal]
    OUT[output/<br/>WLED, OpenRGB, Govee,<br/>Nanoleaf, Chroma]
    CFG[src/config.rs<br/>config.json]
    TRAY[src/tray.rs]

    UI -->|invoke| CMD
    CMD --> STATE
    STATE --> WORKER
    WORKER --> ENGINE
    ENGINE --> SAMPLER
    SAMPLER --> CAP
    SAMPLER --> OUT
    WORKER -->|Events| UI
    TRAY --> STATE
    CFG --> STATE
    STATE --> CFG
```

## Grenzen

- **Die Oberfläche kennt keine Domäne.** Sie spricht nur Befehle (`invoke`) und
  empfängt Ereignisse; gerechnet wird in Rust. Ein Zustand, den das Frontend
  führt, kommt aus dem Modell — nicht aus einer eigenen Kopie.
- **Aufnahme und Ausgabe laufen in einem eigenen Thread.** Die Oberfläche darf
  nie auf das Portal oder ein Gerät warten; der Worker meldet Fortschritt als
  Ereignis.
- **Konfiguration ist eine Datei.** `config.json` ist die einzige Wahrheit für
  Geräte, Zonen und Einstellungen; sie wird beim Ändern geschrieben, nicht beim
  Beenden gesammelt.
- **Plattformunterschiede sind Daten.** Aufnahme (Portal, Windows Graphics
  Capture, CoreGraphics) und Tray liegen hinter `cfg`-Modulen, aber die
  Oberfläche sieht davon nichts.

## Abhängigkeiten

`ui/` → Tauri-Befehle → `state`/`engine`/`sampler`/`capture`/`output` →
`config`. Keine Rückrichtung außer über Ereignisse.
