use bevy::prelude::*;
use rand::Rng;

use crate::audio::resources::SoundAssets;
use crate::audio::systems::play_sound;
use crate::common::constants::*;
use crate::enemy::components::SpawnAnimation;
use crate::enemy::systems::EnemyDied;
use crate::particles::systems::spawn_scrap_sparkle;

use super::components::ScrapDrop;

/// Total scrap value of a drop (base loot + recovered stolen scrap).
pub fn compute_scrap_value(loot_value: u32, stolen_scrap: u32) -> u32 {
    loot_value + stolen_scrap
}

/// Random scatter offset for scrap drop placement (±6.0 world units per axis).
pub fn compute_drop_offset(rng: &mut impl Rng) -> Vec2 {
    Vec2::new(rng.random_range(-6.0..6.0), rng.random_range(-6.0..6.0))
}

/// Blink alpha for a scrap drop nearing expiry.
///
/// Toggles between 0.3 (dim) and 1.0 (bright) at ~4 Hz during the last
/// 3 seconds. Formula: `(remaining × 4.0)` truncated to `i32`, mod 2
/// selects dim/bright.
///
/// Returns `None` when >= 3.0 s remaining (no blink).
pub fn compute_scrap_alpha(remaining_secs: f32) -> Option<f32> {
    if remaining_secs >= 3.0 {
        return None;
    }
    let alpha = if (remaining_secs * 4.0) as i32 % 2 == 0 {
        0.3
    } else {
        1.0
    };
    Some(alpha)
}

/// Spawn a [`ScrapDrop`] entity with glow child and sparkle particles.
pub fn on_enemy_died_spawn_drop(trigger: On<EnemyDied>, mut commands: Commands) {
    let event = &*trigger;

    let total_value = compute_scrap_value(event.loot_value, event.stolen_scrap);
    if total_value == 0 {
        return;
    }

    let mut rng = rand::rng();
    let offset = compute_drop_offset(&mut rng);
    let pos = event.position + offset;

    commands
        .spawn((
            ScrapDrop {
                value: total_value,
                lifetime: Timer::from_seconds(SCRAP_DROP_LIFETIME, TimerMode::Once),
            },
            Sprite::from_color(SCRAP_COLOR, Vec2::splat(8.0)),
            Transform::from_translation(pos.extend(1.5)).with_scale(Vec3::ZERO),
            SpawnAnimation {
                timer: Timer::from_seconds(0.2, TimerMode::Once),
            },
        ))
        .with_child((
            // Faint glow behind the scrap drop.
            Sprite::from_color(SCRAP_COLOR.with_alpha(0.15), Vec2::splat(16.0)),
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
        ));

    spawn_scrap_sparkle(&mut commands, pos);
}

/// Play the scrap-drop sound when loot is awarded.
pub fn on_enemy_died_scrap_sound(
    trigger: On<EnemyDied>,
    mut commands: Commands,
    sounds: Res<SoundAssets>,
) {
    let total_value = compute_scrap_value(trigger.loot_value, trigger.stolen_scrap);
    if total_value == 0 {
        return;
    }
    play_sound(&mut commands, &sounds.scrap_drop, 0.25);
}

/// Slowly rotate scrap drops for visual flair (diamond ↔ square oscillation).
pub fn scrap_idle_rotation(mut drops: Query<&mut Transform, With<ScrapDrop>>, time: Res<Time>) {
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
        if let Some(alpha) = compute_scrap_alpha(remaining) {
            sprite.color = SCRAP_COLOR.with_alpha(alpha);
        }

        if drop.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    // -- compute_scrap_value --

    #[test]
    fn scrap_value_adds_loot_and_stolen() {
        assert_eq!(compute_scrap_value(10, 5), 15);
    }

    #[test]
    fn scrap_value_zero_when_both_zero() {
        assert_eq!(compute_scrap_value(0, 0), 0);
    }

    #[test]
    fn scrap_value_loot_only() {
        assert_eq!(compute_scrap_value(10, 0), 10);
    }

    #[test]
    fn scrap_value_stolen_only() {
        assert_eq!(compute_scrap_value(0, 7), 7);
    }

    // -- compute_drop_offset --

    #[test]
    fn drop_offset_within_bounds() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            let offset = compute_drop_offset(&mut rng);
            assert!(
                (-6.0..=6.0).contains(&offset.x),
                "x out of range: {}",
                offset.x
            );
            assert!(
                (-6.0..=6.0).contains(&offset.y),
                "y out of range: {}",
                offset.y
            );
        }
    }

    #[test]
    fn drop_offset_deterministic() {
        let mut rng1 = SmallRng::seed_from_u64(123);
        let mut rng2 = SmallRng::seed_from_u64(123);
        assert_eq!(compute_drop_offset(&mut rng1), compute_drop_offset(&mut rng2));
    }

    // -- compute_scrap_alpha --

    #[test]
    fn scrap_alpha_none_above_threshold() {
        assert!(compute_scrap_alpha(3.0).is_none());
        assert!(compute_scrap_alpha(5.0).is_none());
        assert!(compute_scrap_alpha(10.0).is_none());
    }

    #[test]
    fn scrap_alpha_some_below_threshold() {
        assert!(compute_scrap_alpha(2.9).is_some());
        assert!(compute_scrap_alpha(1.0).is_some());
        assert!(compute_scrap_alpha(0.0).is_some());
    }

    #[test]
    fn scrap_alpha_alternates() {
        // 0.0: (0.0 * 4.0) as i32 = 0, 0 % 2 == 0 → dim
        assert_eq!(compute_scrap_alpha(0.0), Some(0.3));
        // 0.25: (1.0) as i32 = 1, 1 % 2 != 0 → bright
        assert_eq!(compute_scrap_alpha(0.25), Some(1.0));
        // 0.5: (2.0) as i32 = 2, 2 % 2 == 0 → dim
        assert_eq!(compute_scrap_alpha(0.5), Some(0.3));
    }

    #[test]
    fn scrap_alpha_only_valid_values() {
        for i in 0..30 {
            let remaining = i as f32 * 0.1;
            if let Some(alpha) = compute_scrap_alpha(remaining) {
                assert!(
                    alpha == 0.3 || alpha == 1.0,
                    "unexpected alpha {alpha} at remaining={remaining}"
                );
            }
        }
    }

    #[test]
    fn scrap_alpha_toggle_count() {
        // ~4 Hz over 3 seconds: integer part of (remaining × 4.0) ranges
        // from 11 down to 0, giving 11 transitions (toggles).
        let mut prev = compute_scrap_alpha(2.999).unwrap();
        let mut toggles = 0u32;
        // Sample at 1ms resolution across the 3-second window.
        for i in 1..3000 {
            let t = 2.999 - i as f32 * 0.001;
            let alpha = compute_scrap_alpha(t).unwrap();
            if alpha != prev {
                toggles += 1;
                prev = alpha;
            }
        }
        assert_eq!(toggles, 11, "expected 11 toggles, got {toggles}");
    }
}
