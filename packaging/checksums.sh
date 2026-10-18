#!/bin/sh
# Write dist/SHA256SUMS.txt covering every release artifact in dist/.
set -eu

. "$(dirname "$0")/env.sh"
cd "$DIST"

rm -f SHA256SUMS.txt
find . -maxdepth 1 -type f \
    \( -name "*.tar.gz" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \
       -o -name "*.flatpak" -o -name "*.zip" \) \
    -exec sha256sum {} + | sed 's|  \./|  |' | sort -k2 > SHA256SUMS.txt
cat SHA256SUMS.txt
