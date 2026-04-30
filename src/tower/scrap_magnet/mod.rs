pub mod systems;

use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, SCRAP_MAGNET_RANGE, TOWER_HP_COST_MULT};
use crate::states::{GameState, PlayPhase};
use crate::tower::components::*;

/// Marker for the dedicated Magnet tower type.
/// Only this tower pulls enemies (via magnetic_pull_enemies).
#[derive(Component)]
pub struct ScrapMagnet;

pub struct ScrapMagnetPlugin;

impl Plugin for ScrapMagnetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register).add_systems(
            FixedUpdate,
            systems::magnetic_pull_enemies
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending)),
        );
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
            let range = SCRAP_MAGNET_RANGE;
            let slow_factor = 0.5;
            let color = Color::srgb(0.2, 0.5, 0.8);
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
                TowerColor(color),
                SlowOnHit {
                    range: Range(range),
                    factor: slow_factor,
                    duration: 0.5,
                },
                TowerTier(0),
                TowerName("Magnet"),
                PanelStats::default(),
                BaseCost(100),
            ));
            let max_hp = 100.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
        },
    });
}
