use bevy::prelude::*;

use crate::tower::components::*;

#[derive(Component)]
pub struct TarPit;

pub struct TarPitPlugin;

impl Plugin for TarPitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "TarPit",
        cost: 75,
        color: Color::srgb(0.3, 0.25, 0.2),
        ui_color: Color::srgb(0.7, 0.55, 0.35),
        key: KeyCode::Digit2,
        special_label: "SLOW",
        spawn_fn: |cmds| {
            let damage = 2.0;
            let range = 70.0;
            let slow_factor = 0.4;
            let color = Color::srgb(0.3, 0.25, 0.2);
            let stats = TowerStats { damage, range };
            cmds.insert((
                RangeRingConfig {
                    range: stats.range,
                    color: Color::srgba(0.3, 0.1, 0.35, 0.25),
                },
                AuraRingConfig {
                    range: stats.range,
                    color: Color::srgba(0.4, 0.15, 0.45, 0.55),
                },
                TarPit,
                // No BlocksNav — enemies walk through
                stats,
                SlowOnHit {
                    factor: slow_factor,
                    duration: 0.5,
                },
                TowerTier(0),
                TowerName("TarPit"),
                BaseStats {
                    cost: 75,
                    damage,
                    range,
                    cooldown_secs: 0.0,
                    aoe_radius: 0.0,
                    aoe_damage: 0.0,
                    slow_factor,
                    color,
                },
            ));
        },
    });
}
