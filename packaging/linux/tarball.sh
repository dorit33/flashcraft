#!/bin/sh
# Build the release binaries and a plain tree tarball:
#   dist/flashcraft-<ver>-linux-<arch>.tar.gz  (bin/ + share/ layout)
set -eu

cd "$(dirname "$0")/../.."
VER=$(cargo xtask version)
ARCH=$(uname -m)
PKG="flashcraft-${VER}-linux-${ARCH}"
OUT="dist/${PKG}"

# Portable payload: prefer the container build against old glibc so the
# artifact runs on older distros too; fall back to a host build when
# podman is unavailable (host-glibc-pinned, dev machines only).
BINDIR="target/release"
if [ "${FLASHCRAFT_HOST_BUILD:-0}" = "1" ] || \
   ! ./packaging/linux/portable-build.sh; then
    echo "tarball.sh: falling back to host build (host glibc required)" >&2
    cargo build --release -p flashcraft -p flashcraft-cli
else
    BINDIR="target/linux-portable"
fi

rm -rf "$OUT" "${OUT}.tar.gz"
mkdir -p "$OUT/bin" "$OUT/share/applications" "$OUT/share/metainfo" \
    "$OUT/share/doc/flashcraft"
cp "$BINDIR/flashcraft" "$BINDIR/flashcraft-cli" "$OUT/bin/"
cp packaging/linux/flashcraft.desktop "$OUT/share/applications/dev.flashcraft.app.desktop"
cp packaging/linux/dev.flashcraft.app.metainfo.xml "$OUT/share/metainfo/"
cp LICENSE ATTRIBUTION.md "$OUT/share/doc/flashcraft/"
if [ -d assets/app-icon/hicolor ]; then
    mkdir -p "$OUT/share/icons"
    cp -r assets/app-icon/hicolor "$OUT/share/icons/"
fi
tar -C dist -czf "${OUT}.tar.gz" "$PKG"
echo "dist/${PKG}.tar.gz"
