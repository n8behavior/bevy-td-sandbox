//! Components specific to the Mortar tower.
//!
//! Mortar demonstrates the issue #81 acceptance criterion: a tower with a
//! unique behavioral phase (`Reloading`), a unique stat (`ReloadSecs`), and
//! a unique upgrade dimension (`ReloadKind`) — all defined inside this
//! module without touching shared tower code.

use bevy::prelude::*;

use crate::tower::upgrade::UpgradeKind;

/// Marker for Mortar towers.
#[derive(Component)]
pub struct Mortar;

/// Active reload state. Inserted by `mortar_fire_observer` immediately after
/// a Mortar fires; removed by `tick_reload` when the timer expires. While
/// present, `block_mortar_during_reload` forces `Turret.phase = Idle` each
/// FixedUpdate so the shared turret state machine never advances to `Fire`.
#[derive(Component)]
pub struct Reloading {
    pub timer: Timer,
}

/// Live reload duration in seconds. Mutated by `scale_reload_on_track` when
/// `UpgradeApplied<ReloadKind>` fires (ratio against the previous tier's
/// multiplier — no separate snapshot needed).
#[derive(Component, Clone, Copy)]
pub struct ReloadSecs(pub f32);

/// The reload upgrade dimension: spend scrap (L key) to reduce the Mortar's
/// reload time. A self-contained `UpgradeKind` impl that demonstrates how a
/// per-tower module declares its own track without touching the shared
/// upgrade machinery.
#[derive(Debug)]
pub struct ReloadKind;

impl UpgradeKind for ReloadKind {
    const KEY: KeyCode = KeyCode::KeyL;
    const MAX_TIER: u8 = 2;
    const LABEL: &'static str = "Reload";
    const FLOAT_COLOR: Color = Color::srgb(0.95, 0.55, 0.25);

    fn cost(current_tier: u8, _base_cost: u32) -> u32 {
        const COSTS: [u32; 2] = [60, 120];
        COSTS[current_tier as usize]
    }
}

/// Per-tier multiplier for `ReloadSecs`. Each upgrade shrinks the reload
/// duration toward 50% of its base value.
pub(super) const RELOAD_MULT: [f32; 3] = [1.0, 0.75, 0.5];
