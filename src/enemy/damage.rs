//! Enemy damage: [`damage_enemy`], the only code that lowers enemy
//! `Health`, and [`EnemyDamageSystems`], the set every damage system joins.

use bevy::prelude::*;

use crate::records::{DamageDealt, DamageSource};

use super::components::Health;

/// Every system that damages enemies, directly or through the observers of
/// a trigger it fires (projectile hits, Chain Lightning, radioactive
/// terrain). `check_enemy_death` runs after this set.
///
/// Bevy inserts a sync point on that ordering edge, which applies the
/// set's commands and so runs the observers of its triggers. A lethal hit
/// is therefore detected in the same fixed tick.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnemyDamageSystems;

/// Apply `raw` damage from `source` to an enemy, write one [`DamageDealt`],
/// and return a copy of it.
///
/// `armor` is a flat reduction per hit, but at least 1 point always gets
/// through. Sources that ignore armor pass `None`. `kind` is the enemy's
/// `EnemyName`, copied into the record.
pub fn damage_enemy(
    target: Entity,
    kind: Option<&'static str>,
    health: &mut Health,
    armor: Option<f32>,
    raw: f32,
    source: DamageSource,
    records: &mut MessageWriter<DamageDealt>,
) -> DamageDealt {
    let effective = match armor {
        Some(reduction) => (raw - reduction).max(1.0),
        None => raw,
    };
    let dealt = effective.min(health.current).max(0.0);
    health.current -= effective;

    let record = DamageDealt {
        source,
        target,
        target_kind: kind,
        raw,
        dealt,
        overkill: effective - dealt,
    };
    records.write(record);
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::components::Terrain;
    use bevy::ecs::system::RunSystemOnce;

    const SOURCE: DamageSource = DamageSource::Terrain(Terrain::Radioactive);

    /// What one `damage_enemy` call did to an enemy.
    struct Outcome {
        returned: DamageDealt,
        written: Vec<DamageDealt>,
        health_after: f32,
    }

    /// Call `damage_enemy` once on an enemy at `current` HP.
    fn hit(current: f32, armor: Option<f32>, raw: f32) -> Outcome {
        let mut world = World::new();
        world.init_resource::<Messages<DamageDealt>>();
        let enemy = world
            .spawn(Health {
                current,
                max: 100.0,
            })
            .id();

        let returned = world
            .run_system_once(
                move |mut healths: Query<&mut Health>, mut records: MessageWriter<DamageDealt>| {
                    let mut health = healths.get_mut(enemy).unwrap();
                    damage_enemy(
                        enemy,
                        Some("Brute"),
                        &mut health,
                        armor,
                        raw,
                        SOURCE,
                        &mut records,
                    )
                },
            )
            .unwrap();

        let messages = world.resource::<Messages<DamageDealt>>();
        let written = messages.get_cursor().read(messages).copied().collect();
        Outcome {
            returned,
            written,
            health_after: world.get::<Health>(enemy).unwrap().current,
        }
    }

    #[test]
    fn writes_the_record_it_returns() {
        let out = hit(100.0, None, 25.0);
        assert_eq!(out.written.len(), 1);
        let written = out.written[0];
        assert_eq!(written.target, out.returned.target);
        assert_eq!(written.target_kind, Some("Brute"));
        assert_eq!(written.source, SOURCE);
        assert_eq!(written.raw, out.returned.raw);
        assert_eq!(written.dealt, out.returned.dealt);
        assert_eq!(written.overkill, out.returned.overkill);
    }

    #[test]
    fn no_armor_full_damage() {
        let out = hit(100.0, None, 25.0);
        assert_eq!(out.health_after, 75.0);
        assert_eq!(out.returned.raw, 25.0);
        assert_eq!(out.returned.dealt, 25.0);
        assert_eq!(out.returned.overkill, 0.0);
    }

    #[test]
    fn armor_reduces_damage() {
        let out = hit(100.0, Some(10.0), 25.0);
        assert_eq!(out.health_after, 85.0); // 25 - 10 = 15 damage
        assert_eq!(out.returned.raw, 25.0);
        assert_eq!(out.returned.dealt, 15.0);
    }

    #[test]
    fn armor_cannot_negate_all() {
        let out = hit(100.0, Some(50.0), 5.0);
        assert_eq!(out.health_after, 99.0); // min 1 damage
        assert_eq!(out.returned.dealt, 1.0);
    }

    #[test]
    fn zero_damage_with_armor() {
        let out = hit(100.0, Some(10.0), 0.0);
        assert_eq!(out.health_after, 99.0); // (0 - 10).max(1) = 1
        assert_eq!(out.returned.dealt, 1.0);
    }

    #[test]
    fn dealt_plus_overkill_is_damage_after_armor() {
        // (health, armor, raw, damage after armor)
        let cases = [
            (100.0, None, 25.0, 25.0),
            (10.0, None, 25.0, 25.0),
            (100.0, Some(10.0), 25.0, 15.0),
            (12.0, Some(10.0), 25.0, 15.0),
            (0.5, Some(50.0), 5.0, 1.0),
            (0.0, None, 8.0, 8.0),
            (-3.0, Some(5.0), 20.0, 15.0),
        ];
        for (current, armor, raw, after_armor) in cases {
            let out = hit(current, armor, raw);
            let sum = out.returned.dealt + out.returned.overkill;
            assert!(
                (sum - after_armor).abs() < 1e-4,
                "health {current}, armor {armor:?}, raw {raw}: \
                 dealt + overkill = {sum}, want {after_armor}"
            );
            assert!(
                (out.health_after - (current - after_armor)).abs() < 1e-4,
                "health should drop by the damage after armor"
            );
        }
    }

    #[test]
    fn dealt_never_exceeds_health_left() {
        let out = hit(10.0, None, 25.0);
        assert_eq!(out.returned.dealt, 10.0);
        assert_eq!(out.returned.overkill, 15.0);
        assert_eq!(out.health_after, -15.0);

        let out = hit(12.0, Some(10.0), 25.0);
        assert_eq!(out.returned.dealt, 12.0);
        assert_eq!(out.returned.overkill, 3.0);
    }

    #[test]
    fn hit_at_zero_hp_deals_nothing() {
        let out = hit(0.0, None, 8.0);
        assert_eq!(out.returned.dealt, 0.0);
        assert_eq!(out.returned.overkill, 8.0);

        // Below zero (lethal hit already landed, not yet marked dying).
        let out = hit(-3.0, Some(5.0), 20.0);
        assert_eq!(out.returned.dealt, 0.0);
        assert_eq!(out.returned.overkill, 15.0);
    }
}
