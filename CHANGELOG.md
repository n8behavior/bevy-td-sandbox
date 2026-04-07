# Changelog

## Unreleased

### Refactors

- **Enemy state machine** — Replaced scattered marker components (`EnemyPhase`, `Dead`, `Dying`) with a unified `EnemyState` enum (Approaching, Fleeing, Dying, Dead). Systems use `is_alive()` helper instead of fragile `Without<>` chains. Eliminated 30+ query filters across 11 files.
- **Tower state machine** — Replaced `Placing` and `TowerRubble` markers with a `TowerState` enum (Placing, Active, Rubble). Systems use `is_operational()`/`is_placed()` helpers. Eliminated 22 query filters across 6 files.
- **Removed enemy wandering** — Enemies that reach an empty pile now flee to the edge instead of wandering aimlessly. Removed `SearchWander` component and related systems. Net -159 lines.

### Bug Fixes

- **Fixed game over deadlock** (#10) — Root cause was enemies wandering on an empty pile forever, blocking game over. With wandering removed, every enemy resolves (flees or dies). Game over check: pile=0, no drops, no stolen scrap, no alive enemies, no spawn queue.

### Infrastructure

- **68 tests** (up from 57), covering state machine helpers, game over corner cases, and all existing functionality

## v0.2.0 — Testing Foundation & Game Over Fix

### New Features

- **Per-tower targeting modes** — Closest, Lowest HP, Highest HP, and Furthest Along Path, selectable via a radial menu on each tower (#7)
- **Tower damage & repair** — Brutes now attack towers, degrading their effectiveness. Damaged towers fire slower; destroyed towers become rubble that can be repaired (#8)
- **Magnet upgrades** — Each tower has its own scrap collection radius upgrade path, 3 tiers (#6)
- **Endless mode** — Continuous spawning with scaling difficulty and run stats tracking (#4)
- **Sound & particles** — Procedural audio for all tower types, enemy deaths, and scrap collection. Death particles, boss screen shake, and spawn animations (#5)
- **Terrain types** — Rubble (impassable), puddles (slow), and radioactive zones (damage over time) (#3)

### Bug Fixes

- **Fixed game over deadlock** (#10) — When enemies stole all scrap and the pile emptied, remaining enemies would wander the empty pile forever and game over never triggered. This bug regressed 3 times because there were no tests to catch it. Simplified the bankruptcy check: pile empty + no scrap on ground = game over.
- **Fixed slow effects not applying** (#9) — System execution order wasn't guaranteed, so slow effects from tar pits and puddles sometimes didn't take effect.

### Infrastructure

- **57 tests** covering pile math, grid conversions, tower health, targeting, enemy stats, wave generation, and ECS systems (pile state, wave completion, game over, slow aura, turret targeting)
- **Extracted lib.rs** from binary crate to enable integration testing
- **Test helpers module** with headless app setup, mock sound assets, and pile initialization utilities
- **GitHub Actions CI** running clippy, tests, and format checks on every push and PR

## v0.1.0 — Initial Release

First playable release with core tower defense gameplay: open-field mazing, scavenging economy, wave system, 6 tower types, 4 enemy types, and grid-based pathfinding.
