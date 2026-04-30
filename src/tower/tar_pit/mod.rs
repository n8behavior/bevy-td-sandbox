use bevy::prelude::*;

use crate::common::constants::MAGNET_AURA_COLOR;
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
            let range = 70.0;
            let slow_factor = 0.4;
            let color = Color::srgb(0.3, 0.25, 0.2);
            cmds.insert((
                RangeRingConfig {
                    range,
                    color: Color::srgba(0.3, 0.1, 0.35, 0.25),
                },
                SlowAuraRingConfig {
                    range,
                    color: Color::srgba(0.4, 0.15, 0.45, 0.55),
                },
                CollectionAuraRingConfig {
                    range: 30.0,
                    color: MAGNET_AURA_COLOR,
                },
                ScrapCollector { range: 30.0 },
                MagnetTier(0),
                TarPit,
                // No BlocksNav — enemies walk through
                TowerColor(color),
                SlowOnHit {
                    range: Range(range),
                    factor: slow_factor,
                    duration: 0.5,
                },
                TowerTier(0),
                TowerName("TarPit"),
                PanelStats::default(),
                BaseCost(75),
            ));
        },
    });
}
