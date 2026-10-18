#!/bin/sh
# Windows artifacts:
#   dist/flashcraft-<ver>-windows-x64-portable.zip
#   dist/flashcraft-<ver>-windows-x64.msi
#   dist/flashcraft-<ver>-windows-x86.msi
#
# Cross-compile needs an llvm-mingw toolchain (x86_64/i686-w64-mingw32-clang)
# on PATH or in ~/.local/toolchains/llvm-mingw/bin, the
# x86_64/i686-pc-windows-gnullvm rust targets, and wixl for the MSIs.
# wixl is searched on PATH and in ~/.local/toolchains/wixl.
set -eu

. "$(dirname "$0")/../env.sh"
cd "$ROOT"

MINGW="${LLVM_MINGW:-$HOME/.local/toolchains/llvm-mingw/bin}"
[ -d "$MINGW" ] && PATH="$MINGW:$PATH"
export PATH

WIXL_DIR="$HOME/.local/toolchains/wixl"
wixl() {
    case "$(command -v wixl)" in
        /*) command wixl "$@" ;;
        *)  LD_LIBRARY_PATH="$WIXL_DIR/usr/lib64" "$WIXL_DIR/usr/bin/wixl" "$@" ;;
    esac
}

build() { # $1 clang prefix (x86_64|i686)  $2 rust target
    clang="$1-w64-mingw32-clang"
    command -v "$clang" >/dev/null 2>&1 || {
        echo "missing $clang (install llvm-mingw to $MINGW)" >&2; exit 1; }
    case "$2" in
        x86_64*) export CARGO_TARGET_X86_64_PC_WINDOWS_GNULLVM_LINKER="$clang" ;;
        i686*)   export CARGO_TARGET_I686_PC_WINDOWS_GNULLVM_LINKER="$clang" ;;
    esac
    cargo build --release --target "$2" -p flashcraft -p flashcraft-cli
}

stage() { # $1 rust_target → staging dir path echoed
    dir="$DIST/windows-$1"
    rm -rf "$dir"; mkdir -p "$dir"
    cp "target/$1/release/flashcraft.exe" "target/$1/release/flashcraft-cli.exe" \
        LICENSE ATTRIBUTION.md "$dir/"
    echo "$dir"
}

# --- portable zip (x64) ---
build x86_64 x86_64-pc-windows-gnullvm
dir="$(stage x86_64-pc-windows-gnullvm)"
rm -f "$DIST/flashcraft-${VER}-windows-x64-portable.zip"
( cd "$dir" && zip -q -r "$DIST/flashcraft-${VER}-windows-x64-portable.zip" . )
echo "$DIST/flashcraft-${VER}-windows-x64-portable.zip"

# --- MSIs (x64 + x86) ---
msi() { # $1 wixl arch  $2 clang prefix  $3 rust target  $4 ProgramFiles dir  $5 out file
    build "$2" "$3"
    dir="$(stage "$3")"
    sed -e "s/@VER@/$VER/g" -e "s/@PF_DIR@/$4/g" \
        "$ROOT/packaging/windows/flashcraft.wxs" > "$dir/flashcraft.wxs"
    ( cd "$dir" && wixl -a "$1" -o "$DIST/$5" flashcraft.wxs )
    echo "$DIST/$5"
}

msi x64 x86_64 x86_64-pc-windows-gnullvm ProgramFiles64Folder "flashcraft-${VER}-windows-x64.msi"
msi x86 i686   i686-pc-windows-gnullvm   ProgramFilesFolder   "flashcraft-${VER}-windows-x86.msi"
