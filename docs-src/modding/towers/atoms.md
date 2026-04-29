# Atom Reference

This is the catalog of every atom you can put in a recipe. Each entry shows the atom's signature, what it does, what role it fills, what it needs from sibling atoms, and a one-line example in context.

Atoms are grouped by **palette** — the same grouping the in-game editor uses. The palette is a presentation choice; the underlying *role* an atom fills is what compatibility checks care about. See [Recipe Anatomy](anatomy.md) for how roles fit together.

> **Status note.** This page is the spec for the atom catalog as currently designed. Implementation is ongoing — some atoms may not be wired up in the build you're running. The in-game build menu shows what's currently live. The design rationale for the full catalog lives in [`TOWER_EDITOR_BRAINSTORM.md`](https://github.com/n8behavior/bevy-td-sandbox/blob/main/TOWER_EDITOR_BRAINSTORM.md).

---

## Triggers

A trigger is an **optional time-based gate** — it decides *when* the rest of the pipeline gets to run. If you don't add one, the tower runs every tick. For combat towers that's almost never what you want, so most recipes start with `Cooldown(N)`. Auras and persistent fields skip the gate entirely.

### `Cooldown(seconds)`

Fires when an internal timer elapses, then resets.

- **Fills:** `Trigger`
- **Needs:** nothing
- **Example:** `Cooldown(1.0)` — fires once per second.

### `OnThreshold(accumulator, value)`

Fires when an `Accumulator` atom crosses the given value. Typically resets the accumulator on fire.

- **Fills:** `Trigger`
- **Needs:** an `Accumulator`-role atom on the same tower.
- **Example:** `OnThreshold("EnergyCharge", 100)` — fires when the tower's `EnergyCharge` accumulator hits 100.

### `OnWorldEvent(kind)`

Fires in response to a world event. The event payload (which enemy, which wave) is passed forward to the rest of the pipeline.

- **Fills:** `Trigger`
- **Needs:** nothing — event payload provides the target if any payloads need one.
- **Valid `kind` values:** `"EnemyDeath"`, `"EnemyPass"`, `"BossSpawn"`, `"WaveStart"`, `"EnemyStep"` (for trap entities).
- **Example:** `OnWorldEvent "EnemyPass"` — the toll-tower trigger.

---

## Acquirers

An acquirer is a **target-producing condition** — it answers *who* (or where) the action applies to. It's also a condition: if no target is found, the rest of the pipeline doesn't run that frame.

### `SingleTarget(mode)`

Picks one enemy in range based on a sort mode.

- **Fills:** `Acquirer`
- **Needs:** a `RangeProvider` (`Range`) sibling.
- **Valid modes:** `"closest"`, `"furthest-along"`, `"highest-hp"`, `"lowest-hp"`, `"furthest"`.
- **Example:** `SingleTarget "highest-hp"` — sniper-style targeting.

### `AllInRange()`

Selects every enemy currently in range.

- **Fills:** `Acquirer`
- **Needs:** a `RangeProvider`.
- **Example:** `AllInRange()` paired with `Aura()` for a slow field.

### `ChainWalk { arc_range, hop_limit }`

Starts from a single seed target and walks outward, jumping to the nearest enemy within `arc_range` of the current hop, up to `hop_limit` jumps.

- **Fills:** `Acquirer` (extends a single-target seed into a chain)
- **Needs:** another `Acquirer` (typically `SingleTarget`) to provide the first target.
- **Example:** `ChainWalk { arc_range = 60, hop_limit = 6 }` — chain lightning.

### `LineOfFire()`

Selects every enemy along a straight line through the tower, picking the angle that hits the most.

- **Fills:** `Acquirer`
- **Needs:** a `RangeProvider`. Pairs naturally with `Pierce()` to hit them all.
- **Example:** `LineOfFire()` for a railgun-style pierce shot.

### `RandomInArea(count)`

Picks `count` random points within range, near where enemies currently are.

- **Fills:** `Acquirer`
- **Needs:** a `RangeProvider`.
- **Example:** `RandomInArea(3)` — meteor strike scatter.

### `NeighborTowers(link_range)`

Acquires *other towers* within `link_range`, instead of enemies. Used for `TowerNetwork` mechanics.

- **Fills:** `Acquirer` (with tower targets, not enemy targets)
- **Needs:** nothing on this tower; needs at least one other tower nearby with `NetworkNode`.
- **Example:** `NeighborTowers(150)` for a network-relay tower.

### `Range(radius)`

Declares the radius used by acquirers and aura-style deliverers. Not an acquirer itself — it's the parameter every range-bound acquirer reads.

- **Fills:** `RangeProvider`
- **Needs:** nothing
- **Example:** `Range(80)`.

---

## Deliverers

A deliverer is the **action-shape** — *how* a hit gets from the tower to the target. Every combat tower needs one.

### `Projectile { speed, trail }`

Spawns a flying projectile that travels to the target.

- **Fills:** `Deliverer`
- **Needs:** a single-target `Acquirer`.
- **Args:** `speed` (units/sec, required); `trail` (boolean, optional).
- **Example:** `Projectile { speed = 200 }`.

### `Hitscan()`

Instant hit. No flight time, no projectile entity.

- **Fills:** `Deliverer`
- **Needs:** any `Acquirer`.
- **Example:** `Hitscan()` for a railgun or a chain-lightning hop.

### `Aura()`

No-op deliverer. The acquirer (`AllInRange`) already touched everyone; payloads apply directly.

- **Fills:** `Deliverer`
- **Needs:** `AllInRange` (or another multi-target acquirer).
- **Example:** `Aura()` paired with `Slow { factor = 0.5, duration = 0.5 }` for a slow field.

### `Beam()`

A sustained line from the tower to the target. Applies payloads each tick while connected.

- **Fills:** `Deliverer`
- **Needs:** a single-target `Acquirer`. Typically used without a `Cooldown` so the beam runs every tick.
- **Example:** `Beam()` for a continuous laser.

### `Trap { template, lifetime }`

Spawns a persistent trap entity at a target *location*. The trap has its own pipeline (typically `OnWorldEvent "EnemyStep"` + payloads).

- **Fills:** `Deliverer` (via `Summoner`)
- **Needs:** an `Acquirer` that yields a location (e.g. `RandomInArea`).
- **Args:** `template` (a sub-recipe defining the trap entity); `lifetime` (seconds before it expires).
- **Example:** see the `MineTower` recipe in [Examples](examples.md). Sub-recipes are an [open design question](open-questions.md).

### `Summon { template, count, lifetime }`

Spawns autonomous combat sub-entities (drones, swarms) that move and target on their own.

- **Fills:** `Deliverer` (via `Summoner`)
- **Needs:** any `Acquirer` (the spawn position is local to the tower).
- **Example:** `Summon { template = "drone", count = 3, lifetime = 30 }`.

### `RadialBeams { count, rotation_speed }`

Spinning straight-line beams emanating from the tower.

- **Fills:** `Deliverer`
- **Needs:** no Trigger (runs every tick) or a low `Cooldown`.
- **Example:** `RadialBeams { count = 4, rotation_speed = 1.5 }` — blade tower.

---

## Payloads

A payload is **what happens** when the deliverer reaches a target. A combat tower has at least one. Multiple payloads on the same tower stack additively — they all apply on each hit.

### `DirectDamage(amount)`

Flat damage on hit.

- **Fills:** `Payload`
- **Needs:** any `Deliverer`.
- **Example:** `DirectDamage(10)`.

### `DirectDamage { formula }`

Damage as a function of game state (boss HP, current charge level, enemy speed). The formula is selected from a small set of named templates rather than a free expression — see [Open Questions](open-questions.md).

- **Fills:** `Payload`
- **Example:** `DirectDamage { formula = "fraction_of_target_hp", value = 0.2 }`.

### `Splash { radius, damage }`

Area damage at the impact point. Implicitly creates an inner acquirer that scoops up enemies in `radius` and applies `damage` to each.

- **Fills:** `Payload`
- **Needs:** any `Deliverer`.
- **Example:** `Splash { radius = 70, damage = 25 }`.

### `Slow { factor, duration }`

Reduces enemy movement speed by multiplying it by `factor` (0.0 = stopped, 1.0 = no slow), for `duration` seconds.

- **Fills:** `Payload`
- **Example:** `Slow { factor = 0.4, duration = 0.5 }` — a tar field.

### `Pull(speed)`

Drags enemies toward the tower's center.

- **Fills:** `Payload`
- **Example:** `Pull(15)` paired with `Aura()` for a magnet field.

### `Burn { dps, duration }`

Damage-over-time effect; deals `dps` per second for `duration` seconds.

- **Fills:** `Payload`
- **Example:** `Burn { dps = 5, duration = 3 }`.

### `Stun(seconds)`

Halts the enemy's actions for `seconds`. The enemy is still on the map and targetable.

- **Fills:** `Payload`
- **Example:** `Stun(0.5)`.

### `Knockback(force)`

Pushes the enemy backward along its path by `force` units.

- **Fills:** `Payload`
- **Example:** `Knockback(20)`.

### `Vulnerability { multiplier, duration }`

Marks the enemy to take `multiplier`× damage from *all* sources for `duration` seconds. Used by boost/curse towers.

- **Fills:** `Payload`
- **Example:** `Vulnerability { multiplier = 2.0, duration = 4 }`.

### `StatDebuff { speed_factor, hp_factor, reward_factor, duration }`

Reduces multiple enemy stats simultaneously. Coarser than composing individual `Slow`/`Vulnerability` payloads — a single "downgrade" effect.

- **Fills:** `Payload`
- **Example:** `StatDebuff { speed_factor = 0.7, hp_factor = 0.7, reward_factor = 0.7, duration = 5 }`.

### `FractionDamage { fraction, min_hp }`

Deals `fraction` of the enemy's *current* HP. Will not reduce HP below `min_hp` — cannot kill.

- **Fills:** `Payload`
- **Example:** `FractionDamage { fraction = 0.2, min_hp = 1 }` — drain pylon.

### `Teleport(distance_back)`

Moves the enemy backward along its path by `distance_back` units. A `Behavior` payload — manipulates position rather than HP.

- **Fills:** `Payload` (`Behavior`-flavor)
- **Example:** `Teleport(200)` — warp gate.

### `Banish(seconds)`

Removes the enemy from the map for `seconds`, then returns it at the same path position. Distinct from `Stun`: the enemy is fully absent during `Banish`.

- **Fills:** `Payload`
- **Example:** `Banish(3)`.

### `IncomeOnTrigger(amount)`

Grants `amount` scrap to the player when the tower fires. Combines with `OnWorldEvent` triggers for toll/bonus towers.

- **Fills:** `Payload`
- **Example:** `IncomeOnTrigger(5)`.

### `HealTarget { amount, target_type }`

Heals a tower or the base instead of damaging an enemy.

- **Fills:** `Payload`
- **Valid `target_type` values:** `"Base"`, `"Self"`, `"NearestTower"`.
- **Example:** `HealTarget { amount = 20, target_type = "NearestTower" }`.

### `GameSpeedSlow { factor, duration }`

Slows global game time by `factor` for `duration`. **Affects every entity** — enemies *and* other towers. Use with care.

- **Fills:** `Payload`
- **Example:** `GameSpeedSlow { factor = 0.5, duration = 2 }` — emc/time tower.

### `Confuse(seconds)`

Enemy targets and attacks nearby enemies instead of advancing for `seconds`. A `Behavior` payload — changes decisions, not state.

- **Fills:** `Payload` (`Behavior`-flavor)
- **Example:** `Confuse(3)` — rage tower.

### `PathAttract(strength)`

Biases enemy pathfinding toward this tower's tile. Strength is the bias magnitude.

- **Fills:** `Payload` (`Behavior`-flavor)
- **Example:** `PathAttract(1.5)` — magnet tower. Affects the navigation grid.

### `PathLoop(region)`

Forces the enemy to retrace a section of road before continuing.

- **Fills:** `Payload` (`Behavior`-flavor)
- **Example:** `PathLoop "current_segment"` — loop tower.

---

## Modifiers

A modifier alters the behavior of *another* atom on the same tower. It doesn't fire or do damage on its own.

### `Homing()`

Modifies a `Projectile` so it tracks the target.

- **Modifies:** `Projectile`
- **Example:** `Homing()`.

### `AimPrecision { tolerance, rotation_speed }`

Modifies a `Projectile` so the tower must aim within `tolerance` radians before firing, rotating at `rotation_speed` rad/sec.

- **Modifies:** `Projectile`
- **Example:** `AimPrecision { tolerance = 0.15, rotation_speed = 2.5 }`. Single-arg shortcut: `AimPrecision(0.15)`.

### `LockOn(seconds)`

Modifies a single-target `Acquirer` so it must continuously see the same target for `seconds` before that target counts as acquired. Resets if the target is lost.

- **Modifies:** `Acquirer` (single-target variants)
- **Example:** `LockOn(1.5)` — sniper-style charge-up.

### `Pierce(damage_retention)`

Modifies a `Projectile` or `Beam` so it passes through enemies and continues, retaining `damage_retention` (0–1) of damage per pierce.

- **Modifies:** `Projectile` or `Beam`
- **Example:** `Pierce(0.8)`.

### `Bounce { count, retention_per_bounce }`

Modifies a `Projectile` so it reflects off enemies onto the next nearest, up to `count` times.

- **Modifies:** `Projectile`
- **Example:** `Bounce { count = 3, retention_per_bounce = 0.7 }`.

### `ArcTrajectory(height)`

Modifies a `Projectile` so it follows a ballistic arc to its target rather than flying in a straight line.

- **Modifies:** `Projectile`
- **Example:** `ArcTrajectory(80)` — mortar.

### `SpraySpread { pellets, spread_angle }`

Modifies a `Projectile` to fire `pellets` projectiles in a fan, spread over `spread_angle` radians.

- **Modifies:** `Projectile`
- **Example:** `SpraySpread { pellets = 5, spread_angle = 0.4 }` — shotgun.

### `ProximityDetonate(radius)`

Modifies a `Projectile` so its payload triggers when it gets within `radius` of any enemy, rather than only on direct contact.

- **Modifies:** `Projectile`
- **Example:** `ProximityDetonate(15)` — flak.

### `ConeAOE { facing_angle, spread_angle }`

Modifies an `AllInRange` acquirer so it only sees enemies within a cone in front of the tower.

- **Modifies:** `AllInRange`
- **Example:** `ConeAOE { facing_angle = 0, spread_angle = 1.0 }`.

### `Boomerang(return_on_miss)`

Modifies a `Projectile` to arc out, then return to the tower. Applies the payload on both passes.

- **Modifies:** `Projectile`
- **Example:** `Boomerang { return_on_miss = true }`.

### `PathConstrained()`

Modifies a `Projectile` so it travels along the road network instead of flying freely.

- **Modifies:** `Projectile`
- **Example:** `PathConstrained()` — zap tower.

### `SpiralTrajectory { radius, frequency }`

Modifies a `Projectile` so its flight path corkscrews.

- **Modifies:** `Projectile`
- **Example:** `SpiralTrajectory { radius = 10, frequency = 2 }`.

### `MultiShot { count, delay }`

Modifies any `Deliverer` so it fires `count` times per trigger, with `delay` seconds between each.

- **Modifies:** `Deliverer` (any)
- **Example:** `MultiShot { count = 8, delay = 0.05 }` — fling burst.

### `DamageFalloff(per_hop)`

Modifies a `ChainWalk` acquirer so each chained hit retains only `per_hop` (0–1) of the previous hit's damage.

- **Modifies:** `ChainWalk`
- **Example:** `DamageFalloff(0.7)`.

### `RangeFalloff(curve)`

Modifies an `Aura` or `Beam` so payload intensity scales with distance from the tower.

- **Modifies:** `Aura` or `Beam`
- **Valid `curve` values:** `"linear"`, `"quadratic"`, `"none"`.
- **Example:** `RangeFalloff "linear"`.

### `ActivityRamp { rate, decay, source }`

Modifies a `Payload`'s magnitude based on an `ActivityCharge` accumulator.

- **Modifies:** `Payload` (any)
- **Needs:** an `ActivityCharge` accumulator.
- **Example:** `ActivityRamp { rate = 0.2, decay = 0.5, source = "ActivityCharge" }`.

---

## Accumulators

An accumulator holds a stateful value that builds up or decays over time. By itself it does nothing — it gates an `OnThreshold` trigger or scales an `ActivityRamp` modifier.

### `EnergyCharge { gain_source, gain_rate, max, decay_rate }`

Builds energy from a configurable source; decays when idle.

- **Fills:** `Accumulator`
- **Valid `gain_source` values:** `"enemy_proximity"`, `"enemy_speed_in_range"`, `"time_while_aiming"`.
- **Example:** `EnergyCharge { gain_source = "enemy_speed_in_range", gain_rate = 1.0, max = 100, decay_rate = 5 }` — shock cannon.

### `StoredShots { max, recharge }`

Accumulates shot-charges up to `max`; releases all at once via `OnThreshold`.

- **Fills:** `Accumulator`
- **Example:** `StoredShots { max = 8, recharge = 0.5 }` — fling tower.

### `ActivityCharge { ramp, decay }`

Rises while the tower is firing, decays while idle. Feeds `ActivityRamp`.

- **Fills:** `Accumulator`
- **Example:** `ActivityCharge { ramp = 0.2, decay = 0.5 }` — charge-gun.

---

## Network

Network atoms let towers interact with *other towers* rather than enemies.

### `NetworkNode(link_range)`

Marks the tower as part of the network. Auto-connects to other towers within `link_range` that also have `NetworkNode`.

- **Fills:** `TowerNetwork`
- **Needs:** nothing on this tower; another `NetworkNode` tower nearby for the link to do anything.
- **Example:** `NetworkNode(150)`.

### `NetworkBeam { color, damage_on_cross }`

Renders a visible link between connected towers, dealing damage to enemies that cross the line.

- **Fills:** `Deliverer` (via the network link)
- **Needs:** `NetworkNode` on this tower and at least one neighbor.
- **Example:** `NetworkBeam { color = "#00FFFF", damage_on_cross = 25 }`.

### `NetworkAmplify(bonus_per_link)`

Adds a flat bonus per connected neighbor to this tower's damage and/or range.

- **Modifies:** this tower's `Payload` and `Range` based on link count
- **Needs:** `NetworkNode`.
- **Example:** `NetworkAmplify(10)` — arc tower's additive firepower.

### `NetworkShare(stat)`

Broadcasts a stat to all connected neighbors.

- **Modifies:** other connected towers
- **Needs:** `NetworkNode`.
- **Valid `stat` values:** `"range"`, `"detection"`.
- **Example:** `NetworkShare "range"` — radar tower.

---

## Passives

A passive runs continuously on the tower itself — it doesn't acquire targets, doesn't fire, doesn't trigger. Add as many as you like.

### `ScrapCollector(range)`

Picks up scrap drops within `range`.

- **Fills:** `Passive`
- **Example:** `ScrapCollector(30)`.

### `PassiveIncome(rate)`

Grants `rate` scrap per second to the player.

- **Fills:** `Passive`
- **Example:** `PassiveIncome(3)` — solar array.

### `NeighborBuff { stat, amount }`

Buffs a stat on adjacent towers.

- **Fills:** `Passive`
- **Example:** `NeighborBuff { stat = "damage", amount = 0.1 }`.

---

## Structure

Structural atoms govern how the tower exists in the world — destructibility, navigation, cost. They don't participate in the combat pipeline.

### `Health(hp)`

The tower has hit points and can be destroyed by enemies that reach it. Without `Health()`, the tower is indestructible.

- **Fills:** `Lifecycle`
- **Args:** `hp` (optional; defaults to a sensible value per tower category).
- **Example:** `Health()` or `Health(150)`.

### `BlocksNav()`

The tower blocks the navigation grid — enemies must path around it. Aura-style towers typically omit this so enemies can walk through them.

- **Fills:** `Lifecycle`
- **Example:** `BlocksNav()`.

### `Cost(scrap)`

Override the tower's `cost` identity field with an atom-level cost. Mostly useful for variant generation in [Advanced Patterns](advanced.md). The identity-field `cost` is the simpler default.

- **Fills:** `Lifecycle`
- **Example:** `Cost(75)`.
