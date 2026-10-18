# Dev log

## 2026-10-18 — M0 skeleton

- Workspace `flashcraft` (core L0 / native L1 / ui-egui L2 / apps L3 + xtask), Craft-style
  layout and rules: `AGENTS.md`, `CLAUDE.md` pointer, `ATTRIBUTION.md`, `ROADMAP.md`.
- `flashcraft-core`: image-kind sniffing (ISO9660/MBR), drive model, `WritePlan` refusals,
  `Progress`, command registry (`drives.list`, `image.inspect`, `media.write`, `media.verify`).
- `flashcraft-native`: sysfs enumeration (`/sys/block`, `/proc/mounts`, `/proc/self/status`),
  4 MiB streaming write + fsync, byte-compare verify. No unsafe, no libc.
- `flashcraft-cli`: `list | inspect | write | verify | commands`; write needs root + `--yes`.
- `apps/flashcraft`: thin eframe shell over `flashcraft-ui-egui` (drive list, picker, progress).
- Tests: image sniff, plan refusals, hostile params, sysfs fixture enumeration,
  write→verify roundtrip on temp files.

**Still open:** decompressor backends (.xz/.gz/.zip), udisks2 unmount/eject, FAT32 formatting,
bad-block scan, checksums, i18n (hu first), real-device soak test,
control channel (`--control PORT`), web build via trunk.

## 2026-10-18 — packaging

- `packaging/icons.sh` renders `assets/app-icon/flashcraft.svg` (original artwork) to hicolor
  PNGs via `rsvg-convert`.
- `packaging/linux/`: `flashcraft.desktop` (validated), `dev.flashcraft.app.metainfo.xml`,
  `tarball.sh`, `deb.sh` (dpkg-deb), `rpm.sh` + `rpm/flashcraft.spec` (rpmbuild, debug_package
  off — binary payload), `appimage.sh` (downloads appimagetool into `packaging/.cache`),
  `flatpak/dev.flashcraft.app.yml`.
- Built and verified: `.rpm` 5.0 MB, `.deb` 4.6 MB, `.tar.gz` 6.9 MB, `.AppImage` 7.2 MB —
  all carry both binaries, desktop file, metainfo and all icon sizes.

**Still open:** signed builds/SHA256SUMS.txt, release workflow (`.github/workflows/release.yml`),
flatpak-builder run, appstreamcli validation.

## 2026-10-18 — Rufus-parity UI

- `ui-egui` reworked to the Rufus dialog, field for field: Drive Properties
  (Device, Boot selection + SELECT, persistent partition size), Format Options
  (Image option, Partition scheme MBR/GPT, derived Target system, Volume label
  autofilled from the image name, File system, Cluster size), advanced
  collapsibles ("Show advanced drive properties" → List USB Hard Drives;
  "Show advanced format options" → Quick format, extended label, bad-block
  passes), Status bar ("READY", red on error, green progress while running),
  Log panel, START bottom-right.
- START → plan → Rufus-style modal: "WARNING: ALL DATA ON DEVICE '…' WILL BE
  DESTROYED", OK/Cancel. Write/verify run on a worker thread reporting
  Progress over a channel; UI stays live and repaints.
- Rufus chrome: square corners everywhere, dark visuals, fixed 480×560 window.
- Format-only controls (File system, Cluster size, bad-block passes) are shown
  but disabled until the format backend lands (M2) — they're tracked as dead
  settings, not silently ignored.
- wasm32 build stays warning-free; all job/device code is cfg'd out on wasm.

## 2026-10-18 — i18n + language picker

- `crates/ui-egui/src/i18n.rs`: `Lang::{Auto,En,Hu}` + catalogs; English is the
  lookup key, missing rows fall back to the key (never blank). `Auto` follows
  `FLASHCRAFT_LANG`/`LC_ALL`/`LANG` (hu_* → Magyar). `tf()` does `{0}`
  substitution.
- Settings strip at the top: "Settings" toggle + "Language" combo
  (Auto/English/Magyar), switches the whole dialog immediately — labels,
  buttons, status, warning modal and log messages all localized.
- Hungarian catalog covers the full Rufus-parity UI (Meghajtó-tulajdonságok,
  Formázási beállítások, TALLÓZÁS, INDÍTÁS, KÉSZ, Mégse…).
