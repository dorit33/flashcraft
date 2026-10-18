# shellcheck shell=bash
# Shared setup for the packaging scripts. Source it: `. "$(dirname "$0")/env.sh"`.
#
# Exports:
#   ROOT     workspace root
#   VER      [workspace.package] version from Cargo.toml (override: FLASHCRAFT_VERSION)
#   DIST     output directory for release artifacts ($ROOT/dist)
#   ARCH     host architecture (uname -m)

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")/.." && pwd)"
export ROOT

VER="${FLASHCRAFT_VERSION:-$(awk '
    /^\[/ { in_pkg = ($0 == "[workspace.package]"); next }
    in_pkg && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }
' "$ROOT/Cargo.toml")}"
export VER

DIST="$ROOT/dist"
ARCH="$(uname -m)"
export DIST ARCH
