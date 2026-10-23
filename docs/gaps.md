# Gaps — the ranked work list

Pick work from the top. When an item lands, update it here and in
`docs/target-app-parity.md` (same commit, with the date).

**Status:** 2026-10-22.

| # | Gap | Why it matters | Where it lands |
| --- | --- | --- | --- |
| 1 | Windows ISO extraction mode (UDF/ISO extract to NTFS/FAT32 + UEFI:NTFS bootloader) | Hybrid Linux ISOs and raw images work; non-hybrid Windows ISOs need file extraction | `crates/native` extractor |
| 2 | Control channel (`--control` JSON-lines + `ui.screenshot`) | Agent-drivable UI per `craftrules/standards/agent-control.md`. MCP side landed 2026-10-23: `flashcraft-cli mcp` serves the registry over stdio JSON-RPC | `crates/ui-egui` |
| 3 | Windows download assistant & FreeDOS bootable disk | Rufus extras | roadmap M5 |
| 4 | FreeBSD / Windows native drive backends | Platform parity beyond Linux sysfs | M5 research |
