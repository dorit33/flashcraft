#!/bin/sh
# Build the tarball, then a .deb from the same payload (dpkg-deb).
# Output: dist/flashcraft-<ver>-linux-<arch>.deb
set -eu

cd "$(dirname "$0")/../.."
VER=$(cargo xtask version)
ARCH=$(uname -m)

./packaging/linux/tarball.sh

case "$ARCH" in
    x86_64)  DEBARCH=amd64 ;;
    aarch64) DEBARCH=arm64 ;;
    *)       DEBARCH="$ARCH" ;;
esac

STAGE="dist/deb-root"
rm -rf "$STAGE"
mkdir -p "$STAGE/usr" "$STAGE/DEBIAN"
tar -xzf "dist/flashcraft-${VER}-linux-${ARCH}.tar.gz" -C "$STAGE/usr" --strip-components 1

INSTALLED=$(du -sk "$STAGE/usr" | cut -f1)
cat > "$STAGE/DEBIAN/control" <<EOF
Package: flashcraft
Version: ${VER}
Section: utils
Priority: optional
Architecture: ${DEBARCH}
Installed-Size: ${INSTALLED}
Maintainer: FlashCraft contributors <noreply@localhost>
Depends: libgtk-3-0, libxcb1, libxkbcommon0, libgbm1, libfontconfig1
Description: Write bootable images to removable drives
 FlashCraft writes bootable images (ISO 9660, disk images) to removable
 drives, byte-for-byte, with read-back verification. A clean-room,
 pure-Rust reimplementation of Rufus: native egui, no Electron.
EOF

dpkg-deb --build "$STAGE" "dist/flashcraft-${VER}-linux-${ARCH}.deb"
echo "dist/flashcraft-${VER}-linux-${ARCH}.deb"
