#!/bin/sh
# Build the WebAssembly browser build and pack it:
#   dist/flashcraft-web-<ver>.zip   (index.html + wasm + js + icon)
# Needs: cargo, wasm-bindgen-cli, the wasm32-unknown-unknown target.
set -eu

. "$(dirname "$0")/../env.sh"
cd "$ROOT"

cargo build --release --target wasm32-unknown-unknown -p flashcraft-web

OUT="$DIST/web"
rm -rf "$OUT"
mkdir -p "$OUT"

wasm-bindgen \
    "target/wasm32-unknown-unknown/release/flashcraft-web.wasm" \
    --out-dir "$OUT" --out-name flashcraft-web \
    --target web --no-typescript

cp apps/flashcraft-web/index.html "$OUT/"
cp assets/app-icon/hicolor/64x64/apps/dev.flashcraft.app.png "$OUT/"

rm -f "$DIST/flashcraft-web-${VER}.zip"
zip -j -q "$DIST/flashcraft-web-${VER}.zip" "$OUT"/*
echo "$DIST/flashcraft-web-${VER}.zip"
