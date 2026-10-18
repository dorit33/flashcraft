<h1 align="center">FlashCraft</h1>

<p align="center">
  <b>Bootable USB media; an open-source, clean-room reimplementation of Rufus, rebuilt in pure Rust.</b><br>
  Pick an image, pick a drive, write, verify. Native app, no Electron, no web view.
</p>

<p align="center">
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Linux · FreeBSD · Web-native" src="https://img.shields.io/badge/Linux%20%C2%B7%20FreeBSD%20%C2%B7%20Web-native-26282e?style=flat-square">
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-3a3a3a?style=flat-square">
  <img alt="Status: early alpha" src="https://img.shields.io/badge/status-early%20alpha-d69e2e?style=flat-square">
</p>

<table>
  <tr>
    <td width="33%" valign="top">
      <h3>Refuses the wrong drive</h3>
      Only removable, unmounted devices can be written — checked structurally, not by a warning you can click through.
    </td>
    <td width="33%" valign="top">
      <h3>Native and small</h3>
      Native egui on desktop, the same code compiles to WebAssembly. No Electron, no web view.
    </td>
    <td width="33%" valign="top">
      <h3>Agent-ready</h3>
      Every action is a command (<code>drives.list</code>, <code>media.write</code>, <code>media.verify</code>), so the UI, the CLI and scripts share one engine.
    </td>
  </tr>
</table>

## Features

- Enumerates removable drives from sysfs: size, vendor, model, mount state.
- Detects image kinds by header sniff (ISO 9660, MBR/GPT disk image, compressed).
- Streams images in 4 MiB chunks with live progress; never loads the file into memory.
- Byte-for-byte read-back verification.
- Every destructive path refuses first: non-removable, mounted, zero-size, oversized.

## Downloads

`dist/` (built locally by the scripts in `packaging/`):

| File | What it is |
|---|---|
| `flashcraft-<ver>-linux-x86_64.rpm` | Fedora, openSUSE, RHEL: `sudo dnf install ./flashcraft-…rpm` |
| `flashcraft-<ver>-linux-x86_64.deb` | Debian, Ubuntu, Mint, Pop!_OS: `sudo apt install ./flashcraft-…deb` |
| `flashcraft-<ver>-linux-x86_64.AppImage` | Runs on almost any distro: `chmod +x` and run |
| `flashcraft-<ver>-linux-x86_64.tar.gz` | Plain tree (`bin/`, `share/`) for everyone else |
| `packaging/flatpak/dev.flashcraft.app.yml` | Flatpak manifest (`flatpak-builder`) |

## Get started

You need a recent stable Rust (1.85 or newer). On Linux, install the windowing headers first
(Debian/Ubuntu: `sudo apt install libxkbcommon-dev libwayland-dev libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev`;
Fedora: `sudo dnf install libxkbcommon-devel wayland-devel libX11-devel libXrandr-devel libXi-devel mesa-libGL-devel`).

```sh
cargo run -p flashcraft-cli -- list                         # what can I write to?
cargo run -p flashcraft-cli -- inspect fedora.iso         # what kind of image is it?
sudo ./target/debug/flashcraft-cli write fedora.iso /dev/sdb --yes
sudo ./target/debug/flashcraft-cli verify fedora.iso /dev/sdb
cargo run --release -p flashcraft                         # the desktop app
cargo xtask ci                                            # fmt, clippy, tests, layers, wasm
```

Block-device writes need root; the supported grant is a polkit prompt (`pkexec`) or `sudo`.

## Status

Early alpha. Linux enumeration, image inspect, write and verify work today; the egui app is a
thin shell over the same commands. See `ROADMAP.md` for what's next (decompression backends,
bad-block check, persistent partitions, FAT32 formatting, packaging).

## License

MIT OR Apache-2.0, at your option.
