//! Components specific to the Chain Lightning tower.

use bevy::prelude::*;

/// Marker component for Chain Lightning towers.
#[derive(Component)]
pub struct ChainLightningMarker;

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
