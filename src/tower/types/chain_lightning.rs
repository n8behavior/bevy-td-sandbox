use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::tower::components::*;

#[derive(Component)]
pub struct ChainLightningMarker;

pub struct ChainLightningPlugin;

impl Plugin for ChainLightningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Chain Lightning",
        cost: 125,
        color: Color::srgb(0.3, 0.6, 1.0),
        ui_color: Color::srgb(0.4, 0.7, 1.0),
        key: KeyCode::Digit6,
        special_label: "Chain",
        spawn_fn: |cmds| {
            let damage = 20.0;
            let range = 90.0;
            let cooldown = 2.0;
            let arc_range = 60.0;
            let color = Color::srgb(0.3, 0.6, 1.0);
            let stats = TowerStats { damage, range };
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range: stats.range,
                    color: Color::srgba(0.3, 0.5, 1.0, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                ChainLightningMarker,
                BlocksNav,
                TargetingMode::default(),
                stats,
                ChainLightning {
                    arc_range,
                    damage_falloff: 0.7,
                },
                BaseArcRange(arc_range),
                ChainCooldown::new(cooldown),
                TowerTier(0),
                TowerName("Chain Lightning"),
                BaseStats {
                    cost: 125,
                    damage,
                    range,
                    cooldown_secs: cooldown,
                    aoe_radius: 0.0,
                    aoe_damage: 0.0,
                    slow_factor: 1.0,
                    color,
                },
            ));
            let max_hp = 125.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
            cmds.insert((
                MagnetTier(0),
                BaseMagnetRange(collect_range),
                CollectionAuraRingConfig {
                    range: collect_range,
                    color: MAGNET_AURA_COLOR,
                },
            ));
        },
    });
}
