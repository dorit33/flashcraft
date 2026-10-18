# Shared Rust components (craft-libs)

App repos **don't share code with each other** — each is self-contained. Genuinely reusable
Rust components live in `storytold/craft-libs` and are consumed as **cargo dependencies
pinned by immutable git revision** (`git = "…", rev = "<sha>"`), never by branch or path.

- A component belongs in craft-libs when ≥2 apps need the same non-trivial implementation
  (color transforms, fonts plumbing, control-channel helpers, …).
- Pin to a commit sha; bump the pin deliberately in a single commit, with `cargo xtask ci`
  green after.
- What stays in the app: product naming, UI, domain logic, packaging. What stays out of
  every repo: fonts (`fonts.md`) and test corpora (`test-corpora.md`).
