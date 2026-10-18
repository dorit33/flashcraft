# Test corpora

Real-file corpora (proprietary-format samples, oracles produced by the reference app) live in
**separate `storytold/<app>-corpus` repos**, fetched into the gitignored `corpus/` directory —
never committed to the app repo.

- `cargo xtask corpus --all` fetches every corpus at the pins in
  `xtask/src/corpus_pins.rs` (immutable git revision + sha256-verified).
- Corpus tests are **opt-in** behind a cargo feature (`corpus`): plain `cargo test` skips
  them; with the feature on, a missing corpus fails with "run `cargo xtask corpus --all`".
- `cargo xtask test-corpus` fetches and runs the whole suite; `--changed` runs what the diff
  touches. CI always runs them (a `corpus` job cached by pin).
- New oracles are authored against the reference app, pushed to the corpus repo, then
  land here as a **pin bump** — never as files in the app repo.
- CC0/public-domain test media may be downloaded into `corpus/` with the source recorded;
  the same rules apply.
