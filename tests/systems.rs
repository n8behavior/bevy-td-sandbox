use bevy::prelude::*;
use bevy_northstar::prelude::*;
use bevy_td_sandbox::audio::{GameAudioPlugin, GameSound, PlaySound};
use bevy_td_sandbox::camera::components::{CameraController, ScreenShake};
use bevy_td_sandbox::camera::systems::{apply_screen_shake, camera_reset};
use bevy_td_sandbox::common::constants::{
    PAPER_COLOR, PILE_COLLECTOR_RANGE, PILE_COLOR, PUDDLE_SLOW_FACTOR, SCRAP_MAGNET_RANGE,
};
use bevy_td_sandbox::economy::components::ScrapDrop;
use bevy_td_sandbox::enemy::components::*;
use bevy_td_sandbox::grid::components::GridCell;
use bevy_td_sandbox::grid::systems::grid_to_world_cfg;
use bevy_td_sandbox::pathfinding::systems::recalculate_enemy_paths;
use bevy_td_sandbox::pile::components::PileCell;
use bevy_td_sandbox::pile::resources::{PileScrap, PileState};
use bevy_td_sandbox::pile::systems::{update_pile_state, update_pile_visuals};
use bevy_td_sandbox::states::{GameState, PlayPhase};
use bevy_td_sandbox::terrain::components::{Terrain, TerrainMap};
use bevy_td_sandbox::terrain::systems::{apply_puddle_slow, apply_radioactive_damage};
use bevy_td_sandbox::test_helpers::*;
use bevy_td_sandbox::tower::components::*;
use bevy_td_sandbox::tower::systems::slow_aura;
use bevy_td_sandbox::wave::resources::WaveManager;
use bevy_td_sandbox::wave::systems::{check_wave_complete, on_wave_complete, spawn_enemies};

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
// update_pile_state — caching
// ---------------------------------------------------------------------------

#[test]
fn update_pile_state_cache_skips_fractional_change() {
    let mut app = test_app();
    insert_pile(&mut app, 50_000);
    app.add_systems(Update, update_pile_state);

    // Consume the initial "changed" resource.
    app.update();
    let cells_before = app.world().resource::<PileState>().cells.len();

    // Tiny scrap bump — same integer radius, different float radius.
    let old_radius = bevy_td_sandbox::pile::pile_radius(50_000);
    let new_radius = bevy_td_sandbox::pile::pile_radius(50_001);
    assert_eq!(
        old_radius as u32, new_radius as u32,
        "precondition: integer radius should not change"
    );

    app.world_mut().resource_mut::<PileScrap>().amount = 50_001;
    app.update();

    let pile_state = app.world().resource::<PileState>();
    assert_eq!(
        pile_state.cells.len(),
        cells_before,
        "cells should not recompute on fractional radius change"
    );
    // Float radius should still be updated even when cells are cached.
    assert!(
        (pile_state.radius_tiles - new_radius).abs() < 1e-6,
        "radius_tiles should reflect updated float value"
    );
}

#[test]
fn update_pile_state_cache_bypassed_when_empty() {
    let mut app = test_app();
    let config = test_grid_config();
    let center = config.center();

    // Manually set up: non-zero scrap, matching last_radius_int, but empty cells.
    let radius = bevy_td_sandbox::pile::pile_radius(50_000);
    app.insert_resource(PileScrap { amount: 50_000 });
    app.insert_resource(PileState {
        cells: std::collections::HashSet::new(),
        center,
        radius_tiles: radius,
        last_radius_int: radius as u32,
    });
    app.insert_resource(bevy_td_sandbox::pile::resources::EdgeCells(Vec::new()));
    app.insert_resource(config);
    app.add_systems(Update, update_pile_state);

    app.update();

    let pile_state = app.world().resource::<PileState>();
    assert!(
        !pile_state.cells.is_empty(),
        "cache should be bypassed when cells are empty, even if last_radius_int matches"
    );
}

// ---------------------------------------------------------------------------
// update_pile_visuals
// ---------------------------------------------------------------------------

#[test]
fn update_pile_visuals_adds_pile_cell_and_color() {
    let mut app = test_app();
    insert_pile(&mut app, 50_000);

    let config = test_grid_config();
    let center = config.center();

    // Spawn a grid cell at the pile center — should be inside the pile.
    let cell = app
        .world_mut()
        .spawn((
            GridCell {
                coord: IVec2::new(center.x as i32, center.y as i32),
            },
            Sprite::from_color(PAPER_COLOR, Vec2::splat(19.0)),
        ))
        .id();

    app.add_systems(Update, update_pile_visuals);
    app.update();

    assert!(
        app.world().get::<PileCell>(cell).is_some(),
        "cell inside pile should have PileCell marker"
    );
    let sprite = app.world().get::<Sprite>(cell).unwrap();
    assert_eq!(
        sprite.color, PILE_COLOR,
        "cell inside pile should be PILE_COLOR"
    );
}

#[test]
fn update_pile_visuals_removes_pile_cell_and_restores_color() {
    let mut app = test_app();
    insert_empty_pile(&mut app, 0, test_grid_config());

    let config = test_grid_config();
    let center = config.center();

    // Spawn a cell that already has PileCell and PILE_COLOR but is NOT in the pile.
    let cell = app
        .world_mut()
        .spawn((
            GridCell {
                coord: IVec2::new(center.x as i32, center.y as i32),
            },
            Sprite::from_color(PILE_COLOR, Vec2::splat(19.0)),
            PileCell,
        ))
        .id();

    app.add_systems(Update, update_pile_visuals);
    // First update processes the "changed" resource from insert_empty_pile which
    // computes the empty pile state. We need a second update after update_pile_state
    // ran, but since we only added update_pile_visuals, the PileState is already
    // marked as changed from insertion.
    app.update();

    assert!(
        app.world().get::<PileCell>(cell).is_none(),
        "cell outside pile should not have PileCell marker"
    );
    let sprite = app.world().get::<Sprite>(cell).unwrap();
    assert_eq!(
        sprite.color, PAPER_COLOR,
        "cell outside pile should be restored to PAPER_COLOR"
    );
}

#[test]
fn update_pile_visuals_skips_tower_entities() {
    let mut app = test_app();
    insert_pile(&mut app, 50_000);

    let config = test_grid_config();
    let center = config.center();

    // Spawn a grid cell with Tower — should be excluded by Without<Tower> filter.
    let cell = app
        .world_mut()
        .spawn((
            GridCell {
                coord: IVec2::new(center.x as i32, center.y as i32),
            },
            Sprite::from_color(PAPER_COLOR, Vec2::splat(19.0)),
            Tower,
        ))
        .id();

    app.add_systems(Update, update_pile_visuals);
    app.update();

    assert!(
        app.world().get::<PileCell>(cell).is_none(),
        "tower cell should not get PileCell even if inside pile"
    );
    let sprite = app.world().get::<Sprite>(cell).unwrap();
    assert_eq!(
        sprite.color, PAPER_COLOR,
        "tower cell color should be unchanged"
    );
}

#[test]
fn update_pile_visuals_skips_terrain_entities() {
    let mut app = test_app();
    insert_pile(&mut app, 50_000);

    let config = test_grid_config();
    let center = config.center();

    // Spawn a grid cell with Terrain — should be excluded by Without<Terrain> filter.
    let cell = app
        .world_mut()
        .spawn((
            GridCell {
                coord: IVec2::new(center.x as i32, center.y as i32),
            },
            Sprite::from_color(PAPER_COLOR, Vec2::splat(19.0)),
            Terrain::Rubble,
        ))
        .id();

    app.add_systems(Update, update_pile_visuals);
    app.update();

    assert!(
        app.world().get::<PileCell>(cell).is_none(),
        "terrain cell should not get PileCell even if inside pile"
    );
    let sprite = app.world().get::<Sprite>(cell).unwrap();
    assert_eq!(
        sprite.color, PAPER_COLOR,
        "terrain cell color should be unchanged"
    );
}

// ---------------------------------------------------------------------------
// Pile constants
// ---------------------------------------------------------------------------

#[test]
fn pile_collector_range_derives_from_magnet() {
    assert_eq!(
        PILE_COLLECTOR_RANGE,
        SCRAP_MAGNET_RANGE * 1.5,
        "pile collector range should be 1.5x the Scrap Magnet range"
    );
}

#[test]
fn init_pile_spawns_collector_with_constant_range() {
    use bevy_td_sandbox::pile::init_pile;
    use bevy_td_sandbox::states::GameMode;

    let mut app = test_app();
    app.insert_resource(test_grid_config());
    app.insert_resource(GameMode::Classic);
    app.add_systems(Update, init_pile);
    app.update();

    let mut found = false;
    for collector in app.world_mut().query::<&ScrapCollector>().iter(app.world()) {
        if collector.range == PILE_COLLECTOR_RANGE {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "init_pile should spawn a ScrapCollector with range == PILE_COLLECTOR_RANGE ({PILE_COLLECTOR_RANGE})"
    );
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
    app.insert_resource(test_wave_manager());

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

/// Non-empty spawn queue prevents wave from completing.
#[test]
fn wave_does_not_resolve_with_nonempty_queue() {
    let mut app = wave_app(100);
    app.world_mut()
        .resource_mut::<WaveManager>()
        .spawn_queue
        .push(bevy_td_sandbox::wave::resources::SpawnEntry {
            enemy_type: EnemyType::Shambler,
            health_multiplier: 1.0,
            speed_multiplier: 1.0,
            boss_trait: None,
        });
    app.update();
    app.update();
    assert_eq!(
        *app.world().resource::<State<PlayPhase>>().get(),
        PlayPhase::Defending,
        "wave should not resolve while spawn queue has entries"
    );
}

/// on_wave_complete advances current_wave when scrap > 0.
#[test]
fn on_wave_complete_increments_wave() {
    let mut app = wave_app(100);
    app.update();
    app.update();
    let wave_mgr = app.world().resource::<WaveManager>();
    assert_eq!(
        wave_mgr.current_wave, 1,
        "current_wave should advance from 0 to 1"
    );
}

/// Completing the last wave still transitions to Building (game continues until pile runs out).
#[test]
fn on_wave_complete_past_final_wave() {
    let mut app = wave_app(100);
    app.world_mut().resource_mut::<WaveManager>().current_wave = 19;
    app.update();
    app.update();
    let wave_mgr = app.world().resource::<WaveManager>();
    assert_eq!(
        wave_mgr.current_wave, 20,
        "current_wave should be 20 after completing wave 19"
    );
    assert_eq!(
        *app.world().resource::<State<PlayPhase>>().get(),
        PlayPhase::Building,
    );
}

// ---------------------------------------------------------------------------
// spawn_enemies
// ---------------------------------------------------------------------------

/// Helper: app with all resources needed by spawn_enemies.
fn spawn_enemies_app() -> App {
    use bevy_td_sandbox::common::constants::CHUNK_SIZE;
    use bevy_td_sandbox::wave::resources::SpawnEntry;

    let mut app = test_app();
    let config = test_grid_config();

    insert_pile(&mut app, 200);

    // Override EdgeCells with a known edge cell (insert_pile sets empty).
    app.insert_resource(bevy_td_sandbox::pile::resources::EdgeCells(vec![
        UVec3::new(0, 0, 0),
    ]));

    let settings = GridSettingsBuilder::new_2d(config.width, config.height)
        .chunk_size(CHUNK_SIZE)
        .build();
    let mut grid = OrdinalGrid::new(&settings);
    for x in 0..config.width {
        for y in 0..config.height {
            grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
        }
    }
    grid.build();
    app.world_mut().spawn(grid);

    let mut mgr = test_wave_manager();
    mgr.spawn_queue.push(SpawnEntry {
        enemy_type: EnemyType::Shambler,
        health_multiplier: 1.0,
        speed_multiplier: 1.0,
        boss_trait: None,
    });
    mgr.spawn_timer = Timer::from_seconds(0.0, TimerMode::Repeating);
    app.insert_resource(mgr);

    // Use Update (not FixedUpdate) so the system runs with real time deltas
    // in test. FixedUpdate doesn't accumulate enough wall-clock time between
    // app.update() calls in a test harness.
    app.add_systems(Update, spawn_enemies);
    app
}

/// Timer not finished → no enemy spawned, queue intact.
#[test]
fn spawn_enemies_timer_gates_spawning() {
    let mut app = spawn_enemies_app();
    app.world_mut().resource_mut::<WaveManager>().spawn_timer =
        Timer::from_seconds(999.0, TimerMode::Repeating);

    app.update();

    let enemy_count = app
        .world_mut()
        .query_filtered::<(), With<Enemy>>()
        .iter(app.world())
        .count();
    assert_eq!(
        enemy_count, 0,
        "no enemy should spawn before timer finishes"
    );
    assert_eq!(
        app.world().resource::<WaveManager>().spawn_queue.len(),
        1,
        "queue should still have the entry"
    );
}

/// Timer fires → enemy entity spawned, queue emptied.
#[test]
fn spawn_enemies_pops_queue_and_spawns() {
    let mut app = spawn_enemies_app();

    // Multiple updates: first has dt=0, subsequent have real deltas
    // that will exceed the 0.01s timer.
    for _ in 0..3 {
        app.update();
    }

    let enemy_count = app
        .world_mut()
        .query_filtered::<(), With<Enemy>>()
        .iter(app.world())
        .count();
    assert_eq!(enemy_count, 1, "one enemy should be spawned from the queue");
    assert!(
        app.world().resource::<WaveManager>().spawn_queue.is_empty(),
        "queue should be empty after spawning"
    );
}

// ---------------------------------------------------------------------------
// recalculate_enemy_paths (#11)
// ---------------------------------------------------------------------------

/// #11: placing a tower mid-game should invalidate stale NextPos AND Path so
/// bevy_northstar's next_position system doesn't pop from the old path.
#[test]
fn regression_11_recalculate_clears_stale_path_state() {
    let mut app = test_app();
    insert_pile(&mut app, 200);

    // Spawn enemy mid-movement: has stale NextPos AND old Path from previous route.
    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            AgentPos(UVec3::new(5, 5, 0)),
            NextPos(UVec3::new(6, 5, 0)),
            Path::new(vec![UVec3::new(6, 5, 0), UVec3::new(10, 5, 0)], 5),
        ))
        .id();

    app.add_systems(Update, recalculate_enemy_paths);
    app.update();

    assert!(
        app.world().get::<NextPos>(enemy).is_none(),
        "stale NextPos should be removed after path recalculation"
    );
    assert!(
        app.world().get::<Path>(enemy).is_none(),
        "stale Path should be removed so next_position doesn't pop old waypoints"
    );
    assert!(
        app.world().get::<Pathfind>(enemy).is_some(),
        "new Pathfind should be inserted after path recalculation"
    );
}

/// #11 integration: after blocking a cell and recalculating, bevy_northstar
/// produces a new path that avoids the blocked cell.
#[test]
fn regression_11_enemy_reroutes_around_blocked_cell() {
    use bevy_td_sandbox::common::constants::{CHUNK_SIZE, GridConfig};

    let mut app = test_app();
    app.add_plugins(NorthstarPlugin::<OrdinalNeighborhood>::default());

    // Small 16x16 grid (must be chunk-aligned).
    let size: u32 = 16;
    assert_eq!(size % CHUNK_SIZE, 0);
    let config = GridConfig {
        width: size,
        height: size,
        pixel_scale: 1,
    };
    insert_empty_pile(&mut app, 200, config);

    let settings = GridSettingsBuilder::new_2d(size, size)
        .chunk_size(CHUNK_SIZE)
        .build();
    let mut grid = OrdinalGrid::new(&settings);
    for x in 0..size {
        for y in 0..size {
            grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
        }
    }
    grid.build();
    let grid_entity = app.world_mut().spawn(grid).id();

    // Enemy at left edge, pathing toward pile center (8,8) along row 8.
    // recalculate_enemy_paths uses nearest_pile_cell which returns (8,8).
    let start = UVec3::new(0, 8, 0);
    let goal = UVec3::new(size / 2, size / 2, 0); // pile center
    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            AgentPos(start),
            AgentOfGrid(grid_entity),
            Pathfind::new(goal).mode(PathfindMode::Waypoints),
        ))
        .id();

    // Let bevy_northstar compute the initial path.
    for _ in 0..5 {
        app.update();
    }
    let has_path =
        app.world().get::<Path>(enemy).is_some() || app.world().get::<NextPos>(enemy).is_some();
    assert!(has_path, "enemy should have an active path after init");

    // Block a cell on the straight-line path (NOT the goal itself).
    let blocked = UVec3::new(4, 8, 0);
    {
        let mut state = app.world_mut().query::<&mut OrdinalGrid>();
        let mut grid = state.single_mut(app.world_mut()).unwrap();
        grid.set_nav(blocked, Nav::Impassable);
        grid.build();
    }

    // Recalculate paths (same as GridChanged observer does).
    app.add_systems(Update, recalculate_enemy_paths);
    for _ in 0..5 {
        app.update();
    }

    // The new path must not pass through the blocked cell.
    if let Some(path) = app.world().get::<Path>(enemy) {
        assert!(
            !path.is_position_in_path(blocked),
            "recalculated path should not contain the blocked cell"
        );
    }
    if let Some(next) = app.world().get::<NextPos>(enemy) {
        assert_ne!(
            next.0, blocked,
            "next waypoint should not be the blocked cell"
        );
    }
}

// ---------------------------------------------------------------------------
// spawn_nav_grid
// ---------------------------------------------------------------------------

#[test]
fn spawn_nav_grid_creates_passable_grid() {
    use bevy_td_sandbox::grid::systems::spawn_nav_grid;

    let mut app = test_app();
    app.add_plugins(NorthstarPlugin::<OrdinalNeighborhood>::default());

    let config = test_grid_config();
    app.insert_resource(config);
    app.add_systems(Update, spawn_nav_grid);
    app.update();

    // Verify an OrdinalGrid entity was spawned with viable paths.
    let mut query = app.world_mut().query::<&OrdinalGrid>();
    let grid = query.single(app.world()).unwrap();
    let start = UVec3::new(0, 0, 0);
    let goal = UVec3::new(5, 5, 0);
    assert!(
        grid.pathfind(&mut PathfindArgs::new(start, goal).astar())
            .is_some(),
        "interior path should be viable"
    );
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

// ---------------------------------------------------------------------------
// apply_puddle_slow
// ---------------------------------------------------------------------------

#[test]
fn puddle_slow_reduces_speed() {
    let mut app = test_app();
    let config = test_grid_config();

    // Place puddle at cell (5, 5).
    let mut terrain_map = TerrainMap::default();
    terrain_map.cells.insert(IVec2::new(5, 5), Terrain::Puddle);

    let world_pos = grid_to_world_cfg(UVec3::new(5, 5, 0), &config);

    app.insert_resource(config);
    app.insert_resource(terrain_map);

    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(world_pos.extend(0.0)),
            MoveSpeed {
                base: 100.0,
                current: 100.0,
            },
        ))
        .id();

    app.add_systems(Update, apply_puddle_slow);
    app.update();

    let speed = app.world().get::<MoveSpeed>(enemy).unwrap();
    let expected = 100.0 * PUDDLE_SLOW_FACTOR;
    assert!(
        (speed.current - expected).abs() < 0.01,
        "speed should be {expected}, got {}",
        speed.current
    );
}

#[test]
fn puddle_no_effect_off_puddle() {
    let mut app = test_app();
    let config = test_grid_config();

    // Terrain map has a puddle, but the enemy is on a different cell.
    let mut terrain_map = TerrainMap::default();
    terrain_map.cells.insert(IVec2::new(5, 5), Terrain::Puddle);

    let world_pos = grid_to_world_cfg(UVec3::new(10, 10, 0), &config);

    app.insert_resource(config);
    app.insert_resource(terrain_map);

    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(world_pos.extend(0.0)),
            MoveSpeed {
                base: 100.0,
                current: 100.0,
            },
        ))
        .id();

    app.add_systems(Update, apply_puddle_slow);
    app.update();

    let speed = app.world().get::<MoveSpeed>(enemy).unwrap();
    assert!(
        (speed.current - 100.0).abs() < 0.01,
        "speed should be unchanged, got {}",
        speed.current
    );
}

#[test]
fn puddle_slow_idempotent_across_frames() {
    let mut app = test_app();
    let config = test_grid_config();

    let mut terrain_map = TerrainMap::default();
    terrain_map.cells.insert(IVec2::new(5, 5), Terrain::Puddle);

    let world_pos = grid_to_world_cfg(UVec3::new(5, 5, 0), &config);

    app.insert_resource(config);
    app.insert_resource(terrain_map);

    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(world_pos.extend(0.0)),
            MoveSpeed {
                base: 100.0,
                current: 100.0,
            },
        ))
        .id();

    app.add_systems(Update, apply_puddle_slow);

    // Run multiple frames.
    for _ in 0..5 {
        app.update();
    }

    let speed = app.world().get::<MoveSpeed>(enemy).unwrap();
    let expected = 100.0 * PUDDLE_SLOW_FACTOR;
    assert!(
        (speed.current - expected).abs() < 0.01,
        "speed should stabilize at {expected}, got {}",
        speed.current
    );
}

// ---------------------------------------------------------------------------
// apply_radioactive_damage
// ---------------------------------------------------------------------------

#[test]
fn radioactive_damage_reduces_health() {
    let mut app = test_app();
    let config = test_grid_config();

    let mut terrain_map = TerrainMap::default();
    terrain_map
        .cells
        .insert(IVec2::new(5, 5), Terrain::Radioactive);

    let world_pos = grid_to_world_cfg(UVec3::new(5, 5, 0), &config);

    app.insert_resource(config);
    app.insert_resource(terrain_map);

    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(world_pos.extend(0.0)),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();

    app.add_systems(Update, apply_radioactive_damage);

    // First update initializes time (delta=0), second has a real delta.
    app.update();
    app.update();

    let health = app.world().get::<Health>(enemy).unwrap();
    assert!(
        health.current < 100.0,
        "health should decrease from radioactive damage, got {}",
        health.current
    );
}

#[test]
fn radioactive_no_damage_off_grid() {
    let mut app = test_app();
    let config = test_grid_config();

    let mut terrain_map = TerrainMap::default();
    terrain_map
        .cells
        .insert(IVec2::new(5, 5), Terrain::Radioactive);

    app.insert_resource(config);
    app.insert_resource(terrain_map);

    // Enemy far off-grid.
    let enemy = app
        .world_mut()
        .spawn((
            Enemy,
            EnemyState::Approaching,
            Transform::from_translation(Vec3::new(10000.0, 10000.0, 0.0)),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();

    app.add_systems(Update, apply_radioactive_damage);
    app.update();

    let health = app.world().get::<Health>(enemy).unwrap();
    assert!(
        (health.current - 100.0).abs() < 0.01,
        "off-grid enemy should not take damage, got {}",
        health.current
    );
}

// ---------------------------------------------------------------------------
// apply_screen_shake
// ---------------------------------------------------------------------------

#[test]
fn screen_shake_decays_over_updates() {
    let mut app = test_app();
    app.insert_resource(ScreenShake {
        intensity: 10.0,
        timer: Timer::from_seconds(2.0, TimerMode::Once),
        decay: 0.5,
        current_offset: Vec3::ZERO,
    });
    app.world_mut().spawn(Camera2d);
    app.add_systems(Update, apply_screen_shake);

    // Run several updates so time advances and decay kicks in.
    for _ in 0..10 {
        app.update();
    }

    let shake = app.world().resource::<ScreenShake>();
    assert!(
        shake.intensity < 10.0,
        "intensity should have decayed, got {}",
        shake.intensity
    );
}

#[test]
fn screen_shake_zero_intensity_no_offset() {
    let mut app = test_app();
    app.insert_resource(ScreenShake {
        intensity: 0.0,
        timer: Timer::from_seconds(1.0, TimerMode::Once),
        decay: 0.5,
        current_offset: Vec3::ZERO,
    });
    app.world_mut().spawn(Camera2d);
    app.add_systems(Update, apply_screen_shake);
    app.update();

    let transform = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera2d>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        transform.translation,
        Vec3::ZERO,
        "zero intensity should produce no offset"
    );
}

#[test]
fn screen_shake_undoes_offset_after_expiry() {
    let mut app = test_app();
    // Pre-tick the timer past its duration so the system sees it as finished.
    let mut timer = Timer::from_seconds(0.01, TimerMode::Once);
    timer.tick(std::time::Duration::from_secs(1));
    app.insert_resource(ScreenShake {
        intensity: 8.0,
        timer,
        decay: 0.01,
        current_offset: Vec3::ZERO,
    });
    app.world_mut().spawn(Camera2d);
    app.add_systems(Update, apply_screen_shake);

    app.update();

    let shake = app.world().resource::<ScreenShake>();
    assert_eq!(
        shake.intensity, 0.0,
        "intensity should be zero after expiry"
    );
    assert_eq!(
        shake.current_offset,
        Vec3::ZERO,
        "offset should be zero after expiry"
    );
}

// ---------------------------------------------------------------------------
// camera_reset
// ---------------------------------------------------------------------------

#[test]
fn reset_restores_home_position_and_scale() {
    let mut app = test_app();
    app.world_mut().spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(100.0, 200.0, 0.0)),
        CameraController {
            min_scale: 0.1,
            max_scale: 10.0,
            zoom_step: 1.15,
            home_translation: Vec3::ZERO,
            home_scale: 1.0,
        },
    ));

    // Simulate pressing the Home key.
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::Home);
    app.insert_resource(keys);

    app.add_systems(Update, camera_reset);
    app.update();

    let (transform, projection) = app
        .world_mut()
        .query_filtered::<(&Transform, &Projection), With<Camera2d>>()
        .single(app.world())
        .unwrap();

    assert_eq!(
        transform.translation,
        Vec3::ZERO,
        "translation should be reset to home"
    );
    if let Projection::Orthographic(ortho) = projection {
        assert_eq!(ortho.scale, 1.0, "scale should be reset to home_scale");
    } else {
        panic!("expected orthographic projection");
    }
}

#[test]
fn reset_no_op_without_home_press() {
    let mut app = test_app();
    app.world_mut().spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(50.0, 75.0, 0.0)),
        CameraController {
            min_scale: 0.1,
            max_scale: 10.0,
            zoom_step: 1.15,
            home_translation: Vec3::ZERO,
            home_scale: 1.0,
        },
    ));

    // Insert empty key input (no Home press).
    app.insert_resource(ButtonInput::<KeyCode>::default());

    app.add_systems(Update, camera_reset);
    app.update();

    let transform = app
        .world_mut()
        .query_filtered::<&Transform, With<Camera2d>>()
        .single(app.world())
        .unwrap();

    assert_eq!(
        transform.translation,
        Vec3::new(50.0, 75.0, 0.0),
        "translation should be unchanged without Home press"
    );
}

// ---------------------------------------------------------------------------
// Audio: PlaySound observer
// ---------------------------------------------------------------------------

#[test]
fn play_sound_observer_spawns_audio_player() {
    let mut app = test_app_with_assets();
    app.add_plugins((bevy::audio::AudioPlugin::default(), GameAudioPlugin));
    app.update(); // Startup: init_sound_assets

    app.world_mut()
        .commands()
        .trigger(PlaySound(GameSound::TowerScrapgun));
    app.update();

    let count = app
        .world_mut()
        .query::<&AudioPlayer<Pitch>>()
        .iter(app.world())
        .count();
    assert_eq!(count, 1, "expected one AudioPlayer entity");
}

#[test]
fn play_sound_multiple_triggers_spawn_multiple() {
    let mut app = test_app_with_assets();
    app.add_plugins((bevy::audio::AudioPlugin::default(), GameAudioPlugin));
    app.update();

    app.world_mut()
        .commands()
        .trigger(PlaySound(GameSound::EnemyDeath));
    app.world_mut()
        .commands()
        .trigger(PlaySound(GameSound::WaveStart));
    app.update();

    let count = app
        .world_mut()
        .query::<&AudioPlayer<Pitch>>()
        .iter(app.world())
        .count();
    assert_eq!(count, 2);
}

#[test]
fn init_sound_assets_serves_all_variants() {
    let mut app = test_app_with_assets();
    app.add_plugins((bevy::audio::AudioPlugin::default(), GameAudioPlugin));
    app.update();

    // Trigger every variant — if any handle is missing, the observer panics.
    for &sound in GameSound::ALL {
        app.world_mut().commands().trigger(PlaySound(sound));
    }
    app.update();

    let count = app
        .world_mut()
        .query::<&AudioPlayer<Pitch>>()
        .iter(app.world())
        .count();
    assert_eq!(count, GameSound::ALL.len());
}

// ---------------------------------------------------------------------------
// Particle systems
// ---------------------------------------------------------------------------

#[test]
fn impact_particles_despawn_after_timer() {
    use bevy_td_sandbox::particles::components::ImpactParticle;
    use bevy_td_sandbox::particles::systems::animate_impact_particles;

    let mut app = test_app();
    let mut timer = Timer::from_seconds(0.01, TimerMode::Once);
    timer.tick(std::time::Duration::from_secs(1));
    app.world_mut().spawn((
        ImpactParticle {
            timer,
            velocity: Vec2::new(10.0, 0.0),
        },
        Sprite::from_color(Color::WHITE, Vec2::splat(3.0)),
        Transform::default(),
    ));
    app.add_systems(Update, animate_impact_particles);
    app.update();

    let count = app
        .world_mut()
        .query::<&ImpactParticle>()
        .iter(app.world())
        .count();
    assert_eq!(count, 0, "impact particle should be despawned after timer");
}

#[test]
fn death_particles_despawn_after_timer() {
    use bevy_td_sandbox::particles::components::DeathParticle;
    use bevy_td_sandbox::particles::systems::animate_death_particles;

    let mut app = test_app();
    let mut timer = Timer::from_seconds(0.01, TimerMode::Once);
    timer.tick(std::time::Duration::from_secs(1));
    app.world_mut().spawn((
        DeathParticle {
            timer,
            velocity: Vec2::new(10.0, 5.0),
        },
        Sprite::from_color(Color::WHITE, Vec2::splat(4.0)),
        Transform::default(),
    ));
    app.add_systems(Update, animate_death_particles);
    app.update();

    let count = app
        .world_mut()
        .query::<&DeathParticle>()
        .iter(app.world())
        .count();
    assert_eq!(count, 0, "death particle should be despawned after timer");
}

#[test]
fn sparkle_particles_despawn_after_timer() {
    use bevy_td_sandbox::particles::components::SparkleParticle;
    use bevy_td_sandbox::particles::systems::animate_sparkle_particles;

    let mut app = test_app();
    let mut timer = Timer::from_seconds(0.01, TimerMode::Once);
    timer.tick(std::time::Duration::from_secs(1));
    app.world_mut().spawn((
        SparkleParticle { timer },
        Sprite::from_color(Color::WHITE, Vec2::splat(2.0)),
        Transform::default(),
    ));
    app.add_systems(Update, animate_sparkle_particles);
    app.update();

    let count = app
        .world_mut()
        .query::<&SparkleParticle>()
        .iter(app.world())
        .count();
    assert_eq!(count, 0, "sparkle particle should be despawned after timer");
}

#[test]
fn spawn_impact_particles_count() {
    use bevy_td_sandbox::particles::components::ImpactParticle;
    use bevy_td_sandbox::particles::systems::spawn_impact_particles;
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    let mut app = test_app();

    // Determine expected count from a parallel seeded RNG.
    let mut rng_check = SmallRng::seed_from_u64(42);
    let expected_count = rng_check.random_range(3u32..=5);

    let mut rng = SmallRng::seed_from_u64(42);
    spawn_impact_particles(
        &mut app.world_mut().commands(),
        Vec2::new(100.0, 50.0),
        Color::WHITE,
        &mut rng,
    );
    app.update(); // flush commands

    let count = app
        .world_mut()
        .query::<&ImpactParticle>()
        .iter(app.world())
        .count();
    assert_eq!(count, expected_count as usize);
}

#[test]
fn death_particles_gravity_pulls_down() {
    use bevy_td_sandbox::particles::components::DeathParticle;
    use bevy_td_sandbox::particles::systems::animate_death_particles;

    let mut app = test_app();
    let initial_vel_y = 50.0;
    app.world_mut().spawn((
        DeathParticle {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
            velocity: Vec2::new(0.0, initial_vel_y),
        },
        Sprite::from_color(Color::WHITE, Vec2::splat(4.0)),
        Transform::default(),
    ));
    app.add_systems(Update, animate_death_particles);

    // First update has dt=0, second has real dt.
    app.update();
    app.update();

    let particle = app
        .world_mut()
        .query::<&DeathParticle>()
        .single(app.world())
        .unwrap();
    assert!(
        particle.velocity.y < initial_vel_y,
        "gravity should reduce y velocity: got {}",
        particle.velocity.y
    );
}

#[test]
fn sparkle_particles_float_upward() {
    use bevy_td_sandbox::particles::components::SparkleParticle;
    use bevy_td_sandbox::particles::systems::animate_sparkle_particles;

    let mut app = test_app();
    app.world_mut().spawn((
        SparkleParticle {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
        },
        Sprite::from_color(Color::WHITE, Vec2::splat(2.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),
    ));
    app.add_systems(Update, animate_sparkle_particles);

    app.update();
    app.update();

    let tf = app
        .world_mut()
        .query_filtered::<&Transform, With<SparkleParticle>>()
        .single(app.world())
        .unwrap();
    assert!(
        tf.translation.y > 0.0,
        "sparkle should float upward, got y={}",
        tf.translation.y
    );
}
