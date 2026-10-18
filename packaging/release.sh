#!/bin/sh
# Publish built artifacts next to the source dir, Craft-family style:
#   <Product> v<ver> (<Target>)/
#     <slug>-<ver>/            this repo
#     <release files>          copied here from dist/
#     SHA256SUMS.txt           regenerated over them
set -eu

. "$(dirname "$0")/env.sh"

PARENT="$(cd "$ROOT/.." && pwd)"

find "$DIST" -maxdepth 1 -type f \
    \( -name "*.tar.gz" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \
       -o -name "*.flatpak" -o -name "*.zip" \) \
    -exec cp -f {} "$PARENT/" \;

cd "$PARENT"
rm -f SHA256SUMS.txt
find . -maxdepth 1 -type f \
    \( -name "*.tar.gz" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \
       -o -name "*.flatpak" -o -name "*.zip" -o -name "*.msi" \) \
    -exec sha256sum {} + | sed 's|  \./|  |' | sort -k2 > SHA256SUMS.txt
cat SHA256SUMS.txt
