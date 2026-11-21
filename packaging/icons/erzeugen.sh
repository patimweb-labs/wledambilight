#!/usr/bin/env bash
# Erzeugt aus den SVG-Quellen in assets/ alle Icons, die gebaut und installiert
# werden (ADR 0002). Nicht von Hand ablegen: dieses Skript ist die einzige
# Brücke von der Zeichnung zu PNG, ICO und den rohen RGBA-Bildern.
#
# Aufruf aus dem Wurzelverzeichnis: packaging/icons/erzeugen.sh
# Braucht: resvg, rsvg-convert oder ImageMagick (mit librsvg) – was zuerst da ist.
# Für ICO und RGBA zusätzlich ImageMagick.
#
# 16, 24 und 32 px kommen aus `wled-ambilight-klein.svg` (ohne Streupixel, die
# bei der Größe nur Rauschen wären), alles darüber aus dem vollen Zeichen.
set -euo pipefail
cd "$(dirname "$0")/../.."

QUELLE=assets
ZIEL=assets/icons
mkdir -p "$ZIEL"

if command -v resvg >/dev/null; then
  zeichne() { resvg --background '#00000000' --width "$2" --height "$2" "$1" "$3"; }
elif command -v rsvg-convert >/dev/null; then
  zeichne() { rsvg-convert --background-color='#00000000' --width "$2" --height "$2" -o "$3" "$1"; }
elif command -v magick >/dev/null; then
  zeichne() { magick -background none "$1" -resize "${2}x${2}" "PNG32:$3"; }
else
  echo "Kein SVG-Zeichner gefunden: resvg, rsvg-convert oder ImageMagick installieren." >&2
  exit 1
fi
command -v magick >/dev/null || { echo "ImageMagick (magick) fehlt für ICO und RGBA." >&2; exit 1; }

# --- PNGs -----------------------------------------------------------------
for n in 16 24 32; do
  zeichne "$QUELLE/wled-ambilight-klein.svg" "$n" "$ZIEL/icon-$n.png"
done
for n in 48 64 128 256 512 1024; do
  zeichne "$QUELLE/wled-ambilight-logo.svg" "$n" "$ZIEL/icon-$n.png"
done

# --- Rohbilder für Fenster und Tray (RGBA, 8 Bit je Kanal, ohne Kopf) -------
magick "$ZIEL/icon-32.png" -depth 8 "RGBA:$ZIEL/tray-32.rgba"
magick "$ZIEL/icon-64.png" -depth 8 "RGBA:$ZIEL/tray-64.rgba"
cp "$ZIEL/tray-64.rgba" "$ZIEL/app-64.rgba"

# --- Windows: ein ICO mit allen kleinen Größen -----------------------------
magick "$ZIEL/icon-16.png" "$ZIEL/icon-24.png" "$ZIEL/icon-32.png" \
  "$ZIEL/icon-48.png" "$ZIEL/icon-64.png" "$ZIEL/icon-256.png" "$ZIEL/app.ico"

# --- SVG: skalierbar und symbolisch ----------------------------------------
cp "$QUELLE/wled-ambilight-logo.svg" assets/icon.svg

echo "Icons erzeugt:"
find "$ZIEL" -type f -printf '  %p %s Byte\n' | sort
