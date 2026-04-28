# Open Questions

This page tracks design questions about tower modding that haven't been settled yet. They're called out here so you know where the system is still in motion. Anything documented elsewhere in this manual reflects the *current best guess*; this page is what's still being chewed on.

The companion [`TOWER_EDITOR_BRAINSTORM.md`](https://github.com/n8behavior/bevy-td-sandbox/blob/main/TOWER_EDITOR_BRAINSTORM.md) holds the broader design discussion.

## Runtime and language

### Lua version

We're using **Lua 5.4** via the [`mlua`](https://crates.io/crates/mlua) crate. 5.4 brings the integer/float split and `goto`. LuaJIT (`mlua` also supports it) would be faster but loses 5.4-specific features and has a different WASM story. Sticking with 5.4 unless we hit a real perf wall.

### WASM build

The web (itch.io) target uses `mlua`'s WASI backend. This means recipes load and run identically in browser and native builds. *Need to verify*: that `mlua-wasi` doesn't add an unacceptable WASM bundle size, and that the WASI sandbox doesn't conflict with our own sandbox layer.

### Sandbox specifics

Currently allowed: `string`, `math`, `table`. Currently blocked: `os`, `io`, `package`, `require`, `debug`, `dofile`, `loadfile`, raw `load`.

Open: should we allow `coroutine`? A `safe_print` for recipe debugging? `error`, `pcall`, `xpcall`? `string.dump`? The rule of thumb is "allow if it can't escape the sandbox or read external state." Some of these are borderline.

### Hot reload

Dev builds reload recipes when their `.lua` file changes. Towers already placed on the map keep their previously-resolved behavior; freshly-built ones get the new recipe. *Open*: do we surface that asymmetry to the player ("this tower's recipe was edited, replace it?") or just leave them as snapshots?

## Recipe shape

### Argument coercion

Should `Range(80)` and `Range(80.0)` both work? Lua 5.4 distinguishes integers from floats; most atoms want a float. The current plan is to coerce silently — the modder shouldn't have to think about it. The editor saves canonical form (always `.0` for floats), so the question only matters for hand-written recipes.

### Sub-recipe syntax

`Trap` and (eventually) `Splash` involve sub-recipes — atoms whose parameters are themselves an atom list. The current design has them as a `template` field with a list value:

```lua
Trap {
    lifetime = 60,
    template = { OnWorldEvent "EnemyStep", DirectDamage(50) },
}
```

Open: is that the right shape, or should sub-recipes be referenced by name (`template = "mine_default"`) so they can be reused and edited independently? The first is more local (everything you need is in the parent recipe); the second composes better at scale.

### `when` / `do_` / `passive` grouping

[Recipe Anatomy](anatomy.md) describes optional cosmetic grouping. Open: do we keep these as recommended convention, or drop them to keep one canonical shape? The editor will produce flat lists either way; the question is whether hand-written recipes benefit from the visual structure.

### Versioning

Recipes will outlive atom-catalog changes. If we rename `Cooldown` to `Tick` next year, what happens to old recipes? Options:

1. Hard-break — old recipes fail to load with a clear "rename your atom" error.
2. Compat shim — keep an alias `Cooldown = Tick` for one release, then remove.
3. Recipe schema version — recipes declare `schema = 2`; the loader applies migrations.

We don't need this yet. We will eventually.

## Distribution

### Mod folder discovery

Right now recipes load from `assets/towers/` only. Open:

- A separate user-mods folder (e.g. `~/.config/bevy-td-sandbox/mods/towers/`) that survives game updates?
- Per-mod-pack folders (`assets/mods/<pack-name>/towers/...`) so a downloaded pack stays bundled?
- An in-game mod manager that subscribes to a remote registry?

Probably yes to (b) and (c), eventually. (a) for desktop is straightforward; for WASM/browser it's harder (no filesystem).

### Sharing format

A single recipe file is already shareable — paste the Lua text in chat. Open: do we want a *bundle* format for distributing multiple recipes plus a manifest plus optional assets (icons, sound effects)? Probably yes, but it's blocked on the broader mod pack story.

### Recipe browsability

If players are sharing recipes, there's a discoverability problem: how do new players find good ones? In-game gallery? External vending-machine site? Steam Workshop, eventually? All open.

## Editor

### Editor IR

Does the in-game editor render directly from the recipe data, or does it maintain an intermediate representation while editing? An IR is more flexible (supports undo, partial validity, "what if I add this") but more code. A direct-render editor is simpler but coarser.

### Editor as a player tool vs. editor as a developer tool

Same UI, two audiences. Players want guardrails ("hard to build something broken" — a brainstorm principle). Developers want freedom to compose anything for testing. Open: one mode? Two modes? A "dev panel" toggle?

---

If something on this page becomes settled, it'll move to its proper home in the manual and be removed from here. If something *not* on this page comes up that you think should be, the [project's GitHub issues](https://github.com/n8behavior/bevy-td-sandbox/issues) is the place.
