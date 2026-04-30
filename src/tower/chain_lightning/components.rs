//! Components specific to the Chain Lightning tower.

use bevy::prelude::*;

use crate::tower::components::{Damage, Range};

/// Marker component for Chain Lightning towers.
#[derive(Component)]
pub struct ChainLightningMarker;

/// Chain lightning capability. Owns the per-shot damage, the primary
/// acquisition range used to find the first target, the per-hop arc range
/// used to walk the chain, and the falloff applied to each hop's damage.
/// Replaces the previous reliance on the legacy `TowerStats` for damage and
/// range.
#[derive(Component)]
pub struct ChainLightning {
    pub damage: Damage,
    pub primary_range: Range,
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
