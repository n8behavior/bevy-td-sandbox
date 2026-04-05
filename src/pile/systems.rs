use bevy::prelude::*;
use std::collections::HashSet;
use std::f32::consts::PI;

use crate::common::constants::*;
use crate::grid::components::GridCell;
use crate::terrain::components::Terrain;
use crate::tower::components::Tower;

use super::components::PileCell;
use super::resources::{PileScrap, PileState};

/// Compute pile radius in tiles from scrap amount.
pub fn pile_radius(scrap: u32) -> f32 {
    if scrap == 0 {
        return 0.0;
    }
    (scrap as f32 / (PI * SCRAP_PER_TILE)).sqrt()
}

/// Compute which grid cells fall within the pile's circular footprint.
pub fn compute_pile_cells(center: UVec3, radius: f32, width: u32, height: u32) -> HashSet<UVec3> {
    let mut cells = HashSet::new();
    let cx = center.x as f32;
    let cy = center.y as f32;
    let r2 = radius * radius;

    let min_x = (cx - radius).floor().max(0.0) as u32;
    let max_x = ((cx + radius).ceil() as u32).min(width.saturating_sub(1));
    let min_y = (cy - radius).floor().max(0.0) as u32;
    let max_y = ((cy + radius).ceil() as u32).min(height.saturating_sub(1));

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            if dx * dx + dy * dy <= r2 {
                cells.insert(UVec3::new(x, y, 0));
            }
        }
    }
    cells
}

/// Recompute pile state when scrap amount changes enough to affect the radius.
pub fn update_pile_state(
    pile_scrap: Res<PileScrap>,
    mut pile_state: ResMut<PileState>,
    config: Res<GridConfig>,
) {
    if !pile_scrap.is_changed() {
        return;
    }

    let new_radius = pile_radius(pile_scrap.amount);
    let new_radius_int = new_radius as u32;

    if new_radius_int == pile_state.last_radius_int && !pile_state.cells.is_empty() {
        // Radius hasn't changed by a full tile — skip recomputation.
        pile_state.radius_tiles = new_radius;
        return;
    }

    pile_state.radius_tiles = new_radius;
    pile_state.last_radius_int = new_radius_int;
    pile_state.cells =
        compute_pile_cells(pile_state.center, new_radius, config.width, config.height);
}

/// Sync the visual appearance of grid cells to match current pile state.
/// Adds/removes PileCell marker and swaps sprite colors.
pub fn update_pile_visuals(
    mut commands: Commands,
    pile_state: Res<PileState>,
    mut cells: Query<
        (Entity, &GridCell, &mut Sprite, Option<&PileCell>),
        (Without<Tower>, Without<Terrain>),
    >,
) {
    if !pile_state.is_changed() {
        return;
    }

    for (entity, grid_cell, mut sprite, has_pile) in &mut cells {
        let coord = UVec3::new(grid_cell.coord.x as u32, grid_cell.coord.y as u32, 0);
        let should_be_pile = pile_state.cells.contains(&coord);

        if should_be_pile && has_pile.is_none() {
            commands.entity(entity).insert(PileCell);
            sprite.color = PILE_COLOR;
        } else if !should_be_pile && has_pile.is_some() {
            commands.entity(entity).remove::<PileCell>();
            sprite.color = PAPER_COLOR;
        }
    }
}

/// Find the nearest pile cell to a given position.
pub fn nearest_pile_cell(pos: UVec3, pile_state: &PileState) -> UVec3 {
    pile_state
        .cells
        .iter()
        .min_by_key(|c| {
            let dx = c.x as i32 - pos.x as i32;
            let dy = c.y as i32 - pos.y as i32;
            (dx * dx + dy * dy) as u32
        })
        .copied()
        .unwrap_or(pile_state.center)
}

/// Find the nearest edge cell to a given position.
pub fn nearest_edge_cell(pos: UVec3, edge_cells: &[UVec3]) -> UVec3 {
    edge_cells
        .iter()
        .min_by_key(|c| {
            let dx = c.x as i32 - pos.x as i32;
            let dy = c.y as i32 - pos.y as i32;
            (dx * dx + dy * dy) as u32
        })
        .copied()
        .unwrap_or(pos)
}
