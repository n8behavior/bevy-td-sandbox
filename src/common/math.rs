//! Shared 2D math utilities.
//!
//! Rotation helpers follow the official Bevy 2D rotation pattern
//! (dot-product + [`Transform::rotate_z`]) from `examples/2d/rotation.rs`.
//! All functions assume the **X-axis** is the sprite's forward direction.

use bevy::prelude::*;

/// Rotate `transform` toward `target_dir` at up to `speed` radians per
/// second.
///
/// Computes the signed angle between the transform's local X-axis and
/// `target_dir`, then applies the smaller of `speed * dt` and the
/// remaining angle so the rotation never overshoots. If the transform is
/// already aligned (dot product ≈ 1.0), no rotation is applied.
///
/// `target_dir` **must** be normalized; behaviour is undefined otherwise.
pub fn rotate_toward(transform: &mut Transform, target_dir: Vec2, speed: f32, dt: f32) {
    let forward = (transform.rotation * Vec3::X).xy();
    let forward_dot = forward.dot(target_dir);

    // Already facing the target — skip to avoid acos(1.0+ε) → NaN.
    if (forward_dot - 1.0).abs() < f32::EPSILON {
        return;
    }

    let cross = forward.perp_dot(target_dir);
    let rotation_sign = f32::copysign(1.0, cross);
    let max_angle = forward_dot.clamp(-1.0, 1.0).acos();
    let rotation_angle = rotation_sign * (speed * dt).min(max_angle);
    transform.rotate_z(rotation_angle);
}

/// Angle in radians between a transform's forward (X-axis) and
/// `target_dir`.
///
/// Returns a value in `[0, π]`. Useful for aim-tolerance checks
/// (e.g. "is the tower pointed close enough to fire?").
///
/// `target_dir` **must** be normalized; behaviour is undefined otherwise.
pub fn angle_to_dir(transform: &Transform, target_dir: Vec2) -> f32 {
    let forward = (transform.rotation * Vec3::X).xy();
    forward.dot(target_dir).clamp(-1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

    /// Helper: transform facing along angle `radians` from X-axis.
    fn facing(radians: f32) -> Transform {
        Transform::from_rotation(Quat::from_rotation_z(radians))
    }

    // -----------------------------------------------------------------------
    // rotate_toward
    // -----------------------------------------------------------------------

    #[test]
    fn already_facing_target_no_rotation() {
        let mut tf = facing(0.0);
        let original = tf.rotation;
        rotate_toward(&mut tf, Vec2::X, 10.0, 1.0);
        assert!(
            (tf.rotation.dot(original) - 1.0).abs() < 1e-5,
            "should not rotate when already aligned"
        );
    }

    #[test]
    fn rotates_ccw_toward_target_above() {
        // Facing right (+X), target is up (+Y) → should rotate CCW (positive Z).
        let mut tf = facing(0.0);
        rotate_toward(&mut tf, Vec2::Y, 100.0, 1.0);
        let angle = tf.rotation.to_euler(EulerRot::ZYX).0;
        assert!(
            (angle - FRAC_PI_2).abs() < 0.01,
            "expected ~π/2, got {angle}"
        );
    }

    #[test]
    fn rotates_cw_toward_target_below() {
        // Facing right (+X), target is down (-Y) → should rotate CW (negative Z).
        let mut tf = facing(0.0);
        rotate_toward(&mut tf, Vec2::NEG_Y, 100.0, 1.0);
        let angle = tf.rotation.to_euler(EulerRot::ZYX).0;
        assert!(
            (angle + FRAC_PI_2).abs() < 0.01,
            "expected ~-π/2, got {angle}"
        );
    }

    #[test]
    fn clamped_by_speed() {
        // Target is 90° away, but speed × dt only allows 45°.
        let mut tf = facing(0.0);
        rotate_toward(&mut tf, Vec2::Y, FRAC_PI_4, 1.0);
        let angle = tf.rotation.to_euler(EulerRot::ZYX).0;
        assert!(
            (angle - FRAC_PI_4).abs() < 0.01,
            "expected ~π/4, got {angle}"
        );
    }

    #[test]
    fn no_overshoot_when_remaining_less_than_step() {
        // Target is 10° away, speed allows 90°/s.
        let small_angle = 10f32.to_radians();
        let mut tf = facing(0.0);
        let target = Vec2::new(small_angle.cos(), small_angle.sin());
        rotate_toward(&mut tf, target, FRAC_PI_2, 1.0);
        let angle = tf.rotation.to_euler(EulerRot::ZYX).0;
        assert!(
            (angle - small_angle).abs() < 0.01,
            "should snap to target, got {angle}"
        );
    }

    #[test]
    fn zero_speed_no_rotation() {
        let mut tf = facing(0.0);
        let original = tf.rotation;
        rotate_toward(&mut tf, Vec2::Y, 0.0, 1.0);
        assert!(
            (tf.rotation.dot(original) - 1.0).abs() < 1e-5,
            "zero speed should not rotate"
        );
    }

    #[test]
    fn large_dt_clamps_to_max_angle() {
        // Target is 90° away. Speed × huge dt would overshoot, but clamping
        // limits rotation to the actual 90°.
        let mut tf = facing(0.0);
        rotate_toward(&mut tf, Vec2::Y, 1.0, 1000.0);
        let angle = tf.rotation.to_euler(EulerRot::ZYX).0;
        assert!(
            (angle - FRAC_PI_2).abs() < 0.01,
            "should clamp to π/2, got {angle}"
        );
    }

    #[test]
    fn target_directly_behind_rotates_consistently() {
        let mut tf = facing(0.0);
        rotate_toward(&mut tf, Vec2::NEG_X, 1.0, 0.1);
        let angle = tf.rotation.to_euler(EulerRot::ZYX).0;
        // Should rotate some non-zero amount (direction depends on perp_dot
        // sign for exactly opposite — Vec2::NEG_X has y=0, so perp_dot=0
        // and copysign returns +1.0, giving CCW rotation).
        assert!(angle.abs() > 0.0, "should rotate away from dead-behind");
    }

    // -----------------------------------------------------------------------
    // angle_to_dir
    // -----------------------------------------------------------------------

    #[test]
    fn angle_zero_when_aligned() {
        let tf = facing(0.0);
        assert!((angle_to_dir(&tf, Vec2::X)).abs() < 1e-5);
    }

    #[test]
    fn angle_half_pi_when_perpendicular() {
        let tf = facing(0.0);
        assert!((angle_to_dir(&tf, Vec2::Y) - FRAC_PI_2).abs() < 0.01);
    }

    #[test]
    fn angle_pi_when_opposite() {
        let tf = facing(0.0);
        assert!((angle_to_dir(&tf, Vec2::NEG_X) - PI).abs() < 0.01);
    }

    #[test]
    fn angle_rotated_transform() {
        // Facing up (+Y direction, i.e. rotated 90° CCW).
        let tf = facing(FRAC_PI_2);
        // Target is also up → 0 angle.
        assert!((angle_to_dir(&tf, Vec2::Y)).abs() < 0.01);
        // Target is right (+X) → 90° away.
        assert!((angle_to_dir(&tf, Vec2::X) - FRAC_PI_2).abs() < 0.01);
    }
}
