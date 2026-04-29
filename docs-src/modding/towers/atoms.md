# Atom Reference

Every recipe is built from two kinds of top-level atoms:

- **Deliverers** — combat units. One block per "thing the tower does." Each takes a table of named properties: a cooldown (or none), a range, a target mode, damage values, status effects.
- **Passives** — properties of the tower itself. Small calls.

Plus the **identity** header (`name`, `cost`, `color`) on the `Tower` constructor.

> **Status note.** This is the spec as currently designed. Implementation is ongoing — some deliverers and properties may not be wired up in the build you're running. The in-game build menu shows what's currently live. The runtime model behind the DSL lives in [`TOWER_EDITOR_BRAINSTORM.md`](https://github.com/n8behavior/bevy-td-sandbox/blob/main/TOWER_EDITOR_BRAINSTORM.md).

---

## Deliverers

### `Projectile { ... }`

A flying object spawned at the tower, traveling to the acquired target. The shape every turret follows.

```lua
Projectile {
  cooldown = 1.0,
  target = "closest",
  range = 80,
  speed = 200,
  damage = 10,
}
```

Common properties: `cooldown`, `target`, `range`, `speed`, `damage`, `splash`, `aim_precision`, `homing`, `pierce`, `bounce`, `arc`, `boomerang`, `path_constrained`, `spiral`, `multi_shot`, `proximity_detonate`, `lock_on`, `spray`. Plus any status effect or behavior payload (see Property cheatsheet).

### `Hitscan { ... }`

Instant hit — no projectile entity, no flight time. Good for railguns, chains, event-driven towers.

```lua
Hitscan {
  cooldown = 5.0,
  target = "closest",
  range = 160,
  damage = 50,
}
```

Common properties: `cooldown`, `target`, `range`, `damage`, `splash`, `chain`, `pierce`, `lock_on`, `multi_shot`. Plus status effects and behavior payloads.

### `Beam { ... }`

A sustained line from tower to target. Applies damage every tick while connected. Skip `cooldown` for continuous lasers.

```lua
Beam {
  target = "closest",
  range = 100,
  damage = 30,  -- per second
  burn = { dps = 5, duration = 2 },
}
```

Common properties: `cooldown`, `target`, `range`, `damage`, `burn`, `pierce`, `range_falloff`. Plus status effects.

### `Aura { ... }`

Touches every enemy in range every tick. Skip `cooldown` for continuous fields; add one for pulse-style auras.

```lua
Aura {
  range = 70,
  slow = { factor = 0.4, duration = 0.5 },
  range_falloff = "linear",
}
```

Common properties: `cooldown`, `range`, `range_falloff`, `cone`, `damage` (per tick). Plus status effects and behavior payloads.

### `Trap { ... }`

Spawns a placed entity at a target location. The placed entity has its own pipeline, defined inline as a sub-table.

```lua
Trap {
  cooldown = 4.0,
  range = 120,
  placement = "random",
  count = 3,
  lifetime = 60,
  explosion = {
    trigger = "enemy_step",
    damage = 50,
    splash = { radius = 60, damage = 40 },
  },
}
```

Common properties: `cooldown`, `range`, `placement` (`"random"` / `"target"`), `count`, `lifetime`, `explosion` (sub-recipe table — supports `trigger`, `damage`, `splash`, status effects).

### `Summon { ... }`

Spawns autonomous combat sub-entities (drones, swarms) that move and target on their own.

```lua
Summon {
  cooldown = 8.0,
  range = 100,
  template = "drone",
  count = 3,
  lifetime = 30,
}
```

### `NetworkLink { ... }`

The link between this tower and another `NetworkLink` tower is the weapon. A single linked tower does nothing alone.

```lua
NetworkLink {
  range = 150,
  color = "#00FFFF",
  damage_on_cross = 25,
}
```

---

## Property cheatsheet

Properties shared across deliverers, alphabetical:

| Property             | Type   | Meaning                                                       |
| -------------------- | ------ | ------------------------------------------------------------- |
| `aim_precision`      | number | required aim tolerance (radians) before fire                  |
| `arc`                | number | ballistic arc height (Projectile)                             |
| `banish`             | number | seconds enemy is removed from the map                         |
| `boomerang`          | bool   | projectile returns to tower                                   |
| `bounce`             | table  | `{ count, retention }` reflect between targets                |
| `burn`               | table  | `{ dps, duration }` damage-over-time                          |
| `chain`              | table  | `{ arc_range, hop_limit, damage_falloff }`                    |
| `cone`               | table  | `{ facing, spread }` restrict aura/all-in-range to a cone     |
| `confuse`            | number | seconds enemy attacks other enemies                           |
| `cooldown`           | number | seconds between fires; omit to run every tick                 |
| `damage`             | number | direct damage on hit                                          |
| `homing`             | bool   | projectile tracks the target                                  |
| `income`             | number | scrap granted on fire (combines with `trigger = "enemy_pass"`) |
| `knockback`          | number | push enemy backward along path                                |
| `lock_on`            | number | seconds target must be held before fire                       |
| `multi_shot`         | table  | `{ count, delay }` fire N times per trigger                   |
| `path_constrained`   | bool   | projectile follows the road network                           |
| `pierce`             | number | retention per pierce (0–1)                                    |
| `proximity_detonate` | number | trigger radius near enemy                                     |
| `range`              | number | acquisition radius                                            |
| `range_falloff`      | string | `"linear"` / `"quadratic"` / `"none"`                         |
| `slow`               | table  | `{ factor, duration }`                                        |
| `splash`             | table  | `{ radius, damage }` area damage at impact                    |
| `spray`              | table  | `{ pellets, spread }` shotgun spread                          |
| `spiral`             | table  | `{ radius, frequency }` corkscrew flight                      |
| `stun`               | number | seconds enemy can't act                                       |
| `target`             | string | acquisition mode (see below)                                  |
| `teleport`           | number | distance to push enemy backward along path                    |
| `trigger`            | string | event-driven fire instead of cooldown (see below)             |
| `vulnerability`      | table  | `{ multiplier, duration }` boost incoming damage              |

### Target modes

```lua
target = "closest"          -- nearest enemy in range
target = "highest-hp"       -- toughest enemy
target = "lowest-hp"        -- weakest enemy (cleanup)
target = "furthest-along"   -- closest to the goal
target = "all_in_range"     -- every enemy (default for Aura)
target = "line"             -- every enemy along a straight line through the tower
target = "random"           -- random points (Trap placement)
```

### Trigger modes (in place of `cooldown`)

```lua
trigger = "enemy_pass"      -- fires when an enemy crosses (toll towers)
trigger = "enemy_death"     -- fires when an enemy dies in range (bonus beacons)
trigger = "boss_spawn"      -- fires once per boss
trigger = "wave_start"      -- fires at the start of each wave
trigger = "enemy_step"      -- used inside Trap.explosion sub-recipes
```

You can use either `cooldown` or `trigger`, not both. Omit both for "run every tick."

---

## Passives

Top-level atoms in the body that aren't deliverers.

| Passive                                      | Meaning                                                   |
| -------------------------------------------- | --------------------------------------------------------- |
| `Health(hp?)`                                | Tower has hit points and can be destroyed. Indestructible without it. |
| `BlocksNav()`                                | Enemies must path around. Auras usually omit.             |
| `ScrapCollector(range)`                      | Pickup radius for dropped scrap.                          |
| `PassiveIncome(rate)`                        | Scrap per second granted to the player.                   |
| `NeighborBuff { stat, amount }`              | Buffs adjacent towers.                                    |
| `NetworkAmplify { range, bonus_per_link, boost }` | Boosts deliverer property by linked-neighbor count.   |
| `NetworkShare { range, stat }`               | Broadcasts a stat to linked neighbors.                    |
| `Cost(scrap)`                                | Atom-level override of the identity `cost` field.         |
