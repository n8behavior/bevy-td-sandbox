use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::tower::components::*;

#[derive(Component)]
pub struct ScrapGun;

pub struct ScrapGunPlugin;

impl Plugin for ScrapGunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "ScrapGun",
        cost: 50,
        color: Color::srgb(0.7, 0.7, 0.3),
        ui_color: Color::srgb(0.95, 0.9, 0.4),
        key: KeyCode::Digit1,
        special_label: "",
        spawn_fn: |cmds| {
            let damage = 10.0;
            let range = 80.0;
            let cooldown = 1.0;
            let color = Color::srgb(0.7, 0.7, 0.3);
            let stats = TowerStats { damage, range };
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range: stats.range,
                    color: Color::srgba(0.6, 0.6, 0.0, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                ScrapGun,
                BlocksNav,
                TargetingMode::default(),
                stats,
                AimTolerance(0.15),
                TurretState::with_cooldown(cooldown),
                ProjectileVisuals {
                    speed: 200.0,
                    color: Color::srgb(1.0, 1.0, 0.6),
                    size: Vec2::splat(6.0),
                    trail_color: Color::srgb(1.0, 1.0, 0.4),
                    trail_interval: 0.03,
                    particle_size: 4.0,
                    particle_lifetime: 0.2,
                },
                TowerTier(0),
                TowerName("ScrapGun"),
                BaseStats {
                    cost: 50,
                    damage,
                    range,
                    cooldown_secs: cooldown,
                    aoe_radius: 0.0,
                    aoe_damage: 0.0,
                    slow_factor: 1.0,
                    color,
                },
            ));
            let max_hp = 50.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
            cmds.insert((
                MagnetTier(0),
                BaseMagnetRange(collect_range),
                CollectionAuraRingConfig {
                    range: collect_range,
                    color: MAGNET_AURA_COLOR,
                },
            ));
        },
    });
}
