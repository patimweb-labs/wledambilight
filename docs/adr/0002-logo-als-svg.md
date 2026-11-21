# 0002 – Logo nach der Vorlage, Farben aus dem Zeichen

Stand: 2026-11-21 · entschieden · Ausnahme zu [AGENTS.md §8](../../AGENTS.md),
wie crosspoint ADR 0008

## Kontext

Wunsch des Nutzers: „das ist mein Logo und App-Icon bitte“ – die Vorlage
„WLED Ambilight Markenübersicht.png“ (1536 × 1024): ein Monitor in
Pixeloptik, umgeben von Lichtblöcken, die wie ein Ambilight im Farbkreis
leuchten (oben Cyan, links Cyan und Gelb, rechts Magenta und Koralle, unten
Gelb bis Orange), dazu verstreute Pixel und ein Glühen; als App-Icon auf
dunkler, abgerundeter Kachel, daneben helle Kachel, Mono-Kachel, Zeichen ohne
Kachel und eine kleine Fassung.

AGENTS.md §8 verlangt ein zweifarbiges ASCII-Raster (`assets/<name>.pm`) mit
Achteck-Kachel aus `apps/markenwerk`. Dieses Logo hat Verläufe, ein Glühen und
sechs Lichtfarben – das Raster beschreibt es nicht.

## Entscheidung

1. **Quelle sind SVG-Dateien** in `assets/`, nachgebaut auf einem
   512er-Raster (Maße an der Vorlage ausgemessen):

   | Datei | Was |
   |---|---|
   | `wled-ambilight-logo.svg` | App-Icon: dunkle Kachel, Zeichen, Glühen |
   | `wled-ambilight-zeichen.svg` | Zeichen ohne Kachel (Kopfzeile im Fenster) |
   | `wled-ambilight-hell.svg` | helle Kachel |
   | `wled-ambilight-mono.svg` | Mono: weiße Zeichnung auf dunkler Kachel |
   | `wled-ambilight-klein.svg` | ohne Streupixel, für 16–32 px |
   | `wled-ambilight-symbolic.svg` | einfarbig, Schreibtisch-Icon |

2. **Erzeugt wird mit `packaging/icons/erzeugen.sh`**: PNG 16–1024
   (`assets/icons/`), `app.ico` für Windows, rohes RGBA für Fenster und Tray,
   `assets/icon.svg`. Nichts davon wird von Hand abgelegt.
   `assets/wled-ambilight.pm` entfällt.
3. **Farben als Tokens:** Akzent ist das Cyan oben am Bildschirm
   (`#23F0FE`), Signal die Glut am Fuß (`#FD860E`), der Grund das Nachtblau
   der Kachel. `thema::tests::zeichen_und_tokens_tragen_dieselben_farben`
   hält Logo und Oberfläche zusammen.

## Kosten

- Zwei Quellen für Marken in der Familie: markenwerk für die anderen Apps,
  SVGs hier. Wer die Familie ändert, zieht hier von Hand nach.
- Nachgezeichnet, nicht kopiert: auf ein paar Pixel genau, bei 16 px bewusst
  vereinfacht.
- Die Wortmarke „WLED AMBILIGHT“ der Vorlage ist nicht als Pfad nachgebaut;
  im Fenster steht der Name in Inter SemiBold neben dem Zeichen.
