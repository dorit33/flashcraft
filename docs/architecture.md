# Architecture

Three crates below two apps, same shape as the Craft App Suite:

```text
apps/flashcraft      (eframe shell) ─┐
apps/flashcraft-cli  (headless)    ─┤ dispatch command ids
        │                           │
crates/ui-egui  L2  thin UI, view state only
crates/native   L1  device I/O: sysfs enumeration, write, verify   (no wasm)
crates/core     L0  pure data + command registry                   (wasm-safe)
```

## The command seam

`crates/core/src/commands.rs` is the single registry. Every user-visible behaviour is a
`CommandSpec { id, label, summary, params_doc, destructive }`:

| id | backend | destructive |
|---|---|---|
| `drives.list` | `native::linux::list_drives_live` | no |
| `image.inspect` | `native::inspect_image` | no |
| `media.write` | `native::write_image` | yes |
| `media.verify` | `native::verify_image` | no |

Frontends never open device nodes themselves; they build a `WritePlan` (which validates) and
call the backend. New behaviour = new spec + backend fn + tests.

## Safety model

`WritePlan::new` is the only way to get a writable plan. It refuses:

- non-removable devices (sysfs `removable` flag)
- devices with mounted partitions (`/proc/mounts`)
- zero-size devices and empty images
- images larger than the drive
- compressed images until a decompressor backend exists

The CLI adds two more gates: `effective_uid() == 0` and an explicit `--yes`.

## Linux backend

Enumeration is pure sysfs text parsing (`/sys/block/<name>/{size,removable,device/{vendor,model}}`),
mounts come from `/proc/mounts`, uid from `/proc/self/status` — no ioctls, no unsafe, no libc.
`list_drives(sysfs_root, mounts_text)` takes the roots as parameters so tests run against
fixtures.

Writes stream `File → File` in 4 MiB chunks with a `Progress` callback and end in `sync_all`.
Verify re-reads the device over the image length and compares bytes.

## Parity with Rufus (target app)

Done: removable-drive listing, ISO/raw writing, verify.
Missing: FAT32/NTFS/exFAT formatting, Windows To Go, compressed image writing, bad-block check,
ISO download, persistent partitions, FreeDOS, UEFI:NTFS, Windows-only features (by design —
FlashCraft is Linux-first).

## WebAssembly

`core` and `ui-egui` build for `wasm32-unknown-unknown`; `native` and everything touching the
OS is `cfg(not(target_arch = "wasm32"))`. In the browser the UI renders the layout and reports
that device access is unavailable — same trick as craft-launcher's web catalog.
