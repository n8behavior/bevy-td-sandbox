# Validation & Errors

Recipe validation happens in three phases. Each phase produces a different kind of error message, and knowing which phase failed makes debugging much faster.

| Phase | When it runs | What it checks |
| --- | --- | --- |
| **Parse** | When the Lua file is loaded | Syntax, undefined names, sandbox violations |
| **Load** | After parse, before the tower is registered | Role compatibility — does every atom's `needs` get filled? |
| **Place** | When the player tries to place the tower | World-context viability — out of scope for the recipe itself |

The recipe loader prints errors to the game's log and (in dev builds) to a console panel. Each error includes the file name and line number where possible.

## Parse-time errors

These are the same kinds of errors you'd get from any Lua program.

### Syntax error

```text
sparkler.lua:7: '}' expected (to close '{' at line 1) near 'BlocksNav'
```

You forgot a comma between two atoms, or a closing brace. Lua errors are usually pointed and easy to act on.

### Unknown atom

```text
sparkler.lua:6: unknown atom 'Cooldwon' — did you mean 'Cooldown'?
```

Typo, or you reached for an atom that doesn't exist. Check [Atom Reference](atoms.md). The loader does a string-similarity check and suggests the closest match.

### Sandbox violation

```text
sparkler.lua:3: attempt to use blocked module 'os'
```

Recipes can't touch `os`, `io`, `package`, `require`, `debug`, `dofile`, `loadfile`, or raw `load`. See [Lua Conventions](lua.md) for the full list of what's allowed.

### Bare nullary atom

```text
sparkler.lua:9: 'Health' is an atom constructor, not an atom — try Health()
```

You wrote `Health` instead of `Health()`. Lua doesn't elide parens on zero-argument calls.

## Load-time errors

The recipe parsed, but the atoms don't add up to a working tower.

### Missing required role

```text
sparkler.lua: tower 'Sparkler' has Projectile but no Acquirer
                 — add SingleTarget(mode) so the projectile knows where to fire
```

`Projectile` is a `Deliverer` that needs an `Acquirer` to provide a target. The error names the missing role and suggests common atoms that fill it. Most "tower won't load" errors are this shape.

### Damage tower with no rate limit (warning, not an error)

```text
sparkler.lua: tower 'Sparkler' has Projectile + DirectDamage but no rate-limiting
                 condition — this will fire every frame at ~60 shots/second
                 — add Cooldown(secs) if that's not what you want
```

Triggers are optional — towers without one run every tick. For damage-dealing turrets that's almost never intended. Auras and persistent fields are the legitimate "no trigger" case, so this is a warning rather than a hard error.

### Conflicting atoms

```text
sparkler.lua: tower 'Sparkler' has two Triggers — Cooldown and OnWorldEvent
                 — only one Trigger per tower
```

A tower can only have one Trigger. If you want fire-on-cooldown-AND-fire-on-event, that's an [open design question](open-questions.md) — for now, pick one.

### Modifier without its target

```text
sparkler.lua: Homing modifies Projectile, but no Projectile is on this tower
```

Modifiers need a sibling atom of a specific kind. Either remove the modifier or add the target.

### Duplicate tower name

```text
load order: 'Sparkler' from sparkler.lua collides with 'Sparkler' from another_file.lua
                 — second registration is ignored
```

Tower names must be unique. The first one to load wins; the rest are skipped (with a warning).

## Place-time issues

These aren't errors per se — the recipe loaded fine. They're "this tower won't actually do anything where I'm putting it" warnings, surfaced as visual feedback during placement rather than text errors:

- A turret with no enemies in range shows a *quiet* range circle.
- A `NetworkBeam` tower placed alone shows a "needs a neighbor" preview indicator.
- An `OnWorldEvent "EnemyPass"` tower not on a path shows a "won't trigger here" hint.

These are runtime / editor concerns, not recipe-validity concerns. The recipe is correct; the placement isn't useful.

## Debugging a recipe that loads but doesn't fire

If your tower appears in the build menu, places successfully, but never fires, walk through this checklist:

1. **Does the trigger ever evaluate true?** Open the tower's debug panel (right-click the tower in dev builds). The `Cooldown` lane shows the timer; the `Acquirer` lane shows whether a target is found.
2. **Is `Range` set?** A turret without `Range` defaults to 0 and finds no targets. A common forget when copy-pasting recipes.
3. **Does the acquirer's mode match the situation?** `SingleTarget "highest-hp"` will idle if no enemies are in range — that's not a bug, that's the acquirer doing its job.
4. **Is there a payload?** The pipeline runs Trigger → Acquirer → Deliverer → *and then nothing*, if no payload atom is present. The deliverer fires; nothing happens to the target.
5. **Check the log.** Recipe load warnings (suspicious-but-not-fatal patterns) are logged at the `warn` level. Run with `RUST_LOG=bevy_td_sandbox::tower=debug just run` to see tower lifecycle traces.

If the tower fires but does *less damage than expected*, check for `RangeFalloff` (payloads scale by distance), `DamageFalloff` (chains lose damage per hop), or `ActivityRamp` (damage scales with charge).
