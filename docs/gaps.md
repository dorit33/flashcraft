# Gaps — the ranked work list

Pick work from the top. When an item lands, update it here and in
`docs/target-app-parity.md` (same commit, with the date).

**Status:** 2026-10-18.

| # | Gap | Why it matters | Where it lands |
| --- | --- | --- | --- |
| 1 | ISO mode (extract to FAT32 + bootloader install) | The signature Rufus feature; without it Windows ISOs don't boot | `crates/native` formatter + `crates/core` plan |
| 2 | Filesystem formatting (FAT32 first, then exFAT/NTFS/ext) | Unlocks the Format Options controls; prerequisite for ISO mode | `crates/native` |
| 3 | Compressed image decompression on write | Rufus writes .xz/.gz/.zip directly | `crates/native` stream reader |
| 4 | Checksums (MD5/SHA-1/SHA-256) | Verify media beyond read-back | `crates/native` |
| 5 | Unmount/eject via udisks2 | Safe writes without manual unmount; polkit path | `crates/native` |
| 6 | Bad-block scan | Reliability check for flaky sticks | `crates/native` |
| 7 | Persistent partitions (Ubuntu/Fedora live images) | Second marquee feature; controls already hidden correctly | `crates/core` plan + `native` |
| 8 | Control channel (`--control` JSON-lines + `ui.screenshot`) | Agent-drivable UI per `craftrules/standards/agent-control.md` | `crates/ui-egui` |
| 9 | Windows download assistant, FreeDOS, UEFI:NTFS | Rufus extras | roadmap M3+ |
| 10 | FreeBSD/Windows backends | Platform parity | M5 research |
