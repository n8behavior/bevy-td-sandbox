# Tower Modding

A **tower** is an entity in the game world. It sits on the map, watches what's happening, and does things — fires projectiles, slows enemies, grants scrap, links to its neighbors. Every tower is built out of small composable pieces called **atoms**.

A **recipe** is a Lua file that declares one tower: its identity (name, cost, color, hotkey) and the list of atoms that give it behavior. The game loads recipe files from `assets/towers/` at startup and registers each one as a tower the player can build.

## The mental model

Look at any recipe and you'll see two parts:

1. **Identity** — what the tower is called, what it costs, what hotkey selects it, what color it's drawn in.
2. **Atoms** — what the tower *does*. Conditions that have to be true (`Cooldown(1.0)`, `SingleTarget("closest")`, `Range(80)`) and actions that happen when they are (`Projectile { speed = 200 }`, `DirectDamage(10)`). Plus passive components like `Health()` and `BlocksNav()` that affect how the tower exists in the world rather than what it does in combat.

That's it. There's no scripting, no event handlers, no main loop you write. The game's engine handles *when* and *how*; the recipe declares *what*.

## What to read next

- **[Quick Start](quickstart.md)** — a complete tower from blank file to in-game in under five minutes.
- **[Recipe Anatomy](anatomy.md)** — every part of the `Tower { ... }` constructor explained in detail.
- **[Lua Conventions](lua.md)** — the Lua subset recipes run in, and the calling conventions atoms use.
- **[Atom Reference](atoms.md)** — every atom in the catalog, what it does, what it needs.
- **[Examples](examples.md)** — every built-in tower as a Lua recipe, plus stretch designs.

If you've never written Lua before, Quick Start gets you running first; the Lua page explains the language as you need it.
