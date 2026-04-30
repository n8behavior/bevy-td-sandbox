//! Systems specific to the Mortar tower.

use bevy::prelude::*;

use super::components::{Mortar, RELOAD_MULT, ReloadKind, ReloadSecs, Reloading};
use crate::tower::components::{AoEOnHit, ProjectileVisuals, Turret, TurretPhase};
use crate::tower::events::TowerWantsToFire;
use crate::tower::upgrade::{
    PanelSection, PanelSections, UpgradeApplied, UpgradeKind, UpgradeTrack,
};

/// Color used by the Mortar's reload-button text in the upgrade panel when
/// it's affordable. Mirrors the `LABEL_COLOR` used by other panel buttons
/// — duplicated here so the Mortar module remains self-contained.
const RELOAD_LABEL_COLOR: Color = Color::srgb(0.95, 0.85, 0.5);
/// Color used when the reload upgrade is unaffordable.
const RELOAD_BUSTED_COLOR: Color = Color::srgb(0.9, 0.3, 0.3);
/// Color used by the `RELOAD: MAX` indicator once the track is maxed out.
const RELOAD_MAX_COLOR: Color = Color::srgb(0.95, 0.55, 0.25);

/// Holds `Turret.phase = Idle` on every Mortar that is currently `Reloading`.
/// Runs `.before(turret_state_machine)` so the shared turret FSM only ever
/// observes the Idle phase during reload — it can advance one step (Idle →
/// Acquiring) per tick but can never reach Fire.
pub fn block_mortar_during_reload(mut towers: Query<&mut Turret, (With<Mortar>, With<Reloading>)>) {
    for mut turret in &mut towers {
        turret.phase = TurretPhase::Idle;
    }
}

/// Tick the active reload timer; remove `Reloading` once it finishes so the
/// Mortar can resume normal turret behavior on the next FixedUpdate.
pub fn tick_reload(
    mut commands: Commands,
    mut towers: Query<(Entity, &mut Reloading)>,
    time: Res<Time>,
) {
    for (entity, mut reload) in &mut towers {
        reload.timer.tick(time.delta());
        if reload.timer.is_finished() {
            commands.entity(entity).remove::<Reloading>();
        }
    }
}

/// Mortar's custom fire observer. Spawns the projectile (with Mortar's
/// AoE payload) and inserts `Reloading` so the tower cannot fire again
/// until the reload timer expires.
///
/// Mortars carry the `CustomFire` marker so `default_fire_observer` skips
/// them and this observer takes over.
pub fn mortar_fire_observer(
    trigger: On<TowerWantsToFire>,
    mut commands: Commands,
    mortars: Query<
        (
            &Transform,
            &ProjectileVisuals,
            Option<&AoEOnHit>,
            &ReloadSecs,
        ),
        With<Mortar>,
    >,
) {
    let Ok((tower_tf, visuals, aoe, reload_secs)) = mortars.get(trigger.entity) else {
        return;
    };
    crate::tower::systems::spawn_projectile(
        &mut commands,
        visuals,
        tower_tf,
        trigger.damage,
        trigger.target,
        aoe,
    );
    commands.entity(trigger.entity).insert(Reloading {
        timer: Timer::from_seconds(reload_secs.0, TimerMode::Once),
    });
}

/// Per-track scaler: reacts to `UpgradeApplied<ReloadKind>` and shrinks
/// `ReloadSecs` via the `RELOAD_MULT` ratio.
pub fn scale_reload_on_track(
    mut events: MessageReader<UpgradeApplied<ReloadKind>>,
    mut towers: Query<&mut ReloadSecs>,
) {
    for ev in events.read() {
        let Ok(mut reload) = towers.get_mut(ev.tower) else {
            continue;
        };
        let ratio = RELOAD_MULT[ev.new_tier as usize] / RELOAD_MULT[ev.old_tier as usize];
        reload.0 *= ratio;
    }
}

/// Spawn a text node and attach it as a child of `panel`. Local copy of the
/// shared `spawn_text_line_world` helper so the Mortar module remains
/// self-contained.
fn spawn_panel_text(world: &mut World, panel: Entity, text: String, color: Color, font_size: f32) {
    let child = world
        .spawn((
            Text::new(text),
            TextColor(color),
            TextFont {
                font_size,
                ..default()
            },
        ))
        .id();
    world.entity_mut(panel).add_child(child);
}

/// Panel section: render `[L] Reload: $X` (or `RELOAD: MAX`) on Mortars.
fn render_mortar_section(world: &mut World, panel: Entity, tower: Entity, pile_scrap: u32) {
    let Some(track_tier) = world
        .entity(tower)
        .get::<UpgradeTrack<ReloadKind>>()
        .map(|t| t.tier)
    else {
        return;
    };

    let (text, color) = if track_tier < ReloadKind::MAX_TIER {
        let cost = ReloadKind::cost(track_tier, 0);
        let color = if pile_scrap >= cost {
            RELOAD_LABEL_COLOR
        } else {
            RELOAD_BUSTED_COLOR
        };
        (format!("[L] Reload: ${cost}"), color)
    } else {
        ("RELOAD: MAX".to_string(), RELOAD_MAX_COLOR)
    };

    spawn_panel_text(world, panel, text, color, 13.0);
}

/// Register the Mortar's panel section between the magnet (200) and
/// repair (300) sections.
pub fn register_mortar_section(mut sections: ResMut<PanelSections>) {
    sections.add(PanelSection {
        order: 250,
        render: render_mortar_section,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// `tick_reload` removes the `Reloading` component once the timer
    /// finishes, restoring the Mortar's ability to advance through the
    /// turret state machine.
    #[test]
    fn reload_timer_removes_component_when_finished() {
        // Skip `TimePlugin` and drive `Time` manually so we can set the
        // delta deterministically — `TimePlugin`'s `time_system` would
        // overwrite an `advance_by` call with the real wall-clock delta.
        let mut app = App::new();
        app.init_resource::<Time>();
        app.add_systems(Update, tick_reload);

        let entity = app
            .world_mut()
            .spawn(Reloading {
                timer: Timer::from_seconds(0.1, TimerMode::Once),
            })
            .id();

        // Run one tick with delta = 0 — Reloading should still be present.
        app.update();
        assert!(
            app.world().get::<Reloading>(entity).is_some(),
            "Reloading should still be present before the timer elapses"
        );

        // Advance the clock past the timer duration and tick once more.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.2));
        app.update();
        assert!(
            app.world().get::<Reloading>(entity).is_none(),
            "Reloading should be removed after the timer finishes"
        );
    }

    /// `scale_reload_on_track` applies the per-tier ratio to `ReloadSecs`.
    #[test]
    fn reload_scaling_applies_tier_ratio() {
        let mut app = App::new();
        app.add_message::<UpgradeApplied<ReloadKind>>();
        app.add_systems(Update, scale_reload_on_track);

        let entity = app.world_mut().spawn(ReloadSecs(4.0)).id();
        app.world_mut()
            .write_message(UpgradeApplied::<ReloadKind>::new(entity, 0, 1));
        app.update();

        let r = app.world().get::<ReloadSecs>(entity).unwrap();
        // 4.0 * (0.75 / 1.0) = 3.0
        assert!(
            (r.0 - 3.0).abs() < 1e-4,
            "expected reload scaled to 3.0, got {}",
            r.0
        );
    }
}
