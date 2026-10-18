# Clean-room

When a Craft app reimplements a proprietary one (Rufus→FlashCraft, Photoshop→PhotoCraft):

- Study the original's **behaviour and feature list** — never copy code, icons, fonts or
  compiled resources. Implement from public specs and observation.
- Write a parity document (`docs/<target>-parity.md`) listing every surface and its state:
  done / partial / missing / not-applicable, with a measured scorecard.
- **The one licensed exception is explicit and documented.** If a GPL/CC-BY-SA resource is
  reused (e.g. a translation catalog), the project's license must be compatible — that is a
  maintainer's call, recorded in `ATTRIBUTION.md` and the license files, never an accident.
- Keep the original source clone OUT of the repo; reference it by path or URL.
