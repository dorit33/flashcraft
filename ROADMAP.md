# Roadmap

**Current focus:** M0 — working skeleton lands (done): workspace, core/native/ui-egui layers,
CLI `list`/`inspect`/`write`/`verify`, egui shell, `cargo xtask` gates.

| Milestone | Scope | Status |
|---|---|---|
| M0 skeleton | workspace, layers, command registry, Linux sysfs enumeration, raw write + verify, CLI, egui shell | done |
| M1 write UX | compressed images (.xz/.gz/.zip decompressor backends), unmount via udisks2, eject | next |
| M2 filesystems | FAT32 format, persistent partition creation, UEFI:NTFS support | planned |
| M3 checks | bad-block scan, checksum computation (MD5/SHA-1/SHA-256), write-then-verify in one pass | planned |
| M4 polish | themes/tokens, i18n (all 38 Rufus locales + RTL), app icon, packaging (.rpm/.deb/AppImage/Flatpak/tarball), `xtask version` | done |
| M5 platform | FreeBSD backend, Windows port research | research |

Parity vs Rufus is tracked in `docs/target-app-parity.md` and `docs/gaps.md`. Behaviour is designed from public
specs and observed behaviour only (clean-room — see `AGENTS.md`).
