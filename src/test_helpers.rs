//! Shared test infrastructure: app setup, resource helpers, and entity builders.
//!
//! Entity builders ([`TowerBuilder`], [`EnemyBuilder`]) centralize component
//! construction so that adding or changing a required component propagates
//! automatically instead of requiring updates at every test spawn site.

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use std::collections::HashSet;

use crate::common::constants::{CHUNK_SIZE, GridConfig};
use crate::enemy::components::{Enemy, EnemyState, Health};
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::{compute_pile_cells, pile_radius};
use crate::states::{GameMode, GameState, PlayPhase};
use crate::stats::resources::RunStats;
use crate::tower::components::{
    Damage, ProjectileVisuals, Range, SlowOnHit, TargetingMode, Tower, TowerState, Turret,
};
use crate::wave::resources::WaveManager;

/// Minimal headless App for testing (time ticking, no window/renderer).
pub fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app
}

/// Minimal headless App with asset support (for testing materials, meshes, etc.).
pub fn test_app_with_assets() -> App {
    let mut app = test_app();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app
}

/// Deterministic 40x32 grid config (equivalent to an 800x640 window).
pub fn test_grid_config() -> GridConfig {
    GridConfig {
        width: 40,
        height: 32,
        pixel_scale: 1,
    }
}

/// Initialize pile-related resources with a known scrap amount.
pub fn insert_pile(app: &mut App, scrap: u32) {
    let config = test_grid_config();
    let center = config.center();
    let radius = pile_radius(scrap);
    let cells = compute_pile_cells(center, radius, config.width, config.height);

    app.insert_resource(PileScrap { amount: scrap });
    app.insert_resource(PileState {
        cells,
        center,
        radius_tiles: radius,
        last_radius_int: radius as u32,
    });
    app.insert_resource(EdgeCells(Vec::new()));
    app.insert_resource(config);
}

/// Initialize pile resources with empty pile state (for manual control).
pub fn insert_empty_pile(app: &mut App, scrap: u32, config: GridConfig) {
    let center = config.center();
    app.insert_resource(PileScrap { amount: scrap });
    app.insert_resource(PileState {
        cells: HashSet::new(),
        center,
        radius_tiles: 0.0,
        last_radius_int: 0,
    });
    app.insert_resource(EdgeCells(Vec::new()));
    app.insert_resource(config);
}

/// Default WaveManager for tests: wave 0, no queue, 1s timer.
pub fn test_wave_manager() -> WaveManager {
    WaveManager {
        current_wave: 0,
        waves: Vec::new(),
        spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        spawn_queue: Vec::new(),
    }
}

/// `RunStats` with `start_time = 0` and all counters zeroed.
pub fn make_test_stats() -> RunStats {
    RunStats::new(0.0)
}

/// Minimal headless App for UI system testing.
///
/// Sets up states ([`GameState`], [`PlayPhase`]) and [`GameMode`] resource,
/// then transitions to `Playing` / `Building` so UI systems can run.
pub fn ui_app() -> App {
    let mut app = test_app();
    app.init_state::<GameState>();
    app.add_sub_state::<PlayPhase>();
    app.init_resource::<GameMode>();

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();
    app.world_mut()
        .resource_mut::<NextState<PlayPhase>>()
        .set(PlayPhase::Building);
    app.update();

    app
}

/// Sensible `BaseStats` defaults for unit tests. Only `cost` varies; other
/// fields use typical ScrapGun-like values.
pub fn test_base_stats(cost: u32) -> crate::tower::components::BaseStats {
    crate::tower::components::BaseStats {
        cost,
        damage: 10.0,
        range: 80.0,
        cooldown_secs: 1.0,
        aoe_radius: 0.0,
        aoe_damage: 0.0,
        slow_factor: 1.0,
        color: Color::srgb(0.5, 0.5, 0.5),
    }
}

// ---------------------------------------------------------------------------
// Entity builders
// ---------------------------------------------------------------------------

/// Spawn an all-passable [`OrdinalGrid`] and return its [`Entity`].
///
/// Every cell is set to `Nav::Passable(1)` before `.build()` is called.
/// `width` and `height` must be multiples of [`CHUNK_SIZE`].
pub fn spawn_test_grid(app: &mut App, width: u32, height: u32) -> Entity {
    debug_assert_eq!(
        width % CHUNK_SIZE,
        0,
        "width must be a multiple of CHUNK_SIZE"
    );
    debug_assert_eq!(
        height % CHUNK_SIZE,
        0,
        "height must be a multiple of CHUNK_SIZE"
    );

    let settings = GridSettingsBuilder::new_2d(width, height)
        .chunk_size(CHUNK_SIZE)
        .build();
    let mut grid = OrdinalGrid::new(&settings);
    for x in 0..width {
        for y in 0..height {
            grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
        }
    }
    grid.build();
    app.world_mut().spawn(grid).id()
}

/// Builder for tower entities in tests.
///
/// Spawns with `Tower`, `TowerState::Active`, and `Transform`. Optional
/// components — `SlowOnHit` (aura) and `Turret` (firing) — are added via
/// method chaining.
///
/// # Defaults
/// - `pos`: `Vec3::ZERO`
/// - `damage`: `0.0`
/// - `range`: `100.0`
///
/// # Examples
/// ```ignore
/// TowerBuilder::aura().spawn(&mut app);
/// TowerBuilder::new().damage(10.0).turret(1.0).spawn(&mut app);
/// ```
pub struct TowerBuilder {
    pos: Vec3,
    damage: f32,
    range: f32,
    slow: Option<(f32, f32)>,
    turret_cooldown: Option<f32>,
    aim_tolerance: Option<f32>,
    projectile_visuals: Option<ProjectileVisuals>,
    targeting: Option<TargetingMode>,
}

impl Default for TowerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TowerBuilder {
    /// Base tower at origin with damage=0, range=100.
    pub fn new() -> Self {
        Self {
            pos: Vec3::ZERO,
            damage: 0.0,
            range: 100.0,
            slow: None,
            turret_cooldown: None,
            aim_tolerance: None,
            projectile_visuals: None,
            targeting: None,
        }
    }

    /// Preset: aura tower with `SlowOnHit { factor: 0.5, duration: 2.0 }`.
    pub fn aura() -> Self {
        Self::new().slow(0.5, 2.0)
    }

    /// Preset: turret tower with damage=10, cooldown=1s, and default visuals.
    pub fn turret() -> Self {
        Self::new().damage(10.0).with_turret(1.0)
    }

    /// Set world position.
    pub fn at(mut self, pos: Vec3) -> Self {
        self.pos = pos;
        self
    }

    /// Set tower damage.
    pub fn damage(mut self, d: f32) -> Self {
        self.damage = d;
        self
    }

    /// Set tower range.
    pub fn range(mut self, r: f32) -> Self {
        self.range = r;
        self
    }

    /// Add `SlowOnHit` component.
    pub fn slow(mut self, factor: f32, duration: f32) -> Self {
        self.slow = Some((factor, duration));
        self
    }

    /// Add a `Turret` aggregate (with default 0.1 aim tolerance), default
    /// `ProjectileVisuals`, and `TargetingMode::Closest`.
    pub fn with_turret(mut self, cooldown: f32) -> Self {
        self.turret_cooldown = Some(cooldown);
        self.aim_tolerance = Some(0.1);
        self.projectile_visuals = Some(ProjectileVisuals {
            speed: 200.0,
            color: Color::WHITE,
            size: Vec2::splat(4.0),
            trail_color: Color::WHITE,
            trail_interval: 0.05,
            particle_size: 2.0,
            particle_lifetime: 0.3,
        });
        self.targeting = Some(TargetingMode::Closest);
        self
    }

    /// Override targeting mode (call after `.with_turret()`).
    pub fn targeting(mut self, mode: TargetingMode) -> Self {
        self.targeting = Some(mode);
        self
    }

    /// Spawn the tower entity and return its [`Entity`].
    pub fn spawn(self, app: &mut App) -> Entity {
        let mut entity = app.world_mut().spawn((
            Tower,
            TowerState::Active,
            Transform::from_translation(self.pos),
        ));
        if let Some((factor, duration)) = self.slow {
            entity.insert(SlowOnHit {
                range: Range(self.range),
                factor,
                duration,
            });
        }
        if let Some(cooldown) = self.turret_cooldown {
            let tolerance = self.aim_tolerance.unwrap_or(0.1);
            entity.insert(Turret::new(
                Damage(self.damage),
                Range(self.range),
                cooldown,
                tolerance,
            ));
        }
        if let Some(visuals) = self.projectile_visuals {
            entity.insert(visuals);
        }
        if let Some(mode) = self.targeting {
            entity.insert(mode);
        }
        entity.id()
    }
}

/// Builder for enemy entities in tests.
///
/// Spawns with `Enemy` and `EnemyState::Approaching` (the default state).
/// All other components are optional and added via method chaining.
///
/// # Minimal spawn
/// `EnemyBuilder::new().spawn(&mut app)` produces only `(Enemy, EnemyState::Approaching)`.
/// For pathfinding tests, also call `.agent_pos()`, `.on_grid()`, and `.pathfind_to()`.
/// For targeting/damage tests, also call `.health()` and `.at()`.
///
/// # Examples
/// ```ignore
/// // Targeting test enemy with health and position
/// EnemyBuilder::new()
///     .health(100.0, 100.0)
///     .at(Vec3::new(30.0, 0.0, 0.0))
///     .spawn(&mut app);
/// ```
pub struct EnemyBuilder {
    pos: Option<Vec3>,
    health: Option<(f32, f32)>,
    agent_pos: Option<UVec3>,
    agent_of_grid: Option<Entity>,
    pathfind_goal: Option<UVec3>,
    next_pos: Option<UVec3>,
    path: Option<(Vec<UVec3>, u32)>,
}

impl Default for EnemyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EnemyBuilder {
    /// Minimal enemy: `Enemy` + `EnemyState::Approaching`.
    pub fn new() -> Self {
        Self {
            pos: None,
            health: None,
            agent_pos: None,
            agent_of_grid: None,
            pathfind_goal: None,
            next_pos: None,
            path: None,
        }
    }

    /// Set world position via `Transform`.
    pub fn at(mut self, pos: Vec3) -> Self {
        self.pos = Some(pos);
        self
    }

    /// Add `Health` component.
    pub fn health(mut self, current: f32, max: f32) -> Self {
        self.health = Some((current, max));
        self
    }

    /// Set `AgentPos` (grid coordinates).
    pub fn agent_pos(mut self, pos: UVec3) -> Self {
        self.agent_pos = Some(pos);
        self
    }

    /// Set `AgentOfGrid` (the grid entity this enemy navigates).
    pub fn on_grid(mut self, grid_entity: Entity) -> Self {
        self.agent_of_grid = Some(grid_entity);
        self
    }

    /// Add `Pathfind` request toward `goal` with `PathfindMode::AStar`.
    pub fn pathfind_to(mut self, goal: UVec3) -> Self {
        self.pathfind_goal = Some(goal);
        self
    }

    /// Set stale `NextPos` (for path-invalidation tests).
    pub fn next_pos(mut self, pos: UVec3) -> Self {
        self.next_pos = Some(pos);
        self
    }

    /// Set stale `Path` (for path-invalidation tests).
    pub fn path(mut self, waypoints: Vec<UVec3>, cost: u32) -> Self {
        self.path = Some((waypoints, cost));
        self
    }

    /// Spawn the enemy entity and return its [`Entity`].
    pub fn spawn(self, app: &mut App) -> Entity {
        let mut entity = app.world_mut().spawn((Enemy, EnemyState::Approaching));
        if let Some(pos) = self.pos {
            entity.insert(Transform::from_translation(pos));
        }
        if let Some((current, max)) = self.health {
            entity.insert(Health { current, max });
        }
        if let Some(pos) = self.agent_pos {
            entity.insert(AgentPos(pos));
        }
        if let Some(grid_entity) = self.agent_of_grid {
            entity.insert(AgentOfGrid(grid_entity));
        }
        if let Some(goal) = self.pathfind_goal {
            entity.insert(Pathfind::new(goal).mode(PathfindMode::AStar));
        }
        if let Some(pos) = self.next_pos {
            entity.insert(NextPos(pos));
        }
        if let Some((waypoints, cost)) = self.path {
            entity.insert(Path::new(waypoints, cost));
        }
        entity.id()
    }
}
