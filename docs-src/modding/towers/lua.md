# Lua Conventions

Recipes are Lua files. You don't need to be a Lua programmer to write one — most recipes are a `Tower` constructor with a couple of deliverer blocks inside — but it helps to know the small handful of syntactic shortcuts the recipes use.

This page covers:

- the calling forms recipes use (deliverer blocks and passive atoms)
- the Lua stdlib subset available (and what's blocked)
- which Lua version the engine runs

If you've used Lua before in another modding context, none of this will surprise you. If you haven't, it'll take ten minutes.

## Calling forms

Three patterns show up across every recipe:

```lua
-- 1. Deliverer block — table of named properties
Projectile {
  cooldown = 1.0,
  target = "closest",
  range = 80,
  damage = 10,
}

-- 2. Passive with one positional argument
ScrapCollector(30)

-- 3. Passive with no arguments (parens REQUIRED)
Health()
BlocksNav()
```

These are all just Lua function calls. `Projectile { ... }` is sugar for `Projectile({ ... })`; `ScrapCollector "name"` works for string args; `Atom()` is the bare-call form. Pick whichever reads cleanest.

| Form | When to use it |
| --- | --- |
| `Atom { name = value, ... }` | Deliverer blocks and any passive that takes multiple/named properties. |
| `Atom(value)` | Passive with one positional value: `ScrapCollector(30)`, `PassiveIncome(3)`. |
| `Atom "string"` | Passive with one string argument. |
| `Atom()` | Bare passives. **The parens are required.** |

### Why nullary atoms always need parens

Lua doesn't elide parens on zero-argument calls. The expression `Health` (no parens) refers to the constructor itself — a function value — not a call. The runtime catches this at recipe-load time, but it's the one place the sugar doesn't help you.

Always write `Health()` and `BlocksNav()`, not `Health` and `BlocksNav`.

## What you can use

Recipes run in a sandboxed Lua 5.4 environment. The full language is available except for the parts that touch the outside world.

| Module | Available | Notes |
| --- | --- | --- |
| `string` | yes | All standard string operations. |
| `math` | yes | All math functions including `math.random`. |
| `table` | yes | All table operations: `table.insert`, `table.unpack`, etc. |
| `os`, `io`, `package`, `require`, `debug`, `dofile`, `loadfile`, raw `load` | **blocked** | Sandbox escape vectors. |

If a recipe tries to use any blocked module, it fails at load time with a clear error.

You can't `require` other Lua files — the sandbox is per-recipe. Define helpers inline at the top of a recipe, or return a list of multiple Towers from one file (see [Advanced Patterns](advanced.md)).

## Lua version

The engine embeds **Lua 5.4** via [`mlua`](https://crates.io/crates/mlua). The web (WASM) build uses the same version with `mlua`'s WASI backend — recipes run identically on browser and native.

## Lua syntax cheatsheet

```lua
-- variable
local x = 5

-- function
local function double(n) return n * 2 end

-- if / else
if x > 3 then print("big") else print("small") end

-- for loop
for i = 1, 10 do print(i) end

-- table literal (mixed indexed and named — same shape as a recipe body)
local t = {
  name = "thing",
  1, 2, 3,
}

-- string concat and format
local greeting = "hello, " .. name
local msg = string.format("%d shots, %.1f sec cooldown", 5, 1.5)
```

For more, the [Lua 5.4 reference manual](https://www.lua.org/manual/5.4/) is short and well-organized.
