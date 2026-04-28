# Examples

Every built-in tower as a Lua recipe, plus stretch designs that exercise the wider catalog. Each is annotated with what's interesting about it — the design choice on display.

Save any of these to `assets/towers/<name>.lua` to load them.

---

## Built-ins

The six towers shipped with the game. Useful as starting points for variations.

### ScrapGun — basic projectile turret

```lua
return Tower {
    name  = "ScrapGun",
    cost  = 50,
    color = "#E5CC4D",
    key   = "1",

    Cooldown(1.0),
    SingleTarget "closest",
    Range(80),
    Projectile { speed = 200 },
    AimPrecision(0.15),
    DirectDamage(10),

    ScrapCollector(30),
    Health(),
    BlocksNav(),
}
```

The bread-and-butter shape. One condition each from `Trigger`, `Acquirer`, `RangeProvider`. One `Deliverer`. One `Payload`. The rest is structural.

### Explosive — splash damage

```lua
return Tower {
    name  = "Explosive",
    cost  = 125,
    color = "#E07B00",
    key   = "2",

    Cooldown(3.3),
    SingleTarget "closest",
    Range(100),
    Projectile { speed = 200 },
    AimPrecision(0.15),
    DirectDamage(25),
    Splash { radius = 70, damage = 25 },

    ScrapCollector(30),
    Health(),
    BlocksNav(),
}
```

Same shape as ScrapGun plus a `Splash` payload. `Splash` implicitly creates an inner acquirer at the impact point — you don't have to spell that out.

### Railgun — long-range single-shot

```lua
return Tower {
    name  = "Railgun",
    cost  = 150,
    color = "#5C7A8C",
    key   = "3",

    Cooldown(5.0),
    SingleTarget "closest",
    Range(160),
    Projectile { speed = 2000 },
    AimPrecision(0.05),
    DirectDamage(50),

    ScrapCollector(30),
    Health(),
    BlocksNav(),
}
```

The interesting thing about this build is what it *doesn't* have. It's the same shape as ScrapGun, just retuned: longer cooldown, longer range, faster projectile, tighter aim, more damage. Most "different feel" towers are just parameter changes — the atom catalog stays small because the parameter space is wide.

### ChainLightning — single shot, multiple hops

```lua
return Tower {
    name  = "ChainLightning",
    cost  = 125,
    color = "#4FA1E0",
    key   = "4",

    Cooldown(2.0),
    SingleTarget "closest",
    Range(90),
    ChainWalk { arc_range = 60, hop_limit = math.huge },
    Hitscan(),
    DirectDamage(20),
    DamageFalloff(0.7),

    ScrapCollector(30),
    Health(),
    BlocksNav(),
}
```

Two acquirers stacked: `SingleTarget` seeds the first hit, `ChainWalk` extends it outward. `Hitscan` instead of `Projectile` because the chain visits each hop instantly. `DamageFalloff` is a modifier on `ChainWalk`. Note the use of `math.huge` for "no hop limit" — Lua's stdlib in action.

### TarPit — slow aura

```lua
return Tower {
    name  = "TarPit",
    cost  = 75,
    color = "#5C3A0F",
    key   = "5",

    ContinuousTick(),
    AllInRange(),
    Range(70),
    Aura(),
    Slow { factor = 0.4, duration = 0.5 },
    RangeFalloff "linear",

    ScrapCollector(30),
    -- no Health or BlocksNav — TarPit is a field, not a structure
}
```

The opposite shape from a turret. `ContinuousTick` instead of `Cooldown`, `AllInRange` instead of `SingleTarget`, `Aura` instead of `Projectile`. The `Slow` payload re-applies every frame to every enemy in the field. Notable: no `Health`, no `BlocksNav` — the tar pit is a passable field, not a destructible structure.

### ScrapMagnet — pull aura

```lua
return Tower {
    name  = "ScrapMagnet",
    cost  = 100,
    color = "#4F7CE0",
    key   = "6",

    ContinuousTick(),
    AllInRange(),
    Range(90),
    Aura(),
    Slow { factor = 0.5, duration = 0.5 },
    Pull(15),
    RangeFalloff "linear",

    Health(),
    BlocksNav(),
}
```

Same shape as TarPit with two payloads stacked (`Slow` and `Pull`) and the structural atoms back in. Multiple payloads on one tower is normal — they all apply per hit.

---

## Stretch designs

Recipes that exercise the wider catalog. Use these as references for what's possible.

### Frostnova — periodic AOE burst

```lua
return Tower {
    name  = "Frostnova",
    cost  = 200,
    color = "#A0E0FF",

    Cooldown(8.0),
    AllInRange(),
    Range(80),
    Aura(),
    Slow { factor = 0.0, duration = 1.0 },
    DirectDamage(20),

    Health(),
    BlocksNav(),
}
```

Aura geometry, but on a long cooldown instead of `ContinuousTick`. Bursts instead of continuous. `Slow { factor = 0 }` is a freeze.

### Snipefire — fast-fire homing burn

```lua
return Tower {
    name  = "Snipefire",
    cost  = 175,
    color = "#FF4040",

    Cooldown(0.4),
    SingleTarget "highest-hp",
    Range(140),
    Projectile { speed = 600 },
    Homing(),
    AimPrecision(0.2),
    DirectDamage(5),
    Burn { dps = 2, duration = 3 },

    Health(),
    BlocksNav(),
}
```

Targeting mode `"highest-hp"` makes this a tank-killer. The `Homing` modifier on `Projectile` lets the projectile track moving targets at its modest speed. `Burn` is a damage-over-time payload — small per-hit damage stacks via the burn.

### HeavySniper — charged sniper using `LockOn`

```lua
return Tower {
    name  = "HeavySniper",
    cost  = 250,
    color = "#7090A0",

    Cooldown(3.0),
    SingleTarget "furthest-along",
    LockOn(1.5),
    Range(200),
    Projectile { speed = 2000 },
    DirectDamage(80),

    Health(),
    BlocksNav(),
}
```

The `LockOn` modifier requires a 1.5-second continuous lock on the target before the acquirer "completes." `Cooldown(3.0)` independently caps fire rate. Combining them gives a weapon a dedicated charge-up trigger couldn't express: *fires at most every 3s, and only after 1.5s of lock.*

### Mortar — ballistic arc

```lua
return Tower {
    name  = "Mortar",
    cost  = 175,
    color = "#404040",

    Cooldown(3.5),
    SingleTarget "closest",
    Range(140),
    Projectile { speed = 150 },
    ArcTrajectory(80),
    DirectDamage(30),
    Splash { radius = 70, damage = 30 },

    Health(),
    BlocksNav(),
}
```

`ArcTrajectory` is a modifier that converts straight-line projectile motion into a ballistic arc. Combined with `Splash`, this is the classic mortar shape.

### MineTower — trap with a sub-recipe

```lua
return Tower {
    name  = "MineTower",
    cost  = 200,
    color = "#806040",

    Cooldown(4.0),
    RandomInArea(3),
    Range(120),
    Trap {
        lifetime = 60,
        template = {
            OnWorldEvent "EnemyStep",
            DirectDamage(50),
            Splash { radius = 60, damage = 40 },
        },
    },

    Health(),
    BlocksNav(),
}
```

`Trap` is the most compositionally interesting deliverer: it spawns a sub-entity that has its own pipeline. The `template` field is a list of atoms — a sub-recipe inside the parent recipe. The trap entity uses `OnWorldEvent "EnemyStep"` as its trigger, then applies its own payloads on detonation. Sub-recipes are an [open design question](open-questions.md); this shape is the current best guess.

### MatrixBeam — networked link weapon

```lua
return Tower {
    name  = "MatrixBeam",
    cost  = 150,
    color = "#00CCCC",

    NetworkNode(150),
    NetworkBeam { color = "#00FFFF", damage_on_cross = 25 },

    Health(),
    BlocksNav(),
}
```

The recipe describes one node. A single MatrixBeam tower does nothing — the *link* between two of them is the weapon. `NetworkNode` marks it as part of the network; `NetworkBeam` defines what happens when a link forms. The editor surfaces "this tower needs a neighbor" feedback during placement (see [Open Questions](open-questions.md)).

### TollGate — passive income via world event

```lua
return Tower {
    name  = "TollGate",
    cost  = 50,
    color = "#806000",

    OnWorldEvent "EnemyPass",
    Hitscan(),
    IncomeOnTrigger(5),

    BlocksNav(),
}
```

No `Acquirer`, no `Range`, no damage payload. The `OnWorldEvent` trigger carries the enemy through the pipeline as the implicit target; `Hitscan` is a no-op deliverer; `IncomeOnTrigger` is the actual effect — 5 scrap per enemy that passes. Place on a chokepoint.

### RageTower — pure crowd-control behavior

```lua
return Tower {
    name  = "RageTower",
    cost  = 100,
    color = "#C03060",

    Cooldown(4.0),
    AllInRange(),
    Range(80),
    Aura(),
    Confuse(3),

    Health(),
    BlocksNav(),
}
```

Zero damage. The `Confuse` payload is a `Behavior` payload — it changes what affected enemies *do* (attack other enemies for 3 seconds) rather than their stats. The pipeline shape is identical to a Frostnova; only the payload changes.

---

## Reading these as a learning exercise

If you skim through the recipes above, three patterns become obvious:

1. **Almost every combat tower is `Cooldown` (or `ContinuousTick`) + an `Acquirer` + `Range` + a `Deliverer` + at least one `Payload`.** Most variation is in *which* acquirer, deliverer, and payloads — not in the shape of the recipe.
2. **Modifiers do most of the differentiation.** `ArcTrajectory`, `Homing`, `LockOn`, `Pierce`, `Splash`, `RangeFalloff` — these are what make a tower feel distinct, more than the underlying skeleton.
3. **Behavioral payloads (`Confuse`, `Teleport`, `PathLoop`) and economic payloads (`IncomeOnTrigger`, `HealTarget`) reuse the exact same recipe shape as a damage tower.** A toll booth and a sniper share a skeleton.

That repetition is the point. Once you know one recipe, you know the shape of all of them.
