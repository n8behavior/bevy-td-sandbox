# Lua Conventions

Recipes are Lua files. You don't need to be a Lua programmer to write one — most recipes are flat lists of atom calls — but it helps to know the small handful of syntactic shortcuts the recipes use.

This page is the language reference. It covers:

- the four ways to call an atom constructor (and which to use when)
- the Lua stdlib subset available in recipes (and what's blocked)
- which Lua version the engine runs

If you've used Lua before in another game's modding context, none of this will surprise you. If you haven't, read on — it'll take ten minutes.

## Calling atom constructors

Lua has a few sugar forms for function calls. Recipes use all four, depending on what reads best:

```lua
Cooldown(1.0)             -- ordinary call: one positional argument
SingleTarget "closest"    -- string call: one string argument, no parens needed
Projectile { speed = 200 }  -- table call: a single table arg, no parens needed
Health()                  -- empty call: no args (parens REQUIRED)
```

These four forms are all just function calls. The compiler treats them identically. The choice is purely about readability:

| Form | When to use it |
| --- | --- |
| `Atom(value)` | One positional number argument: `Cooldown(1.0)`, `Range(80)`, `DirectDamage(10)`. |
| `Atom "string"` | One positional string argument: `SingleTarget "closest"`, `Color "blue"`. |
| `Atom { name = value, ... }` | Two or more arguments, or anything where named args read better: `Projectile { speed = 200, trail = true }`. Also fine for one named arg if you prefer the clarity. |
| `Atom()` | No arguments. Bare markers like `Health`, `BlocksNav`, `ContinuousTick`. **The parens are required.** |

### Why nullary atoms always need parens

Lua doesn't elide parens on zero-argument calls. The expression `Health` (no parens) refers to the atom *constructor itself* — a function value — not a call. The runtime detects this mistake at recipe-load time and reports it, but it's the one place the sugar doesn't help you.

Always write `Health()` and `BlocksNav()`, not `Health` and `BlocksNav`.

## What you can use

Recipes run in a sandboxed Lua 5.4 environment. The full Lua language is available except for the parts that touch the outside world.

### Available stdlib

| Module | Available | Notes |
| --- | --- | --- |
| `string` | yes | All standard string operations: concatenation, formatting, pattern matching. |
| `math` | yes | All math functions including `math.random` (deterministic per-recipe-load). |
| `table` | yes | All table operations: `table.insert`, `table.concat`, etc. |

### Blocked stdlib

| Module | Blocked because |
| --- | --- |
| `os` | Filesystem access, environment, time-of-day reads. |
| `io` | File reading and writing. |
| `package` / `require` | Loading other Lua files. |
| `debug` | Introspection tools, also a sandbox-escape vector. |
| `dofile`, `loadfile`, raw `load` | Arbitrary code loading. |

If a recipe tries to use any blocked module, it fails at load time with a clear error pointing at the offending line.

### What about modules / shared helpers?

You can't `require` other Lua files. The sandbox is per-recipe. If you want to share helper code between recipes, the pattern is to define helpers in-line at the top of a recipe, or to use a recipe-library approach where one Lua file returns multiple Towers (see [Advanced Patterns](advanced.md)).

## Lua version and runtime

The engine embeds **Lua 5.4** via the [`mlua`](https://crates.io/crates/mlua) Rust crate. That's the same Lua you find in modern Lua-modded games. The web (WASM) build uses the same version with `mlua`'s WASI backend.

If you write a recipe that targets `Lua 5.4` features (like integer-vs-float distinction or the `goto` statement), it will run identically on both native and web builds.

## Lua syntax cheatsheet

A few Lua basics in case you've never seen the language:

```lua
-- variable assignment
local x = 5

-- function definition
local function double(n)
    return n * 2
end

-- if / else
if x > 3 then
    print("big")
else
    print("small")
end

-- for loop
for i = 1, 10 do
    print(i)
end

-- table literal (mixed indexed and named, just like a recipe)
local t = {
    name = "thing",
    1, 2, 3,
}

-- string concatenation: ..
local greeting = "hello, " .. name

-- string formatting (printf-style)
local msg = string.format("%d shots, %.1f sec cooldown", 5, 1.5)
```

That's enough Lua to write any recipe in this manual. For more, the official [Lua 5.4 reference manual](https://www.lua.org/manual/5.4/) is short and well-organized.
