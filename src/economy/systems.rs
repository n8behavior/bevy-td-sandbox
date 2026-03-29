use bevy::prelude::*;

use crate::common::constants::*;
use crate::enemy::systems::EnemyDied;

use super::components::ScrapDrop;
use super::resources::PlayerScrap;

pub fn on_enemy_died(trigger: On<EnemyDied>, mut commands: Commands) {
    let event = &*trigger;
    commands.spawn((
        ScrapDrop {
            value: event.loot_value,
            lifetime: Timer::from_seconds(SCRAP_DROP_LIFETIME, TimerMode::Once),
        },
        Sprite::from_color(Color::srgb(1.0, 0.85, 0.1), Vec2::splat(8.0)),
        Transform::from_translation(event.position.extend(1.5)),
    ));
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
            sprite.color = Color::srgba(1.0, 0.85, 0.1, alpha);
        }

        if drop.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Right-click to vacuum collect ALL nearby scrap
pub fn click_to_collect_scrap(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    drops: Query<(Entity, &ScrapDrop, &Transform)>,
    mut scrap: ResMut<PlayerScrap>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = cameras.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };

    let collect_radius = TILE_SIZE * 2.5;

    // Collect ALL nearby scrap, not just one
    for (entity, drop, tf) in &drops {
        let dist = world_pos.distance(tf.translation.truncate());
        if dist <= collect_radius {
            scrap.0 += drop.value;
            commands.entity(entity).despawn();
        }
    }
}
