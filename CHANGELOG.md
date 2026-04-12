# Changelog

## v0.3.3

- Automate release process into single just release command
- Disable Bevy default features to trim unused dependencies


## v0.3.2 — Release Process

### Infrastructure

- **Release flight-check recipe** — `just release vX.Y.Z` validates clean tree, tag, version, changelog, and CI before creating GitHub release.
- **Local web testing** — `just web-serve` serves web build at localhost:4000 for browser testing.
- **Release workflow trigger** — Changed from tag push to GitHub release publish, gating itch.io deploys behind explicit human action.
- **RELEASING.md** — Documented pre-release, cut, and post-release checklists.

## v0.3.1 — WASM Terrain Fix

### Bug Fixes

- **Fixed terrain not generating on WASM/web builds** — `generate_terrain` queried for the `OrdinalGrid` entity before `spawn_nav_grid`'s deferred commands were applied. The single-threaded WASM executor scheduled these without a sync point, causing a silent early return. Fix: added explicit `.after(spawn_nav_grid)` ordering.

## v0.3.0 — Codebase Quality Pass

### Bug Fixes

- **Fixed enemy freeze on mid-game tower placement** (#66) — Enemies froze when towers were placed during the defend phase. Root cause: `recalculate_enemy_paths` wasn't clearing stale path state correctly.
- **Fixed camera pan jump when cursor re-enters window** (#25) — Camera snapped unexpectedly when the mouse pointer moved back into the window.
- **Fixed negative grid coords in pile visual update** (#46) — Guard against underflow when updating pile visuals near grid edges.
- **Fixed leaked particle and AoE burst entities** (#51) — Particle and burst entities were never despawned, accumulating indefinitely.
- **Balanced first boss wave to be winnable** (#13) — Tuned boss wave parameters so the first boss encounter is survivable.
- **Fixed hardcoded sell refund and tier index bounds** (#15) — Sell refund now uses actual tower cost; added bounds check on upgrade tier index.

### UI Polish

- **Targeting legend, ESC quit confirmation, HUD toggle** (#12, #70, #72) — Added a targeting-mode legend overlay, ESC key quit confirmation dialog, and a key to toggle the HUD.

### Refactors

- **Codebase-wide pure-function extraction** — Extracted testable pure functions from ECS systems across every module: enemy, tower, economy, wave, particles, audio, camera, terrain, pathfinding, projectile, shader, grid, stats, UI, endless, and pile.
- **Data-driven enemy stats** (#33) — Consolidated `EnemyType` stats into a single data-driven lookup table.
- **Event-based audio** (#19, #20, #21) — Refactored audio to trigger from events/observers instead of inline system logic.
- **Entity builders in test helpers** (#26) — Builder pattern for spawning test entities, reducing boilerplate.
- **Shared 2D rotation helpers** — Extracted common rotation logic, adopted official Bevy pattern.
- **Named constants, dead code removal, DRY helpers** — Across all modules: magic numbers replaced with named constants, unused code deleted, duplicated helpers consolidated.

### Infrastructure

- **Justfile build system** — Added `just run/check/clippy/test/ci/doc` recipes with dynamic linking for fast recompiles.
- **Upgraded rand 0.9 → 0.10, getrandom 0.3 → 0.4**
- **CI fixes** — Consolidated duplicate workflows, fixed doctest linker errors.
- **README updates** — Added Fedora/Arch Linux prerequisites and tracing tips.

### Testing

- **364 tests** (up from 62) — Unit and integration test coverage added to every module.

## v0.2.2 — Pathing Fix & Event Cleanup

### Bug Fixes

- **Fixed tower placement not changing enemy pathing** (#11) — Enemies walked through towers placed mid-game. Root cause: `recalculate_enemy_paths` inserted a new `Pathfind` but left stale `Path` and `NextPos` components. bevy_northstar's `next_position` system immediately popped the next waypoint from the old path. Fix: remove both `Path` and `NextPos` before re-inserting `Pathfind`.
- **Fixed double tower destruction sound** (#14) — `brute_attack_towers` no longer plays `tower_destroyed`; `on_tower_becomes_rubble` owns it.

### Refactors

- **Simplified enemy lifecycle** — `EnemyState` reduced to just `Approaching | Fleeing`. Removing the `Enemy` component IS the death transition; `DeathAnimation` serves as the implicit corpse marker. `check_wave_complete` uses `enemies.is_empty()` (O(1)) instead of iterating all enemies.
- **Extracted events** — `WaveComplete` event separates detection from outcome handling. `EnemyDied` and `EnemyEscaped` events with observers for sound, particles, and screen shake.
- **Deduplicated targeting** — Shared `best_target_from` function replaces duplicated `find_chain_target` logic.

### Infrastructure

- **62 tests** — added 2 regression tests for #11; net decrease from 68 due to dead-state tests removed when the enemy lifecycle was simplified

## v0.2.1 — State Machines & Game Over Fix

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
