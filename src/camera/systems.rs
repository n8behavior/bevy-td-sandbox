use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use rand::Rng;

use super::components::*;

pub fn camera_zoom(
    mut scroll: MessageReader<MouseWheel>,
    mut camera_q: Query<
        (
            &Camera,
            &GlobalTransform,
            &mut Projection,
            &mut Transform,
            &CameraController,
        ),
        With<Camera2d>,
    >,
    windows: Query<&Window>,
) {
    let total: f32 = scroll
        .read()
        .map(|ev| match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 100.0,
        })
        .sum();

    if total == 0.0 {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Ok((camera, global_tf, mut projection, mut transform, controller)) = camera_q.single_mut()
    else {
        return;
    };
    let Projection::Orthographic(ref mut ortho) = *projection else {
        return;
    };

    // World point under cursor before zoom.
    let cursor_pos = window.cursor_position();
    let world_before = cursor_pos.and_then(|cp| camera.viewport_to_world_2d(global_tf, cp).ok());

    // Apply zoom.
    let factor = if total > 0.0 {
        1.0 / controller.zoom_step
    } else {
        controller.zoom_step
    };
    ortho.scale = (ortho.scale * factor).clamp(controller.min_scale, controller.max_scale);

    // Adjust translation so the world point stays under cursor.
    if let (Some(cp), Some(before)) = (cursor_pos, world_before) {
        // Recompute GlobalTransform after scale change for accurate projection.
        let new_global = GlobalTransform::from(*transform);
        let new_ortho = ortho.clone();
        let new_cam = camera.clone();
        if let Ok(after) = new_cam.viewport_to_world_2d(&new_global, cp) {
            let _ = new_ortho; // used above via clone
            let delta = before - after;
            transform.translation += delta.extend(0.0);
        }
    }
}

pub fn camera_pan(
    mouse: Res<ButtonInput<MouseButton>>,
    mut pan: ResMut<PanState>,
    mut camera_q: Query<(&Camera, &GlobalTransform, &mut Transform), With<Camera2d>>,
    windows: Query<&Window>,
) {
    let Ok(window) = windows.single() else { return };
    let Ok((camera, global_tf, mut transform)) = camera_q.single_mut() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        pan.dragging = false;
        return;
    };

    if mouse.just_pressed(MouseButton::Middle)
        && let Ok(world_pos) = camera.viewport_to_world_2d(global_tf, cursor_pos)
    {
        pan.dragging = true;
        pan.last_world_pos = world_pos;
    }

    if mouse.just_released(MouseButton::Middle) {
        pan.dragging = false;
    }

    if pan.dragging
        && let Ok(current_world) = camera.viewport_to_world_2d(global_tf, cursor_pos)
    {
        let delta = pan.last_world_pos - current_world;
        transform.translation += delta.extend(0.0);
    }
}

pub fn camera_reset(
    keys: Res<ButtonInput<KeyCode>>,
    mut camera_q: Query<(&mut Projection, &mut Transform, &CameraController), With<Camera2d>>,
) {
    if !keys.just_pressed(KeyCode::Home) {
        return;
    }
    let Ok((mut projection, mut transform, controller)) = camera_q.single_mut() else {
        return;
    };
    transform.translation = controller.home_translation;
    if let Projection::Orthographic(ref mut ortho) = *projection {
        ortho.scale = controller.home_scale;
    }
}

/// Apply random offset to camera while shake is active.
pub fn apply_screen_shake(
    mut shake: ResMut<ScreenShake>,
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
    time: Res<Time>,
) {
    let Ok(mut transform) = camera_q.single_mut() else {
        return;
    };

    // Undo last frame's offset to restore logical position.
    transform.translation -= shake.current_offset;
    shake.current_offset = Vec3::ZERO;

    if shake.intensity <= 0.0 {
        return;
    }

    shake.timer.tick(time.delta());
    shake.intensity *= shake.decay.powf(time.delta_secs());

    if shake.timer.is_finished() {
        shake.intensity = 0.0;
        return;
    }

    let mut rng = rand::rng();
    let offset = Vec3::new(
        rng.random_range(-shake.intensity..shake.intensity),
        rng.random_range(-shake.intensity..shake.intensity),
        0.0,
    );
    transform.translation += offset;
    shake.current_offset = offset;
}
