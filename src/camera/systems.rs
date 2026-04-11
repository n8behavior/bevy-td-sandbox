use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use rand::{Rng, RngExt};

use super::components::*;

/// Apply a multiplicative zoom `factor` to the current orthographic scale,
/// clamping the result to `[min, max]`.
///
/// ```
/// # use bevy_td_sandbox::camera::systems::clamp_ortho_scale;
/// assert_eq!(clamp_ortho_scale(1.0, 0.5, 0.25, 3.0), 0.5);
/// assert_eq!(clamp_ortho_scale(0.3, 0.5, 0.25, 3.0), 0.25); // clamped to min
/// ```
pub fn clamp_ortho_scale(current: f32, factor: f32, min: f32, max: f32) -> f32 {
    (current * factor).clamp(min, max)
}

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
    ortho.scale = clamp_ortho_scale(
        ortho.scale,
        factor,
        controller.min_scale,
        controller.max_scale,
    );

    // Adjust translation so the world point stays under cursor.
    if let (Some(cp), Some(before)) = (cursor_pos, world_before) {
        // Recompute GlobalTransform after scale change for accurate projection.
        let new_global = GlobalTransform::from(*transform);
        let new_cam = camera.clone();
        if let Ok(after) = new_cam.viewport_to_world_2d(&new_global, cp) {
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
        pan.cursor_absent = true;
        return;
    };

    if mouse.just_pressed(MouseButton::Middle)
        && let Ok(world_pos) = camera.viewport_to_world_2d(global_tf, cursor_pos)
    {
        pan.dragging = true;
        pan.last_world_pos = world_pos;
    }

    if !mouse.pressed(MouseButton::Middle) {
        pan.dragging = false;
    }

    if pan.dragging
        && let Ok(current_world) = camera.viewport_to_world_2d(global_tf, cursor_pos)
    {
        if pan.cursor_absent {
            // Cursor just returned — refresh position to prevent jump.
            pan.last_world_pos = current_world;
            pan.cursor_absent = false;
        } else {
            let delta = pan.last_world_pos - current_world;
            transform.translation += delta.extend(0.0);
        }
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

/// Compute a random 2D offset (z=0) for screen shake based on current intensity.
pub fn compute_shake_offset(intensity: f32, rng: &mut impl Rng) -> Vec3 {
    if intensity <= 0.0 {
        return Vec3::ZERO;
    }
    Vec3::new(
        rng.random_range(-intensity..intensity),
        rng.random_range(-intensity..intensity),
        0.0,
    )
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
    let offset = compute_shake_offset(shake.intensity, &mut rng);
    transform.translation += offset;
    shake.current_offset = offset;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    // -- clamp_ortho_scale --

    #[test]
    fn zoom_in_reduces_scale() {
        assert_eq!(clamp_ortho_scale(1.0, 0.8, 0.1, 10.0), 0.8);
    }

    #[test]
    fn zoom_out_increases_scale() {
        assert_eq!(clamp_ortho_scale(1.0, 1.25, 0.1, 10.0), 1.25);
    }

    #[test]
    fn clamps_at_min_boundary() {
        // 0.2 * 0.5 = 0.1, exactly min
        assert_eq!(clamp_ortho_scale(0.2, 0.5, 0.1, 10.0), 0.1);
    }

    #[test]
    fn clamps_below_min() {
        // 0.15 * 0.5 = 0.075, below min → clamped to 0.1
        assert_eq!(clamp_ortho_scale(0.15, 0.5, 0.1, 10.0), 0.1);
    }

    #[test]
    fn clamps_at_max_boundary() {
        // 8.0 * 1.25 = 10.0, exactly max
        assert_eq!(clamp_ortho_scale(8.0, 1.25, 0.1, 10.0), 10.0);
    }

    #[test]
    fn clamps_above_max() {
        // 10.0 * 1.25 = 12.5, above max → clamped to 10.0
        assert_eq!(clamp_ortho_scale(10.0, 1.25, 0.1, 10.0), 10.0);
    }

    #[test]
    fn identity_factor_preserves_scale() {
        assert_eq!(clamp_ortho_scale(5.0, 1.0, 0.1, 10.0), 5.0);
    }

    // -- compute_shake_offset --

    #[test]
    fn offset_within_intensity_bounds() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            let offset = compute_shake_offset(5.0, &mut rng);
            assert!(
                offset.x >= -5.0 && offset.x <= 5.0,
                "x out of range: {}",
                offset.x
            );
            assert!(
                offset.y >= -5.0 && offset.y <= 5.0,
                "y out of range: {}",
                offset.y
            );
            assert_eq!(offset.z, 0.0);
        }
    }

    #[test]
    fn zero_intensity_yields_zero_offset() {
        let mut rng = SmallRng::seed_from_u64(42);
        let offset = compute_shake_offset(0.0, &mut rng);
        assert_eq!(offset, Vec3::ZERO);
    }

    #[test]
    fn deterministic_with_seeded_rng() {
        let mut rng_a = SmallRng::seed_from_u64(123);
        let mut rng_b = SmallRng::seed_from_u64(123);
        let a = compute_shake_offset(10.0, &mut rng_a);
        let b = compute_shake_offset(10.0, &mut rng_b);
        assert_eq!(a, b);
    }
}
