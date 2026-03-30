use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy;

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

    pub fn ui_color(&self) -> Color {
        match self {
            EnemyType::Shambler => Color::srgb(0.5, 0.9, 0.4),
            EnemyType::Runner => Color::srgb(1.0, 0.9, 0.3),
            EnemyType::Brute => Color::srgb(0.8, 0.4, 0.7),
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

/// Marker for entities pending despawn. All systems must use `Without<Dead>`
/// in queries to avoid operating on doomed entities. Actual despawn happens
/// in `cleanup_dead` which runs last.
#[derive(Component)]
pub struct Dead;

/// Scale-up animation on spawn (enemies & towers).
#[derive(Component)]
pub struct SpawnAnimation {
    pub timer: Timer,
}

/// Marker for entities playing a death animation. Game logic should exclude
/// these via `Without<Dying>` so they aren't targeted or moved.
#[derive(Component)]
pub struct Dying;

/// Shrink + fade death animation. Inserts `Dead` when complete.
#[derive(Component)]
pub struct DeathAnimation {
    pub timer: Timer,
}

/// Small random offset within a cell so enemies don't all walk the exact same pixel path.
#[derive(Component)]
pub struct WanderOffset(pub Vec2);

/// Brief white flash on damage.
#[derive(Component)]
pub struct DamageFlash {
    pub timer: Timer,
    pub original_color: Color,
}

/// Expanding/fading AoE burst visual.
#[derive(Component)]
pub struct AoEBurst {
    pub timer: Timer,
    pub max_radius: f32,
}
