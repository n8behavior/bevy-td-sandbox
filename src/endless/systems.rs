use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::prelude::IndexedRandom;
use rand::{Rng, RngExt};

use crate::common::constants::GridConfig;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Enemy, EnemyType};
use crate::enemy::systems::spawn_enemy;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::nearest_pile_cell;
use crate::states::{GameState, PlayPhase};
use crate::wave::resources::BossTrait;

use super::resources::EndlessSpawner;

// ---------------------------------------------------------------------------
// Spawn-interval escalation
// ---------------------------------------------------------------------------

/// Starting spawn cadence (seconds between spawns).
const BASE_SPAWN_INTERVAL: f32 = 1.5;

/// How much faster spawns get per 10 s of elapsed game time.
/// Interval decreases linearly: `BASE - (elapsed / 10) * RATE`.
const SPAWN_INTERVAL_DECAY_RATE: f32 = 0.02;

/// Hard floor — spawns never come faster than this (seconds).
const MIN_SPAWN_INTERVAL: f32 = 0.25;

// ---------------------------------------------------------------------------
// Difficulty scaling (per-minute multipliers)
// ---------------------------------------------------------------------------

/// Enemy HP multiplier growth per minute: `1.0 + minutes * RATE`.
const HEALTH_SCALING_PER_MINUTE: f32 = 0.15;

/// Enemy speed multiplier growth per minute: `1.0 + minutes * RATE`.
const SPEED_SCALING_PER_MINUTE: f32 = 0.05;

// ---------------------------------------------------------------------------
// Enemy-type thresholds & probabilities
// ---------------------------------------------------------------------------

/// Elapsed seconds before Runners can appear.
const RUNNER_UNLOCK_SECS: f32 = 60.0;

/// Base chance for a Runner once unlocked.
const RUNNER_BASE_CHANCE: f32 = 0.30;

/// Maximum Runner chance (cap).
const RUNNER_MAX_CHANCE: f32 = 0.40;

/// Seconds over which Runner chance ramps from base to cap.
const RUNNER_RAMP_SECS: f32 = 300.0;

/// Additional chance gained across the full ramp window.
const RUNNER_RAMP_AMOUNT: f32 = 0.10;

/// Elapsed seconds before Brutes can appear.
const BRUTE_UNLOCK_SECS: f32 = 150.0;

/// Base chance for a Brute once unlocked.
const BRUTE_BASE_CHANCE: f32 = 0.15;

/// Maximum Brute chance (cap).
const BRUTE_MAX_CHANCE: f32 = 0.25;

/// Seconds over which Brute chance ramps from base to cap.
const BRUTE_RAMP_SECS: f32 = 600.0;

/// Additional chance gained across the full ramp window.
const BRUTE_RAMP_AMOUNT: f32 = 0.10;

/// Elapsed seconds before Bosses can appear.
const BOSS_UNLOCK_SECS: f32 = 300.0;

/// Starting boss spawn chance at [`BOSS_UNLOCK_SECS`].
const BOSS_BASE_CHANCE: f32 = 0.02;

/// Additional boss chance gained per minute beyond [`BOSS_UNLOCK_SECS`].
const BOSS_CHANCE_PER_MINUTE: f32 = 0.005;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Skip the Building phase and go straight to Defending in Endless mode.
pub fn skip_building_phase(mut next_phase: ResMut<NextState<PlayPhase>>) {
    next_phase.set(PlayPhase::Defending);
}

/// Initialize the endless spawner resource when Defending phase begins.
pub fn init_endless(mut commands: Commands) {
    commands.insert_resource(EndlessSpawner {
        elapsed_time: 0.0,
        spawn_timer: Timer::from_seconds(BASE_SPAWN_INTERVAL, TimerMode::Repeating),
        enemies_spawned: 0,
    });
}

/// Continuously spawn enemies with time-based difficulty scaling.
///
/// Runs in [`FixedUpdate`] (matching wave mode) for deterministic,
/// frame-rate-independent spawn timing.
pub fn endless_spawn_enemies(
    mut commands: Commands,
    mut spawner: ResMut<EndlessSpawner>,
    time: Res<Time>,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<OrdinalGrid>>,
    edge_cells: Res<EdgeCells>,
    pile_state: Res<PileState>,
) {
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };

    if edge_cells.0.is_empty() || pile_state.cells.is_empty() {
        return;
    }

    spawner.elapsed_time += time.delta_secs();
    let elapsed = spawner.elapsed_time;

    let new_interval = compute_spawn_interval(elapsed);
    let current_duration = spawner.spawn_timer.duration().as_secs_f32();
    if (new_interval - current_duration).abs() > 0.01 {
        spawner
            .spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(new_interval));
    }

    spawner.spawn_timer.tick(time.delta());
    if !spawner.spawn_timer.just_finished() {
        return;
    }

    let mut rng = rand::rng();

    let (enemy_type, boss_trait) = pick_enemy_type(elapsed, &mut rng);
    let (health_mult, speed_mult) = compute_scaling(elapsed);

    let spawn_pos = *edge_cells.0.choose(&mut rng).unwrap();
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    spawn_enemy(
        &mut commands,
        enemy_type,
        spawn_pos,
        goal_pos,
        grid_entity,
        health_mult,
        speed_mult,
        &config,
        boss_trait,
    );

    spawner.enemies_spawned += 1;
}

/// Game over in endless mode: truly bankrupt with no recovery possible.
///
/// Unlike classic mode (which ends when the spawn queue empties), endless
/// spawning is infinite. Game over only triggers when the pile is empty
/// **and** no recovery is possible (no drops to collect, no enemies whose
/// kills could produce drops).
pub fn endless_check_game_over(
    pile_scrap: Res<PileScrap>,
    drops: Query<(), With<ScrapDrop>>,
    enemies: Query<(), With<Enemy>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if pile_scrap.amount > 0 {
        return;
    }
    if can_recover(drops.is_empty(), enemies.is_empty()) {
        return;
    }
    next_state.set(GameState::GameOver);
}

// ---------------------------------------------------------------------------
// Pure helpers (public for unit testing)
// ---------------------------------------------------------------------------

/// Whether the player can still recover scrap from the field.
///
/// Returns `true` when drops remain to collect or enemies remain whose
/// kills could produce new drops.
pub fn can_recover(drops_empty: bool, enemies_empty: bool) -> bool {
    !drops_empty || !enemies_empty
}

/// Spawn-interval curve: starts at [`BASE_SPAWN_INTERVAL`] (1.5 s) and
/// decays by [`SPAWN_INTERVAL_DECAY_RATE`] (0.02 s) per 10 s of elapsed
/// game time, floored at [`MIN_SPAWN_INTERVAL`] (0.25 s).
///
/// Design intent: gradual pressure ramp — early game is relaxed (one spawn
/// every 1.5 s), late game is frantic (one every 0.25 s). The floor
/// prevents the game from becoming physically unplayable.
pub fn compute_spawn_interval(elapsed: f32) -> f32 {
    (BASE_SPAWN_INTERVAL - (elapsed / 10.0) * SPAWN_INTERVAL_DECAY_RATE).max(MIN_SPAWN_INTERVAL)
}

/// Per-minute difficulty multipliers for HP and speed.
///
/// Returns `(health_mult, speed_mult)`:
/// - Health: `1.0 + minutes × 0.15`  (+15 % per minute)
/// - Speed:  `1.0 + minutes × 0.05`  (+5 % per minute)
///
/// Health scales faster than speed so enemies become meatier without
/// outrunning tower coverage too quickly.
pub fn compute_scaling(elapsed: f32) -> (f32, f32) {
    let minutes = elapsed / 60.0;
    let health_mult = 1.0 + minutes * HEALTH_SCALING_PER_MINUTE;
    let speed_mult = 1.0 + minutes * SPEED_SCALING_PER_MINUTE;
    (health_mult, speed_mult)
}

/// Probability of spawning a Runner at the given elapsed time.
///
/// Returns 0.0 before [`RUNNER_UNLOCK_SECS`] (60 s), then ramps linearly
/// from [`RUNNER_BASE_CHANCE`] (30 %) to [`RUNNER_MAX_CHANCE`] (40 %) over
/// [`RUNNER_RAMP_SECS`] (300 s).
pub fn runner_chance(elapsed: f32) -> f32 {
    if elapsed < RUNNER_UNLOCK_SECS {
        return 0.0;
    }
    (RUNNER_BASE_CHANCE + (elapsed - RUNNER_UNLOCK_SECS) / RUNNER_RAMP_SECS * RUNNER_RAMP_AMOUNT)
        .min(RUNNER_MAX_CHANCE)
}

/// Probability of spawning a Brute at the given elapsed time.
///
/// Returns 0.0 before [`BRUTE_UNLOCK_SECS`] (150 s), then ramps linearly
/// from [`BRUTE_BASE_CHANCE`] (15 %) to [`BRUTE_MAX_CHANCE`] (25 %) over
/// [`BRUTE_RAMP_SECS`] (600 s).
pub fn brute_chance(elapsed: f32) -> f32 {
    if elapsed < BRUTE_UNLOCK_SECS {
        return 0.0;
    }
    (BRUTE_BASE_CHANCE + (elapsed - BRUTE_UNLOCK_SECS) / BRUTE_RAMP_SECS * BRUTE_RAMP_AMOUNT)
        .min(BRUTE_MAX_CHANCE)
}

/// Probability of spawning a Boss at the given elapsed time.
///
/// Returns 0.0 before [`BOSS_UNLOCK_SECS`] (300 s), then starts at
/// [`BOSS_BASE_CHANCE`] (2 %) and grows by [`BOSS_CHANCE_PER_MINUTE`]
/// (0.5 %) per minute beyond the unlock point. Unlike Runner/Brute, there
/// is no cap — boss chance grows indefinitely to keep late-game dangerous.
pub fn boss_chance(elapsed: f32) -> f32 {
    if elapsed < BOSS_UNLOCK_SECS {
        return 0.0;
    }
    BOSS_BASE_CHANCE + (elapsed - BOSS_UNLOCK_SECS) / 60.0 * BOSS_CHANCE_PER_MINUTE
}

/// Pick an enemy type based on elapsed time with escalating mix.
///
/// Thresholds are evaluated top-down — **Boss → Brute → Runner → Shambler**.
/// Each tier rolls against its time-dependent probability; the first hit
/// wins. If nothing hits, a Shambler is returned as the default.
pub fn pick_enemy_type(elapsed: f32, rng: &mut impl Rng) -> (EnemyType, Option<BossTrait>) {
    let boss_traits = [
        BossTrait::Regeneration,
        BossTrait::Armor,
        BossTrait::Splitting,
    ];

    // Boss: appears after 300 s, starts at 2 % chance, +0.5 % per minute.
    let bc = boss_chance(elapsed);
    if bc > 0.0 && rng.random_range(0.0..1.0) < bc {
        let trait_val = *boss_traits.choose(rng).unwrap();
        return (EnemyType::Boss, Some(trait_val));
    }

    // Brute: appears after 150 s, starts at 15 % chance, caps at 25 %.
    let brc = brute_chance(elapsed);
    if brc > 0.0 && rng.random_range(0.0..1.0) < brc {
        return (EnemyType::Brute, None);
    }

    // Runner: appears after 60 s, starts at 30 % chance, caps at 40 %.
    let rc = runner_chance(elapsed);
    if rc > 0.0 && rng.random_range(0.0..1.0) < rc {
        return (EnemyType::Runner, None);
    }

    (EnemyType::Shambler, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;

    /// Deterministic RNG replacing `StepRng` (removed in rand 0.10).
    struct MockRng {
        value: u64,
        step: u64,
    }

    impl MockRng {
        fn new(value: u64, step: u64) -> Self {
            Self { value, step }
        }
    }

    impl rand::TryRng for MockRng {
        type Error = Infallible;

        fn try_next_u32(&mut self) -> Result<u32, Infallible> {
            Ok(self.try_next_u64()? as u32)
        }

        fn try_next_u64(&mut self) -> Result<u64, Infallible> {
            let v = self.value;
            self.value = self.value.wrapping_add(self.step);
            Ok(v)
        }

        fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Infallible> {
            rand::rand_core::utils::fill_bytes_via_next_word(dst, || self.try_next_u64())
        }
    }

    // -----------------------------------------------------------------------
    // can_recover
    // -----------------------------------------------------------------------

    #[test]
    fn no_recovery_when_both_empty() {
        assert!(!can_recover(true, true));
    }

    #[test]
    fn recovery_possible_with_drops() {
        assert!(can_recover(false, true));
    }

    #[test]
    fn recovery_possible_with_enemies() {
        assert!(can_recover(true, false));
    }

    #[test]
    fn recovery_possible_with_both() {
        assert!(can_recover(false, false));
    }

    // -----------------------------------------------------------------------
    // compute_spawn_interval
    // -----------------------------------------------------------------------

    #[test]
    fn spawn_interval_starts_at_base() {
        assert!((compute_spawn_interval(0.0) - BASE_SPAWN_INTERVAL).abs() < f32::EPSILON);
    }

    #[test]
    fn spawn_interval_decreases_over_time() {
        assert!(compute_spawn_interval(100.0) < compute_spawn_interval(0.0));
    }

    #[test]
    fn spawn_interval_never_below_floor() {
        assert!((compute_spawn_interval(10_000.0) - MIN_SPAWN_INTERVAL).abs() < f32::EPSILON);
    }

    #[test]
    fn spawn_interval_hits_floor_at_expected_time() {
        // Floor reached when BASE - (t/10)*RATE = MIN
        //   → t = (BASE - MIN) * 10 / RATE
        let t_floor = (BASE_SPAWN_INTERVAL - MIN_SPAWN_INTERVAL) * 10.0 / SPAWN_INTERVAL_DECAY_RATE;
        assert!((compute_spawn_interval(t_floor) - MIN_SPAWN_INTERVAL).abs() < 0.001);
        assert!(compute_spawn_interval(t_floor - 10.0) > MIN_SPAWN_INTERVAL);
    }

    // -----------------------------------------------------------------------
    // compute_scaling
    // -----------------------------------------------------------------------

    #[test]
    fn scaling_at_zero_is_baseline() {
        let (hp, spd) = compute_scaling(0.0);
        assert!((hp - 1.0).abs() < f32::EPSILON);
        assert!((spd - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scaling_at_one_minute() {
        let (hp, spd) = compute_scaling(60.0);
        assert!((hp - 1.15).abs() < 0.001);
        assert!((spd - 1.05).abs() < 0.001);
    }

    #[test]
    fn scaling_at_five_minutes() {
        let (hp, spd) = compute_scaling(300.0);
        assert!((hp - 1.75).abs() < 0.001);
        assert!((spd - 1.25).abs() < 0.001);
    }

    // -----------------------------------------------------------------------
    // Threshold predicates (pure, no RNG)
    // -----------------------------------------------------------------------

    #[test]
    fn runner_chance_zero_before_unlock() {
        assert!((runner_chance(59.9)).abs() < f32::EPSILON);
    }

    #[test]
    fn runner_chance_at_unlock() {
        assert!((runner_chance(60.0) - RUNNER_BASE_CHANCE).abs() < 0.001);
    }

    #[test]
    fn runner_chance_caps() {
        assert!((runner_chance(10_000.0) - RUNNER_MAX_CHANCE).abs() < f32::EPSILON);
    }

    #[test]
    fn brute_chance_zero_before_unlock() {
        assert!((brute_chance(149.9)).abs() < f32::EPSILON);
    }

    #[test]
    fn brute_chance_at_unlock() {
        assert!((brute_chance(150.0) - BRUTE_BASE_CHANCE).abs() < 0.001);
    }

    #[test]
    fn brute_chance_caps() {
        assert!((brute_chance(10_000.0) - BRUTE_MAX_CHANCE).abs() < f32::EPSILON);
    }

    #[test]
    fn boss_chance_zero_before_unlock() {
        assert!((boss_chance(299.9)).abs() < f32::EPSILON);
    }

    #[test]
    fn boss_chance_at_unlock() {
        assert!((boss_chance(300.0) - BOSS_BASE_CHANCE).abs() < 0.001);
    }

    #[test]
    fn boss_chance_increases_over_time() {
        let c300 = boss_chance(300.0);
        let c600 = boss_chance(600.0);
        assert!(c600 > c300);
        // At 600 s: 0.02 + (300/60)*0.005 = 0.02 + 0.025 = 0.045
        assert!((c600 - 0.045).abs() < 0.001);
    }

    // -----------------------------------------------------------------------
    // pick_enemy_type — deterministic tests via MockRng
    // -----------------------------------------------------------------------

    /// MockRng that always returns 0.0 from `random_range` — every roll hits.
    fn always_hit_rng() -> MockRng {
        MockRng::new(0, 0)
    }

    /// MockRng that always returns ~1.0 from `random_range` — every roll misses.
    fn always_miss_rng() -> MockRng {
        MockRng::new(u64::MAX, 0)
    }

    // -- threshold: 0 s (only Shambler) --

    #[test]
    fn at_0s_always_shambler() {
        let (ty, boss) = pick_enemy_type(0.0, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Shambler);
        assert!(boss.is_none());
    }

    // -- threshold: 60 s (Runner unlocks) --

    #[test]
    fn at_60s_runner_on_hit() {
        let (ty, boss) = pick_enemy_type(60.0, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Runner);
        assert!(boss.is_none());
    }

    #[test]
    fn at_59s_no_runner() {
        let (ty, _) = pick_enemy_type(59.9, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Shambler);
    }

    #[test]
    fn at_60s_shambler_on_miss() {
        let (ty, _) = pick_enemy_type(60.0, &mut always_miss_rng());
        assert_eq!(ty, EnemyType::Shambler);
    }

    // -- threshold: 150 s (Brute unlocks) --

    #[test]
    fn at_150s_brute_on_hit() {
        let (ty, boss) = pick_enemy_type(150.0, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Brute);
        assert!(boss.is_none());
    }

    #[test]
    fn at_149s_no_brute() {
        let (ty, _) = pick_enemy_type(149.9, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Runner);
    }

    #[test]
    fn at_150s_shambler_on_miss() {
        let (ty, _) = pick_enemy_type(150.0, &mut always_miss_rng());
        assert_eq!(ty, EnemyType::Shambler);
    }

    // -- threshold: 300 s (Boss unlocks) --

    #[test]
    fn at_300s_boss_on_hit() {
        let (ty, boss) = pick_enemy_type(300.0, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Boss);
        assert!(boss.is_some());
    }

    #[test]
    fn at_299s_no_boss() {
        let (ty, boss) = pick_enemy_type(299.9, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Brute);
        assert!(boss.is_none());
    }

    #[test]
    fn at_300s_shambler_on_miss() {
        let (ty, _) = pick_enemy_type(300.0, &mut always_miss_rng());
        assert_eq!(ty, EnemyType::Shambler);
    }

    // -- boss trait distribution --

    #[test]
    fn boss_trait_is_valid_variant() {
        let valid = [
            BossTrait::Regeneration,
            BossTrait::Armor,
            BossTrait::Splitting,
        ];
        for seed in 0..20u64 {
            let mut rng = MockRng::new(seed, seed.wrapping_add(1));
            let (ty, boss) = pick_enemy_type(600.0, &mut rng);
            if ty == EnemyType::Boss {
                assert!(
                    valid.contains(&boss.unwrap()),
                    "unexpected boss trait: {boss:?}",
                );
            }
        }
    }

    // -- cascade priority --

    #[test]
    fn boss_takes_priority_over_brute_and_runner() {
        let (ty, _) = pick_enemy_type(300.0, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Boss);
    }

    #[test]
    fn brute_takes_priority_over_runner() {
        let (ty, _) = pick_enemy_type(150.0, &mut always_hit_rng());
        assert_eq!(ty, EnemyType::Brute);
    }
}
