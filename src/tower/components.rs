use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Base tower marker
// ---------------------------------------------------------------------------

/// Base marker for all towers.
#[derive(Component)]
pub struct Tower;

// ---------------------------------------------------------------------------
// Placement state
// ---------------------------------------------------------------------------

/// Tower is being positioned by the player — not yet committed to the grid.
#[derive(Component)]
pub struct Placing;

/// Whether the tower's current placement position is valid.
#[derive(Component)]
pub struct PlacementValid(pub bool);

// ---------------------------------------------------------------------------
// Behavioral components (shared across tower types)
// ---------------------------------------------------------------------------

/// Entities with this block navigation and require path validation on placement.
#[derive(Component)]
pub struct BlocksNav;

#[derive(Component, Clone)]
pub struct TowerStats {
    pub damage: f32,
    /// Effective range in world units.
    pub range: f32,
}

/// Aim tolerance in radians. Only on turret towers.
#[derive(Component)]
pub struct AimTolerance(pub f32);

#[derive(Component)]
pub struct SlowOnHit {
    pub factor: f32,
    pub duration: f32,
}

#[derive(Component)]
pub struct AoEOnHit {
    pub radius: f32,
    pub damage: f32,
}

/// Marker for the visual aura ring sprites (children of an aura tower).
#[derive(Component)]
pub struct AuraVisual;

/// Marker for the range ring preview child (despawned on placement).
#[derive(Component)]
pub struct RangeRing;

/// Projectile visual configuration. Towers with this fire projectiles.
#[derive(Component, Clone)]
pub struct ProjectileVisuals {
    pub speed: f32,
    pub color: Color,
    pub size: Vec2,
    pub trail_color: Color,
    pub trail_interval: f32,
    pub particle_size: f32,
    pub particle_lifetime: f32,
}


// ---------------------------------------------------------------------------
// Turret state machine
// ---------------------------------------------------------------------------

/// Turret firing state machine. Ticks cooldown in all phases, fires when
/// aimed + cooldown ready. Only present on projectile-firing towers.
#[derive(Component)]
pub struct TurretState {
    pub phase: TurretPhase,
    pub cooldown: Timer,
}

impl TurretState {
    pub fn with_cooldown(secs: f32) -> Self {
        let mut cooldown = Timer::from_seconds(secs, TimerMode::Once);
        // Start fully charged so first shot fires on aim lock.
        cooldown.tick(cooldown.duration());
        Self {
            phase: TurretPhase::Idle,
            cooldown,
        }
    }

    /// Extract target entity from any phase that has one.
    pub fn target(&self) -> Option<Entity> {
        match self.phase {
            TurretPhase::Acquiring { target } | TurretPhase::Tracking { target } => Some(target),
            TurretPhase::Idle => None,
        }
    }
}

#[derive(Default)]
pub enum TurretPhase {
    #[default]
    Idle,
    Acquiring {
        target: Entity,
    },
    Tracking {
        target: Entity,
    },
}

// ---------------------------------------------------------------------------
// Tower registry
// ---------------------------------------------------------------------------

/// A blueprint describing how to spawn a tower type. Tower-type plugins
/// register one of these during startup.
pub struct TowerBlueprint {
    pub name: &'static str,
    pub cost: u32,
    pub color: Color,
    pub ui_color: Color,
    pub key: KeyCode,
    pub special_label: &'static str,
    /// Called on the EntityCommands of a freshly spawned tower entity to insert
    /// all type-specific components (marker, stats, visuals, etc.).
    pub spawn_fn: fn(&mut EntityCommands),
}

/// Registry of all available tower types. Tower-type plugins push blueprints
/// here during startup.
#[derive(Resource, Default)]
pub struct TowerRegistry {
    pub blueprints: Vec<TowerBlueprint>,
}

// ---------------------------------------------------------------------------
// Range ring config (reactive system spawns the actual shader mesh)
// ---------------------------------------------------------------------------

/// Insert on a tower entity to request a range ring child.
/// A reactive system converts this into a `Mesh2d` + `CircleMaterial`.
#[derive(Component)]
pub struct RangeRingConfig {
    pub range: f32,
    pub color: Color,
}

/// Insert on a tower entity to request gradient aura ring children.
#[derive(Component)]
pub struct AuraRingConfig {
    pub range: f32,
    pub color: Color,
}

/// Pulls nearby scrap drops toward this entity and auto-collects on contact.
/// Present on the pile, the dedicated Magnet tower, and mechanical towers.
#[derive(Component)]
pub struct ScrapCollector {
    pub range: f32,
}
