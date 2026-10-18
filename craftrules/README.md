# craftrules — shared standards and learnings for the Craft apps

Documentation only — **never code**. This directory holds the shared Craft standards,
bundled inside the app so the repo is self-contained; `AGENTS.md` references these paths as
`craftrules/<standard>`. Edit the standards here; the change ships to future apps through
the template.

## Contributor-facing files in every app repo

- `AGENTS.md` is the single guide for agents and contributors — the product-specific rules
  live there, the shared ones are referenced as `craftrules/…`.
- `CLAUDE.md` is a one-line pointer to it (`Read AGENTS.md first`) — never a copy. Copies
  drift; some older apps still carry a duplicated CLAUDE.md from before this convention.
- `docs/contributors.md`, `log/devlog.md` and the progress docs follow `standards/`.

## Layout

- `standards/` — engineering rules every app shares
- `release/` — how a version gets cut and shipped
- `scripts/` — the few shared tools every app calls by relative path (`contributors.py`)

## The standards

| File | Rule |
| --- | --- |
| `standards/never-crash.md` | No panic, no unsafe, hostile-input posture, deny-lints |
| `standards/layering.md` | crates → apps layering, wasm boundary, xtask layers |
| `standards/i18n.md` | English-as-key TSV catalogs, fallback, RTL, CJK |
| `standards/clean-room.md` | Reimplementing a proprietary app without copying it |
| `standards/assets.md` | Asset policy: licences, ATTRIBUTION.md rows, brand exception |
| `standards/fonts.md` | craft-fonts via `CRAFT_FONTS_DIR`; never commit font files |
| `standards/test-corpora.md` | Pinned, gitignored corpus repos; `xtask corpus`/`test-corpus` |
| `standards/agent-control.md` | Everything is a command; `--control` JSON-lines channel; MCP |
| `standards/progress-docs.md` | ROADMAP/parity/gaps/scorecard docs and how they stay current |
| `standards/contributors.md` | About-window credits: `contributors.json` + `people.toml` |
| `standards/shared-libs.md` | craft-libs components pinned by git revision |
| `standards/licensing.md` | License choice, ATTRIBUTION.md, shipping LICENSE |
| `standards/packaging.md` | The installer matrix and the metadata each format needs |
| `release/playbook.md` | Single-source versioning and the release recipe |
| `release/cut-a-release.md` | Checklist for shipping one version |
| `scripts/contributors.py` | Generates `contributors/contributors.json`; `--add-me`, `--check` |
