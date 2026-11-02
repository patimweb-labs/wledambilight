# Datenfluss

Was passiert, wenn der Anwender etwas tut — vom Klick bis zum Licht.

## „Zone verschieben"

1. **UI** (`ui/src/…`): Zeiger zieht einen Zonenrahmen; das Frontend rechnet nur
   Pixel in normalisierte Koordinaten (0..1) um und ruft den Tauri-Befehl auf.
2. **Befehl** (`src/commands.rs`): prüft die Grenzen, schreibt über
   `src/state.rs` in `config.json` (Zonen sind Teil der Konfiguration, nicht des
   Laufzeitzustands).
3. **Ereignis**: Der Befehl meldet den neuen Stand zurück; die Oberfläche
   zeichnet aus dem Modell — sie führt den Rahmen nicht selbst.
4. **Worker** (`src/worker.rs`): liest die Zonen beim nächsten Bild. Die
   Sampler-Schleife (`src/sampler.rs`) mittelt je Zone die Pixel aus dem
   Aufnahmepuffer.
5. **Ausgabe** (`src/output/`): Die Farben gehen als DDP/DRGB an WLED oder über
   das jeweilige Protokoll an OpenRGB, Govee, Nanoleaf, Chroma.
6. **Anzeige**: Der Worker schickt Bildrate und Zonenfarben als Ereignis an die
   Oberfläche; Vorschau und Gerätestatus zeigen damit das, was wirklich rausgeht.

## „Aufnahme starten"

1. UI ruft `start_capture`.
2. `capture/` fordert über `xdg-desktop-portal` einen ScreenCast an (PipeWire) —
   beim ersten Mal mit Dialog, danach über den gespeicherten `restore_token`.
3. Der Portal-Stream landet in einer GStreamer-Pipeline, die Frames in den
   Aufnahmepuffer schreibt.
4. `sampler` liest daraus, `output` sendet; ohne Stream bleibt die Anzeige still
   statt erfundene Werte zu zeigen.

## Grundsatz

Jede Änderung hat **einen** Weg: Oberfläche → Befehl → Modell → Worker →
Gerät. Der Rückweg ist immer ein Ereignis aus dem Modell, nie eine zweite
Wahrheit im Frontend.
