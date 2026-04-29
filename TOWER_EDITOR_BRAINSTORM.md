# Tower Editor — Brainstorm

I want to start work on a player-facing editor for this tower-defense game. The first piece is a **tower creator** — let players mix and match behaviors to build their own towers, then play with them. Eventually the same approach extends to maps and enemies.

This is an early sketch, not a spec. I'd love to kick it around before we start building. Holes, additions, and "what about…" are all very welcome.

The design goal in one line: **easiest and most fun tower-building editor ever**. Players should drop in a couple of choices, see something cool happen on screen, and immediately want to tweak it.

---

## How I want to think about this

Four lenses, used together:

1. **Worked examples first.** Start with concrete towers — our six built-ins, plus weird "what if" recipes — and decompose each into pieces. The vocabulary of pieces falls out of what the recipes actually need. There's a [great community list of tower ideas](https://love2d.org/forums/viewtopic.php?t=85229) we can mine for stretch examples.
2. **Components + interfaces** for the data model. Each piece is a component; pieces declare what _roles_ they fill and what _roles_ they need from siblings. Compatibility is automatic — no hand-maintained "X works with Y" tables.
3. **Runtime pipeline** for temporal reasoning. When we ask "in what order does stuff happen?", we look at the pipeline. The data model says _what_ a tower is; the pipeline says _when_ each part runs.
4. **Decision tree only at the UI layer.** The player sees progressive disclosure ("you picked Projectile, now pick a payload") but that's a presentation skin — the underlying model stays free-form.

Why this combination: as a Bevy/ECS dev, components-and-interfaces is how I want to actually build it. The pipeline is how I'll reason about correctness. The recipes are how we'll know we're solving real design problems and not building abstractions in a vacuum.

---

## The runtime pipeline (mental model)

Every tower's behavior unfolds along this pipeline. Different delivery styles short-circuit different stages:

```
TICK ─► [Trigger] ─► [Acquire] ─► [Deliver] ─► [On-Hit] ─► [Linger]
         when?       who?         how?         what?       (DoT, slow,
        (cooldown,  (targeting,  (projectile,  (damage,     debuff
         charge,     range,       chain,        splash,     timers)
         continuous) area)        aura, beam)   stun…)
```

- **Projectile turret:** Trigger on cooldown → Acquire one target → Deliver a flying thing → On-Hit applies payloads → some payloads Linger (slow timer, burn DoT).
- **Aura:** Trigger every tick → Acquire = "everyone in range" → Deliver is a no-op → On-Hit applies payloads each tick → Linger handles brief slow timers.
- **Chain:** Trigger on cooldown → Acquire primary target → Deliver hops between secondary targets → On-Hit applies (with attenuation) at each hop.
- **Pulse / Frostnova:** Aura, but Trigger is gated by a cooldown — bursts instead of constant.

This is also exactly what the **timeline editor view** (more on that below) would visualize.

---

## The data model — components & interfaces

The atoms of the editor. Each component fills one or more **roles**. Compatibility = "do all `needs` get filled by something on the same tower?"

### Roles (interfaces)

- `Trigger` — decides _when_ to act (cooldown, every-tick, charge-up, or in response to a world event).
- `Acquirer` — produces a list of targets: enemies, locations, or even other towers.
- `Deliverer` — turns "we triggered with these targets" into actual hits (instant, projectile, beam, trap, summon…).
- `Payload` — does something to a hit target (damage, slow, burn, debuff, teleport…).
- `RangeProvider` — declares the radius for `Acquirer` and aura systems.
- `Modifier` — alters another component's behavior (homing, pierce, arc trajectory, bounce…).
- `Passive` — runs continuously on the tower itself, not on hits (scrap collector, neighbor buff…).
- `Lifecycle` — placement, destruction, repair behavior.
- `Accumulator` — holds a stateful value (energy, charge, stored-shot count) that feeds into a `Trigger` or scales a `Payload`. Neither a Trigger nor a Payload itself: it _gates or modifies_ them. Needed for shock towers (charge from enemy proximity), fling towers (hold shots until released), charge/idle towers (ramp damage up or down with activity).
- `Summoner` — a Deliverer variant whose output is a persistent sub-entity rather than an immediate hit. The spawned entity has its own position, lifetime, and optionally its own mini-pipeline (e.g. a mine that triggers `OnEnemyStep`). Needed for trap towers, swarm launchers, UFO towers.
- `TowerNetwork` — allows a tower to link to _other towers_ rather than targeting enemies. Links carry a payload or modify it (additive damage, range sharing). Needed for matrix/beam-between-towers designs and the arc tower's additive-firepower mechanic.
- `Behavior` — a payload that modifies an enemy's _decisions_ rather than its state. Slow changes speed; Behavior changes what the enemy does next. Needed for rage towers (attack neighbors), loop towers (retrace path), warp towers (teleport back), magnet towers (path attraction).

### Components (a starter catalog)

**Triggers**

- `Cooldown(secs)` — fills `Trigger`
- `OnThreshold(accumulator, value)` — fills `Trigger`; fires when an `Accumulator` crosses a value (fling tower releasing its stored shots, shock tower instakilling at full charge)
- `OnWorldEvent(event_type)` — fills `Trigger`; fires in response to a world-state change: `EnemyDeath`, `EnemyPass`, `BossSpawn`, `WaveStart`. Unblocks the entire toll/bonus/nuke family.

**Acquirers**

- `SingleTarget(mode)` — fills `Acquirer`; mode is closest / lowest-hp / highest-hp / furthest-along
- `AllInRange` — fills `Acquirer`; needs `RangeProvider`
- `ChainWalk(arc_range, hop_limit)` — fills `Acquirer`; starts from a single target and walks outward
- `LineOfFire` — fills `Acquirer`; selects all enemies along a straight line through the tower (rail tower, gamma laser)
- `RandomInArea(count)` — fills `Acquirer`; picks random positions near enemies (meteor tower, blast tower scatter)
- `NeighborTowers(link_range)` — fills `Acquirer` with other tower entities rather than enemies; required by `TowerNetwork` towers

**Deliverers**

- `Projectile(speed, trail)` — fills `Deliverer`; needs `Acquirer` that yields a single target
- `Hitscan` — fills `Deliverer`; instant
- `Aura` — fills `Deliverer`; no-op (the Acquirer already touched everyone)
- `Beam` — fills `Deliverer`; sustained line from tower to target, applies payloads each tick while connected (laser, proton, gamma, beam towers)
- `Trap(template, lifetime)` — fills `Deliverer` via `Summoner`; deploys a persistent entity at a location on the map with its own trigger (typically `OnWorldEvent(EnemyStep)`)
- `Summon(template, count, lifetime)` — fills `Deliverer` via `Summoner`; spawns autonomous combat sub-entities (swarms, UFOs) that have their own targeting and movement
- `RadialBeams(line_count, rotation_speed)` — fills `Deliverer`; spinning line beams emanating from the tower (blade tower, beacon tower)

**Payloads**

- `DirectDamage(amount)`
- `DirectDamage(formula)` — damage expressed as a function of game state: enemy current HP, tower charge level, boss HP, etc. Needed for drain tower (fraction of current HP), charge/idle towers (ramp scaling), nuke tower (fraction of boss HP).
- `Splash(radius, damage)` — implicitly creates an inner Acquirer at impact point
- `Slow(factor, duration)`
- `Pull(speed)`
- `Burn(dps, duration)` _(future)_
- `Stun(duration)` _(future)_
- `Knockback(force)` _(future)_
- `Vulnerability(multiplier, duration)` — marks enemy to take increased damage from _all_ sources; used by boost/curse towers. Different from `DirectDamage` because it affects other towers' output.
- `StatDebuff(speed_factor, hp_factor, reward_factor, duration)` — reduces multiple enemy stats simultaneously (lower tower). Coarser than composing individual Slow/etc. — might be the right call for enemy-level downgrade semantics.
- `FractionDamage(fraction, min_hp)` — deals a percentage of current HP; cannot kill (drain tower). Complements `DirectDamage` for percentage-based designs.
- `Teleport(distance_back)` — moves enemy backward along its path by a fixed path distance (warp tower). A `Behavior` payload: it manipulates position, not HP or speed.
- `Banish(duration)` — removes enemy from the map temporarily, returns it at the same path position (121jw tower). Distinct from Stun: enemy is fully absent.
- `IncomeOnTrigger(amount)` — grants scrap to the player when the tower fires (toll tower, bonus tower). Combines with `OnWorldEvent` triggers.
- `HealTarget(amount, target_type)` — heals a tower or the base rather than damaging an enemy. `target_type` is `Base`, `Self`, or `NearestTower` (repair tower).
- `GameSpeedSlow(factor, duration)` — slows global time, not just one enemy's movement speed (emc/time tower). Affects all entities; flag this as high-risk for game feel.
- `Confuse(duration)` — fills `Behavior`; enemy targets and attacks nearby enemies instead of advancing (rage tower).
- `PathAttract(strength)` — fills `Behavior`; biases enemy pathfinding toward this tower's tile (magnet tower).
- `PathLoop(region)` — fills `Behavior`; forces enemy to retrace a section of road before continuing (loop tower).

**Modifiers**

- `Homing` — modifies `Projectile`
- `DamageFalloff(per_hop)` — modifies `ChainWalk`
- `RangeFalloff(curve)` — modifies aura payload intensity by distance
- `AimPrecision(tolerance, rotation_speed)` — modifies `Projectile`
- `Pierce(damage_retention)` — modifies `Projectile` or `Beam`; passes through enemies and continues, retaining a fraction of damage per hit (rail tower, gamma laser)
- `Bounce(count, retention_per_bounce)` — modifies `Projectile`; reflects off enemies and continues toward the next (plasma tower)
- `ArcTrajectory(height)` — modifies `Projectile`; ballistic arc instead of straight line, lands at target position (mortar, bomb, rocket, meteor towers)
- `SpraySpread(pellet_count, spread_angle)` — modifies `Projectile`; fires multiple projectiles in a fan simultaneously (shot tower, rotor spray burst)
- `ProximityDetonate(trigger_radius)` — modifies `Projectile`; triggers payload when near an enemy rather than on direct contact (flak tower)
- `ConeAOE(facing_angle, spread_angle)` — modifies `Acquirer`; restricts `AllInRange` to a directional cone in front of the tower (sonic tower)
- `LockOn(secs)` — modifies `Acquirer` (single-target variants); the Acquirer must continuously see the same target for `secs` before it's considered acquired. Replaces an earlier "ChargeUp Trigger" sketch — lock-time is a targeting concern, not a timing concern, so it lives on the Acquirer side and the Trigger stays pure (no back-channel from Acquirer to Trigger). Composes with `Cooldown` to express weapons a dedicated charge-Trigger couldn't (e.g. "fires every 5s, but only after 1.5s of lock").
- `Boomerang(return_on_miss)` — modifies `Projectile`; projectile arcs out and returns to tower, applying payload on both passes (rang tower)
- `PathConstrained` — modifies `Projectile`; constrains movement to the road network rather than flying freely (zap tower)
- `SpiralTrajectory(radius, frequency)` — modifies `Projectile`; projectile corkscrews around its flight path (spiral tower, nova spiral pattern)
- `MultiShot(count, delay_between)` — modifies any `Deliverer`; fires the full sequence `count` times per trigger, with optional spacing (fling burst release, blast tower barrage)
- `ActivityRamp(rate, decay)` — modifies `Payload`; scales a payload's magnitude up or down based on how recently the tower has fired (charge tower, idle tower)

**Accumulators**

- `EnergyCharge(gain_source, gain_rate, max, decay_rate)` — fills `Accumulator`; value rises from a source (nearby enemy proximity, enemy speed, time-while-aiming) and decays when idle. Gates an `OnThreshold` trigger. Needed by shock tower.
- `StoredShots(max_count, recharge_rate)` — fills `Accumulator`; accumulates shot charges up to a cap, releases all at once via `OnThreshold`. Needed by fling tower.
- `ActivityCharge(ramp_rate, decay_rate)` — fills `Accumulator`; rises while the tower is firing, decays while idle. Feeds `ActivityRamp`. Needed by charge and idle towers.

**TowerNetwork**

- `NetworkNode(link_range)` — marks a tower as linkable; automatically connects to nearby towers that also have `NetworkNode`
- `NetworkBeam(color, damage_on_cross)` — renders a visible link between connected towers and deals damage to enemies that cross it (matrix/barbed/gear towers)
- `NetworkAmplify(bonus_per_link)` — adds a flat damage or range bonus per connected neighbor (arc tower's additive firepower mechanic)
- `NetworkShare(stat)` — broadcasts a stat (range, detection) to all connected neighbors (radar tower)

**Passives**

- `ScrapCollector(range)`
- `NeighborBuff(stat, amount)` _(future)_
- `PassiveIncome(rate)` _(future)_

**Lifecycle / structural**

- `BlocksNav`
- `Health(hp_curve)` — destructibility + degradation
- `Cost(scrap)`

**Identity**

- `Name`, `Color`, `Icon`, `Label` *(no `Hotkey` — bindings are an in-game UI concern, not a recipe field; avoids cross-mod key collisions)*

### Compatibility rules emerge from `needs`

```
Projectile          needs: Trigger, Acquirer(single)
Hitscan             needs: Trigger, Acquirer(single)
Aura                needs: Trigger(continuous-or-cooldown), Acquirer(all-in-range)
Beam                needs: Trigger(continuous-or-cooldown), Acquirer(single)
Trap                needs: Summoner, location target; spawned entity needs its own Trigger
Summon              needs: Summoner, template definition
RadialBeams         needs: Trigger(continuous-or-cooldown)
ChainWalk           needs: another Acquirer to seed the first target
LineOfFire          needs: Trigger, RangeProvider
AimPrecision        needs: Projectile
DamageFalloff       needs: ChainWalk
Splash              needs: Deliverer (any)
RangeFalloff        needs: Aura or Beam
ScrapCollector      needs: nothing — purely passive
Pierce              needs: Projectile or Beam
Bounce              needs: Projectile
ArcTrajectory       needs: Projectile
SpraySpread         needs: Projectile
ProximityDetonate   needs: Projectile
ConeAOE             needs: AllInRange
LockOn              needs: SingleTarget (or other single-target Acquirer)
Boomerang           needs: Projectile
PathConstrained     needs: Projectile
ActivityRamp        needs: Payload (any), ActivityCharge
EnergyCharge        needs: OnThreshold Trigger (to do anything useful)
StoredShots         needs: OnThreshold Trigger
OnThreshold         needs: Accumulator
OnWorldEvent        needs: nothing — always valid as a Trigger
NetworkNode         needs: nothing — self-contained
NetworkBeam         needs: NetworkNode on at least two towers
NetworkAmplify      needs: NetworkNode
NetworkShare        needs: NetworkNode
Confuse             needs: Deliverer (any)
PathAttract         needs: Deliverer or Passive scope; affects pathfinding grid
PathLoop            needs: Deliverer or Passive scope; affects pathfinding grid
Teleport            needs: Trigger, Acquirer(single)
Banish              needs: Trigger, Acquirer(single)
HealTarget          needs: Trigger; target_type determines Acquirer
GameSpeedSlow       needs: Trigger (any); high-impact, use with care
FractionDamage      needs: Trigger, Acquirer(single)
Vulnerability       needs: Deliverer (any); marks enemy for other towers to benefit from
```

The editor greys out incompatible options as you build. No big lookup table — each component declares its own needs. The four new roles (`Accumulator`, `Summoner`, `TowerNetwork`, `Behavior`) are structurally distinct enough that the editor could surface them as a separate "advanced" tier rather than mixing them into the main palette.

---

## Worked examples (the test suite)

Decomposing our six built-ins. If the model can't express these, it's broken. I've also tossed in a couple of "what if" recipes to stretch it.

> **DSL note.** Recipe sketches in this section use a flat syntax (`Cooldown + SingleTarget + Range + Projectile + ...`) for analytical clarity — to enumerate the runtime atoms each tower decomposes into. The actual *player-facing* DSL groups these atoms inside deliverer blocks (`Projectile { cooldown = 1.0, target = "closest", range = 80, damage = 10 }`) so each combat unit is self-contained. See the [TOWER_MODDING manual](https://n8behavior.github.io/bevy-td-sandbox/modding/towers/) for the surface form. The runtime model below — atoms, roles, compatibility — describes what the engine sees underneath.

```
ScrapGun        = Cooldown(1.0) + SingleTarget(closest) + Range(80)
                + Projectile(speed=200) + AimPrecision(0.15)
                + DirectDamage(10)
                + ScrapCollector(30) + Health + BlocksNav
                + Identity{name="ScrapGun", color=yellow, cost=50}

Explosive       = Cooldown(3.3) + SingleTarget(closest) + Range(100)
                + Projectile(speed=200) + AimPrecision(0.15)
                + DirectDamage(25) + Splash(radius=70, damage=25)
                + ScrapCollector(30) + Health + BlocksNav
                + Identity{name="Explosive", color=orange, cost=125}

Railgun         = Cooldown(5.0) + SingleTarget(closest) + Range(160)
                + Projectile(speed=2000) + AimPrecision(0.05)
                + DirectDamage(50)
                + ScrapCollector(30) + Health + BlocksNav
                + Identity{name="Railgun", color=blue-grey, cost=150}

ChainLightning  = Cooldown(2.0) + SingleTarget(closest) + Range(90)
                + ChainWalk(arc_range=60, hop_limit=∞)
                + Hitscan + DirectDamage(20) + DamageFalloff(0.7)
                + ScrapCollector(30) + Health + BlocksNav
                + Identity{name="ChainLightning", color=blue, cost=125}

TarPit          = AllInRange + Range(70)
                + Aura + Slow(0.4, 0.5) + RangeFalloff(linear)
                + ScrapCollector(30)
                + Identity{name="TarPit", color=brown, cost=75}
                  (no Health, no BlocksNav)

ScrapMagnet     = AllInRange + Range(90)
                + Aura + Slow(0.5, 0.5) + Pull(15) + RangeFalloff(linear)
                + Health + BlocksNav
                + Identity{name="ScrapMagnet", color=blue, cost=100}

# stretch examples

Frostnova       = Cooldown(8.0) + AllInRange + Range(80)
                + Aura + Slow(0.0, 1.0) + DirectDamage(20)
                + Health + BlocksNav

Snipefire       = Cooldown(0.4) + SingleTarget(highest-hp) + Range(140)
                + Projectile(speed=600) + Homing + AimPrecision(0.2)
                + DirectDamage(5) + Burn(2/s, 3s)

HeavySniper     = Cooldown(3.0) + SingleTarget(furthest-along) + LockOn(1.5)
                + Range(200) + Projectile(speed=2000)
                + DirectDamage(80)
                  (lock-time on the Acquirer, not a ChargeUp Trigger;
                   Cooldown still caps fire rate independently)

InfernoBeam     = Cooldown(0.0) + SingleTarget(closest) + Range(100)
                + Beam + DirectDamage(30/s) + Burn(5/s, 2s)

GravityWell     = AllInRange + Range(60)
                + Aura + Pull(40) + RangeFalloff(quadratic)
                  (no damage; pure crowd control)

# community-derived examples (stress-test for the extended model)

RailGun         = Cooldown(5.0) + SingleTarget(highest-total-in-line) + Range(160)
                + Projectile(speed=2000) + LineOfFire + Pierce(1.0)
                + DirectDamage(50)
                + Health + BlocksNav
                  (LineOfFire overrides SingleTarget: picks the angle that threads the most enemies)

Mortar          = Cooldown(3.5) + SingleTarget(closest) + Range(140)
                + Projectile(speed=150) + ArcTrajectory(height=80)
                + DirectDamage(30) + Splash(radius=70, damage=30)
                + Health + BlocksNav

MineTower       = Cooldown(4.0) + RandomInArea(3) + Range(120)
                + Trap(template=mine_entity, lifetime=60s)
                  [mine_entity: OnWorldEvent(EnemyStep) + DirectDamage(50) + Splash(radius=60, damage=40)]
                + Health + BlocksNav
                  (the Trap deliverer is a Summoner; the spawned entity carries its own pipeline)

FlingSurge      = StoredShots(max=8, recharge=0.5/s)
                + OnThreshold(StoredShots, threshold=8) [or player-triggered release]
                + SingleTarget(closest) + Range(100)
                + Projectile(speed=350) + MultiShot(count=8, delay=0.05s)
                + DirectDamage(12)
                + Health + BlocksNav

ShockCannon     = EnergyCharge(gain_source=enemy_speed_in_range, max=100, decay=5/s)
                + OnThreshold(EnergyCharge, threshold=100)
                + SingleTarget(closest) + Range(80)
                + Hitscan + DirectDamage(formula: instakill)
                + Health + BlocksNav
                  (charges faster when faster enemies are nearby; instakills on full charge)

ChargeGun       = ActivityCharge(ramp=0.2/shot, decay=0.5/s)
                + Cooldown(0.8) + SingleTarget(closest) + Range(80)
                + Projectile(speed=200)
                + DirectDamage(10) + ActivityRamp(ramp=ActivityCharge)
                + Health + BlocksNav
                  (first shot weak; sustained fire ramps to 5× damage)

MatrixBeam      = NetworkNode(link_range=150)
                + NetworkBeam(color=cyan, damage_on_cross=25)
                + Health + BlocksNav
                  (pairs with a second MatrixBeam tower; the link is the weapon)

ArcNode         = NetworkNode(link_range=100)
                + NetworkAmplify(bonus_per_link=+10dmg)
                + Cooldown(2.0) + SingleTarget(closest) + Range(90)
                + Hitscan + DirectDamage(20)
                + Health + BlocksNav
                  (standalone: 20 dmg; with two neighbors: 40 dmg)

RageTower       = Cooldown(4.0) + AllInRange + Range(80)
                + Aura + Confuse(duration=3s)
                  (no damage; enemies turn on each other for 3s)

WarpGate        = Cooldown(3.0) + SingleTarget(furthest-along) + Range(100)
                + Hitscan + Teleport(distance_back=200)
                + BlocksNav
                  (no health — fragile structure; no damage — pure disruption)

DrainPylon      = Cooldown(2.0) + SingleTarget(highest-hp) + Range(80)
                + Hitscan + FractionDamage(fraction=0.2, min_hp=1)
                + Health + BlocksNav
                  (always leaves enemies alive; synergizes with instakill towers)

SolarArray      = PassiveIncome(rate=3/s)
                  (no combat components at all)

TollGate        = OnWorldEvent(EnemyPass) + [no Acquirer needed — event carries the enemy]
                + Hitscan + IncomeOnTrigger(amount=5)
                + BlocksNav
                  (makes money from traffic; place on a chokepoint)

BonusBeacon     = OnWorldEvent(EnemyDeath, in_range=true) + AllInRange + Range(80)
                + Hitscan + IncomeOnTrigger(amount=10)
                  (no health, no BlocksNav — a scoring structure, not a weapon)

RepairStation   = Cooldown(3.0) + [Acquirer: NearestTower(range=100)]
                + Hitscan + HealTarget(amount=20, target_type=NearestTower)
                  (targets towers not enemies; needs Acquirer extended to non-enemy entities)
```

A few things stand out from this exercise. `Trap` is the most compositionally different: a tower whose Deliverer spawns a child entity that itself has a full pipeline. Whether that's elegant or over-clever is an open question. `TowerNetwork` towers require at least two placed towers to do anything — that's a UX challenge the editor needs to handle (preview mode showing "link will activate when neighbor is placed"). The `Behavior` payloads (Confuse, Teleport, PathLoop) are the most fragile relative to pathfinding: they all need write access to the nav grid or enemy AI state, which is a bigger implementation lift than any payload in the original model.

If we can express _all_ of these from the component set above, we're in good shape.

---

## The editor — two views

### View 1: Component palette (the "build" view)

Drag-and-drop assembly of the recipe. Progressive disclosure: pick a Deliverer first, then valid Acquirers and Triggers fill in, then payloads and modifiers compatible with what you picked become visible. Sliders for the numeric parameters. Identity tab for name/color/icon. This is the data view.

### View 2: Timeline (the "behavior" view)

This is the part I got excited about — it'd work like a Flash timeline editor. Time runs left-to-right. Each role/component gets a lane. The player sees, at a glance, what their tower _does_ during one shot cycle.

```
              0s        0.5s       1.0s       1.5s       2.0s
              ┌─────────┬─────────┬─────────┬─────────┬───────►
Cooldown      │██████░░░░░░░░░░░░░░░░░░░░░░░░░░░░██████░░░░░  (1.0s gate)
Acquire       │ ▼ pick target                       ▼
Projectile    │  ●━━━━━━━━━━━━●                     ●━━━━━●
Hit           │              ✦                            ✦
DirectDmg     │              ▌10                          ▌10
Slow (linger) │              ░░░░░░░░░░  (0.5s)
              └────────────────────────────────────────────────►
```

Same tower as a continuous aura would just show full bars on every lane every tick. A chain tower would show multiple hit markers fanning outward. A charge-up turret would show the charge bar growing before the projectile fires.

Why this is exciting:

- **Debugging by inspection.** "Why is my tower not firing?" → look at the cooldown lane.
- **Teaches the pipeline.** The same lanes the timeline shows _are_ the runtime stages.
- **Tweak in temporal terms.** Drag the cooldown shorter; drag the slow's linger longer; visual immediate feedback.
- **A second authoring surface.** Some players will think in components ("I want a turret with splash"); others in timing ("I want something that fires fast then has a long cooldown"). Two views serve both.

This is speculative — I have no idea yet how hard it is to build. But it feels worth holding as a design north star.

---

## How the editor knows its atoms

Rust has no runtime introspection, so the editor needs a **catalog** layer separate from the runtime ECS layer. Two layers, two jobs:

1. **Runtime layer.** What's on a tower entity at play time. Plain Bevy components.
2. **Catalog layer.** Static knowledge of *what atoms exist*, what slots they fill and need, what knobs they expose, how to spawn one. The editor reads this; the game doesn't.

Three credible ways to build the catalog. I'd combine the second and third.

**A. Hand-rolled `AtomDef` registry.** Most explicit, zero magic — an entry per atom (via `inventory::submit!` or similar). Clean to read, but each atom becomes two definitions (the `Component` and the `AtomDef`) that can drift.

**B. Lean on `bevy_reflect`.** `#[derive(Reflect)]` plus `#[reflect(Atom)]` registers a type-data adapter. The editor walks the `TypeRegistry` for every type implementing `Atom`, generates parameter sliders from `Reflect` field iteration and `@range` attributes, and serializes recipes to RON for free. This is how `bevy-inspector-egui` works and is the idiomatic Bevy editor pattern. Recipe save/load (principle #5 — copy-pasteable text) falls out for free.

**C. Marker components for runtime compatibility.** Pair each atom with a `Fills*` marker via Bevy 0.18's `#[require(...)]`. Then "does this tower satisfy `needs`?" is a vanilla `Query` — no reflection at runtime, just ECS.

Combined sketch:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role { Trigger, Acquirer, Deliverer, Payload, RangeProvider,
                Modifier, Accumulator, Summoner, TowerNetwork, Behavior,
                Passive, Lifecycle }

/// UI grouping for the palette. Deliberately distinct from `Role` —
/// this is a presentation concern, not a semantic claim about the atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Palette { Trigger, Acquirer, Deliverer, Payload, Modifier,
                   Accumulator, Network, Passive, Structure, Identity }

pub trait Atom: Reflect {
    const PALETTE: &'static [Palette];   // slice future-proofs cross-listing
    const FILLS:   &'static [Role];
    const NEEDS:   &'static [Role];
}

#[derive(Component)] pub struct FillsTrigger;

#[derive(Component, Reflect, Default)]
#[reflect(Component, Atom)]
#[require(FillsTrigger)]
pub struct Cooldown {
    #[reflect(@0.1_f32..=10.0)]
    pub seconds: f32,
}

impl Atom for Cooldown {
    const PALETTE: &'static [Palette] = &[Palette::Trigger];
    const FILLS:   &'static [Role]    = &[Role::Trigger];
    const NEEDS:   &'static [Role]    = &[];
}
```

### Why `Palette` is its own enum (not a string, not just `Role`)

- **Enum, not string.** Closed set, no typo risk, exhaustive `match` in the palette layout code. Strings buy nothing here.
- **Slice, not single value.** Most atoms have `&[Palette::X]` with one entry, but the slice shape future-proofs cross-listing (a `RangeProvider` modifier could appear under both Acquirers and Modifiers).
- **Distinct from `Role`.** They overlap today but the jobs differ: `Role` describes pipeline semantics (used for `needs` validation); `Palette` describes UI grouping. Identity atoms (`Name`, `Color`) fill no role yet need a palette home. Lifecycle atoms (`Health`, `BlocksNav`) are structural, not pipeline stages. Keeping them separate lets either evolve independently — subdivide `Structure` into `Defense` and `Economy`, or split `Modifier` into `ProjectileModifier` / `AcquirerModifier`, without touching role logic.

---

## Design principles

1. **Single-screen build view.** Pick deliverer + a couple payloads + tweak sliders. No nested menus.
2. **Instant playable preview.** In-progress tower placed in a sandbox arena, watching it work.
3. **Hard to build something boring.** Smart defaults; you only edit what you care about.
4. **Hard to build something broken.** Caps and `needs` rules prevent 1-cost god towers without lecturing.
5. **Easy to share and remix.** A recipe is a small data blob — copy/paste to a friend.
6. **All six built-ins expressible as recipes.** If the model can't express what we already have, the model is wrong.

---

## Stuff I'm not sure about — would love your take

1. **One Deliverer per tower, or stackable?** A tower that's both turret _and_ aura is interesting but doubles balance and UI complexity. Lean toward one for v1.
2. **Status-effect catalog at v1.** Slow only? Add burn/stun/freeze? Each adds expression and balance surface.
3. **Player-defined upgrade curves vs one fixed curve.** Lean fixed for v1.
4. **Targeting priority — recipe-level or runtime knob?** Today it's runtime. Could be both.
5. **Visual customization depth.** Color + icon library for v1, full particle editor later?
6. **Live preview.** Sandboxed wave of test enemies during edit? Probably yes.
7. **Sharing format.** Lean toward small text recipe — copy-pasteable.
8. **Build-view first or timeline-view first?** Timeline is the cooler idea but harder. Build view is the safe MVP. They ultimately live side-by-side.
9. **Does Splash spawn an inner Acquirer?** The model implies yes (splash = "Acquirer at impact point + payloads"). That's elegant and lets us reuse falloff curves on splash. But it might be over-clever.
10. **What's the right granularity for `Modifier`?** Homing vs ProjectileWithHoming as a deliverer; DamageFalloff as a separate modifier vs baked into ChainWalk. Granularity affects what the editor surfaces. **One case resolved:** lock-time is a `LockOn` Modifier on Acquirer, *not* a `ChargeUp` Trigger or a `LockOnTarget` Acquirer variant. Reason: keeping every Trigger pure (no back-channel from Acquirer) preserves the forward-only pipeline, and `LockOn` then composes with `Cooldown` to express weapons the dedicated-Trigger version couldn't. This sets a precedent: prefer Modifiers over baked-in Acquirer/Deliverer variants when the behavior is orthogonal to the host.
11. **Which of the four new roles are v1 scope?** `Accumulator` and event-driven `Trigger` are relatively self-contained. `Summoner` (spawned sub-entities) and `TowerNetwork` (inter-tower links) are each their own significant system. `Behavior` payloads (Confuse, PathAttract, PathLoop) require write access to pathfinding, which is the most invasive. Suggesting: implement Accumulator + OnWorldEvent first, gate the others behind a feature flag.
12. **Trap towers and nested pipelines.** `Trap` as a Deliverer means a spawned entity that itself has a Trigger + Payload. Does the editor let players author that nested pipeline? Or do we treat trap entities as atomic (not editable, just selectable)? Atomic is much simpler for v1.
13. **TowerNetwork UX: orphaned towers.** A `NetworkNode` tower does nothing alone. The editor needs a way to show "this tower needs a neighbor" — either a preview of where links would form during placement, or a warning state when placed in isolation. How much does the editor take responsibility for that?
14. **`Behavior` payloads and pathfinding ownership.** `PathAttract`, `PathLoop`, and `Teleport` all need to write to the nav grid or enemy movement state. That's a different integration surface than every other payload. Is `Behavior` actually a separate system tier rather than just another payload type?
15. **`FormulaPayload` expression language.** If `DirectDamage(formula)` is a real thing, what does the designer see? A text field is too raw; a slider won't cover `boss_hp * 0.1`. Could be a set of pre-defined formula templates ("% of current HP", "scales with charge level") rather than a free expression.
16. **`GameSpeedSlow` as a tower payload — is it fun?** Slowing global time is a powerful feel moment but it affects all other towers too (they also slow down). Could be intentional design space or could be deeply confusing. Worth playtesting very early if it's in scope.
17. **Modifier "modifies a sibling" axis.** `Homing modifies Projectile` isn't really fills/needs — it's "this atom requires a sibling atom of a specific component type on the same tower." Add a third axis (e.g. `MODIFIES: &'static [TypeId]`) to the `Atom` trait, or fold it into `NEEDS` with a Role-vs-Component disambiguation? The first is honest about the relationship; the second keeps the trait surface smaller. Related: open questions #9 and #12 push toward atoms whose parameters are themselves recipes (Splash → inner Acquirer; Trap → nested pipeline) — if either is in scope, the catalog gains a "subrecipe" parameter type and the editor's parameter UI has to recurse.

---

## Things I haven't started thinking about

- Map editor — hand-painted vs parametric? Same axis-discovery exercise will be needed.
- Enemy editor — likely the same component-and-interface model with a different role catalog.
- Recipe validation: "this doesn't make sense" vs "this is allowed but bad." Where's the line?
- Balance: does the editor cost-rate the recipe, or do players just declare a cost?
- Discoverability for shared recipes (browser? gallery? in-game vending machine?).

---

If any of this sparks something — "you forgot X," "Y is way more important than Z," "what if towers could do W" — please yell.
