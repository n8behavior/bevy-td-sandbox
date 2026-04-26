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

- `Trigger` — decides _when_ to act (cooldown, every-tick, charge-up…).
- `Acquirer` — produces a list of target enemies (single, all-in-range, chain-walk…).
- `Deliverer` — turns "we triggered with these targets" into actual hits (instant, projectile, beam…).
- `Payload` — does something to a hit enemy (damage, slow, burn…).
- `RangeProvider` — declares the radius for `Acquirer` and aura systems.
- `Modifier` — alters another component's behavior (homing on a projectile, falloff on a chain…).
- `Passive` — runs continuously on the tower itself, not on hits (scrap collector, neighbor buff…).
- `Lifecycle` — placement, destruction, repair behavior.

### Components (a starter catalog)

**Triggers**

- `Cooldown(secs)` — fills `Trigger`
- `ContinuousTick` — fills `Trigger` (fires every frame)
- `ChargeUp(secs)` — fills `Trigger` (must hold target)

**Acquirers**

- `SingleTarget(mode)` — fills `Acquirer`; mode is closest / lowest-hp / highest-hp / furthest-along
- `AllInRange` — fills `Acquirer`; needs `RangeProvider`
- `ChainWalk(arc_range, hop_limit)` — fills `Acquirer`; starts from a single target and walks outward

**Deliverers**

- `Projectile(speed, trail)` — fills `Deliverer`; needs `Acquirer` that yields a single target
- `Hitscan` — fills `Deliverer`; instant
- `Aura` — fills `Deliverer`; no-op (the Acquirer already touched everyone)

**Payloads**

- `DirectDamage(amount)`
- `Splash(radius, damage)` — implicitly creates an inner Acquirer at impact point
- `Slow(factor, duration)`
- `Pull(speed)`
- `Burn(dps, duration)` _(future)_
- `Stun(duration)` _(future)_
- `Knockback(force)` _(future)_

**Modifiers**

- `Homing` — modifies `Projectile`
- `DamageFalloff(per_hop)` — modifies `ChainWalk`
- `RangeFalloff(curve)` — modifies aura payload intensity by distance
- `AimPrecision(tolerance, rotation_speed)` — modifies `Projectile`

**Passives**

- `ScrapCollector(range)`
- `NeighborBuff(stat, amount)` _(future)_
- `PassiveIncome(rate)` _(future)_

**Lifecycle / structural**

- `BlocksNav`
- `Health(hp_curve)` — destructibility + degradation
- `Cost(scrap)`

**Identity**

- `Name`, `Color`, `Hotkey`, `Icon`, `Label`

### Compatibility rules emerge from `needs`

```
Projectile      needs: Trigger, Acquirer(single)
Hitscan         needs: Trigger, Acquirer(single)
Aura            needs: Trigger(continuous-or-cooldown), Acquirer(all-in-range)
ChainWalk       needs: another Acquirer to seed the first target
AimPrecision    needs: Projectile
DamageFalloff   needs: ChainWalk
Splash          needs: Deliverer (any)
RangeFalloff    needs: Aura
ScrapCollector  needs: nothing — purely passive
```

The editor greys out the incompatible options as you build. No big lookup table — each component declares its own needs.

---

## Worked examples (the test suite)

Decomposing our six built-ins. If the model can't express these, it's broken. I've also tossed in a couple of "what if" recipes to stretch it.

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

TarPit          = ContinuousTick + AllInRange + Range(70)
                + Aura + Slow(0.4, 0.5) + RangeFalloff(linear)
                + ScrapCollector(30)
                + Identity{name="TarPit", color=brown, cost=75}
                  (no Health, no BlocksNav)

ScrapMagnet     = ContinuousTick + AllInRange + Range(90)
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

InfernoBeam     = Cooldown(0.0) + SingleTarget(closest) + Range(100)
                + Beam + DirectDamage(30/s) + Burn(5/s, 2s)
                  (Beam = new Deliverer to add)

GravityWell     = ContinuousTick + AllInRange + Range(60)
                + Aura + Pull(40) + RangeFalloff(quadratic)
                  (no damage; pure crowd control)
```

If we can express _all_ of these from a small set of components, we're in good shape.

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
10. **What's the right granularity for `Modifier`?** Homing vs ProjectileWithHoming as a deliverer; DamageFalloff as a separate modifier vs baked into ChainWalk. Granularity affects what the editor surfaces.

---

## Things I haven't started thinking about

- Map editor — hand-painted vs parametric? Same axis-discovery exercise will be needed.
- Enemy editor — likely the same component-and-interface model with a different role catalog.
- Recipe validation: "this doesn't make sense" vs "this is allowed but bad." Where's the line?
- Balance: does the editor cost-rate the recipe, or do players just declare a cost?
- Discoverability for shared recipes (browser? gallery? in-game vending machine?).

---

If any of this sparks something — "you forgot X," "Y is way more important than Z," "what if towers could do W" — please yell.
