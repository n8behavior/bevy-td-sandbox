use crate::enemy::components::EnemyType;
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BossTrait {
    Regeneration,
    Armor,
    Splitting,
}

#[derive(Resource)]
pub struct WaveManager {
    pub current_wave: u32,
    pub waves: Vec<WaveConfig>,
    pub spawn_timer: Timer,
    /// Pre-shuffled queue of enemies to spawn this wave.
    pub spawn_queue: Vec<SpawnEntry>,
}

/// A single enemy to spawn (flattened from WaveEnemy × count).
pub struct SpawnEntry {
    pub enemy_type: EnemyType,
    pub health_multiplier: f32,
    pub speed_multiplier: f32,
    pub boss_trait: Option<BossTrait>,
}

pub struct WaveConfig {
    pub enemies: Vec<WaveEnemy>,
    pub spawn_interval: f32,
}

pub struct WaveEnemy {
    pub enemy_type: EnemyType,
    pub count: u32,
    pub health_multiplier: f32,
    pub speed_multiplier: f32,
    pub boss_trait: Option<BossTrait>,
}
