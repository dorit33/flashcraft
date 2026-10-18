#!/bin/sh
# Build the tarball, then an .rpm from it.
# Output: ~/rpmbuild/RPMS/<arch>/flashcraft-<ver>-1.<dist>.<arch>.rpm
set -eu

cd "$(dirname "$0")/../.."
VER=$(cargo xtask version)
ARCH=$(uname -m)

./packaging/linux/tarball.sh

TARBALL="dist/flashcraft-${VER}-linux-${ARCH}.tar.gz"
mkdir -p "$HOME/rpmbuild/SOURCES"
cp "$TARBALL" "$HOME/rpmbuild/SOURCES/"

rpmbuild -bb packaging/linux/rpm/flashcraft.spec \
    --define "_arch $ARCH"
find "$HOME/rpmbuild/RPMS" -name "flashcraft-${VER}-*.rpm" -newer "$TARBALL" \
    -exec cp {} dist/ \;
ls "dist/flashcraft-${VER}"-*.rpm
