# AGENTS.md: guide for AI agents and contributors

FlashCraft writes bootable images to removable drives — an open-source, clean-room
reimplementation of Rufus, written in **Rust only** (no JavaScript or TypeScript). **No Tauri,
Electron or webview shells:** the desktop app is native egui/eframe, and the web build is the
same Rust compiled to WebAssembly (trunk + wasm-bindgen). The product name is always
**FlashCraft** in user-facing text: UI, window titles, About, installers, docs prose. Machine
names stay lowercase: crates (`flashcraft-*`), binaries (`flashcraft`, `flashcraft-cli`), file
names, app id `dev.flashcraft.app`. The goal is Rufus feature parity where it makes sense on
Linux (ISO/IMG writing, verification, bad-block checks, persistent partitions) — but the first
platform is Linux, where Rufus does not run at all.

Standards shared across the Craft apps live in `craftrules/` (bundled copy — self-contained
repo). This file records what is FlashCraft-specific; the shared rules are referenced as
`craftrules/<standard>`, and parity is tracked in `docs/target-app-parity.md` + `docs/gaps.md`
(`craftrules/standards/progress-docs.md`).

## 1. Orientation (5 minutes)

| Read | Why |
|---|---|
| `docs/architecture.md` | Crate map, dependency layers, the engine/backend seam |
| `docs/development.md` | Build, test, run, drive the app from the CLI |
| `ROADMAP.md` | Milestones, current focus, what's next |
| `crates/<name>/src/lib.rs` | Public API of that crate |

## 2. Workspace map

```text
crates/
  core      flashcraft-core       L0 pure, wasm-safe: image detection, drive model,
                                  write plans, progress, the command registry
  native    flashcraft-native     L1 OS backends: sysfs enumeration, image I/O (Linux first)
  ui-egui   flashcraft-ui-egui    L2 the shared egui UI (desktop and web)
apps/
  flashcraft                      L3 desktop shell (eframe): window only
  flashcraft-cli                  L3 headless CLI: list | inspect | write | verify | commands
xtask/                            cargo xtask ci | layers | wasm | version [set X.Y.Z]
docs/                             architecture, development
packaging/                        platform packaging scripts (planned)
assets/app-icon/                  the app's own icon (placeholder until drawn)
log/devlog.md                     terse dev log; the next agent starts there
```

**Layering is enforced** by `cargo xtask layers`. `core` depends on nothing in the workspace and
must build for `wasm32-unknown-unknown`. `native` may use `core`. `ui-egui` may use `core` (and
`native` only behind `cfg(not(target_arch = "wasm32"))` or through a trait). Nothing below the
apps opens windows. Nothing below L2 depends on egui/eframe/winit/rfd.

## 3. Golden rules

### Never crash (outranks feature work)

A malformed image, a removed drive mid-write, a hostile JSON param or a permission error must
produce an error the user can act on, never a panic. A half-written USB stick is a brick the
user asked for; a panic is a bug they didn't.

- **Non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`
  or `unimplemented!`. Return `flashcraft_core::Error` and propagate with `?`.
- Every production crate root carries
  `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.
  Tests may `unwrap` (`clippy.toml` allows it in `#[cfg(test)]`).
- **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`. Device access goes through
  `/sys`, `/proc` and regular file I/O — never ioctls or FFI.
- **Everything from the user or the OS is hostile.** `get()` instead of indexing, checked
  arithmetic for sizes and offsets, cap sizes read from input, guard division by zero.

### Never write to the wrong device (outranks everything else)

`media.write` destroys every byte on the target. The protections are structural, not a warning
dialog:

- Only `WritePlan::new` produces a writable plan, and it refuses non-removable devices,
  mounted devices, zero-size devices and oversized images. The UI, the CLI and any agent all
  get the same refusal.
- The CLI additionally requires `--yes` and root; the UI requires an explicit checkbox.
- Tests cover every refusal path. New refusals come with tests; removed refusals need a very
  good reason stated in the commit message.

### Clean-room

We studied Rufus's *behaviour and feature list* only. Never copy its code, icons, fonts or
resources. Implement from public docs (sysfs, ISO 9660, UEFI boot specs) and observation.
One deliberate exception: the UI translations are imported from `res/loc/rufus.loc` by
`tools/loc-import.py`. That file is GPLv3 — exactly why this project is licensed
GPL-3.0-only, so the reuse is legal. Every other Rufus resource stays off-limits.
Third-party assets must get a row in `ATTRIBUTION.md`.

### Other rules

1. **Everything is a command.** User-visible behaviour = a `CommandSpec` in
   `crates/core/src/commands.rs` (id, label, summary, params doc, `destructive` flag) plus a
   backend implementation. The UI, the CLI and a future control channel dispatch by id.
2. **The UI is thin and data-driven.** `FlashcraftUi` holds only view state; it reads the
   registry and calls backends. Colours and spacing come from egui defaults or theme tokens —
   never hard-code magic colours outside the theme.
3. **Tests are the gate.** Format sniffing, plan validation and params parsing get unit tests
   with adversarial inputs; native enumeration tests run against fixture directories.
4. **Never break wasm.** `cargo xtask wasm` must pass. File-system and OS code is
   `cfg(not(target_arch = "wasm32"))` or behind `flashcraft-native`.
5. **Performance is a feature.** Writes stream in 4 MiB chunks with live progress; never load a
   whole image into memory, never scan the device twice when once suffices.

## 4. Before you finish a task

```sh
cargo xtask ci        # fmt --check, clippy -D warnings, tests, layers, wasm check
```

Or piecemeal: `cargo test -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`,
`cargo xtask layers`, `cargo xtask wasm`.

Then append a terse entry to `log/devlog.md` (what landed, what's still open). Sessions can end
abruptly; the dev log plus a green tree is how the next agent picks up. Keep the tree building
at every step.

## 5. Releasing

The version lives only in `[workspace.package] version` in the root `Cargo.toml`
(`cargo xtask version`, `cargo xtask version set X.Y.Z[-pre]`). `set` validates semver, writes the
manifest atomically, refreshes `Cargo.lock`, and stamps the two places that can't read Cargo
metadata at build time: the RPM spec's `Version:` header and the AppStream `<release>` entry.
The version label in the UI compiles in via `env!("CARGO_PKG_VERSION")` — never hard-code it.
Every installer carries `LICENSE` + `ATTRIBUTION.md` under `share/doc/flashcraft/`
(`%license`/`%doc` for RPM, a `copyright` file for DEB).

```sh
cargo xtask version set 0.2.0
./packaging/linux/tarball.sh   # then deb.sh | rpm.sh | appimage.sh
./packaging/web/web.sh         # wasm build → dist/flashcraft-web-<ver>.zip (needs wasm-bindgen-cli)
./packaging/windows/windows.sh # x64 portable zip + x64/x86 msi
                               # (needs llvm-mingw + wixl; user-space installs
                               #  live in ~/.local/toolchains/{llvm-mingw,wixl})
./packaging/source.sh          # source zip → dist/flashcraft-<ver>.zip
./packaging/release.sh         # copy dist/* next to the source dir + SHA256SUMS.txt
# flatpak: flatpak run org.flatpak.Builder packaging/flatpak/build \
#   packaging/flatpak/dev.flashcraft.app.yml --force-clean --user \
#   --install-deps-from=flathub --repo=packaging/flatpak/repo
# flatpak build-bundle packaging/flatpak/repo dist/flashcraft-<ver>-x86_64.flatpak dev.flashcraft.app
```

The Windows binaries cross-compile and run (`*-pc-windows-gnullvm` via llvm-mingw), but
`flashcraft-native` enumerates and writes drives through Linux sysfs/polkit only — on
Windows the app opens with an empty device list. A real Windows backend (WMI/SetupAPI +
raw disk I/O) is roadmap work; the installers exist so the matrix and the UI can be
exercised on Windows meanwhile.

## 6. Parallel agents

- Use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`) and edit only the files you
  own.
- Keep every `Cargo.toml` valid at all times: one broken manifest breaks everyone's build.
  Write manifests to a temp file and `mv` them into place.
- If someone else's in-progress edit breaks the build, wait and retry; don't fix their files.

## 7. Running it

```sh
cargo run -p flashcraft-cli -- list                       # enumerate removable drives
sudo ./target/debug/flashcraft-cli write fedora.iso /dev/sdb --yes
cargo run -p flashcraft                                  # the desktop app
```

Block-device writes need root. The supported way to grant it is a polkit prompt
(`pkexec ./target/debug/flashcraft-cli …`) or plain `sudo` — never a setuid binary.
