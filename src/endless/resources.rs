use bevy::prelude::*;

#[derive(Resource)]
pub struct EndlessSpawner {
    /// Total elapsed game time in seconds.
    pub elapsed_time: f32,
    /// Timer controlling spawn cadence.
    pub spawn_timer: Timer,
    /// Total enemies spawned (used for escalation thresholds).
    pub enemies_spawned: u32,
}
