# FlashCraft ↔ Rufus parity assessment

**Status:** draft assessment, 2026-10-18 — update the rows this work moves, with the date
(`craftrules/standards/progress-docs.md`).
**Reference:** Rufus 4.x on Windows (IDD_DIALOG in `rufus.rc`, `res/loc/rufus.loc`).

## Dimensions

| Dimension | Weight | State | Evidence |
| --- | --- | --- | --- |
| Dialog layout | high | **parity-close** | Every IDD_DIALOG control on its .rc coordinates; RTL mirror; dark theme |
| Localization | high | **parity** | All 38 Rufus languages from `rufus.loc` (GPLv3), auto-detect, CJK/RTL |
| Device enumeration & safety | high | **parity+** | sysfs; `WritePlan` refuses internal/mounted/oversized — structural |
| Raw image writing (DD mode) | high | **working** | 4 MiB stream + read-back verify; CLI + UI + progress |
| ISO mode (extract to FAT32 + bootloader) | high | **missing** | Writes are byte-for-byte only; Windows install ISOs won't boot |
| Format backends (FAT32/exFAT/NTFS/ext) | high | **missing** | Controls shown disabled; no formatter yet |
| Compressed images (.xz/.gz/.zip) | medium | **missing** | Detected but not decompressed on the fly |
| Bad-block scan | medium | **missing** | Checkbox disabled |
| Checksums (#) | low | **missing** | MD5/SHA-1/SHA-256 of written data |
| Persistent partitions | medium | **hidden** | Controls hidden like Rufus on unsupported images; not implemented |
| Windows download assistant / FreeDOS / UEFI:NTFS | low | **missing** | Roadmap |
| Platforms | — | Linux only | FreeBSD/Windows: research (M5) |

## Summary

UI + language parity is close; functionality parity stands at *raw write + verify*. The
decisive gap is **ISO mode + format backends** — without them a Windows ISO produces a raw
dump, not a bootable installer.
