use bevy::prelude::*;

use crate::common::constants::MAGNET_AURA_COLOR;
use crate::tower::components::*;

#[derive(Component)]
pub struct Explosive;

pub struct ExplosivePlugin;

impl Plugin for ExplosivePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Explosive",
        cost: 125,
        color: Color::srgb(0.9, 0.3, 0.1),
        ui_color: Color::srgb(1.0, 0.5, 0.2),
        key: KeyCode::Digit3,
        special_label: "AOE",
        spawn_fn: |cmds| {
            let stats = TowerStats { damage: 25.0, range: 100.0 };
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig { range: stats.range, color: Color::srgba(0.9, 0.2, 0.0, 0.15) },
                AuraRingConfig { range: collect_range, color: MAGNET_AURA_COLOR },
                ScrapCollector { range: collect_range },
                Explosive,
                BlocksNav,
                stats,
                AimTolerance(0.15),
                TurretState::with_cooldown(3.33),

                ProjectileVisuals {
                    speed: 200.0,
                    color: Color::srgb(1.0, 1.0, 0.6),
                    size: Vec2::splat(6.0),
                    trail_color: Color::srgb(0.9, 0.4, 0.1),
                    trail_interval: 0.03,
                    particle_size: 4.0,
                    particle_lifetime: 0.2,
                },
                AoEOnHit {
                    radius: 70.0,
                    damage: 25.0,
                },
            ));
        },
    });
}
