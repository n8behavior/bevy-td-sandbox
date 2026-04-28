# Advanced Patterns

Recipes are normal Lua files. That means anything Lua can do, your recipes can do too. This page shows three patterns that take advantage of Lua-the-language to write recipes that would be tedious or impossible to express in a flat data format.

If you're new to Lua and writing your first few towers, skip this page. The flat-list shape from [Recipe Anatomy](anatomy.md) handles 95% of cases.

## 1. Parameterized recipes

A single Lua file can return *multiple* towers by computing them from parameters. Useful when you want a family of related towers — different cooldowns, different damage values, different colors — without copy-pasting the same recipe ten times.

```lua
-- assets/towers/snipers.lua

local function make_sniper(tier, cost, range, damage)
    return Tower {
        name  = "Sniper-" .. tier,
        cost  = cost,
        color = "#5C7A8C",

        Cooldown(5.0),
        SingleTarget "highest-hp",
        Range(range),
        Projectile { speed = 2000 },
        AimPrecision(0.05),
        DirectDamage(damage),

        Health(),
        BlocksNav(),
    }
end

return {
    make_sniper("I",   100, 120, 30),
    make_sniper("II",  200, 160, 60),
    make_sniper("III", 400, 200, 120),
}
```

A recipe file can return either:
- **One `Tower`** (the typical case, every example so far).
- **A list of `Tower`s** (the loader iterates and registers each).

Both forms are valid; the loader detects which one you returned.

## 2. Recipe libraries — sharing helpers

Sometimes you want the same atom-bundle across multiple recipes — a "structural defaults" set, a "tank-killer payload combo," a custom color palette. Define helpers at the top of the file and reuse them.

```lua
-- assets/towers/standard_pack.lua

local function structural()
    return {
        Health(),
        BlocksNav(),
        ScrapCollector(30),
    }
end

local function tank_killer_payload()
    return {
        DirectDamage(50),
        Burn { dps = 5, duration = 3 },
        Vulnerability { multiplier = 1.5, duration = 4 },
    }
end

return {
    Tower {
        name = "TankBreaker", cost = 200, color = "#FF4040",

        Cooldown(2.0),
        SingleTarget "highest-hp",
        Range(120),
        Projectile { speed = 600 },
        AimPrecision(0.1),

        table.unpack(tank_killer_payload()),
        table.unpack(structural()),
    },

    Tower {
        name = "BurnLance", cost = 150, color = "#FF8000",

        Cooldown(0.5),
        SingleTarget "closest",
        Range(100),
        Beam(),

        table.unpack(tank_killer_payload()),
        table.unpack(structural()),
    },
}
```

`table.unpack` flattens a returned atom-list into the surrounding `Tower { ... }` table. (Lua 5.1 calls this `unpack`; `table.unpack` is the Lua 5.4 name. Both work.)

You can't `require` *another* file from a recipe — the sandbox is per-recipe — but within one file you can define as many helpers as you want.

## 3. Programmatic identity

Names, costs, and colors can be computed too. Useful when you're generating a series and want labeling and pricing to follow a rule.

```lua
local TIERS = { "Bronze", "Silver", "Gold", "Platinum" }
local BASE_COST = 75
local BASE_DAMAGE = 8

local function tower_for_tier(i)
    local tier = TIERS[i]
    return Tower {
        name  = "Spark-" .. tier,
        cost  = BASE_COST * i,
        color = ({ "#CD7F32", "#C0C0C0", "#FFD700", "#E5E4E2" })[i],

        Cooldown(1.0),
        SingleTarget "closest",
        Range(80 + i * 20),
        Projectile { speed = 200 },
        DirectDamage(BASE_DAMAGE * i),

        Health(),
        BlocksNav(),
    }
end

local towers = {}
for i = 1, #TIERS do
    table.insert(towers, tower_for_tier(i))
end
return towers
```

This produces four towers (`Spark-Bronze`, `Spark-Silver`, `Spark-Gold`, `Spark-Platinum`) with progressively more range, more damage, and higher cost.

## Tradeoffs of the advanced patterns

Computed recipes are powerful, but they have one cost worth knowing about: **the in-game editor cannot round-trip them.**

The editor reads a Lua file, lets you edit a tower visually, and writes the result back. If your recipe is a flat `Tower { ... }` table, the editor reads it, lets you tweak any atom, and re-emits an equivalent flat recipe. If your recipe is a function call (`make_sniper("I", 100, ...)`) or uses `table.unpack`, the editor opens the *result* of running the recipe — a flat snapshot — but it can't preserve the function structure when saving. Save will overwrite the file with the flattened version.

If you want a recipe to stay editor-editable, keep it flat. If you want a recipe family that's easy to maintain by hand-editing the source, use the patterns above and accept the editor as read-only for that file. See [Editor Compatibility](editor.md) for more.
