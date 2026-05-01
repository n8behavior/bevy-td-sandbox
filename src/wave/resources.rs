use bevy::prelude::*;

#[derive(Resource)]
pub struct WaveManager {
    pub current_wave: u32,
    pub waves: Vec<WaveConfig>,
    pub spawn_timer: Timer,
    /// Pre-shuffled queue of enemies to spawn this wave.
    pub spawn_queue: Vec<SpawnEntry>,
}

/// A single enemy to spawn (flattened from `WaveEnemy` × count).
///
/// Carries only the blueprint name. Wave-difficulty scaling is applied
/// at spawn time via the `EnemySpawned` event — wave/ doesn't compute
/// multipliers anymore. Capabilities (e.g. armor, splitting) are
/// declared by the blueprint itself.
pub struct SpawnEntry {
    pub enemy_blueprint: &'static str,
}

pub struct WaveConfig {
    pub enemies: Vec<WaveEnemy>,
    pub spawn_interval: f32,
}

pub struct WaveEnemy {
    pub enemy_blueprint: &'static str,
    pub count: u32,
}
