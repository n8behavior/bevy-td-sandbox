# Tower Modding

A **tower** is an entity in the game world. It sits on the map, watches what's happening, and does things — fires projectiles, slows enemies, grants scrap, links to its neighbors. Every tower is built out of small composable pieces called **atoms**.

A **recipe** is a Lua file that declares one tower: its identity (name, cost, color) and the list of atoms that give it behavior. The game loads recipe files from `assets/towers/` at startup and registers each one as a tower the player can build.

## The mental model

Look at any recipe and you'll see two parts:

1. **Identity** — name, cost, color, and other build-menu metadata. (Hotkeys are assigned in the in-game UI, not in the recipe — see [Recipe Anatomy](anatomy.md).)
2. **Body** — one or more **deliverer blocks** (`Projectile { ... }`, `Aura { ... }`, etc.) describing what the tower does in combat, plus **passive atoms** (`Health()`, `BlocksNav()`, `ScrapCollector(30)`) that affect the tower itself.

Each deliverer block is self-contained: it declares its own cooldown, range, target mode, damage, and effects. There's no scripting, no event handlers, no main loop — the engine handles *when* and *how*; the recipe declares *what*.

## What to read next

- **[Quick Start](quickstart.md)** — a complete tower from blank file to in-game in under five minutes.
- **[Recipe Anatomy](anatomy.md)** — every part of the `Tower { ... }` constructor explained in detail.
- **[Lua Conventions](lua.md)** — the Lua subset recipes run in, and the calling conventions atoms use.
- **[Atom Reference](atoms.md)** — every atom in the catalog, what it does, what it needs.
- **[Examples](examples.md)** — every built-in tower as a Lua recipe, plus stretch designs.

If you've never written Lua before, Quick Start gets you running first; the Lua page explains the language as you need it.
