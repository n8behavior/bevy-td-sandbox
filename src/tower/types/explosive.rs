use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::tower::components::*;

#[derive(Component)]
pub struct Explosive;

pub struct ExplosivePlugin;

impl Plugin for ExplosivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Explosive",
        cost: 125,
        color: Color::srgb(0.9, 0.3, 0.1),
        ui_color: Color::srgb(1.0, 0.5, 0.2),
        key: KeyCode::Digit3,
        special_label: "AOE",
        spawn_fn: |cmds| {
            let damage = 25.0;
            let range = 100.0;
            let cooldown = 3.33;
            let aoe_radius = 70.0;
            let aoe_damage = 25.0;
            let color = Color::srgb(0.9, 0.3, 0.1);
            let stats = TowerStats { damage, range };
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range: stats.range,
                    color: Color::srgba(0.9, 0.2, 0.0, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                Explosive,
                BlocksNav,
                TargetingMode::default(),
                stats,
                AimTolerance(0.15),
                TurretState::with_cooldown(cooldown),
                ProjectileVisuals {
                    speed: 200.0,
                    color: Color::srgb(1.0, 1.0, 0.6),
                    size: Vec2::splat(6.0),
                    trail_color: Color::srgb(0.9, 0.4, 0.1),
                    trail_interval: 0.03,
                    particle_size: 4.0,
                    particle_lifetime: 0.2,
                },
                AoEOnHit {
                    radius: aoe_radius,
                    damage: aoe_damage,
                },
                TowerTier(0),
                TowerName("Explosive"),
                BaseStats {
                    cost: 125,
                    damage,
                    range,
                    cooldown_secs: cooldown,
                    aoe_radius,
                    aoe_damage,
                    slow_factor: 1.0,
                    color,
                },
            ));
            let max_hp = 125.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
            cmds.insert((
                MagnetTier(0),
                BaseMagnetRange(collect_range),
                MagnetAuraConfig {
                    range: collect_range,
                    color: MAGNET_AURA_COLOR,
                },
            ));
        },
    });
}
