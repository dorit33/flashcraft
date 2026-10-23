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

## 2026-10-18 — craftrules bundled + progress docs

- craftrules/ bundled into the repo (self-contained standards); AGENTS.md references
  craftrules/<standard>. contributors/people.toml added; plan/ + corpus/ gitignored.
- docs/target-app-parity.md + docs/gaps.md created (progress-docs standard);
  ROADMAP M4 marked done.

## 2026-10-18 — repo moved under flashcraft-0.1.0/

- Source tree moved into flashcraft-0.1.0/ so the folder matches the Craft
  release layout (source dir + installers at the top level); dist artifacts
  moved next to it.
- xtask: child cargo runs via the CARGO env path AND gets the cargo dir on
  PATH (cargo-fmt/clippy resolve); ci green from the new root.

## 2026-10-18 — full release matrix

- New apps/flashcraft-web crate: eframe WebRunner shell over ui-egui
  (canvas element lookup via web-sys; trunk-independent index.html with a
  plain module loader, since trunk fails to build on this toolchain).
- New packaging scripts: env.sh (shared ROOT/VER/DIST prelude),
  web/web.sh (wasm-bindgen → flashcraft-web-<ver>.zip), source.sh
  (git archive → flashcraft-<ver>.zip), checksums.sh, release.sh
  (publishes dist/* next to the source dir + regenerates SHA256SUMS).
- Release dir now: 5 linux installers + source zip + web zip + SHA256SUMS.
  Windows/macOS rows intentionally absent — flashcraft-native is sysfs/polkit
  only; reason documented in AGENTS.md and craftrules packaging.md.

## 2026-10-18 — Windows artifacts (llvm-mingw + wixl, no sudo)

- Toolchains installed user-space: llvm-mingw 20261006 to
  ~/.local/toolchains/llvm-mingw, wixl 0.106.58 + libmsi + libgcab extracted
  from Fedora rpms to ~/.local/toolchains/wixl.
- rustup targets added: x86_64/i686-pc-windows-gnullvm; both compile
  flashcraft + flashcraft-cli cleanly (no unix-only code in native crate).
- packaging/windows/: flashcraft.wxs (wixl schema, per-arch ProgramFiles
  substitution) + windows.sh → x64 portable zip, x64 msi, x86 msi.
- release.sh now copies *.msi too; parent dir matches the Craft matrix:
  8 installers + source zip + web zip + SHA256SUMS.
- AGENTS.md documents that Windows binaries run but see no drives
  (sysfs/polkit backend is Linux-only; Windows backend is roadmap work).

## 2026-10-18 — Rufus-parity: auto-unmount on write

- linux::unmount_partitions: udisksctl then umount fallback per mounted
  partition, verified by re-reading /proc/mounts. WritePlan stays strict —
  no plan is ever built for a mounted drive; the unmount runs inside the
  write path after the confirmation gate (UI warning dialog / CLI --yes).
- UI: START on a mounted drive now opens the warning (blocked_only_by_mounts
  checks the sole refusal), the job unmounts in the thread then re-plans.
  JobMsg::Status streams localized status keys ("Unmounting mounted
  partitions", hu.tsv entry added, others fall back to English).
- CLI: write unmounts after root + --yes checks; verify stays strict.

## 2026-10-18 — polkit elevation (the Rufus UAC flow)

- GUI runs unprivileged; write/verify on a non-root session spawns
  `pkexec flashcraft-cli <op> … --progress` — sibling binary lookup next
  to the exe, PATH fallback. polkit shows the password prompt at Start.
- flashcraft-cli --progress emits one JSON Progress line per update on
  stdout; the GUI parses them into the progress bar, stderr carries the
  human-readable errors (last non-empty line → status bar).
- When the GUI itself runs as root, everything stays in-process
  (unmount + plan + write), no subprocess.

## 2026-10-18 — live phase status (Rufus-style "READY" bar)

- core::Progress gains a Phase field (Writing/Verifying/Syncing); write_image
  emits a final Syncing event before sync_all, so the 100% bar no longer
  sits silent while the kernel flushes.
- UI status line now shows the phase live: "Írás: X%" / "Ellenőrzés: X%" /
  "Szinkronizálás az eszközre…" → "WRITE DONE — KÉSZ" (hu.tsv keys added).
- CLI --progress also emits {"status":"unmounting"} so the elevated GUI path
  shows the unmount phase; human mode prints a sync note once.

## 2026-10-18 — volume label applied (ISO 9660 PVD)

- WritePlan carries volume_label (with_label() builder). write_image patches
  the ISO 9660 Primary Volume Descriptor volume identifier on the fly —
  the typed label lands on the written media, like Rufus.
- Sanitize: ISO d-characters only (A-Z 0-9 _), space-padded to 32 bytes.
- verify_image applies the same patch to the image side before comparing,
  so a labeled write verifies clean. CLI gains --label for write+verify;
  the GUI forwards the label through pkexec too. Tests: labeled ISO
  write+verify roundtrip, d-char sanitization.

## 2026-10-18 — unmount robustness under pkexec

- unmount_partitions now skips udisksctl when already root (pkexec has no
  user session → udisks2 polkit lookup fails spuriously), goes straight to
  umount, and falls back to `umount -l` (lazy, the Rufus "force dismount"
  equivalent) for busy mounts.
- Failures now carry the tools' own stderr in the error message.
- Test updated: a stale (not actually mounted) entry is a no-op, matching
  the idempotent semantics.

## 2026-10-18 — fresh full release matrix

- Rebuilt all 11 artifacts with the auto-unmount/pkexec/phase/label work:
  rpm, deb, AppImage, tar.gz, flatpak, x64+x86 msi, x64 portable zip,
  source zip, web zip, SHA256SUMS.txt. `sha256sum -c` all green.
- Fixed rpm.spec %install: it copied share/applications/flashcraft.desktop
  but the tarball ships dev.flashcraft.app.desktop (stale name → build break).
- rpm.sh now copies the built rpm into dist/ (release.sh publishes from
  dist/ only — the parent dir kept a stale rpm before this fix).
- checksums.sh find now covers *.msi as well.

## 2026-10-18 — portable glibc baseline (Pop!_OS fix)

- Reported on Pop!_OS 24.04 (deb installed, icon present, app would not
  start): the host-built binaries need GLIBC_2.43 — they do not even load
  on glibc 2.39 systems.
- New packaging/linux/portable-build.sh: podman + ubuntu:22.04 container
  build → glibc 2.35 requirement (Ubuntu 22.04+/Pop 24.04/Debian 12+/Fedora).
  tarball.sh uses those binaries; falls back to host build only when
  podman is missing (FLASHCRAFT_HOST_BUILD=1 forces host build).
- SELinux note: the podman bind mount needs :Z or the container cannot
  write into the repo (cargo install died with EACCES).
- Rebuilt tarball/deb/appimage/rpm; all now carry GLIBC_2.35-max binaries.
- scripts/checksums.sh already covers .msi; rpm lands in dist/ for
  release.sh.

## 2026-10-22 — Craft-style icon, full Rufus controls wired, formatting & checksums

- New Craft-family app icon (`assets/app-icon/flashcraft.svg` + all hicolor
  PNG renders + scalable SVG): 512x512 rounded tile (`rx=112`), warm paper
  (`#efe9dc`), deep ink (`#0b0b0c`) with engraved cross-hatching, electric
  crimson-amber tile (`#d9381e`) and golden lightning bolt (`#f59f00`).
  Wired into the desktop window (`ViewportBuilder::with_icon`) and header badge.
- Pure-Rust streaming MD5, SHA-1, SHA-256 in `flashcraft-core::checksum`
  (`image.checksum` command, CLI `checksum`, and UI `#` Checksums window).
- On-the-fly compressed image streaming (`.xz`, `.gz`, `.zst`, `.bz2`, `.zip`)
  on both write and verify.
- Standalone drive partitioning & formatting (`Non bootable` mode in Rufus /
  `media.format` command / CLI `format`): MBR & GPT via `sfdisk`, plus
  `FAT32`, `NTFS`, `exFAT`, `UDF`, `ext4`, `ext3`, `ext2` formatters with
  configurable cluster size, volume label, quick/full zero-fill, and
  `autorun.inf` extended label creation.
- Bad-block scanning (1..=4 passes, `media.badblocks`), legacy BIOS MBR
  active-flag fix (`--old-bios-fixes`), post-write verification toggle
  (`--verify`), and persistent `writable` ext4 partition creation (`--persistence`).
- Responsive, structured Rufus UI layout with section dividers, active
  controls, overlaid progress bar status + MB/s speed, elapsed timer,
  Cancel during active jobs, Verify button, Checksums (`#`), Log and About.

## 2026-10-23 — v0.2.0 local rebuild + artifact refresh

- `cargo xtask ci` green (fmt/clippy/tests/layers/wasm).
- Full packaging matrix rebuilt from clean target: tar.gz, deb, rpm (fc44),
  AppImage, flatpak bundle, web zip, windows x64 msi + portable zip + x86 msi,
  source zip. `release.sh` published all 11 artifacts next to the source dir
  with a fresh `SHA256SUMS.txt`, then cleaned `target/`, `dist/` and
  `.flatpak-builder` caches.
- Checksums differ from the published GitHub v0.2.0 assets (build metadata —
  builds are not byte-reproducible); published release left untouched.
- craft-gyujto catalog repo pushed to github.com/dorit33/craft-gyujto, Pages on.

## 2026-10-23 — v0.3.0 release

- `cargo xtask version set 0.3.0`; workspace, Cargo.lock, RPM spec and AppStream
  release stamped. `cargo xtask ci` green.
- Full matrix rebuilt: tar.gz, deb, rpm (fc44), AppImage, flatpak bundle, web zip,
  windows x64 msi + portable zip + x86 msi, source zip; `release.sh` published all
  11 artifacts with fresh `SHA256SUMS.txt` and cleaned build caches.
- Tagged v0.3.0; GitHub release carries the full artifact matrix.

## 2026-10-23 — MCP server (`flashcraft-cli mcp`)

- `flashcraft-cli mcp` serves the command registry over MCP: newline-delimited
  JSON-RPC 2.0 on stdio (initialize/ping/tools/list/tools/call, resources and
  prompts as empty lists, `notifications/progress` when the caller passes a
  `progressToken`).
- Tool names are registry ids with `_` for `.` (`drives_list`, `media_write`,
  …). `CommandSpec` gained `input_schema` (JSON Schema) — same registry feeds
  CLI help and MCP `tools/list`.
- Destructive tools keep the structural gates: `confirm: true` in arguments
  plus root (same as `--yes` + sudo on the CLI).
- Verified over stdio: initialize, ping, tools/list (7 tools + schemas),
  drives_list (live sysfs data + writable reasons), image_inspect (Pop!_OS
  ISO), image_checksum, refusal paths (no-root write/format, bad drive, bad
  image, unknown tool → `isError`, unknown method → -32601).
- Registered in `~/.config/devin/mcp_config.json` as the `flashcraft` server;
  `cargo xtask ci` green.
- MCP destructive tools now elevate like the GUI: when the server is not
  root, media.write/verify/format/badblocks run `pkexec flashcraft-cli …
  --progress` and stream the child's JSON progress lines onward as MCP
  `notifications/progress`. `precheck_dev` refuses structurally-unsuitable
  drives in the parent before any polkit prompt (mounted devices pass — the
  elevated child unmounts them itself).
- Live-tested: `media_verify` on /dev/sdd read back the full 3.9 GB Pop!_OS
  image through pkexec and reported success; refusal paths (no `confirm`,
  non-removable drive) still short-circuit before elevation.
