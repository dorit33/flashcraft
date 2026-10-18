# Layering

```text
crates/
  core      L0  pure, wasm-safe: data model, parsing, plans, the command registry
  native    L1  OS backends: enumeration, file/device I/O, spawning
  ui-egui   L2  the shared egui UI (desktop and web)
apps/
  <app>     L3  desktop shell (eframe): window, tray, notifications
  <app>-cli L3  headless CLI over the same commands
xtask/          cargo xtask ci | layers | wasm | version
```

Rules:

- `core` depends on no workspace crate and must `cargo check` for
  `wasm32-unknown-unknown`. `native` may use `core`. `ui-egui` may use `core`, and `native`
  only behind `#[cfg(not(target_arch = "wasm32"))]` or through a trait the shell implements.
- Nothing below the apps opens windows. Nothing below L2 touches egui/eframe/winit/rfd.
- Enforced mechanically: `cargo xtask layers` lists each crate's forbidden deps and greps its
  manifest. Add new crates to the table in xtask AND here.
- `cargo xtask wasm` checks every L0–L2 crate for `wasm32-unknown-unknown`.
