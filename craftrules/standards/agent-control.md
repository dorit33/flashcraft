# Agent control

Every feature an agent can trigger is a **command** — the UI, CLI, control channel and MCP
all dispatch by the same command id. There is no "GUI-only" way to do something.

## Everything is a command

User-visible behaviour = a `CommandSpec` in the engine/registry (`id`, `label`, menu path,
shortcut, params doc, `destructive` flag, `enabled`, JSON `run`) + tests. Pure view/window
state (zoom, panels) may stay in the shell's `UI_COMMANDS`; everything else goes through the
registry.

## Control channel

The desktop app takes `--control PORT` and serves **JSON lines** on `127.0.0.1:PORT`:

```json
{"id":1,"method":"engine.execute","params":{"command":"<id>","params":{…}}}
{"id":2,"method":"ui.screenshot","params":{"path":"/tmp/shot.png"}}
```

Typical methods: `engine.execute` (run a command), `ui.screenshot` / `ui.render` (grab a
frame — use `ui.render` when no frame is presented, e.g. locked screen), `ui.dialog.set`
etc. The protocol is documented in `docs/control-protocol.md`; the implementation lives in
the UI crate's `control.rs`. Sibling apps run control channels too — **pick a free port**.

## Rules

- **Programmatic calls never open dialogs.** `engine.execute` runs the JSON form with
  defaults; only real menu/toolbar invocation starts interactive prompts.
- The CLI's `mcp` subcommand exposes the same registry over MCP.
- **Verify UI work visually**: drive the app over the control channel, take a screenshot,
  read the PNG — compare with the reference app's observed screenshots.
