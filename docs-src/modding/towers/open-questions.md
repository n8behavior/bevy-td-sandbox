# Open Questions

Design questions about tower modding that haven't settled. Anything documented elsewhere in this manual is the *current best guess*; this page is what's still being chewed on.

The companion [`TOWER_EDITOR_BRAINSTORM.md`](https://github.com/n8behavior/bevy-td-sandbox/blob/main/TOWER_EDITOR_BRAINSTORM.md) holds the broader runtime-model discussion.

## Runtime and language

### Lua version

**Lua 5.4** via [`mlua`](https://crates.io/crates/mlua). Sticking with 5.4 unless we hit a real perf wall.

### WASM build

The web (itch.io) target uses `mlua`'s WASI backend — recipes run identically on browser and native. *Need to verify*: bundle size and sandbox compatibility.

### Sandbox specifics

Currently allowed: `string`, `math`, `table`. Blocked: `os`, `io`, `package`, `require`, `debug`, `dofile`, `loadfile`, raw `load`. Open: `coroutine`? `pcall`/`xpcall`? `error`? A `safe_print` for recipe debugging?

### Hot reload

Dev builds reload recipes on file change. Towers already placed keep their old behavior; new ones get the new recipe. *Open*: surface that asymmetry to the player, or leave them as snapshots?

### Argument coercion

`cooldown = 1` and `cooldown = 1.0` should both work. Lua 5.4 distinguishes integers from floats; the engine coerces silently.

### Versioning

Recipes will outlive property renames. We don't need a strategy yet — eventually we'll want either a hard-break with clear errors, a compat shim, or a `schema = N` declaration.

## Distribution

### Mod folders

Currently recipes load from `assets/towers/`. Open: a separate user-mods folder? Per-mod-pack folders? An in-game mod manager?

### Sharing

Single recipe files are pasteable. Open: a *bundle* format for multiple recipes plus assets and a manifest? Probably yes, eventually.

## Editor

### Mapping nested deliverer blocks to UI

The earlier flat-atom model had an obvious editor layout: a palette of atoms grouped by role. The nested model is more structured — pick a deliverer, then fill in its properties. Likely a wizard-style flow per block, with passives in a sidebar. *Open*: how to visualize multi-deliverer towers — one panel per block?

### IR

Render directly from recipe data, or maintain an intermediate representation? An IR is more flexible (undo, partial validity), more code. Direct-render is simpler.

### Player tool vs developer tool

Same UI, two audiences. Players want guardrails ("hard to build something broken"); developers want freedom for testing. One mode? Two modes? A "dev panel" toggle?

---

If something here becomes settled, it'll move to its proper home in the manual and be removed from this page.
