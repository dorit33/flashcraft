# Fonts

**Fonts live in `storytold/craft-fonts`, never in the app repo.** Don't commit font files.

- `craft-fonts` is an **optional build input**, exposed as the environment variable
  `CRAFT_FONTS_DIR` pointing at a checkout: `CRAFT_FONTS_DIR=../craft-fonts cargo build`.
- A crate's `build.rs` embeds the manifest's fonts as a `<app>_engine::CRAFT_FONTS` (or
  equivalent) constant. **Everything must build, test and run with it unset** — the constant
  is just empty; features that need a font degrade (e.g. no CJK glyphs), they don't fail.
- It is never a `Cargo.toml` dependency.
- **Releases always build with it** (`CRAFT_FONTS_REQUIRED=1` in CI) and ship the fonts' OFL
  licences. Tests needing the fonts skip when it is absent.
- CJK coverage comes from craft-fonts (OFL faces; the maintainer-approved exception is
  Google-branded Noto CJK, used unmodified under its OFL — it does not open the door to
  Adobe-named Source Han or any other proprietary face).
- A single app-local exception is allowed when a face is genuinely part of the product
  (e.g. a stroke font implemented in code); record it in `ATTRIBUTION.md` as original work.
