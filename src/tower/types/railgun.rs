use bevy::prelude::*;

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
        color: Color::srgb(0.3, 0.5, 0.9),
        ui_color: Color::srgb(0.5, 0.7, 1.0),
        key: KeyCode::Digit4,
        special_label: "",
        spawn_fn: |cmds, circle| {
            let stats = TowerStats { damage: 50.0, range: 160.0 };
            spawn_range_ring(cmds, stats.range, Color::srgba(0.6, 0.6, 0.0, 0.15), circle);
            cmds.insert((
                Railgun,
                BlocksNav,
                stats,
                AimTolerance(0.05),
                TurretState::with_cooldown(5.0),

                ProjectileVisuals {
                    speed: 2000.0,
                    color: Color::srgb(0.6, 0.8, 1.0),
                    size: Vec2::new(10.0, 4.0),
                    trail_color: Color::srgb(0.4, 0.7, 1.0),
                    trail_interval: 0.008,
                    particle_size: 6.0,
                    particle_lifetime: 0.4,
                },
            ));
        },
    });
}
