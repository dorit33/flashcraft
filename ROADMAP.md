# Roadmap

**Current focus:** M1–M3 landed (2026-10-22): compressed images, auto-unmount, standalone drive
formatting (MBR/GPT + FAT32/NTFS/exFAT/UDF/ext2/3/4), persistent partitions, bad-block scan,
MD5/SHA-1/SHA-256 checksums, Craft-style engraved icon, and full UI control wiring.

| Milestone | Scope | Status |
|---|---|---|
| M0 skeleton | workspace, layers, command registry, Linux sysfs enumeration, raw write + verify, CLI, egui shell | done |
| M1 write UX | compressed images (.xz/.gz/.zst/.bz2/.zip decompressor pipes), automatic unmount + pkexec elevation | done |
| M2 filesystems | MBR/GPT partitioning, FAT32/NTFS/exFAT/UDF/ext2/3/4 formatting, persistent partition creation, extended label (`autorun.inf`) | done |
| M3 checks | bad-block scan (1–4 passes), checksum computation (MD5/SHA-1/SHA-256), write-then-verify option | done |
| M4 polish | Craft-style app icon, responsive Rufus UI layout, i18n (all 38 Rufus locales + RTL), full packaging matrix | done |
| M5 platform | FreeBSD backend, Windows WMI/SetupAPI raw I/O backend | research |

Parity vs Rufus is tracked in `docs/target-app-parity.md` and `docs/gaps.md`. Behaviour is designed from public
specs and observed behaviour only (clean-room — see `AGENTS.md`).
