use bevy::prelude::*;

use crate::tower::components::*;

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
            let stats = TowerStats { damage: 0.0, range: 90.0 };
            cmds.insert((
                RangeRingConfig { range: stats.range, color: Color::srgba(0.2, 0.4, 0.8, 0.2) },
                AuraRingConfig { range: stats.range, color: Color::srgba(0.15, 0.35, 0.7, 0.55) },
                ScrapMagnet,
                BlocksNav,
                stats,
                SlowOnHit { factor: 0.5, duration: 0.5 },
            ));
        },
    });
}
