# Advanced Patterns

Recipes are normal Lua files. Anything Lua can do, your recipes can do too. This page shows three patterns that take advantage of Lua-the-language to write recipes that would be tedious in a flat data format.

If you're new to Lua and writing your first few towers, skip this. The shape from [Recipe Anatomy](anatomy.md) handles 95% of cases.

## 1. Parameterized recipes

A single Lua file can return *multiple* towers by computing them. Useful for families of related towers — different cooldowns, damage values, colors — without copy-paste.

```lua
-- assets/towers/snipers.lua

local function make_sniper(tier, cost, range, damage)
  return Tower {
    name = "Sniper-" .. tier, cost = cost, color = "#5C7A8C",

    Projectile {
      cooldown = 5.0,
      target = "highest-hp",
      range = range,
      speed = 2000,
      aim_precision = 0.05,
      damage = damage,
    },

    Health(), BlocksNav(),
  }
end

return {
  make_sniper("I",   100, 120, 30),
  make_sniper("II",  200, 160, 60),
  make_sniper("III", 400, 200, 120),
}
```

A recipe file can return either one `Tower` or a list of `Tower`s — the loader detects which.

## 2. Recipe libraries — sharing helpers

Reuse the same property bundles across recipes:

```lua
-- assets/towers/standard_pack.lua

local function structural()
  return { Health(), BlocksNav(), ScrapCollector(30) }
end

local function tank_killer(extra)
  local p = {
    cooldown = 2.0, target = "highest-hp", range = 120,
    speed = 600, aim_precision = 0.1,
    damage = 50,
    burn = { dps = 5, duration = 3 },
    vulnerability = { multiplier = 1.5, duration = 4 },
  }
  for k, v in pairs(extra or {}) do p[k] = v end
  return p
end

return {
  Tower {
    name = "TankBreaker", cost = 200, color = "#FF4040",
    Projectile(tank_killer()),
    table.unpack(structural()),
  },

  Tower {
    name = "BurnLance", cost = 150, color = "#FF8000",
    Beam(tank_killer({ cooldown = 0.5 })),
    table.unpack(structural()),
  },
}
```

`table.unpack` flattens a returned atom-list into the surrounding `Tower { ... }` table. (Lua 5.1 calls it `unpack`; both work in 5.4.)

## 3. Programmatic identity

Names, costs, and colors can be computed:

```lua
local TIERS = { "Bronze", "Silver", "Gold", "Platinum" }
local COLORS = { "#CD7F32", "#C0C0C0", "#FFD700", "#E5E4E2" }

local function spark_for_tier(i)
  local tier = TIERS[i]
  return Tower {
    name = "Spark-" .. tier,
    cost = 75 * i,
    color = COLORS[i],

    Projectile {
      cooldown = 1.0,
      target = "closest",
      range = 80 + i * 20,
      speed = 200,
      damage = 8 * i,
    },

    Health(), BlocksNav(),
  }
end

local towers = {}
for i = 1, #TIERS do
  table.insert(towers, spark_for_tier(i))
end
return towers
```

Produces `Spark-Bronze`, `Spark-Silver`, `Spark-Gold`, `Spark-Platinum` with progressively more range and damage.

## Tradeoffs

Computed recipes can't round-trip through the in-game editor. The editor reads them fine — it runs the recipe and gets the resulting Towers — but it can't save changes back as functions or loops. Editing in the in-game UI overwrites the file with the flattened result, losing your helpers.

If you want a recipe to stay editor-editable, keep it a flat `Tower { ... }`. If you want a recipe family that's clean to maintain by hand, use these patterns and accept the editor as read-only for that file. See [Editor Compatibility](editor.md).
