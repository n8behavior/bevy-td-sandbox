# Quick Start — Your First Tower

## 1. Create the recipe file

Tower recipes live in `assets/towers/`. Create a new file:

```text
assets/towers/sparkler.lua
```

## 2. Write the recipe

```lua
return Tower {
  name = "Sparkler", cost = 40, color = "#FFB000",

  Projectile {
    cooldown = 0.6,
    target = "closest",
    range = 70,
    speed = 250,
    damage = 6,
  },

  Health(),
  BlocksNav(),
}
```

That's a complete tower. It fires at the closest enemy within 70 units every 0.6 seconds, dealing 6 damage per shot.

## 3. Run the game

```bash
just run
```

Sparkler should appear in the build menu.

## 4. What just happened

The recipe returns one `Tower` value. The named fields (`name`, `cost`, `color`) are identity — how the tower appears in the build menu. Everything else is a **top-level atom**:

- **`Projectile { ... }`** — the combat unit. Its named properties (`cooldown`, `target`, `range`, `speed`, `damage`) declare *when* it fires, *what* it targets, and *what happens*.
- **`Health()`**, **`BlocksNav()`** — passives that govern how the tower exists in the world (destructible, blocks enemy pathing).

Want it faster? Drop `cooldown` to `0.3`. Want an aura that slows instead? Replace the `Projectile` block:

```lua
Aura {
  range = 70,
  slow = { factor = 0.5, duration = 0.5 },
}
```

See [Recipe Anatomy](anatomy.md) for the full breakdown and [Atom Reference](atoms.md) for what each deliverer accepts.
