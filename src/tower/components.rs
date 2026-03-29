use bevy::prelude::*;

#[derive(Component)]
pub struct Tower;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TowerType {
    ScrapGun,
    TarPit,
    Explosive,
    Railgun,
}

impl TowerType {
    pub fn cost(&self) -> u32 {
        match self {
            TowerType::ScrapGun => 50,
            TowerType::TarPit => 75,
            TowerType::Explosive => 125,
            TowerType::Railgun => 150,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            TowerType::ScrapGun => Color::srgb(0.7, 0.7, 0.3),
            TowerType::TarPit => Color::srgb(0.3, 0.25, 0.2),
            TowerType::Explosive => Color::srgb(0.9, 0.3, 0.1),
            TowerType::Railgun => Color::srgb(0.3, 0.5, 0.9),
        }
    }

    /// Brighter color for UI text readability
    pub fn ui_color(&self) -> Color {
        match self {
            TowerType::ScrapGun => Color::srgb(0.95, 0.9, 0.4),
            TowerType::TarPit => Color::srgb(0.7, 0.55, 0.35),
            TowerType::Explosive => Color::srgb(1.0, 0.5, 0.2),
            TowerType::Railgun => Color::srgb(0.5, 0.7, 1.0),
        }
    }

    pub fn stats(&self) -> TowerStats {
        match self {
            TowerType::ScrapGun => TowerStats {
                damage: 10.0,
                range: 4.0,
                fire_rate: 1.0,
            },
            TowerType::TarPit => TowerStats {
                damage: 2.0,
                range: 3.5,
                fire_rate: 0.5,
            },
            TowerType::Explosive => TowerStats {
                damage: 25.0,
                range: 5.0,
                fire_rate: 0.3,
            },
            TowerType::Railgun => TowerStats {
                damage: 50.0,
                range: 8.0,
                fire_rate: 0.2,
            },
        }
    }
}

#[derive(Component, Clone)]
pub struct TowerStats {
    pub damage: f32,
    pub range: f32,
    pub fire_rate: f32,
}

#[derive(Component)]
pub struct AttackCooldown {
    pub timer: Timer,
}

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
