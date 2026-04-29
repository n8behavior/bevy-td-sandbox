# Validation & Errors

Recipe loading happens in two passes:

| Phase | When | What it checks |
| --- | --- | --- |
| **Parse** | Lua file load | Syntax, unknown atom or property names, sandbox violations |
| **Load** | After parse | Combat tower has a deliverer; deliverer has at least one effect |

Errors print to the game's log and (in dev builds) to a console panel, with file name and line number.

## Parse-time errors

### Syntax error

```text
sparkler.lua:7: '}' expected (to close '{' at line 1) near 'Projectile'
```

Standard Lua errors — usually a missing comma or closing brace.

### Unknown atom

```text
sparkler.lua:5: unknown atom 'Projectil' — did you mean 'Projectile'?
```

Typo on a deliverer or passive name.

### Unknown property

```text
sparkler.lua:7: unknown property 'speeed' on Projectile — did you mean 'speed'?
```

Typo inside a deliverer block. Each deliverer has a fixed property set; see [Atom Reference](atoms.md).

### Sandbox violation

```text
sparkler.lua:3: attempt to use blocked module 'os'
```

See [Lua Conventions](lua.md) for the allow/block list.

### Bare nullary atom

```text
sparkler.lua:11: 'Health' is an atom constructor, not an atom — try Health()
```

Lua doesn't elide parens on zero-argument calls.

## Load-time issues

### Combat tower with no rate limit (warning)

```text
sparkler.lua: tower 'Sparkler' has Projectile + damage but no cooldown
                 — this will fire every frame
                 — add cooldown = N to Projectile if that's not what you want
```

A warning, not an error. Auras and persistent fields are the legitimate "no cooldown" case; for damage-dealing turrets it's almost never intended.

### Duplicate tower name

```text
load: 'Sparkler' from sparkler.lua collides with 'Sparkler' from another_file.lua
                 — second registration is ignored
```

Tower names must be unique. First to load wins.

## Debugging a tower that loads but doesn't fire

1. **Did you forget `target`?** Combat deliverers need to know what to fire at.
2. **Is `range` set?** A deliverer without `range` defaults to 0 and finds no targets.
3. **Does `target` mode match the situation?** `target = "highest-hp"` idles when no enemies are in range — by design.
4. **Is there an effect property?** With no `damage` / `burn` / `slow` / `income`, the deliverer fires but nothing happens.
5. **Check the log.** `RUST_LOG=bevy_td_sandbox::tower=debug just run` shows tower lifecycle traces.
