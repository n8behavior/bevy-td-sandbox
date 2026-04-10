use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, SCRAP_MAGNET_RANGE, TOWER_HP_COST_MULT};
use crate::tower::components::*;

/// Marker for the dedicated Magnet tower type.
/// Only this tower pulls enemies (via magnetic_pull_enemies).
#[derive(Component)]
pub struct ScrapMagnet;

pub struct ScrapMagnetPlugin;

impl Plugin for ScrapMagnetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Magnet",
        cost: 100,
        color: Color::srgb(0.2, 0.5, 0.8),
        ui_color: Color::srgb(0.4, 0.7, 1.0),
        key: KeyCode::Digit5,
        special_label: "PULL",
        spawn_fn: |cmds| {
            let damage = 0.0;
            let range = SCRAP_MAGNET_RANGE;
            let slow_factor = 0.5;
            let color = Color::srgb(0.2, 0.5, 0.8);
            let stats = TowerStats { damage, range };
            cmds.insert((
                RangeRingConfig {
                    range,
                    color: Color::srgba(0.2, 0.4, 0.8, 0.2),
                },
                SlowAuraRingConfig {
                    range,
                    color: MAGNET_AURA_COLOR,
                },
                ScrapMagnet,
                ScrapCollector { range },
                BlocksNav,
                stats,
                SlowOnHit {
                    factor: slow_factor,
                    duration: 0.5,
                },
                TowerTier(0),
                TowerName("Magnet"),
                BaseStats {
                    cost: 100,
                    damage,
                    range,
                    cooldown_secs: 0.0,
                    aoe_radius: 0.0,
                    aoe_damage: 0.0,
                    slow_factor,
                    color,
                },
            ));
            let max_hp = 100.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
        },
    });
}
