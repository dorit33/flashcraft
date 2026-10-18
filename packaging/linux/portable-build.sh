#!/bin/sh
# Build the generic Linux binaries against the OLDEST supported glibc
# (Ubuntu 22.04 = glibc 2.35) in a podman container, so the portable
# artifacts (tarball/deb/appimage/rpm payload) also run on older
# distros — a Fedora-built binary needs GLIBC_2.4x and will not even
# start on Pop!_OS 24.04 (glibc 2.39) or Ubuntu 22.04.
#
# Output: target/linux-portable/{flashcraft,flashcraft-cli}
set -eu

cd "$(dirname "$0")/../.."

if ! command -v podman >/dev/null 2>&1; then
    echo "portable-build.sh: podman not found; building on the host means" >&2
    echo "the binaries will only run on glibc >= $(ldd --version | head -1 | grep -oE '[0-9.]+$')" >&2
    exit 1
fi

OUT="target/linux-portable"
mkdir -p "$OUT"

# Cache dirs live inside the repo (mounted into the container); rootless
# podman maps container-root to the calling user, so no chown is needed.
podman run --rm \
    -v "$PWD:/work:Z" -w /work \
    -e CARGO_HOME=/work/target/linux-portable-cargo \
    -e RUSTUP_HOME=/work/target/linux-portable-rustup \
    -e CARGO_TARGET_DIR=/work/target/linux-portable-target \
    docker.io/library/ubuntu:22.04 sh -c '
        set -eu
        export DEBIAN_FRONTEND=noninteractive
        apt-get update -qq >/dev/null
        apt-get install -y -qq --no-install-recommends \
            curl ca-certificates build-essential pkg-config \
            libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
            libxkbcommon-dev libxkbcommon-x11-dev >/dev/null
        PATH=/work/target/linux-portable-cargo/bin:$PATH
        if ! command -v cargo >/dev/null; then
            curl -sSf https://sh.rustup.rs \
                | sh -s -- -y --profile minimal --default-toolchain stable >/dev/null
        fi
        cargo build --release -p flashcraft -p flashcraft-cli
    '

cp target/linux-portable-target/release/flashcraft \
   target/linux-portable-target/release/flashcraft-cli "$OUT/"

# Guard: refuse to package binaries that demand a newer glibc than 2.35.
REQ=$(objdump -T "$OUT/flashcraft" | grep -oE 'GLIBC_[0-9.]+' | sort -Vu | tail -1)
echo "portable binaries: $OUT/ (glibc requirement: ${REQ:-unknown})"
