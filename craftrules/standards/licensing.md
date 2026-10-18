# Licensing

- Default: `MIT OR Apache-2.0` (`license.workspace = true` in every crate manifest).
- If the project reuses GPL/CC-BY-SA material (translations, fonts, data), relicense the
  whole project compatibly — a relicensing is a maintainer decision, not an agent's.
- `LICENSE` carries the full license text at the repo root.
- `ATTRIBUTION.md` has a row for every third-party asset: path, title, author, source,
  license. Bundled assets without a row fail review.
- **Every installer ships the license**: `share/doc/<app>/{LICENSE,ATTRIBUTION.md}` —
  `%license`/`%doc` for RPM, a Debian `copyright` file for DEB, copied dirs for tarball,
  AppImage and Flatpak. Rebuild artifacts after a license change — the metadata inside is
  what users and package tools read.
