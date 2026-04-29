# Examples

Recipes for the six built-in towers plus a few stretch designs. Drop any of these into `assets/towers/<name>.lua`.

---

### ScrapGun — basic projectile turret

```lua
return Tower {
  name = "ScrapGun", cost = 50, color = "#E5CC4D",

  Projectile {
    cooldown = 1.0,
    target = "closest",
    range = 80,
    speed = 200,
    aim_precision = 0.15,
    damage = 10,
  },

  ScrapCollector(30),
  Health(),
  BlocksNav(),
}
```

The shape every turret follows: one `Projectile` block plus structural passives.

### Explosive — splash damage

```lua
return Tower {
  name = "Explosive", cost = 125, color = "#E07B00",

  Projectile {
    cooldown = 3.3, target = "closest", range = 100,
    speed = 200, aim_precision = 0.15,
    damage = 25,
    splash = { radius = 70, damage = 25 },
  },

  ScrapCollector(30), Health(), BlocksNav(),
}
```

`splash` is a sub-table on `Projectile`. The deliverer handles the inner area-of-effect; you don't declare a separate inner acquirer.

### ChainLightning — chain hops

```lua
return Tower {
  name = "ChainLightning", cost = 125, color = "#4FA1E0",

  Hitscan {
    cooldown = 2.0, target = "closest", range = 90,
    damage = 20,
    chain = { arc_range = 60, hop_limit = math.huge, damage_falloff = 0.7 },
  },

  ScrapCollector(30), Health(), BlocksNav(),
}
```

`chain` lives on `Hitscan` as a sub-table. Per-hop damage falloff is part of the chain config.

### TarPit — slow aura

```lua
return Tower {
  name = "TarPit", cost = 75, color = "#5C3A0F",

  Aura {
    range = 70,
    slow = { factor = 0.4, duration = 0.5 },
    range_falloff = "linear",
  },

  ScrapCollector(30),
}
```

No `cooldown` — auras run every tick. No `Health()` / `BlocksNav()` — the tar pit is a passable field.

### HeavySniper — charged sniper

```lua
return Tower {
  name = "HeavySniper", cost = 250, color = "#7090A0",

  Projectile {
    cooldown = 3.0, target = "furthest-along", range = 200,
    lock_on = 1.5,
    speed = 2000,
    damage = 80,
  },

  Health(), BlocksNav(),
}
```

`lock_on = 1.5` requires a 1.5-second continuous lock before firing. `cooldown = 3.0` independently caps fire rate.

### MineTower — trap with sub-recipe

```lua
return Tower {
  name = "MineTower", cost = 200, color = "#806040",

  Trap {
    cooldown = 4.0, range = 120,
    placement = "random", count = 3, lifetime = 60,
    explosion = {
      trigger = "enemy_step",
      damage = 50,
      splash = { radius = 60, damage = 40 },
    },
  },

  Health(), BlocksNav(),
}
```

Sub-recipes are nested tables. The mine entity has its own trigger and payloads inside the `explosion` block.

### TollGate — event-driven, no acquirer

```lua
return Tower {
  name = "TollGate", cost = 50, color = "#806000",

  Hitscan {
    trigger = "enemy_pass",
    income = 5,
  },

  BlocksNav(),
}
```

`trigger = "enemy_pass"` fires only when an enemy crosses. `income` grants scrap instead of damage. No `cooldown`, no `target`, no `range` — the trigger event provides everything.

### MatrixBeam — networked link weapon

```lua
return Tower {
  name = "MatrixBeam", cost = 150, color = "#00CCCC",

  NetworkLink {
    range = 150,
    color = "#00FFFF",
    damage_on_cross = 25,
  },

  Health(), BlocksNav(),
}
```

A single MatrixBeam tower does nothing — the *link* between two of them is the weapon.

### SolarArray — passive only

```lua
return Tower {
  name = "SolarArray", cost = 100, color = "#FFE040",

  PassiveIncome(3),
}
```

No deliverer at all. Just a passive scrap generator.

---

## Multi-deliverer towers

Stack deliverer blocks for hybrid behavior:

```lua
return Tower {
  name = "Sentry", cost = 200, color = "#0088CC",

  Projectile { cooldown = 1.0, target = "closest", range = 80, speed = 200, damage = 8 },
  Aura { range = 60, slow = { factor = 0.6, duration = 0.5 } },

  Health(), BlocksNav(),
}
```

Each block has its own cooldown and range — they don't have to agree.
