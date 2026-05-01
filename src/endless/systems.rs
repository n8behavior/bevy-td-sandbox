use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::prelude::IndexedRandom;
use rand::{Rng, RngExt};

use crate::common::constants::GridConfig;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Enemy, EnemyRegistry};
use crate::enemy::spawn::spawn_from_blueprint;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::nearest_pile_cell;
use crate::states::{GameState, PlayPhase};

use super::resources::EndlessSpawner;

// ---------------------------------------------------------------------------
// Spawn-interval escalation
// ---------------------------------------------------------------------------

const BASE_SPAWN_INTERVAL: f32 = 1.5;
const SPAWN_INTERVAL_DECAY_RATE: f32 = 0.02;
const MIN_SPAWN_INTERVAL: f32 = 0.25;

// ---------------------------------------------------------------------------
// Enemy-type thresholds & probabilities
// ---------------------------------------------------------------------------

const RUNNER_UNLOCK_SECS: f32 = 60.0;
const RUNNER_BASE_CHANCE: f32 = 0.30;
const RUNNER_MAX_CHANCE: f32 = 0.40;
const RUNNER_RAMP_SECS: f32 = 300.0;
const RUNNER_RAMP_AMOUNT: f32 = 0.10;

const BRUTE_UNLOCK_SECS: f32 = 150.0;
const BRUTE_BASE_CHANCE: f32 = 0.15;
const BRUTE_MAX_CHANCE: f32 = 0.25;
const BRUTE_RAMP_SECS: f32 = 600.0;
const BRUTE_RAMP_AMOUNT: f32 = 0.10;

const BOSS_UNLOCK_SECS: f32 = 300.0;
const BOSS_BASE_CHANCE: f32 = 0.02;
const BOSS_CHANCE_PER_MINUTE: f32 = 0.005;

/// Seconds-per-wave used to translate elapsed time into a difficulty wave
/// number for the per-capability scaling observers. Endless mode doesn't
/// have discrete waves, so we synthesize one — every 30 s of survival
/// corresponds to one "wave" of difficulty growth.
const ENDLESS_SECS_PER_WAVE: f32 = 30.0;

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
pub fn endless_spawn_enemies(
    mut commands: Commands,
    mut spawner: ResMut<EndlessSpawner>,
    time: Res<Time>,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<OrdinalGrid>>,
    edge_cells: Res<EdgeCells>,
    pile_state: Res<PileState>,
    registry: Res<EnemyRegistry>,
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

    let blueprint_name = pick_enemy_type(elapsed, &mut rng);

    let spawn_pos = *edge_cells.0.choose(&mut rng).unwrap();
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    let Some(blueprint) = registry.lookup(blueprint_name) else {
        warn!("endless_spawn_enemies: blueprint '{blueprint_name}' not in registry");
        return;
    };

    let wave = endless_wave_for(elapsed);
    spawn_from_blueprint(
        &mut commands,
        blueprint,
        spawn_pos,
        goal_pos,
        grid_entity,
        &config,
        wave,
    );

    spawner.enemies_spawned += 1;
}

/// Game over in endless mode: pile is empty AND no recovery possible.
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
pub fn can_recover(drops_empty: bool, enemies_empty: bool) -> bool {
    !drops_empty || !enemies_empty
}

/// Spawn-interval curve: starts at [`BASE_SPAWN_INTERVAL`] (1.5 s) and
/// decays by [`SPAWN_INTERVAL_DECAY_RATE`] (0.02 s) per 10 s of elapsed
/// game time, floored at [`MIN_SPAWN_INTERVAL`] (0.25 s).
pub fn compute_spawn_interval(elapsed: f32) -> f32 {
    (BASE_SPAWN_INTERVAL - (elapsed / 10.0) * SPAWN_INTERVAL_DECAY_RATE).max(MIN_SPAWN_INTERVAL)
}

/// Synthesize a wave number from elapsed seconds. Used to feed the
/// per-capability scaling observers — every 30 s of survival counts as
/// one wave.
pub fn endless_wave_for(elapsed: f32) -> u32 {
    1 + (elapsed / ENDLESS_SECS_PER_WAVE) as u32
}

/// Probability of spawning a Runner at the given elapsed time.
pub fn runner_chance(elapsed: f32) -> f32 {
    if elapsed < RUNNER_UNLOCK_SECS {
        return 0.0;
    }
    (RUNNER_BASE_CHANCE + (elapsed - RUNNER_UNLOCK_SECS) / RUNNER_RAMP_SECS * RUNNER_RAMP_AMOUNT)
        .min(RUNNER_MAX_CHANCE)
}

/// Probability of spawning a Brute at the given elapsed time.
pub fn brute_chance(elapsed: f32) -> f32 {
    if elapsed < BRUTE_UNLOCK_SECS {
        return 0.0;
    }
    (BRUTE_BASE_CHANCE + (elapsed - BRUTE_UNLOCK_SECS) / BRUTE_RAMP_SECS * BRUTE_RAMP_AMOUNT)
        .min(BRUTE_MAX_CHANCE)
}

/// Probability of spawning a Boss at the given elapsed time.
pub fn boss_chance(elapsed: f32) -> f32 {
    if elapsed < BOSS_UNLOCK_SECS {
        return 0.0;
    }
    BOSS_BASE_CHANCE + (elapsed - BOSS_UNLOCK_SECS) / 60.0 * BOSS_CHANCE_PER_MINUTE
}

/// Pick an enemy blueprint name based on elapsed time.
///
/// Thresholds are evaluated top-down — **Boss → Brute → Runner → Shambler**.
/// Each tier rolls against its time-dependent probability; the first hit
/// wins. If nothing hits, a Shambler is returned as the default.
pub fn pick_enemy_type(elapsed: f32, rng: &mut impl Rng) -> &'static str {
    let bc = boss_chance(elapsed);
    if bc > 0.0 && rng.random_range(0.0..1.0) < bc {
        return "Boss";
    }

    let brc = brute_chance(elapsed);
    if brc > 0.0 && rng.random_range(0.0..1.0) < brc {
        return "Brute";
    }

    let rc = runner_chance(elapsed);
    if rc > 0.0 && rng.random_range(0.0..1.0) < rc {
        return "Runner";
    }

    "Shambler"
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

    // -- can_recover --

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

    // -- compute_spawn_interval --

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

    // -- endless_wave_for --

    #[test]
    fn endless_wave_starts_at_one() {
        assert_eq!(endless_wave_for(0.0), 1);
    }

    #[test]
    fn endless_wave_increments_every_30s() {
        assert_eq!(endless_wave_for(30.0), 2);
        assert_eq!(endless_wave_for(60.0), 3);
        assert_eq!(endless_wave_for(150.0), 6);
    }

    // -- threshold predicates --

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

    // -- pick_enemy_type --

    fn always_hit_rng() -> MockRng {
        MockRng::new(0, 0)
    }

    fn always_miss_rng() -> MockRng {
        MockRng::new(u64::MAX, 0)
    }

    #[test]
    fn at_0s_always_shambler() {
        assert_eq!(pick_enemy_type(0.0, &mut always_hit_rng()), "Shambler");
    }

    #[test]
    fn at_60s_runner_on_hit() {
        assert_eq!(pick_enemy_type(60.0, &mut always_hit_rng()), "Runner");
    }

    #[test]
    fn at_59s_no_runner() {
        assert_eq!(pick_enemy_type(59.9, &mut always_hit_rng()), "Shambler");
    }

    #[test]
    fn at_60s_shambler_on_miss() {
        assert_eq!(pick_enemy_type(60.0, &mut always_miss_rng()), "Shambler");
    }

    #[test]
    fn at_150s_brute_on_hit() {
        assert_eq!(pick_enemy_type(150.0, &mut always_hit_rng()), "Brute");
    }

    #[test]
    fn at_149s_no_brute() {
        assert_eq!(pick_enemy_type(149.9, &mut always_hit_rng()), "Runner");
    }

    #[test]
    fn at_300s_boss_on_hit() {
        assert_eq!(pick_enemy_type(300.0, &mut always_hit_rng()), "Boss");
    }

    #[test]
    fn at_299s_no_boss() {
        assert_eq!(pick_enemy_type(299.9, &mut always_hit_rng()), "Brute");
    }

    #[test]
    fn at_300s_shambler_on_miss() {
        assert_eq!(pick_enemy_type(300.0, &mut always_miss_rng()), "Shambler");
    }

    #[test]
    fn boss_takes_priority_over_brute_and_runner() {
        assert_eq!(pick_enemy_type(300.0, &mut always_hit_rng()), "Boss");
    }

    #[test]
    fn brute_takes_priority_over_runner() {
        assert_eq!(pick_enemy_type(150.0, &mut always_hit_rng()), "Brute");
    }
}
