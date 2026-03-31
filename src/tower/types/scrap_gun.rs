use bevy::prelude::*;

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
            let stats = TowerStats { damage: 10.0, range: 4.0 };
            spawn_range_ring(cmds, stats.range, Color::srgba(0.8, 0.8, 0.3, 0.08));
            cmds.insert((
                ScrapGun,
                BlocksNav,
                stats,
                AimTolerance(0.15),
                TurretState::with_cooldown(1.0),

                ProjectileVisuals {
                    speed: 200.0,
                    color: Color::srgb(1.0, 1.0, 0.6),
                    size: Vec2::splat(6.0),
                    trail_color: Color::srgb(1.0, 1.0, 0.4),
                    trail_interval: 0.03,
                    particle_size: 4.0,
                    particle_lifetime: 0.2,
                },
            ));
        },
    });
}
