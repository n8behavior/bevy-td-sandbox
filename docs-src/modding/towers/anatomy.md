# Recipe Anatomy

A recipe is one Lua file that returns one `Tower` value:

```lua
return Tower {
  -- identity (named fields)
  name = "ScrapGun", cost = 50, color = "#E5CC4D",

  -- body (top-level atoms)
  Projectile {
    cooldown = 1.0,
    target = "closest",
    range = 80,
    speed = 200,
    damage = 10,
  },

  ScrapCollector(30),
  Health(),
  BlocksNav(),
}
```

The `Tower` constructor takes one Lua table. Named fields are identity; everything else is a top-level atom in the body.

## Identity fields

| Field      | Type         | Required | Meaning                                                                    |
| ---------- | ------------ | -------- | -------------------------------------------------------------------------- |
| `name`     | string       | yes      | Display name in the build menu and tooltips. Unique across loaded recipes. |
| `cost`     | integer      | yes      | Scrap cost to place the tower.                                             |
| `color`    | string (hex) | yes      | Primary color. Used for sprite and build-menu swatch.                      |
| `ui_color` | string (hex) | no       | Brighter variant for the build-menu UI; defaults to lightened `color`.     |
| `label`    | string       | no       | One-line tooltip subtitle.                                                 |
| `icon`     | string       | no       | Path to an icon asset. Defaults to a generated swatch from `color`.        |

> **Hotkeys are not a recipe field.** Build-menu hotkeys are assigned through the in-game UI so players can bind or rebind them without editing files, and so modders don't have to coordinate which numbers their packs claim.

## Body — top-level atoms

The non-named entries in the `Tower` table are the tower's behavior. Two flavors:

**Deliverers** are combat units. One block per "thing the tower does." Each is a named call (`Projectile`, `Beam`, `Aura`, `Hitscan`, `Trap`, `Summon`, `NetworkLink`) followed by a table of properties:

```lua
Projectile {
  cooldown = 1.0,     -- when to fire (omit to run every tick)
  target = "closest", -- who to fire at
  range = 80,         -- how far to look
  speed = 200,        -- how the hit gets there
  damage = 10,        -- what the hit does
}
```

A tower can have multiple deliverer blocks — they run independently with their own cooldowns and ranges:

```lua
return Tower {
  name = "Sentry", cost = 200, color = "#0088CC",

  Projectile { cooldown = 1.0, target = "closest", range = 80, speed = 200, damage = 8 },
  Aura { range = 60, slow = { factor = 0.6, duration = 0.5 } },

  Health(), BlocksNav(),
}
```

**Passives** affect the tower itself rather than firing on enemies. Flat calls or small blocks: `Health()`, `BlocksNav()`, `ScrapCollector(30)`, `NetworkAmplify { ... }`.

The full list of deliverers, properties, and passives lives in [Atom Reference](atoms.md).

## What's required

A combat tower needs at least one deliverer block, and that deliverer almost always needs `target` (so it knows what to hit) and at least one effect property like `damage`. Passives are optional.

A non-combat tower (e.g. `Tower { name = "SolarArray", cost = 50, color = "#FFE040", PassiveIncome(3) }`) has no deliverer at all.

## Comments

```lua
-- single-line comment

--[[
  block comment
  spanning multiple lines
]]
```

## Hot reload

In dev builds, saving a recipe file re-registers the tower without restarting. Towers already placed keep their old behavior; freshly built ones get the new recipe. See [Open Questions](open-questions.md).
