#!/bin/sh
# Pack the committed source tree:
#   dist/flashcraft-<ver>.zip
set -eu

. "$(dirname "$0")/env.sh"
cd "$ROOT"

OUT="$DIST/flashcraft-${VER}.zip"
rm -f "$OUT"
git archive --format=zip --prefix="flashcraft-${VER}/" -o "$OUT" HEAD
echo "$OUT"
