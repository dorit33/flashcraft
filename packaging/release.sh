#!/bin/sh
# Publish built artifacts next to the source dir, Craft-family style:
#   <Product> v<ver> (<Target>)/
#     <slug>-<ver>/            this repo
#     <release files>          copied here from dist/
#     SHA256SUMS.txt           regenerated over them
set -eu

. "$(dirname "$0")/env.sh"

PARENT="$(cd "$ROOT/.." && pwd)"

# Remove any older-version artifacts from the parent directory before copying
find "$PARENT" -maxdepth 1 -type f \
    \( -name "flashcraft-*.tar.gz" -o -name "flashcraft-*.deb" -o -name "flashcraft-*.rpm" \
       -o -name "flashcraft-*.AppImage" -o -name "flashcraft-*.flatpak" \
       -o -name "flashcraft-*.zip" -o -name "flashcraft-*.msi" \) \
    -delete

find "$DIST" -maxdepth 1 -type f \
    \( -name "*${VER}*.tar.gz" -o -name "*${VER}*.deb" -o -name "*${VER}*.rpm" \
       -o -name "*${VER}*.AppImage" -o -name "*${VER}*.flatpak" \
       -o -name "*${VER}*.zip" -o -name "*${VER}*.msi" \) \
    -exec cp -f {} "$PARENT/" \;

cd "$PARENT"
rm -f SHA256SUMS.txt
find . -maxdepth 1 -type f \
    \( -name "*.tar.gz" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \
       -o -name "*.flatpak" -o -name "*.zip" -o -name "*.msi" \) \
    -exec sha256sum {} + | sed 's|  \./|  |' | sort -k2 > SHA256SUMS.txt
cat SHA256SUMS.txt

# Clean build caches so the released source directory stays lean like sibling Craft apps.
rm -rf "$ROOT/target" "$DIST" "$ROOT/.flatpak-builder" \
    "$ROOT/packaging/.flatpak-builder" "$ROOT/packaging/.cache" \
    "$ROOT/packaging/flatpak/build" "$ROOT/packaging/flatpak/repo"
