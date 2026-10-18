# Packaging

One script per format under `packaging/`; everything consumes the released binary tree.
`packaging/env.sh` is the shared prelude (exports `ROOT`, `VERSION` read from
`[workspace.package]`, `DIST`, build SHA/date, `CARGO_TARGET_DIR`).

## The matrix

| Directory | Artifacts |
| --- | --- |
| `packaging/linux/` | `.tar.gz`, `.deb`, `.rpm`, `.AppImage`, `.flatpak` |
| `packaging/windows/` | `.msi` (WiX `*.wxs` + `package.ps1`, optionally `sign.ps1`), portable `.zip` |
| `packaging/macos/` | `.app` bundle / `.dmg` |
| `packaging/web/` | wasm build (`trunk` + `wasm-bindgen`) |
| `packaging/freebsd/` | port/pkg where the platform is supported |
| `packaging/icons.sh` | renders the icon set from the master `assets/app-icon/<app>.svg` |

## Rules

- `packaging/linux/tarball.sh` builds the release binaries and lays out
  `bin/` + `share/{applications,metainfo,icons,doc}` — every other Linux format repackages it.
- `.deb` needs a `DEBIAN/control` plus `usr/share/doc/<app>/copyright`; `.rpm` a real spec
  with `%license`; AppImage an `AppDir` with `AppRun`; Flatpak a manifest building **from
  source** in the freedesktop sandbox (`rust-stable` SDK extension, network build-arg).
- Desktop file and icon are named `<app-id>.desktop` / `<app-id>.png` — Flatpak exports only
  app-id-named files; the metainfo `<launchable>` and `<id>` match.
- The app icon is generated from a master SVG (`packaging/icons.sh` + resvg/rsvg-convert);
  never hand-edit renders.
- Every artifact ships `LICENSE*` + `ATTRIBUTION.md` (`licensing.md`).
- A package that can't state its license, desktop-id and launchable is unfinished.
