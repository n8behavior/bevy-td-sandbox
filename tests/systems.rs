use bevy::prelude::*;
use bevy_td_sandbox::economy::components::ScrapDrop;
use bevy_td_sandbox::enemy::components::*;
use bevy_td_sandbox::pile::resources::{PileScrap, PileState};
use bevy_td_sandbox::pile::systems::update_pile_state;
use bevy_td_sandbox::states::{GameState, PlayPhase};
use bevy_td_sandbox::test_helpers::*;
use bevy_td_sandbox::tower::components::*;
use bevy_td_sandbox::tower::systems::slow_aura;
use bevy_td_sandbox::wave::resources::WaveManager;
use bevy_td_sandbox::wave::systems::{check_wave_complete, on_wave_complete};

// ---------------------------------------------------------------------------
// update_pile_state
// ---------------------------------------------------------------------------

#[test]
fn update_pile_state_recomputes_on_scrap_change() {
    let mut app = test_app();
    let config = test_grid_config();
    let center = config.center();
    app.insert_resource(PileScrap { amount: 0 });
    app.insert_resource(PileState {
        cells: std::collections::HashSet::new(),
        center,
        radius_tiles: 0.0,
        last_radius_int: 0,
    });
    app.insert_resource(config);
    app.add_systems(Update, update_pile_state);

    // First update — scrap is 0, should compute with radius 0 (center cell only).
    app.update();
    let pile_state = app.world().resource::<PileState>();
    assert_eq!(pile_state.cells.len(), 1);

    // Increase scrap significantly.
    app.world_mut().resource_mut::<PileScrap>().amount = 100_000;
    app.update();
    let pile_state = app.world().resource::<PileState>();
    assert!(pile_state.cells.len() > 1, "cells should grow with scrap");
    assert!(pile_state.radius_tiles > 0.0);
}

#[test]
fn update_pile_state_zero_scrap_minimal_cells() {
    let mut app = test_app();
    insert_empty_pile(&mut app, 0, test_grid_config());
    app.add_systems(Update, update_pile_state);

    app.update();
    let pile_state = app.world().resource::<PileState>();
    // Radius 0 still includes the center cell
    assert_eq!(pile_state.cells.len(), 1);
}

// ---------------------------------------------------------------------------
// check_wave_complete
// ---------------------------------------------------------------------------

/// Helper: set up a Defending-phase app with check_wave_complete and given scrap.
fn wave_app(scrap: u32) -> App {
    let mut app = test_app();
    app.init_state::<GameState>();
    app.add_sub_state::<PlayPhase>();

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<PlayPhase>>()
        .set(PlayPhase::Defending);
    app.update();

    app.insert_resource(PileScrap { amount: scrap });
    app.insert_resource(mock_sound_assets());
    app.insert_resource(WaveManager {
        current_wave: 0,
        waves: Vec::new(),
        spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        enemies_remaining: 0,
        spawn_queue: Vec::new(),
    });

    app.add_observer(on_wave_complete);
    app.add_systems(
        Update,
        check_wave_complete.run_if(in_state(PlayPhase::Defending)),
    );
    app
}

// ---------------------------------------------------------------------------
// check_wave_complete — game over integrated
// Game over is checked when wave resolves: pile=0 → GameOver, else → Building
// ---------------------------------------------------------------------------

fn game_state(app: &App) -> GameState {
    app.world().resource::<State<GameState>>().get().clone()
}

/// Wave resolved with scrap remaining → Building phase (next wave).
#[test]
fn wave_complete_with_scrap_transitions_to_building() {
    let mut app = wave_app(100);
    app.update();
    app.update();
    assert_eq!(
        *app.world().resource::<State<PlayPhase>>().get(),
        PlayPhase::Building
    );
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Wave resolved with pile empty → GameOver.
#[test]
fn wave_complete_with_empty_pile_triggers_game_over() {
    let mut app = wave_app(0);
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::GameOver);
}

/// Alive enemies prevent wave from resolving (regardless of pile state).
#[test]
fn wave_does_not_resolve_with_alive_enemies() {
    let mut app = wave_app(0);
    app.world_mut().spawn((Enemy, EnemyState::Approaching));
    app.update();
    app.update();
    // Wave not resolved — still Defending, no game over.
    assert_eq!(
        *app.world().resource::<State<PlayPhase>>().get(),
        PlayPhase::Defending
    );
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Drops on ground prevent wave from resolving.
#[test]
fn wave_does_not_resolve_with_drops() {
    let mut app = wave_app(0);
    app.world_mut().spawn(ScrapDrop {
        value: 10,
        lifetime: Timer::from_seconds(10.0, TimerMode::Once),
    });
    app.update();
    app.update();
    assert_eq!(
        *app.world().resource::<State<PlayPhase>>().get(),
        PlayPhase::Defending
    );
}

/// Dying/Dead enemies (DeathAnimation without Enemy) don't block wave resolution.
#[test]
fn wave_resolves_with_only_dying_and_dead_enemies() {
    let mut app = wave_app(0);
    // These entities have DeathAnimation but no Enemy component — they are corpses.
    app.world_mut().spawn(DeathAnimation {
        timer: Timer::from_seconds(0.3, TimerMode::Once),
    });
    app.world_mut().spawn(DeathAnimation {
        timer: Timer::from_seconds(0.3, TimerMode::Once),
    });
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::GameOver);
}

/// #10: boss steals last scrap, escapes (DeathAnimation + StolenScrap, no Enemy) → game over.
/// The stolen scrap is already lost — it was subtracted from the pile at
/// steal time and escaping doesn't return it.
#[test]
fn regression_10_escaped_enemy_with_stolen_scrap() {
    let mut app = wave_app(0);
    // Boss escaped: has DeathAnimation and StolenScrap but no Enemy component.
    app.world_mut().spawn((
        DeathAnimation {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
        },
        StolenScrap(5),
    ));
    app.update();
    app.update();
    // No Enemy entities → wave resolves → pile=0 → game over.
    assert_eq!(game_state(&app), GameState::GameOver);
}

// ---------------------------------------------------------------------------
// slow_aura
// ---------------------------------------------------------------------------

#[test]
fn slow_aura_affects_enemies_in_range() {
    let mut app = test_app();

    // Spawn aura tower at origin with range 100.
    app.world_mut().spawn((
        Tower,
        TowerState::Active,
        Transform::from_translation(Vec3::ZERO),
        TowerStats {
            damage: 0.0,
            range: 100.0,
        },
        SlowOnHit {
            factor: 0.5,
            duration: 2.0,
        },
    ));

    // Enemy in range (at 50 units away).
    let in_range = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
        ))
        .id();

    // Enemy out of range (at 200 units away).
    let out_of_range = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(Vec3::new(200.0, 0.0, 0.0)),
        ))
        .id();

    app.add_systems(Update, slow_aura);
    app.update();

    assert!(
        app.world().get::<SlowEffect>(in_range).is_some(),
        "enemy in range should have SlowEffect"
    );
    assert!(
        app.world().get::<SlowEffect>(out_of_range).is_none(),
        "enemy out of range should not have SlowEffect"
    );
}

#[test]
fn slow_aura_stronger_at_center() {
    let mut app = test_app();

    app.world_mut().spawn((
        Tower,
        TowerState::Active,
        Transform::from_translation(Vec3::ZERO),
        TowerStats {
            damage: 0.0,
            range: 100.0,
        },
        SlowOnHit {
            factor: 0.5,
            duration: 2.0,
        },
    ));

    let close = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(Vec3::new(10.0, 0.0, 0.0)),
        ))
        .id();

    let far = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(Vec3::new(90.0, 0.0, 0.0)),
        ))
        .id();

    app.add_systems(Update, slow_aura);
    app.update();

    let close_slow = app.world().get::<SlowEffect>(close).unwrap().factor;
    let far_slow = app.world().get::<SlowEffect>(far).unwrap().factor;
    // Closer enemy should have stronger slow (lower factor = slower).
    assert!(
        close_slow < far_slow,
        "close slow ({close_slow}) should be stronger (lower) than far ({far_slow})"
    );
}

// ---------------------------------------------------------------------------
// find_best_target (via system-level test)
// ---------------------------------------------------------------------------

#[test]
fn find_best_target_closest_mode() {
    let mut app = test_app();

    // We test find_best_target indirectly: spawn a tower with turret in Idle,
    // and enemies at different distances. After update, tower should acquire
    // closest enemy.
    use bevy_td_sandbox::tower::systems::turret_state_machine;

    app.insert_resource(mock_sound_assets());
    insert_pile(&mut app, 200);

    // Tower at origin.
    app.world_mut().spawn((
        Tower,
        TowerState::Active,
        Transform::from_translation(Vec3::ZERO),
        TowerStats {
            damage: 10.0,
            range: 100.0,
        },
        TurretState::with_cooldown(1.0),
        AimTolerance(0.1),
        ProjectileVisuals {
            speed: 200.0,
            color: Color::WHITE,
            size: Vec2::splat(4.0),
            trail_color: Color::WHITE,
            trail_interval: 0.05,
            particle_size: 2.0,
            particle_lifetime: 0.3,
        },
        TargetingMode::Closest,
    ));

    // Close enemy.
    let close = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Health {
                current: 100.0,
                max: 100.0,
            },
            Transform::from_translation(Vec3::new(30.0, 0.0, 0.0)),
        ))
        .id();

    // Far enemy.
    app.world_mut().spawn((
        Enemy,
        EnemyState::Approaching,
        Health {
            current: 100.0,
            max: 100.0,
        },
        Transform::from_translation(Vec3::new(80.0, 0.0, 0.0)),
    ));

    app.add_systems(Update, turret_state_machine);
    app.update();

    // Tower should have acquired the closest enemy.
    let turret = app
        .world_mut()
        .query::<&TurretState>()
        .iter(app.world())
        .next()
        .unwrap();
    match turret.phase {
        TurretPhase::Acquiring { target } => assert_eq!(target, close),
        _ => panic!("expected Acquiring phase, got Idle or Tracking"),
    }
}
