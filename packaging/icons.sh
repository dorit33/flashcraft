#!/bin/sh
# Regenerate every icon render from the master SVG (needs rsvg-convert).
# Run from anywhere; outputs land in assets/app-icon/hicolor/.
set -eu

cd "$(dirname "$0")/.."
SRC=assets/app-icon/flashcraft.svg

for size in 16 24 32 48 64 128 256 512; do
    out="assets/app-icon/hicolor/${size}x${size}/apps"
    mkdir -p "$out"
    rsvg-convert -w "$size" -h "$size" "$SRC" -o "$out/dev.flashcraft.app.png"
done
echo "icons rendered"
