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
