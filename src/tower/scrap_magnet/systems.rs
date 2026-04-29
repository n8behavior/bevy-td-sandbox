//! Systems specific to the Scrap Magnet tower.

use bevy::prelude::*;

use crate::common::constants::ENEMY_PULL_SPEED;
use crate::enemy::components::Enemy;

use super::ScrapMagnet;
use crate::tower::components::{ScrapCollector, Tower, TowerState};
use crate::tower::systems::magnetic_pull;

/// Pull enemies toward dedicated Magnet towers, making them struggle against the field.
/// Only the Magnet tower type (ScrapMagnet marker) pulls enemies, not all collectors.
pub fn magnetic_pull_enemies(
    magnets: Query<(&Transform, &ScrapCollector, &TowerState), (With<ScrapMagnet>, With<Tower>)>,
    mut enemies: Query<&mut Transform, (With<Enemy>, Without<Tower>)>,
    time: Res<Time>,
) {
    for mut enemy_tf in &mut enemies {
        let enemy_pos = enemy_tf.translation.truncate();
        for (mag_tf, collector, tower_state) in &magnets {
            if !tower_state.is_operational() {
                continue;
            }
            let mag_pos = mag_tf.translation.truncate();
            let dist = mag_pos.distance(enemy_pos);
            if dist <= collector.range && dist > 2.0 {
                let pull = magnetic_pull(
                    enemy_pos,
                    mag_pos,
                    collector.range,
                    ENEMY_PULL_SPEED,
                    time.delta_secs(),
                );
                enemy_tf.translation += pull.extend(0.0);
            }
        }
    }
}
