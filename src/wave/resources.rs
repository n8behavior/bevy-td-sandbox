use bevy::prelude::*;
use crate::enemy::components::EnemyType;

#[derive(Resource)]
pub struct WaveManager {
    pub current_wave: u32,
    pub waves: Vec<WaveConfig>,
    pub spawn_timer: Timer,
    pub enemies_remaining: u32,
    pub enemies_spawned: u32,
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
}
