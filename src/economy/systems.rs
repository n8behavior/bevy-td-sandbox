use bevy::prelude::*;
use rand::Rng;

use crate::common::constants::*;
use crate::enemy::components::SpawnAnimation;
use crate::enemy::systems::EnemyDied;

use super::components::ScrapDrop;

const SCRAP_COLOR: Color = Color::srgb(1.0, 0.85, 0.1);

pub fn on_enemy_died(trigger: On<EnemyDied>, mut commands: Commands) {
    let event = &*trigger;
    let mut rng = rand::rng();

    // Drop normal loot.
    let total_value = event.loot_value + event.stolen_scrap;
    if total_value == 0 {
        return;
    }

    let offset = Vec2::new(
        rng.random_range(-6.0..6.0),
        rng.random_range(-6.0..6.0),
    );
    let pos = event.position + offset;

    commands
        .spawn((
            ScrapDrop {
                value: total_value,
                lifetime: Timer::from_seconds(SCRAP_DROP_LIFETIME, TimerMode::Once),
            },
            Sprite::from_color(SCRAP_COLOR, Vec2::splat(8.0)),
            Transform::from_translation(pos.extend(1.5))
                .with_scale(Vec3::ZERO),
            SpawnAnimation {
                timer: Timer::from_seconds(0.2, TimerMode::Once),
            },
        ))
        .with_child((
            // Faint glow behind the scrap drop.
            Sprite::from_color(SCRAP_COLOR.with_alpha(0.15), Vec2::splat(16.0)),
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
        ));
}

/// Slowly rotate scrap drops for visual flair (diamond ↔ square oscillation).
pub fn scrap_idle_rotation(
    mut drops: Query<&mut Transform, With<ScrapDrop>>,
    time: Res<Time>,
) {
    for mut tf in &mut drops {
        tf.rotate_z(0.8 * time.delta_secs()); // ~45 deg/sec
    }
}

pub fn scrap_drop_lifetime(
    mut commands: Commands,
    mut drops: Query<(Entity, &mut ScrapDrop, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut drop, mut sprite) in &mut drops {
        drop.lifetime.tick(time.delta());

        let remaining = drop.lifetime.remaining_secs();
        if remaining < 3.0 {
            let alpha = if (remaining * 4.0) as i32 % 2 == 0 {
                0.3
            } else {
                1.0
            };
            sprite.color = SCRAP_COLOR.with_alpha(alpha);
        }

        if drop.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

