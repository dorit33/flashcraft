# Release playbook

- The version lives ONLY in `[workspace.package] version` of the root `Cargo.toml`.
  `cargo xtask version` prints it; `cargo xtask version set X.Y.Z[-pre]` sets it.
- `set` validates semver, writes the manifest atomically, refreshes `Cargo.lock`, and stamps
  the places that can't read Cargo metadata: the RPM spec `Version:` header and the AppStream
  `<release>` entry (with today's date). UI version labels compile in via
  `env!("CARGO_PKG_VERSION")` — never hard-code one.
- Version-scattered places to audit when adding a new artifact type: spec `Version:`,
  metainfo `<release>`, flatpak/MSI metadata, the bundle filename (`<app>-<ver>-<os>-<arch>`).
- Artifact names: `<app>-<ver>-<os>-<arch>.{rpm,deb,AppImage,flatpak,tar.gz}` plus a
  `SHA256SUMS.txt` beside them in `dist/` for releases that leave the machine.
