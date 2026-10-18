#!/bin/sh
# Build the release binaries and a plain tree tarball:
#   dist/flashcraft-<ver>-linux-<arch>.tar.gz  (bin/ + share/ layout)
set -eu

cd "$(dirname "$0")/../.."
VER=$(cargo xtask version)
ARCH=$(uname -m)
PKG="flashcraft-${VER}-linux-${ARCH}"
OUT="dist/${PKG}"

cargo build --release -p flashcraft -p flashcraft-cli

rm -rf "$OUT" "${OUT}.tar.gz"
mkdir -p "$OUT/bin" "$OUT/share/applications" "$OUT/share/metainfo" \
    "$OUT/share/doc/flashcraft"
cp target/release/flashcraft target/release/flashcraft-cli "$OUT/bin/"
cp packaging/linux/flashcraft.desktop "$OUT/share/applications/"
cp packaging/linux/dev.flashcraft.app.metainfo.xml "$OUT/share/metainfo/"
cp LICENSE ATTRIBUTION.md "$OUT/share/doc/flashcraft/"
if [ -d assets/app-icon/hicolor ]; then
    mkdir -p "$OUT/share/icons"
    cp -r assets/app-icon/hicolor "$OUT/share/icons/"
fi
tar -C dist -czf "${OUT}.tar.gz" "$PKG"
echo "dist/${PKG}.tar.gz"
