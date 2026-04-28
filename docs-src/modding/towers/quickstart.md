# Quick Start — Your First Tower

This page walks you through writing a complete, working tower from a blank file.

## 1. Create the recipe file

Tower recipes live in `assets/towers/`. Create a new file there:

```text
assets/towers/sparkler.lua
```

The filename (minus `.lua`) is just for your own organization — the in-game name comes from the `name` field inside the recipe.

## 2. Write the recipe

Paste this into `sparkler.lua`:

```lua
return Tower {
    name  = "Sparkler",
    cost  = 40,
    color = "#FFB000",

    Cooldown(0.6),
    SingleTarget "closest",
    Range(70),
    Projectile { speed = 250 },
    DirectDamage(6),

    Health(),
    BlocksNav(),
}
```

That's a complete tower. It fires at the closest enemy within 70 units every 0.6 seconds, dealing 6 damage per shot.

## 3. Run the game

```bash
just run
```

Open the build menu (or press `1`). The Sparkler should appear alongside the built-in towers, ready to place.

## 4. What just happened

Walking the recipe top-to-bottom:

| Line | Meaning |
| --- | --- |
| `return Tower { ... }` | Every recipe file returns one `Tower` value. |
| `name`, `cost`, `color` | Identity — how the tower appears in the build menu. (Hotkeys are bound in the in-game UI, not in the recipe.) |
| `Cooldown(0.6)` | Wait 0.6s between shots. |
| `SingleTarget "closest"` | Pick one enemy — the closest one. |
| `Range(70)` | The acquirer's radius. |
| `Projectile { speed = 250 }` | When triggered, fly a projectile to the target at speed 250. |
| `DirectDamage(6)` | On hit, deal 6 damage. |
| `Health()` | The tower has hit points and can be destroyed. |
| `BlocksNav()` | The tower blocks enemy pathing — they have to go around it. |

If you want the tower to fire faster, change `Cooldown(0.6)` to `Cooldown(0.3)`. If you want it to be an aura that hits everything in range every tick instead of a turret, replace `SingleTarget "closest"` and `Projectile { ... }` with `AllInRange()` and `Aura()`. The atom catalog is small enough to skim — see [Atom Reference](atoms.md).

## Next steps

- **[Recipe Anatomy](anatomy.md)** for the full breakdown of every part of `Tower { ... }`.
- **[Examples](examples.md)** to see every built-in tower written as a recipe — handy as a starting point for variations.
- **[Lua Conventions](lua.md)** if the `f "x"` and `f { ... }` syntax above looked unfamiliar.
