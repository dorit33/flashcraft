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

## 2026-10-18 — all 38 Rufus languages + GPLv3 relicensing

- `Lang` now covers every locale Rufus ships (38 incl. Auto/English); each has a
  `i18n/<lang>.tsv` catalog and a native-name label in the picker.
- Translations are imported from upstream `res/loc/rufus.loc` by
  `tools/loc-import.py`: our English keys map to Rufus message IDs (incl.
  composed "Show/Hide %s" strings), `%s`→`{0}` conversion, button uppercasing.
  ~30 of 47 keys come from Rufus per language; the rest are hand translations
  (Settings, Language, log/status messages, wasm notices).
- Reused translations are GPLv3, so the project was relicensed
  MIT+Apache-2.0 → GPL-3.0-only (LICENSE, Cargo.toml, spec, metainfo, README,
  ATTRIBUTION.md). This is the one sanctioned non-clean-room reuse; all other
  Rufus resources stay off-limits.
- RTL: `Lang::right_to_left()` (ar/fa/he) wraps the panel in
  `Layout::top_down(Align::Max)`, which makes `prefer_right_to_left()` true so
  nested `ui.horizontal` rows mirror automatically.
- `detect()` maps `<lang>_<region>` prefixes (iw→he, in→id, no/nb/nn→nb,
  pt_BR→pt-BR, zh_TW/zh_HK→zh-TW, else zh→zh-CN).

## 2026-10-18 — UI rebuilt from rufus.rc IDD_DIALOG

- The dialog is now laid out from the actual Rufus dialog template
  (`src/rufus.rc`, IDD_DIALOG 232×326 dialog units) instead of a row flow:
  a `ctl(x,y,w,h)` helper converts dialog units to pixels, and every control
  sits at its .rc coordinates — paired Partition scheme/Target system and
  File system/Cluster size columns, Image option + persistence slider/size/
  units row, advanced sections as checkboxes (BS_AUTOCHECKBOX), Status label,
  always-visible progress bar, START + CLOSE bottom-right.
- New fields: `old_bios_fixes`, `uefi_media_validation`, `bad_blocks` (matching
  the .rc checkboxes; disabled until their backends land). New i18n keys:
  Close, Add fixes for old BIOSes…, Enable runtime UEFI media validation —
  all imported from rufus.loc; "Check device for bad blocks" lost its colon.
- Log is a separate `egui::Window` (mirrors IDD_LOG); CLOSE quits via
  ViewportCommand::Close; the boot combo shows the selected ISO's name.
- loc-import merge order changed: existing catalog rows win over rufus.loc so
  hand-edited translations are never overwritten on re-import.
- x-mirroring kept for ar/fa/he (ctl flips x' = 232 − x − w).

## 2026-10-18 — Win32-dialog visuals + system fonts

- apps/flashcraft: `win32_visuals()` replaces the dark theme — #F0F0F0 dialog
  face, black text, white fields (inactive.bg_fill), #E1E1E1 buttons
  (weak_bg_fill), #0078D7 hover/selection, zero corner radii, 1px #ADADAD
  borders — the classic Rufus-on-Windows look.
- `load_fonts()` adds Liberation Sans (Arial metrics ≈ Segoe UI) as the
  primary font and NotoSansCJK as fallback so CJK catalogs render; missing
  fonts are skipped and egui built-ins remain.
- Labels render in left_to_right child UIs (LTEXT semantics); second-column
  label rects are edge-clamped; status/error text is dark red/amber on the
  light face; progress fill is Rufus green #06B025.

## 2026-10-18 — License packaging audit

- Every artifact now ships the license: tarball + AppImage carry
  share/doc/flashcraft/{LICENSE,ATTRIBUTION.md}; the rpm uses %license/%doc;
  the deb gains a Debian-format copyright file naming the GPLv3 and the
  Rufus-translation attribution.
- Rebuilt dist/ artifacts — the earlier rpm still advertised
  MIT OR Apache-2.0 from before the relicensing.

## 2026-10-18 — Flatpak via org.flatpak.Builder

- org.flatpak.Builder installed user-remote (no root); manifest now builds
  from source in the 25.08 sandbox via rust-stable SDK extension with
  network build-args for crates.io.
- Desktop file renamed to dev.flashcraft.app.desktop across every packager
  (flatpak refuses non-app-id export names); metainfo launchable updated.
- Bundle exported to dist/flashcraft-0.1.0-x86_64.flatpak and verified:
  installs user-remote and the sandboxed CLI lists host devices.

## 2026-10-18 — Versioning, Craft-style

- `cargo xtask version set X.Y.Z[-pre]`: semver-validated, atomic
  manifest write, Cargo.lock refresh; also stamps the RPM spec Version:
  header and the AppStream <release> entry — one source of truth.
- UI version label now compiles in via env!("CARGO_PKG_VERSION").
- AGENTS.md gained a Releasing section with the packaging recipe.
