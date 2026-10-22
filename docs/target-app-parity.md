# FlashCraft ↔ Rufus parity assessment

**Status:** updated assessment, 2026-10-22 — update the rows this work moves, with the date
(`craftrules/standards/progress-docs.md`).
**Reference:** Rufus 4.x on Windows (IDD_DIALOG in `rufus.rc`, `res/loc/rufus.loc`).

## Dimensions

| Dimension | Weight | State | Evidence |
| --- | --- | --- | --- |
| Dialog layout & look | high | **parity** | Craft badge + Rufus section headers, active controls, overlaid progress status, elapsed timer, Cancel/Verify/Checksums/About |
| Localization | high | **parity** | All 38 Rufus languages from `rufus.loc` (GPLv3), auto-detect, CJK/RTL |
| Device enumeration & safety | high | **parity+** | sysfs; `WritePlan` / `FormatPlan` refuse internal/mounted/oversized — structural; automatic unmount + pkexec elevation |
| Raw & ISO image writing | high | **working** | 4 MiB stream + ISO PVD volume label patching + old-BIOS MBR fix + read-back verify |
| Format backends (FAT32/NTFS/exFAT/UDF/ext2/3/4) | high | **working** | `Non bootable` mode partitions (MBR/GPT via `sfdisk`) and formats with cluster size, label, quick/full zero-fill, and `autorun.inf` extended label |
| Compressed images (.xz/.gz/.zst/.bz2/.zip) | medium | **working** | Streaming decompression pipes on write and verify |
| Bad-block scan | medium | **working** | 1–4 passes with live progress before write/format or via `flashcraft-cli badblocks` |
| Checksums (`#`) | medium | **working** | Pure-Rust streaming MD5, SHA-1, SHA-256 in `flashcraft-core` + `#` dialog & `flashcraft-cli checksum` |
| Persistent partitions | medium | **working** | Slider in UI + `sfdisk --append` + `mkfs.ext4 -L writable` after ISO write |
| Windows ISO file-extraction / FreeDOS | low | **missing** | Roadmap |
| Platforms | — | Linux full; Windows/Web shell | FreeBSD/Windows native backends: research (M5) |

## Summary

UI, localization, image writing, on-the-fly decompression, drive partitioning/formatting
(MBR/GPT + FAT32/NTFS/exFAT/UDF/ext2/3/4), bad-block scanning, persistent partitions, and
MD5/SHA-1/SHA-256 checksums are all wired and tested on Linux.
