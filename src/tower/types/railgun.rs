use bevy::prelude::*;

use crate::common::constants::MAGNET_AURA_COLOR;
use crate::tower::components::*;

#[derive(Component)]
pub struct Railgun;

pub struct RailgunPlugin;

impl Plugin for RailgunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Railgun",
        cost: 150,
        color: Color::srgb(0.6, 0.65, 0.75),
        ui_color: Color::srgb(0.75, 0.8, 0.9),
        key: KeyCode::Digit4,
        special_label: "",
        spawn_fn: |cmds| {
            let damage = 50.0;
            let range = 160.0;
            let cooldown = 5.0;
            let color = Color::srgb(0.6, 0.65, 0.75);
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
                Railgun,
                BlocksNav,
                TargetingMode::default(),
                stats,
                AimTolerance(0.05),
                TurretState::with_cooldown(cooldown),
                ProjectileVisuals {
                    speed: 2000.0,
                    color: Color::srgb(0.6, 0.8, 1.0),
                    size: Vec2::new(10.0, 4.0),
                    trail_color: Color::srgb(0.4, 0.7, 1.0),
                    trail_interval: 0.008,
                    particle_size: 6.0,
                    particle_lifetime: 0.4,
                },
                TowerTier(0),
                TowerName("Railgun"),
                BaseStats {
                    cost: 150,
                    damage,
                    range,
                    cooldown_secs: cooldown,
                    aoe_radius: 0.0,
                    aoe_damage: 0.0,
                    slow_factor: 1.0,
                    color,
                },
            ));
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
