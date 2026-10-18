# Cut a release

1. `cargo xtask ci` green; tree committed.
2. `cargo xtask version set X.Y.Z` (then commit the stamp diff).
3. `mv` the project folder to `<App> vX.Y.Z (<Target>)` if folders follow the convention.
4. `./packaging/linux/tarball.sh`, then `deb.sh`, `rpm.sh`, `appimage.sh`; the Flatpak via
   `flatpak run org.flatpak.Builder … --repo=packaging/flatpak/repo` then `build-bundle`.
5. `cd dist && sha256sum *.{rpm,deb,AppImage,flatpak,tar.gz} > SHA256SUMS.txt`.
6. Spot-verify: `rpm -qpl`, `dpkg-deb --contents`, `flatpak install --user --bundle` +
   sandboxed smoke test.
