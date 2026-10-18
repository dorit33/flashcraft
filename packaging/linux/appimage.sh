#!/bin/sh
# Build an AppImage. Downloads appimagetool on first run into packaging/.cache.
# Output: dist/flashcraft-<ver>-linux-<arch>.AppImage
set -eu

cd "$(dirname "$0")/../.."
VER=$(cargo xtask version)
ARCH=$(uname -m)

./packaging/linux/tarball.sh

case "$ARCH" in
    x86_64)  TOOL_ARCH=x86_64 ;;
    aarch64) TOOL_ARCH=aarch64 ;;
    *)       echo "unsupported arch: $ARCH" >&2; exit 1 ;;
esac

CACHE=packaging/.cache
TOOL="$CACHE/appimagetool-${TOOL_ARCH}.AppImage"
mkdir -p "$CACHE"
if [ ! -x "$TOOL" ]; then
    curl -fL -o "$TOOL" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${TOOL_ARCH}.AppImage"
    chmod +x "$TOOL"
fi

APPDIR="dist/AppDir"
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
    "$APPDIR/usr/share/icons/hicolor/128x128/apps" "$APPDIR/usr/share/metainfo"
cp "dist/flashcraft-${VER}-linux-${ARCH}/bin/"* "$APPDIR/usr/bin/"
cp packaging/linux/flashcraft.desktop "$APPDIR/flashcraft.desktop"
cp packaging/linux/dev.flashcraft.app.metainfo.xml "$APPDIR/usr/share/metainfo/"
mkdir -p "$APPDIR/usr/share/doc/flashcraft"
cp LICENSE ATTRIBUTION.md "$APPDIR/usr/share/doc/flashcraft/"
cp assets/app-icon/hicolor/128x128/apps/dev.flashcraft.app.png \
    "$APPDIR/usr/share/icons/hicolor/128x128/apps/"
cp "$APPDIR/usr/share/icons/hicolor/128x128/apps/dev.flashcraft.app.png" "$APPDIR/"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
exec "$(dirname "$0")/usr/bin/flashcraft" "$@"
EOF
chmod +x "$APPDIR/AppRun"

ARCH="$TOOL_ARCH" "$TOOL" "$APPDIR" "dist/flashcraft-${VER}-linux-${ARCH}.AppImage"
echo "dist/flashcraft-${VER}-linux-${ARCH}.AppImage"
