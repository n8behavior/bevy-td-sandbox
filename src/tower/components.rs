use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Base tower marker
// ---------------------------------------------------------------------------

/// Base marker for all towers.
#[derive(Component)]
pub struct Tower;

/// Tower lifecycle state.
#[derive(Component, Default, PartialEq, Eq, Debug, Clone, Copy)]
pub enum TowerState {
    #[default]
    Placing, // being positioned by player, not committed
    Active, // placed and functional
    Rubble, // destroyed, non-functional but still impassable
}

impl TowerState {
    /// Tower is placed and operational (not placing, not rubble).
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Active)
    }
    /// Tower exists on the grid (not in placement preview).
    pub fn is_placed(&self) -> bool {
        matches!(self, Self::Active | Self::Rubble)
    }
}

/// Per-tower targeting priority. Only present on towers with active targeting
/// (turret towers and chain lightning). Aura towers (TarPit, Magnet) don't get this.
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TargetingMode {
    Closest,
    #[default]
    LowestHp,
    HighestHp,
    FurthestAlongPath,
}

impl TargetingMode {
    /// Short label for world-space display on the tower sprite.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Closest => "C",
            Self::LowestHp => "L",
            Self::HighestHp => "H",
            Self::FurthestAlongPath => "F",
        }
    }

    /// Human-readable name for the upgrade panel.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Closest => "Closest",
            Self::LowestHp => "Lowest HP",
            Self::HighestHp => "Highest HP",
            Self::FurthestAlongPath => "Furthest Along",
        }
    }

    /// All modes in radial menu order: top, right, bottom, left.
    pub const ALL: [TargetingMode; 4] = [
        Self::Closest,
        Self::HighestHp,
        Self::FurthestAlongPath,
        Self::LowestHp,
    ];
}

/// Marker for the Text2d child that shows the targeting mode letter on a tower.
#[derive(Component)]
pub struct TargetingModeLabel;

// ---------------------------------------------------------------------------
// Placement state
// ---------------------------------------------------------------------------

/// Whether the tower's current placement position is valid.
#[derive(Component)]
pub struct PlacementValid(pub bool);

// ---------------------------------------------------------------------------
// Behavioral components (shared across tower types)
// ---------------------------------------------------------------------------

/// Entities with this block navigation and require path validation on placement.
#[derive(Component)]
pub struct BlocksNav;

/// Original cost of a placed tower (used for sell refund calculation).
/// Accumulates total investment when upgraded (base cost + upgrade costs).
#[derive(Component)]
pub struct TowerCost(pub u32);

/// Current upgrade tier: 0 = base, 1, 2 = max.
#[derive(Component)]
pub struct TowerTier(pub u8);

/// Snapshot of a tower's base stats at placement time (before upgrades).
/// Used to compute multiplied stats at each tier without accumulation errors.
#[derive(Component, Clone)]
pub struct BaseStats {
    pub cost: u32,
    pub damage: f32,
    pub range: f32,
    pub cooldown_secs: f32,
    pub aoe_radius: f32,
    pub aoe_damage: f32,
    pub slow_factor: f32,
    pub color: Color,
}

/// Display name for a placed tower (set by the tower-type plugin).
#[derive(Component)]
pub struct TowerName(pub &'static str);

/// Marker for the selection ring child spawned on the inspected tower.
#[derive(Component)]
pub struct SelectionRing;

/// Brief white flash after upgrading a tower.
#[derive(Component)]
pub struct UpgradeFlash {
    pub timer: Timer,
    pub target_color: Color,
}

/// Tower hit points. Only on BlocksNav towers (impassable towers that can be attacked).
/// When current reaches 0, the tower becomes rubble.
#[derive(Component)]
pub struct TowerHealth {
    pub current: f32,
    pub max: f32,
}

impl TowerHealth {
    /// Effectiveness multiplier based on HP thresholds.
    /// >50% → 1.0, 25–50% → 0.75, <25% → 0.5, 0% → 0.0
    pub fn effectiveness(&self) -> f32 {
        let frac = self.fraction();
        if frac > 0.5 {
            1.0
        } else if frac > 0.25 {
            0.75
        } else if frac > 0.0 {
            0.5
        } else {
            0.0
        }
    }

    /// HP fraction 0.0..1.0.
    pub fn fraction(&self) -> f32 {
        (self.current / self.max).clamp(0.0, 1.0)
    }
}

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

/// Chain lightning behavior config.
#[derive(Component)]
pub struct ChainLightning {
    pub arc_range: f32,
    pub damage_falloff: f32,
}

/// Base arc range for upgrade calculations (immutable snapshot).
#[derive(Component)]
pub struct BaseArcRange(pub f32);

/// Cooldown timer for instant-fire towers (no aiming phase).
#[derive(Component)]
pub struct ChainCooldown {
    pub timer: Timer,
}

impl ChainCooldown {
    pub fn new(secs: f32) -> Self {
        let mut timer = Timer::from_seconds(secs, TimerMode::Once);
        // Start fully charged so first shot fires immediately.
        timer.tick(timer.duration());
        Self { timer }
    }
}

/// Lightning arc visual (fading line segment between two points).
#[derive(Component)]
pub struct LightningArc {
    pub timer: Timer,
}

/// Pulls nearby scrap drops toward this entity and auto-collects on contact.
/// Present on the pile, the dedicated Magnet tower, and mechanical towers.
#[derive(Component)]
pub struct ScrapCollector {
    pub range: f32,
}

// ---------------------------------------------------------------------------
// Magnet upgrade components
// ---------------------------------------------------------------------------

/// Current magnet upgrade tier: 0 = base, 1–3 = upgraded.
/// Separate from TowerTier (combat upgrades). Not present on ScrapMagnet
/// (which has built-in max collection range).
#[derive(Component)]
pub struct MagnetTier(pub u8);

/// Immutable base scrap collection range, used to compute upgraded range
/// via multiplier table. Set at tower spawn time.
#[derive(Component)]
pub struct BaseMagnetRange(pub f32);

/// Insert on a tower entity to request a magnet collection aura ring child.
/// A reactive system converts this into a `Mesh2d` + `CircleMaterial`.
/// Separate from `AuraRingConfig` so towers can have both (e.g. TarPit has
/// a slow aura AND a collection aura).
#[derive(Component)]
pub struct MagnetAuraConfig {
    pub range: f32,
    pub color: Color,
}

/// Marker for the magnet collection aura ring child (spawned from
/// `MagnetAuraConfig`). Distinct from `AuraVisual` to avoid interference
/// during tower stat upgrades.
#[derive(Component)]
pub struct MagnetAura;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effectiveness_full_health() {
        let h = TowerHealth {
            current: 100.0,
            max: 100.0,
        };
        assert_eq!(h.effectiveness(), 1.0);
    }

    #[test]
    fn effectiveness_above_half() {
        let h = TowerHealth {
            current: 51.0,
            max: 100.0,
        };
        assert_eq!(h.effectiveness(), 1.0);
    }

    #[test]
    fn effectiveness_at_half() {
        let h = TowerHealth {
            current: 50.0,
            max: 100.0,
        };
        // 50% is not > 50%, so falls to 0.75
        assert_eq!(h.effectiveness(), 0.75);
    }

    #[test]
    fn effectiveness_above_quarter() {
        let h = TowerHealth {
            current: 26.0,
            max: 100.0,
        };
        assert_eq!(h.effectiveness(), 0.75);
    }

    #[test]
    fn effectiveness_at_quarter() {
        let h = TowerHealth {
            current: 25.0,
            max: 100.0,
        };
        // 25% is not > 25%, so falls to 0.5
        assert_eq!(h.effectiveness(), 0.5);
    }

    #[test]
    fn effectiveness_low_health() {
        let h = TowerHealth {
            current: 1.0,
            max: 100.0,
        };
        assert_eq!(h.effectiveness(), 0.5);
    }

    #[test]
    fn effectiveness_zero_health() {
        let h = TowerHealth {
            current: 0.0,
            max: 100.0,
        };
        assert_eq!(h.effectiveness(), 0.0);
    }

    #[test]
    fn fraction_clamped() {
        let over = TowerHealth {
            current: 150.0,
            max: 100.0,
        };
        assert_eq!(over.fraction(), 1.0);

        let under = TowerHealth {
            current: -10.0,
            max: 100.0,
        };
        assert_eq!(under.fraction(), 0.0);
    }

    #[test]
    fn tower_state_is_operational() {
        assert!(!TowerState::Placing.is_operational());
        assert!(TowerState::Active.is_operational());
        assert!(!TowerState::Rubble.is_operational());
    }

    #[test]
    fn tower_state_is_placed() {
        assert!(!TowerState::Placing.is_placed());
        assert!(TowerState::Active.is_placed());
        assert!(TowerState::Rubble.is_placed());
    }
}
