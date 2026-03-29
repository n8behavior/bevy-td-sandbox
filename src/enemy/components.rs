use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy {
    pub enemy_type: EnemyType,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyType {
    Shambler,
    Runner,
    Brute,
}

impl EnemyType {
    pub fn color(&self) -> Color {
        match self {
            EnemyType::Shambler => Color::srgb(0.4, 0.7, 0.3),
            EnemyType::Runner => Color::srgb(0.9, 0.8, 0.2),
            EnemyType::Brute => Color::srgb(0.6, 0.2, 0.5),
        }
    }

    pub fn base_health(&self) -> f32 {
        match self {
            EnemyType::Shambler => 50.0,
            EnemyType::Runner => 30.0,
            EnemyType::Brute => 150.0,
        }
    }

    pub fn base_speed(&self) -> f32 {
        match self {
            EnemyType::Shambler => 40.0,
            EnemyType::Runner => 80.0,
            EnemyType::Brute => 25.0,
        }
    }

    pub fn loot_value(&self) -> u32 {
        match self {
            EnemyType::Shambler => 10,
            EnemyType::Runner => 15,
            EnemyType::Brute => 30,
        }
    }

    pub fn size(&self) -> f32 {
        match self {
            EnemyType::Shambler => 14.0,
            EnemyType::Runner => 10.0,
            EnemyType::Brute => 18.0,
        }
    }
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
pub struct MoveSpeed {
    pub base: f32,
    pub current: f32,
}

#[derive(Component)]
pub struct SlowEffect {
    pub factor: f32,
    pub remaining: Timer,
}

#[derive(Component)]
pub struct LootValue(pub u32);
