# Asset policy (absolute — no exceptions)

Nothing from the proprietary products the app reimplements — no iconography, images, fonts,
patterns, templates, artwork or help text, **ever**. Breaking this rule is the most serious
mistake possible in an app repo.

## What may be committed

Every file asset must be one of:

- **our own original work** (icons drawn in code as vectors, procedurally generated demo
  content — recorded as "original work")
- **public domain / CC0**
- **Creative Commons** (CC-BY / CC-BY-SA, attribution honoured)
- **OFL** (fonts — but see `fonts.md`; fonts usually live in craft-fonts, not the repo)
- **permissive open source** (MIT/Apache-2.0/BSD/ISC), or an asset contributed openly licensed
  by its own creator

## Attribution is mandatory

- Every asset gets a row in `ATTRIBUTION.md` (or `assets/ATTRIBUTION.md`) in the **same
  commit**: path, title, author/creator, source URL or "original work", licence, date added,
  modifications.
- The asset's licence text ships next to it when required (e.g. `assets/fonts/OFL-*.txt`).
- Assets without an attribution row must not be committed. Apps enforce this with
  `cargo xtask assets`.

## Exceptions

- `docs/brand/` — the ArtCraft name, wordmark and logos are trademarks, not open source;
  terms in `docs/brand/LICENSE-brand.txt`. Use only unmodified; never redraw, recolour or
  derive.
- Screenshots of the proprietary reference app live only in the gitignored `plan/` and are
  never committed or published.
- Demo/test media for regression corpora follow `test-corpora.md` — fetched, pinned,
  gitignored; never committed to the app repo.
