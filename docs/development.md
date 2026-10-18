# Development

## Build & test

```sh
cargo build --workspace          # everything
cargo test --workspace           # unit + fixture tests
cargo xtask ci                   # the gate: fmt, clippy -D warnings, tests, layers, wasm
```

First time only:

```sh
rustup target add wasm32-unknown-unknown
rustup component add rustfmt clippy
```

Linux windowing headers for the egui app:
`sudo dnf install libxkbcommon-devel wayland-devel libX11-devel libXrandr-devel libXi-devel mesa-libGL-devel`
(the cli and crates build without them).

## Run

```sh
cargo run -p flashcraft-cli -- list
cargo run -p flashcraft-cli -- inspect image.iso
pkexec ./target/debug/flashcraft-cli write image.iso /dev/sdX --yes
pkexec ./target/debug/flashcraft-cli verify image.iso /dev/sdX
cargo run -p flashcraft          # desktop UI
```

## Driving it from scripts / agents

Every action is a command id (`flashcraft-cli commands` lists them). The CLI subcommands map
1:1 onto the registry; a JSON control channel (`--control PORT`) is planned — the registry is
already the seam it will dispatch over.

## Adding a command

1. `CommandSpec` row in `crates/core/src/commands.rs` (+ params doc, `destructive` flag).
2. Params validation in `crates/core/src/plan.rs` (hostile JSON → `Err`, tests).
3. Backend fn in `crates/native` (or a new backend module).
4. Dispatch in the CLI/UI if it needs a new verb.
5. `cargo xtask ci`, then a `log/devlog.md` entry.
