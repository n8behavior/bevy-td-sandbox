use bevy::prelude::*;

#[derive(Component)]
pub struct Enemy;

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyType {
    Shambler,
    Runner,
    Brute,
    Boss,
}

impl EnemyType {
    pub fn color(&self) -> Color {
        match self {
            EnemyType::Shambler => Color::srgb(0.4, 0.7, 0.3),
            EnemyType::Runner => Color::srgb(0.9, 0.8, 0.2),
            EnemyType::Brute => Color::srgb(0.6, 0.2, 0.5),
            EnemyType::Boss => Color::srgb(0.8, 0.1, 0.1),
        }
    }

    pub fn base_health(&self) -> f32 {
        match self {
            EnemyType::Shambler => 50.0,
            EnemyType::Runner => 30.0,
            EnemyType::Brute => 150.0,
            EnemyType::Boss => 500.0,
        }
    }

    pub fn base_speed(&self) -> f32 {
        match self {
            EnemyType::Shambler => 40.0,
            EnemyType::Runner => 80.0,
            EnemyType::Brute => 25.0,
            EnemyType::Boss => 20.0,
        }
    }

    pub fn loot_value(&self) -> u32 {
        match self {
            EnemyType::Shambler => 10,
            EnemyType::Runner => 15,
            EnemyType::Brute => 30,
            EnemyType::Boss => 150,
        }
    }

    pub fn ui_color(&self) -> Color {
        match self {
            EnemyType::Shambler => Color::srgb(0.5, 0.9, 0.4),
            EnemyType::Runner => Color::srgb(1.0, 0.9, 0.3),
            EnemyType::Brute => Color::srgb(0.8, 0.4, 0.7),
            EnemyType::Boss => Color::srgb(1.0, 0.3, 0.3),
        }
    }

    pub fn size(&self) -> f32 {
        match self {
            EnemyType::Shambler => 14.0,
            EnemyType::Runner => 10.0,
            EnemyType::Brute => 18.0,
            EnemyType::Boss => 28.0,
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

/// Tracks whether an enemy is approaching the pile or fleeing with stolen scrap.
#[derive(Component, Default, PartialEq, Eq, Debug)]
pub enum EnemyPhase {
    #[default]
    Approaching,
    Fleeing,
}

/// Scrap stolen from the pile that the enemy is carrying.
#[derive(Component)]
pub struct StolenScrap(pub u32);

/// Marker for the visual decal on enemies carrying stolen scrap.
#[derive(Component)]
pub struct ScrapCarrierDecal;

/// Idle wander state for enemies searching the pile.
#[derive(Component)]
pub struct SearchWander {
    pub target: Vec2,
    pub timer: Timer,
}

/// Boss trait: slow HP recovery over time.
#[derive(Component)]
pub struct Regeneration {
    pub rate: f32,
}

/// Boss trait: flat damage reduction per hit (minimum 1 damage).
#[derive(Component)]
pub struct Armor {
    pub reduction: f32,
}

/// Boss trait: on death, spawn smaller enemies at this position.
#[derive(Component)]
pub struct SplitsOnDeath {
    pub count: u32,
}

/// Brute tower attack: cooldown timer and damage per hit.
/// Only present on Brute enemies.
#[derive(Component)]
pub struct BruteAttack {
    pub cooldown: Timer,
    pub damage: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_TYPES: [EnemyType; 4] = [
        EnemyType::Shambler,
        EnemyType::Runner,
        EnemyType::Brute,
        EnemyType::Boss,
    ];

    #[test]
    fn all_types_have_positive_stats() {
        for ty in &ALL_TYPES {
            assert!(ty.base_health() > 0.0, "{ty:?} health");
            assert!(ty.base_speed() > 0.0, "{ty:?} speed");
            assert!(ty.loot_value() > 0, "{ty:?} loot");
            assert!(ty.size() > 0.0, "{ty:?} size");
        }
    }

    #[test]
    fn runner_faster_than_shambler() {
        assert!(EnemyType::Runner.base_speed() > EnemyType::Shambler.base_speed());
    }

    #[test]
    fn boss_has_highest_health() {
        let boss_hp = EnemyType::Boss.base_health();
        for ty in &ALL_TYPES {
            assert!(boss_hp >= ty.base_health(), "{ty:?} has more HP than Boss");
        }
    }

    #[test]
    fn boss_has_highest_loot() {
        let boss_loot = EnemyType::Boss.loot_value();
        for ty in &ALL_TYPES {
            assert!(
                boss_loot >= ty.loot_value(),
                "{ty:?} has more loot than Boss"
            );
        }
    }

    #[test]
    fn brute_has_second_highest_loot() {
        assert!(EnemyType::Brute.loot_value() > EnemyType::Shambler.loot_value());
        assert!(EnemyType::Brute.loot_value() > EnemyType::Runner.loot_value());
    }
}
