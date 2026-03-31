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
            let stats = TowerStats { damage: 2.0, range: 3.5 };
            spawn_range_ring(cmds, stats.range, Color::srgba(0.3, 0.1, 0.35, 0.25));
            spawn_aura_rings(cmds, stats.range);
            cmds.insert((
                TarPit,
                // No BlocksNav — enemies walk through
                stats,

                SlowOnHit {
                    factor: 0.4,
                    duration: 0.5,
                },
            ));
        },
    });
}
