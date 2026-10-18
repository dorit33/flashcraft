# AGENTS.md — craftrules

This directory holds the **shared Craft learnings** — documentation plus the small shared
tools under `scripts/` that apps call by relative path (e.g. `contributors.py`). It ships
inside each app (scaffolded from the template), so every repo is self-contained. Apps never
share application code. Rules:

- One file per standard; each is short enough to read before a task.
- Write rules so an app that *isn't* a Craft app could still follow them — no product names
  inside standards files; apps keep product specifics in their own AGENTS.md.
- When an app discovers a reusable convention, add or amend the standard here, then link it.
- Keep examples generic (`yourcraft`, `dev.yourcraft.app`).
