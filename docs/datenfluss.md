# Datenfluss

Was passiert, wenn der Anwender etwas tut – vom Klick bis zum Licht.

## „Zone verschieben“

1. **Ansicht** (`crates/fenster/src/ansichten/buehne.rs`): Der Zeiger fasst
   eine Zone an. Die Bühne meldet `Aktion::ZugBeginnen` mit der Zone, wie sie
   beim Anfassen war.
2. **Ziehen:** Jedes Bild rechnet `logik::zone_ziehen` aus dem Weg seit dem
   Anfassen die neue Zone (normalisiert 0..1, auf 0,5 % gerastet, immer im
   Bild) und meldet `Aktion::Zone(i, neu)`.
3. **Logik** (`logik::ausfuehren`): ersetzt die Zone im Modell, bereinigt die
   Konfiguration (`Config::sanitize`), gibt sie mit `Shared::set_config` an die
   Pipeline und merkt das Speichern vor (300 ms nach der letzten Änderung).
4. **Pipeline** (`crates/core/src/worker.rs`): sieht die neue Fassung am
   Versionszähler und liest die Zonen beim nächsten Bild. Der Sampler mittelt
   je Zone die Pixel aus dem Aufnahmepuffer.
5. **Ausgabe** (`crates/core/src/output/`): Die Farben gehen als DDP/DRGB an
   WLED oder über das jeweilige Protokoll an OpenRGB, Govee, Nanoleaf, Chroma.
6. **Anzeige:** Im nächsten Bild liest das Fenster `telemetry::read` – Messwerte,
   gesendete LED-Farben und die Farbe je Zone (`telemetry::zone_colors`,
   Rückwärts-Verdrahtung und BGR zurückgerechnet). Die Zone auf der Bühne ist
   mit dieser Farbe gefüllt: man sieht, was wirklich hinausgeht.

## „Netzwerk durchsuchen“

1. Knopf → `Aktion::Suchen`; das Modell steht auf `Suche::Laeuft`, die
   Gerätespalte zeigt den Drehkreisel.
2. `logik` startet `Netz::suchen` in einem Thread (mDNS, Subnetz, Govee,
   OpenRGB, Synapse; höchstens 12 s).
3. Die Antwort kommt über einen Kanal zurück und weckt das Fenster.
   `logik::antwort` nimmt bereits eingerichtete Geräte heraus
   (`telemetry::without_known`) und setzt `Suche::Fertig(liste)`.
4. „hinzufügen“ → `Aktion::Hinzufuegen` → Gerät in die Konfiguration, weiter
   wie oben ab Schritt 3.

## „Aufnahme starten“

1. Die Pipeline startet die Aufnahme selbst (oder auf `Aktion::AufnahmeNeu`).
2. `capture/` fordert über `xdg-desktop-portal` einen ScreenCast an (PipeWire) –
   beim ersten Mal mit Dialog, danach über den gespeicherten `restore_token`.
   Windows nutzt Windows Graphics Capture, macOS ScreenCaptureKit.
3. Der Strom landet in einer GStreamer-Pipeline, die Bilder in den
   Aufnahmepuffer schreibt; das Fenster lädt das neueste als Textur
   (`vorschau::bild`, höchstens 640 px breit).
4. Ohne Strom bleibt die Anzeige still, statt erfundene Werte zu zeigen.

## Grundsatz

Jede Änderung hat **einen** Weg: Ansicht → Aktion → Logik → Modell und
Pipeline → Gerät. Der Rückweg ist immer das Lesen des gemeinsamen Zustands,
nie eine zweite Wahrheit im Fenster.
