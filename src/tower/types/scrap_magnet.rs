use bevy::prelude::*;

use crate::common::constants::MAGNET_AURA_COLOR;
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
            let magnet_range = 90.0;
            let stats = TowerStats {
                damage: 0.0,
                range: magnet_range,
            };
            cmds.insert((
                RangeRingConfig {
                    range: magnet_range,
                    color: Color::srgba(0.2, 0.4, 0.8, 0.2),
                },
                AuraRingConfig {
                    range: magnet_range,
                    color: MAGNET_AURA_COLOR,
                },
                ScrapMagnet,
                ScrapCollector {
                    range: magnet_range,
                },
                BlocksNav,
                stats,
                SlowOnHit {
                    factor: 0.5,
                    duration: 0.5,
                },
            ));
        },
    });
}
