# Recipe Anatomy

Every recipe file returns one `Tower` value. The `Tower` constructor takes a single Lua table that mixes **named fields** (the tower's identity) with **indexed entries** (the atoms that give it behavior).

```lua
return Tower {
    -- ── identity (named fields) ──
    name  = "ScrapGun",
    cost  = 50,
    color = "#E5CC4D",
    key   = "1",

    -- ── atoms (indexed entries) ──
    Cooldown(1.0),
    SingleTarget "closest",
    Range(80),
    Projectile { speed = 200 },
    AimPrecision(0.15),
    DirectDamage(10),
    ScrapCollector(30),
    Health(),
    BlocksNav(),
}
```

The order of identity fields doesn't matter. The order of atoms doesn't normally matter either — the engine's runtime pipeline figures out when each atom runs based on its role, not its position in the list. (See [Lua Conventions](lua.md) for why this works syntactically.)

## Identity fields

These configure *how the tower presents itself* — the build-menu entry, the placement preview, the debug name. They don't affect combat behavior.

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `name` | string | yes | Display name in the build menu and tooltips. Must be unique across all loaded recipes. |
| `cost` | integer | yes | Scrap cost to place the tower. |
| `color` | string (hex) | yes | Primary color. `"#E5CC4D"` style. Used for the sprite and the build-menu swatch. |
| `key` | string | no | Hotkey to select this tower in the build menu — `"1"` through `"9"`, or `"0"`. If two towers claim the same key, load order wins; the loser becomes mouse-only. |
| `ui_color` | string (hex) | no | Brighter variant for the build-menu UI; defaults to a lightened `color`. |
| `label` | string | no | One-line tooltip subtitle (e.g., `"chains lightning between enemies"`). |
| `icon` | string | no | Path to an icon asset. Defaults to a generated swatch from `color`. |

## Atom list

Anything in the table that *isn't* a named field is treated as an atom. Each atom is a value produced by calling an atom constructor:

```lua
Cooldown(1.0)             -- one number → numeric atom
SingleTarget "closest"    -- one string → enum-like atom
Projectile { speed = 200 }  -- table of named args → multi-param atom
Health()                  -- no args → bare-marker atom (parens required, see Lua Conventions)
BlocksNav()
```

Every atom either:

1. **Fills a role** the tower needs — `Cooldown` fills the `Trigger` role; `Projectile` fills `Deliverer`; `DirectDamage` fills `Payload`; `SingleTarget` fills `Acquirer`; `Range` fills `RangeProvider`; `Health` and `BlocksNav` are structural.
2. **Modifies** another atom — `AimPrecision` modifies `Projectile`; `LockOn` modifies an `Acquirer`; `Splash` modifies a `Deliverer`'s on-hit behavior.

The full catalog is in [Atom Reference](atoms.md).

## What you must include

A combat tower needs, at minimum:

- one `Trigger`-role atom (when to fire)
- one `Acquirer`-role atom (who/where to fire at)
- one `Deliverer`-role atom (how the hit gets there)
- at least one `Payload`-role atom (what happens on hit)

If you leave one out, the recipe loads but the engine reports the missing role at startup. See [Validation & Errors](validation.md).

A non-combat tower (a passive scrap collector, an income beacon) might skip some of these. See `SolarArray` in [Examples](examples.md) for a tower that's purely passive.

## Optional grouping (`when` / `do` / `passive`)

For longer recipes, you can visually group atoms by what they're doing:

```lua
return Tower {
    name = "ScrapGun", cost = 50, color = "#E5CC4D", key = "1",

    when {
        Cooldown(1.0),
        SingleTarget "closest",
        Range(80),
    },
    do_ {
        Projectile { speed = 200 },
        AimPrecision(0.15),
        DirectDamage(10),
    },
    passive {
        ScrapCollector(30),
        Health(),
        BlocksNav(),
    },
}
```

These blocks are **purely cosmetic** — they don't change loading or runtime behavior. Atoms inside them are flattened into the same list as if you'd written them all top-level. Use them when a recipe has more than ~6–8 atoms and the structure helps you read it; skip them otherwise.

> *Note: `do` is a reserved keyword in Lua, so the action block uses `do_`. The `when` and `passive` block names are normal identifiers.*

## Comments

Lua's comment syntax:

```lua
-- single-line comment

--[[
  block comment
  spanning multiple lines
]]
```

Recipes are read more often than they're written — comment freely.

## A word on hot reload

If hot reload is enabled (it is, by default, in dev builds), saving a recipe file re-registers the tower without restarting the game. Towers already placed on the map keep their old behavior; freshly built ones get the new recipe. See [Open Questions](open-questions.md) for the current limitations.
