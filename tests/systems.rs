use bevy::prelude::*;
use bevy_northstar::prelude::*;
use bevy_td_sandbox::audio::{GameAudioPlugin, GameSound, PlaySound};
use bevy_td_sandbox::camera::components::{CameraController, ScreenShake};
use bevy_td_sandbox::camera::systems::{apply_screen_shake, camera_reset};
use bevy_td_sandbox::common::constants::{
    PAPER_COLOR, PILE_COLLECTOR_RANGE, PILE_COLOR, PUDDLE_SLOW_FACTOR, SCRAP_MAGNET_RANGE,
};
use bevy_td_sandbox::economy::components::ScrapDrop;
use bevy_td_sandbox::endless::resources::EndlessSpawner;
use bevy_td_sandbox::endless::systems::endless_check_game_over;
use bevy_td_sandbox::endless::systems::endless_spawn_enemies;
use bevy_td_sandbox::enemy::components::*;
use bevy_td_sandbox::grid::components::GridCell;
use bevy_td_sandbox::grid::systems::grid_to_world_cfg;
use bevy_td_sandbox::pathfinding::systems::recalculate_enemy_paths;
use bevy_td_sandbox::pile::components::PileCell;
use bevy_td_sandbox::pile::resources::{PileScrap, PileState};
use bevy_td_sandbox::pile::systems::{update_pile_state, update_pile_visuals};
use bevy_td_sandbox::states::{GameMode, GameState, PlayPhase};
use bevy_td_sandbox::terrain::components::{Terrain, TerrainMap};
use bevy_td_sandbox::terrain::systems::{apply_puddle_slow, apply_radioactive_damage};
use bevy_td_sandbox::test_helpers::*;
use bevy_td_sandbox::tower::components::*;
use bevy_td_sandbox::tower::placement::SelectedTower;
use bevy_td_sandbox::tower::systems::slow_aura;
use bevy_td_sandbox::ui::hud::*;
use bevy_td_sandbox::ui::tower_menu::{TowerPaletteIndex, highlight_selected_tower};
use bevy_td_sandbox::wave::resources::{WaveConfig, WaveManager};
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
    EnemyBuilder::new().spawn(&mut app);
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
    let enemy = EnemyBuilder::new()
        .agent_pos(UVec3::new(5, 5, 0))
        .next_pos(UVec3::new(6, 5, 0))
        .path(vec![UVec3::new(6, 5, 0), UVec3::new(10, 5, 0)], 5)
        .spawn(&mut app);

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
    use bevy_td_sandbox::common::constants::GridConfig;

    let mut app = test_app();
    app.add_plugins(NorthstarPlugin::<OrdinalNeighborhood>::default());

    // Small 16x16 grid (must be chunk-aligned).
    let size: u32 = 16;
    let config = GridConfig {
        width: size,
        height: size,
        pixel_scale: 1,
    };
    insert_empty_pile(&mut app, 200, config);

    let grid_entity = spawn_test_grid(&mut app, size, size);

    // Enemy at left edge, pathing toward pile center (8,8) along row 8.
    // recalculate_enemy_paths uses nearest_pile_cell which returns (8,8).
    let start = UVec3::new(0, 8, 0);
    let goal = UVec3::new(size / 2, size / 2, 0); // pile center
    let enemy = EnemyBuilder::new()
        .agent_pos(start)
        .on_grid(grid_entity)
        .pathfind_to(goal)
        .spawn(&mut app);

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
    TowerBuilder::aura().spawn(&mut app);

    // Enemy in range (at 50 units away).
    let in_range = EnemyBuilder::new()
        .at(Vec3::new(50.0, 0.0, 0.0))
        .spawn(&mut app);

    // Enemy out of range (at 200 units away).
    let out_of_range = EnemyBuilder::new()
        .at(Vec3::new(200.0, 0.0, 0.0))
        .spawn(&mut app);

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

    TowerBuilder::aura().spawn(&mut app);

    let close = EnemyBuilder::new()
        .at(Vec3::new(10.0, 0.0, 0.0))
        .spawn(&mut app);

    let far = EnemyBuilder::new()
        .at(Vec3::new(90.0, 0.0, 0.0))
        .spawn(&mut app);

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
    TowerBuilder::turret().spawn(&mut app);

    // Close enemy.
    let close = EnemyBuilder::new()
        .health(100.0, 100.0)
        .at(Vec3::new(30.0, 0.0, 0.0))
        .spawn(&mut app);

    // Far enemy.
    EnemyBuilder::new()
        .health(100.0, 100.0)
        .at(Vec3::new(80.0, 0.0, 0.0))
        .spawn(&mut app);

    app.add_systems(Update, turret_state_machine);
    app.update();

    // Tower should have acquired the closest enemy.
    let turret = app
        .world_mut()
        .query::<&Turret>()
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
        .trigger(PlaySound(GameSound::TowerDestroyed));
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
    use rand::rngs::SmallRng;
    use rand::{RngExt, SeedableRng};

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

// ---------------------------------------------------------------------------
// update_hud — wiring verification
// Verifies the system reads resources/queries and writes to the correct Text
// entities. Formatting correctness is covered by unit tests in hud::tests.
// ---------------------------------------------------------------------------

/// Spawn all five HUD marker entities so the `Without<>` queries resolve.
fn spawn_hud_text(app: &mut App) {
    app.world_mut()
        .spawn((Text::new(""), TextColor::default(), ScrapText));
    app.world_mut()
        .spawn((Text::new(""), TextColor::default(), GroundScrapText));
    app.world_mut()
        .spawn((Text::new(""), TextColor::default(), StolenScrapText));
    app.world_mut()
        .spawn((Text::new(""), TextColor::default(), WaveText));
    app.world_mut()
        .spawn((Text::new(""), TextColor::default(), PhaseText));
}

#[test]
fn hud_updates_scrap_texts() {
    let mut app = ui_app();
    app.insert_resource(PileScrap { amount: 42 });
    app.insert_resource(test_wave_manager());
    spawn_hud_text(&mut app);

    // Spawn some ground drops and stolen-scrap enemies.
    app.world_mut().spawn(ScrapDrop {
        value: 10,
        lifetime: Timer::from_seconds(5.0, TimerMode::Once),
    });
    app.world_mut().spawn(ScrapDrop {
        value: 5,
        lifetime: Timer::from_seconds(5.0, TimerMode::Once),
    });
    app.world_mut().spawn((Enemy, StolenScrap(7)));

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let scrap = app
        .world_mut()
        .query_filtered::<&Text, With<ScrapText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**scrap, "Pile: 42");

    let ground = app
        .world_mut()
        .query_filtered::<&Text, With<GroundScrapText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**ground, "Ground: 15");

    let stolen = app
        .world_mut()
        .query_filtered::<&Text, With<StolenScrapText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**stolen, "Stolen: 7");
}

#[test]
fn hud_scrap_texts_empty_world() {
    let mut app = ui_app();
    app.insert_resource(PileScrap { amount: 0 });
    app.insert_resource(test_wave_manager());
    spawn_hud_text(&mut app);

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let ground = app
        .world_mut()
        .query_filtered::<&Text, With<GroundScrapText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**ground, "Ground: 0");

    let stolen = app
        .world_mut()
        .query_filtered::<&Text, With<StolenScrapText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**stolen, "Stolen: 0");
}

#[test]
fn hud_wave_text_classic_mode() {
    let mut app = ui_app();
    app.insert_resource(PileScrap { amount: 100 });
    let mut wm = test_wave_manager();
    wm.current_wave = 4;
    wm.waves = (0..20)
        .map(|_| WaveConfig {
            enemies: Vec::new(),
            spawn_interval: 1.0,
        })
        .collect();
    app.insert_resource(wm);
    spawn_hud_text(&mut app);

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let wave = app
        .world_mut()
        .query_filtered::<&Text, With<WaveText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**wave, "Wave: 5 / 20");
}

#[test]
fn hud_wave_text_endless_mode() {
    let mut app = ui_app();
    app.insert_resource(GameMode::Endless);
    app.insert_resource(PileScrap { amount: 100 });
    app.insert_resource(EndlessSpawner {
        elapsed_time: 125.0,
        spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        enemies_spawned: 0,
    });
    spawn_hud_text(&mut app);

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let wave = app
        .world_mut()
        .query_filtered::<&Text, With<WaveText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**wave, "Time: 02:05");
}

#[test]
fn hud_phase_text_building() {
    let mut app = ui_app();
    app.insert_resource(PileScrap { amount: 100 });
    app.insert_resource(test_wave_manager());
    spawn_hud_text(&mut app);

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let (text, color) = app
        .world_mut()
        .query_filtered::<(&Text, &TextColor), With<PhaseText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**text, "BUILDING [Enter]");
    assert_eq!(color.0, Color::srgb(0.3, 0.9, 0.3));
}

#[test]
fn hud_phase_text_defending() {
    let mut app = ui_app();
    // Transition to Defending phase.
    app.world_mut()
        .resource_mut::<NextState<PlayPhase>>()
        .set(PlayPhase::Defending);
    app.update();

    app.insert_resource(PileScrap { amount: 100 });
    app.insert_resource(test_wave_manager());
    spawn_hud_text(&mut app);

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let (text, color) = app
        .world_mut()
        .query_filtered::<(&Text, &TextColor), With<PhaseText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**text, "DEFENDING");
    assert_eq!(color.0, Color::srgb(0.9, 0.3, 0.3));
}

#[test]
fn hud_phase_text_endless() {
    let mut app = ui_app();
    app.insert_resource(GameMode::Endless);
    app.insert_resource(PileScrap { amount: 100 });
    app.insert_resource(EndlessSpawner {
        elapsed_time: 0.0,
        spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        enemies_spawned: 0,
    });
    spawn_hud_text(&mut app);

    app.add_systems(Update, update_hud.run_if(in_state(GameState::Playing)));
    app.update();

    let (text, color) = app
        .world_mut()
        .query_filtered::<(&Text, &TextColor), With<PhaseText>>()
        .single(app.world())
        .unwrap();
    assert_eq!(**text, "ENDLESS");
    assert_eq!(color.0, Color::srgb(0.9, 0.6, 0.2));
}

// ---------------------------------------------------------------------------
// highlight_selected_tower — wiring verification
// ---------------------------------------------------------------------------

#[test]
fn tower_highlight_selected() {
    let mut app = ui_app();
    app.insert_resource(SelectedTower {
        index: Some(1),
        entity: None,
    });

    app.world_mut()
        .spawn((TowerPaletteIndex(0), BackgroundColor(Color::NONE)));
    app.world_mut()
        .spawn((TowerPaletteIndex(1), BackgroundColor(Color::NONE)));

    app.add_systems(Update, highlight_selected_tower);
    app.update();

    let mut results: Vec<(usize, BackgroundColor)> = app
        .world_mut()
        .query::<(&TowerPaletteIndex, &BackgroundColor)>()
        .iter(app.world())
        .map(|(idx, bg)| (idx.0, *bg))
        .collect();
    results.sort_by_key(|(idx, _)| *idx);

    assert_eq!(results[0].1, BackgroundColor(Color::NONE));
    assert_eq!(
        results[1].1,
        BackgroundColor(Color::srgba(0.5, 0.45, 0.2, 0.6))
    );
}

#[test]
fn tower_highlight_deselected() {
    let mut app = ui_app();
    app.insert_resource(SelectedTower {
        index: None,
        entity: None,
    });

    app.world_mut()
        .spawn((TowerPaletteIndex(0), BackgroundColor(Color::WHITE)));
    app.world_mut()
        .spawn((TowerPaletteIndex(1), BackgroundColor(Color::WHITE)));

    app.add_systems(Update, highlight_selected_tower);
    app.update();

    let all_none = app
        .world_mut()
        .query::<&BackgroundColor>()
        .iter(app.world())
        .all(|bg| *bg == BackgroundColor(Color::NONE));
    assert!(all_none, "all buttons should be deselected");
}

// ===========================================================================
// Endless mode — game-over (three-fold gate) and spawn guards
// ===========================================================================

/// Headless app for testing endless game-over logic.
///
/// Sets up states (Playing / Defending), [`PileScrap`] with the given amount,
/// and registers [`endless_check_game_over`] in `Update`.
fn endless_game_over_app(scrap: u32) -> App {
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

    app.add_systems(
        Update,
        endless_check_game_over.run_if(in_state(PlayPhase::Defending)),
    );
    app
}

/// Scrap remaining → no game over (gate 1 blocks).
#[test]
fn endless_game_over_blocked_by_scrap() {
    let mut app = endless_game_over_app(100);
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Pile empty but drops on field → no game over (gate 2 blocks).
#[test]
fn endless_game_over_blocked_by_drops() {
    let mut app = endless_game_over_app(0);
    app.world_mut().spawn(ScrapDrop {
        value: 10,
        lifetime: Timer::from_seconds(10.0, TimerMode::Once),
    });
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Pile empty but enemies alive → no game over (gate 3 blocks).
#[test]
fn endless_game_over_blocked_by_enemies() {
    let mut app = endless_game_over_app(0);
    app.world_mut().spawn((Enemy, EnemyState::Approaching));
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::Playing);
}

/// Pile empty, no drops, no enemies → GameOver (all gates pass).
#[test]
fn endless_game_over_triggers_when_bankrupt() {
    let mut app = endless_game_over_app(0);
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::GameOver);
}

/// Pile empty but both drops and enemies present → no game over.
#[test]
fn endless_game_over_blocked_by_drops_and_enemies() {
    let mut app = endless_game_over_app(0);
    app.world_mut().spawn(ScrapDrop {
        value: 5,
        lifetime: Timer::from_seconds(10.0, TimerMode::Once),
    });
    app.world_mut().spawn((Enemy, EnemyState::Approaching));
    app.update();
    app.update();
    assert_eq!(game_state(&app), GameState::Playing);
}

// ---------------------------------------------------------------------------
// Endless spawn guards
// ---------------------------------------------------------------------------

/// Headless app for testing endless spawn guards.
///
/// Sets up states (Playing / Defending), an [`EndlessSpawner`], a grid
/// entity, and pile resources. Registers [`endless_spawn_enemies`] in
/// `FixedUpdate` to match production scheduling.
fn endless_spawn_app() -> App {
    use bevy_td_sandbox::common::constants::CHUNK_SIZE;
    use bevy_td_sandbox::pile::resources::EdgeCells;
    use std::collections::HashSet;

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

    let config = test_grid_config();

    app.insert_resource(EndlessSpawner {
        elapsed_time: 0.0,
        spawn_timer: Timer::from_seconds(1.5, TimerMode::Repeating),
        enemies_spawned: 0,
    });

    // Empty edge cells and pile state by default (tests override as needed).
    app.insert_resource(EdgeCells(Vec::new()));
    app.insert_resource(PileState {
        cells: HashSet::new(),
        center: config.center(),
        radius_tiles: 0.0,
        last_radius_int: 0,
    });
    app.insert_resource(config);

    // Spawn a grid entity (required by the system's grid_query).
    let settings = GridSettingsBuilder::new_2d(40, 32)
        .chunk_size(CHUNK_SIZE)
        .build();
    let grid = OrdinalGrid::new(&settings);
    app.world_mut().spawn(grid);

    app.add_systems(
        FixedUpdate,
        endless_spawn_enemies.run_if(in_state(PlayPhase::Defending)),
    );
    app
}

/// Empty edge_cells → no enemies spawned, no panic.
#[test]
fn endless_spawn_guard_empty_edge_cells() {
    let mut app = endless_spawn_app();
    // edge_cells is already empty; add pile cells so only the edge guard fires.
    app.world_mut()
        .resource_mut::<PileState>()
        .cells
        .insert(UVec3::new(20, 16, 0));

    for _ in 0..5 {
        app.update();
    }

    let enemy_count = app
        .world_mut()
        .query_filtered::<(), With<Enemy>>()
        .iter(app.world())
        .count();
    assert_eq!(
        enemy_count, 0,
        "no enemies should spawn with empty edge_cells"
    );
}

/// Empty pile_state.cells → no enemies spawned, no panic.
#[test]
fn endless_spawn_guard_empty_pile_cells() {
    use bevy_td_sandbox::pile::resources::EdgeCells;

    let mut app = endless_spawn_app();
    // Add edge cells so only the pile guard fires.
    app.insert_resource(EdgeCells(vec![UVec3::new(0, 0, 0)]));

    for _ in 0..5 {
        app.update();
    }

    let enemy_count = app
        .world_mut()
        .query_filtered::<(), With<Enemy>>()
        .iter(app.world())
        .count();
    assert_eq!(
        enemy_count, 0,
        "no enemies should spawn with empty pile cells"
    );
}

/// Both empty → no enemies spawned, no panic.
#[test]
fn endless_spawn_guard_both_empty() {
    let mut app = endless_spawn_app();

    for _ in 0..5 {
        app.update();
    }

    let enemy_count = app
        .world_mut()
        .query_filtered::<(), With<Enemy>>()
        .iter(app.world())
        .count();
    assert_eq!(
        enemy_count, 0,
        "no enemies should spawn with both guards empty"
    );
}

// ---------------------------------------------------------------------------
// #66: Tower placement mid-game freezes all enemies
// ---------------------------------------------------------------------------

/// Build a 24×24 grid with a U-shaped tower wall around the pile center,
/// then spawn an enemy and verify it can pathfind through the opening.
///
/// Validates that enemies pathfind correctly through a U-shaped tower wall
/// after mid-game placement narrows the opening.
#[test]
fn regression_66_three_sided_towers_enemy_still_pathfinds() {
    use bevy_td_sandbox::common::constants::GridConfig;
    use bevy_td_sandbox::pathfinding::systems::recalculate_enemy_paths;
    use bevy_td_sandbox::tower::placement::would_block_all_paths;

    let mut app = test_app();
    app.add_plugins(NorthstarPlugin::<OrdinalNeighborhood>::default());

    // 24×24 grid → 3×3 chunks of size 8.
    let size: u32 = 24;
    let config = GridConfig {
        width: size,
        height: size,
        pixel_scale: 1,
    };
    let center = UVec3::new(size / 2, size / 2, 0); // (12, 12)
    insert_empty_pile(&mut app, 200, config);

    // Override pile state with just the center cell so nearest_pile_cell
    // returns a known value.
    {
        let mut pile = app.world_mut().resource_mut::<PileState>();
        pile.cells.insert(center);
    }
    app.insert_resource(bevy_td_sandbox::pile::resources::EdgeCells(vec![
        UVec3::new(0, size / 2, 0),
    ]));

    let grid_entity = spawn_test_grid(&mut app, size, size);

    // Place towers in a U-shape around the pile center (12, 12).
    // The opening faces south (low-y direction).
    //
    // West wall:  x = 10, y = 10..14
    // East wall:  x = 14, y = 10..14
    // North wall: y = 14, x = 10..14
    let mut tower_cells = Vec::new();
    for y in 10..=14 {
        tower_cells.push(UVec3::new(10, y, 0)); // west
        tower_cells.push(UVec3::new(14, y, 0)); // east
    }
    for x in 11..=13 {
        tower_cells.push(UVec3::new(x, 14, 0)); // north
    }

    {
        let mut state = app.world_mut().query::<&mut OrdinalGrid>();
        let mut grid = state.single_mut(app.world_mut()).unwrap();
        for cell in &tower_cells {
            grid.set_nav(*cell, Nav::Impassable);
        }
        grid.build();
    }

    // --- Diagnostic: verify path exists -----------------------------------------
    let enemy_start = UVec3::new(0, 12, 0);
    let astar_ok;
    let thetastar_ok;
    {
        let mut state = app.world_mut().query::<&OrdinalGrid>();
        let grid = state.single(app.world()).unwrap();
        astar_ok = grid
            .pathfind(&mut PathfindArgs::new(enemy_start, center).astar())
            .is_some();
        thetastar_ok = grid
            .pathfind(&mut PathfindArgs::new(enemy_start, center).thetastar())
            .is_some();
    }
    assert!(
        astar_ok,
        "A* should find a path from enemy start to pile center (south opening exists)"
    );
    assert!(
        thetastar_ok,
        "ThetaStar should also find a path through the south opening"
    );

    // --- Integration: spawn enemy, recalculate, verify path -------------------
    let enemy = EnemyBuilder::new()
        .agent_pos(enemy_start)
        .on_grid(grid_entity)
        .pathfind_to(center)
        .spawn(&mut app);

    // Let bevy_northstar compute paths.
    for _ in 0..5 {
        app.update();
    }

    let has_path =
        app.world().get::<Path>(enemy).is_some() || app.world().get::<NextPos>(enemy).is_some();
    assert!(
        has_path,
        "enemy should have an active path with three-sided tower wall"
    );

    // Simulate a mid-game tower placement: block one more cell on the south
    // opening, then recalculate and verify the enemy reroutes.
    let new_tower = UVec3::new(12, 10, 0);
    {
        let mut state = app.world_mut().query::<&mut OrdinalGrid>();
        let mut grid = state.single_mut(app.world_mut()).unwrap();

        // Validation check (same as update_placing_tower).
        let edge_midpoints = [
            UVec3::new(0, size / 2, 0),
            UVec3::new(size - 1, size / 2, 0),
            UVec3::new(size / 2, 0, 0),
            UVec3::new(size / 2, size - 1, 0),
        ];
        let would_block = would_block_all_paths(&mut grid, new_tower, center, &edge_midpoints);
        assert!(
            !would_block,
            "placing tower at {:?} should not block all paths — south opening \
             still has gaps at (11,10) and (13,10)",
            new_tower
        );

        grid.set_nav(new_tower, Nav::Impassable);
        grid.build();
    }

    // Recalculate (same as GridChanged observer).
    app.add_systems(Update, recalculate_enemy_paths);
    for _ in 0..5 {
        app.update();
    }

    let has_path_after =
        app.world().get::<Path>(enemy).is_some() || app.world().get::<NextPos>(enemy).is_some();
    assert!(
        has_path_after,
        "enemy should still have a path after partial south-wall tower placement"
    );
}

/// #66 root cause: HPA*-based modes (Waypoints, Refined) can't represent
/// diagonal cross-chunk moves in the chunk graph. When towers surround the
/// pile at a chunk boundary so the only entry is a diagonal cross-chunk move,
/// HPA* fails while A*/ThetaStar succeed.
///
/// Fix: enemies now use `PathfindMode::AStar` (same algorithm as validation),
/// which operates on the raw grid and bypasses the HPA* chunk graph entirely.
///
/// This test verifies:
/// 1. A* finds the path (raw grid pathfinding works).
/// 2. Waypoints/HPA* fails (confirming the old bug existed).
#[test]
fn regression_66_diagonal_cross_chunk_mismatch() {
    let mut app = test_app();
    app.add_plugins(NorthstarPlugin::<OrdinalNeighborhood>::default());

    let config = test_grid_config(); // 40×32
    insert_empty_pile(&mut app, 200, config);
    let center = UVec3::new(20, 16, 0);
    {
        let mut pile = app.world_mut().resource_mut::<PileState>();
        pile.cells.insert(center);
    }

    let _grid_entity = spawn_test_grid(&mut app, 40, 32);

    // Towers seal all cardinal + intra-chunk-diagonal access to (20, 16).
    // The ONLY way in is a diagonal cross-chunk move: (19,15)→(20,16).
    let towers = [
        UVec3::new(19, 16, 0), // left on border
        UVec3::new(21, 16, 0), // right on border
        UVec3::new(20, 17, 0), // above (inside chunk)
        UVec3::new(19, 17, 0), // upper-left diagonal (inside chunk)
        UVec3::new(21, 17, 0), // upper-right diagonal (inside chunk)
        UVec3::new(20, 15, 0), // below (adjacent chunk — blocks cardinal border node)
    ];

    {
        let mut state = app.world_mut().query::<&mut OrdinalGrid>();
        let mut grid = state.single_mut(app.world_mut()).unwrap();
        for cell in &towers {
            grid.set_nav(*cell, Nav::Impassable);
        }
        grid.build();
    }

    let enemy_start = UVec3::new(0, 16, 0);

    let astar_found;
    let thetastar_found;
    let waypoints_found;
    {
        let mut state = app.world_mut().query::<&mut OrdinalGrid>();
        let grid = state.single_mut(app.world_mut()).unwrap();
        astar_found = grid
            .pathfind(&mut PathfindArgs::new(enemy_start, center).astar())
            .is_some();
        thetastar_found = grid
            .pathfind(&mut PathfindArgs::new(enemy_start, center).thetastar())
            .is_some();
        waypoints_found = grid
            .pathfind(&mut PathfindArgs::new(enemy_start, center).waypoints())
            .is_some();
    }

    assert!(
        astar_found,
        "A* must find a path via diagonal cross-chunk move (19,15)→(20,16)"
    );
    assert!(
        thetastar_found,
        "ThetaStar (our new enemy pathfinding mode) must also find the path"
    );
    // Waypoints/HPA* can't find the path — this confirms the old bug existed.
    assert!(
        !waypoints_found,
        "Waypoints/HPA* should NOT find this path (confirms diagonal cross-chunk \
         limitation that caused bug #66)"
    );
}
