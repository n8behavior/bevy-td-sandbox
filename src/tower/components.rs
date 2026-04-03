use bevy::prelude::*;

/// Pre-generated circle texture for range ring rendering.
#[derive(Resource)]
pub struct CircleImage(pub Handle<Image>);

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
    pub spawn_fn: fn(&mut EntityCommands, &CircleImage),
}

/// Registry of all available tower types. Tower-type plugins push blueprints
/// here during startup.
#[derive(Resource, Default)]
pub struct TowerRegistry {
    pub blueprints: Vec<TowerBlueprint>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Spawn a circular range ring preview as a child (despawned on placement).
pub fn spawn_range_ring(cmds: &mut EntityCommands, range: f32, color: Color, circle: &CircleImage) {
    cmds.with_child((
        RangeRing,
        Sprite {
            image: circle.0.clone(),
            color,
            custom_size: Some(Vec2::splat(range * 2.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
    ));
}

/// Spawn gradient circular aura rings as children (for aura-type towers).
pub fn spawn_aura_rings(cmds: &mut EntityCommands, range: f32, circle: &CircleImage) {
    let rings = 5;
    for i in 0..rings {
        let frac = (i + 1) as f32 / rings as f32;
        let size = range * 2.0 * frac;
        let alpha = 0.45 * (1.0 - frac * 0.6);
        cmds.with_child((
            AuraVisual,
            Sprite {
                image: circle.0.clone(),
                color: Color::srgba(0.3, 0.1, 0.35, alpha),
                custom_size: Some(Vec2::splat(size)),
                ..default()
            },
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.2)),
        ));
    }
}
