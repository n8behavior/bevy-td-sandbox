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
        if grid_cell.coord.x < 0 || grid_cell.coord.y < 0 {
            debug_assert!(false, "negative grid coord: {:?}", grid_cell.coord);
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // -----------------------------------------------------------------------
    // pile_radius
    // -----------------------------------------------------------------------

    #[test]
    fn pile_radius_zero_scrap_returns_zero() {
        assert_eq!(pile_radius(0), 0.0);
    }

    #[test]
    fn pile_radius_known_value() {
        let r = pile_radius(200);
        // r = sqrt(200 / (pi * 2000)) ≈ 0.1784
        assert!((r - 0.1784).abs() < 0.001, "got {r}");
    }

    #[test]
    fn pile_radius_monotonically_increasing() {
        let values: Vec<u32> = vec![0, 10, 100, 500, 1000, 5000, 50000];
        let radii: Vec<f32> = values.iter().map(|&s| pile_radius(s)).collect();
        for w in radii.windows(2) {
            assert!(w[1] >= w[0], "radius should increase: {} >= {}", w[1], w[0]);
        }
    }

    // -----------------------------------------------------------------------
    // compute_pile_cells
    // -----------------------------------------------------------------------

    #[test]
    fn compute_pile_cells_radius_zero_returns_center_only() {
        // Radius 0 still includes the center cell itself (0*0 + 0*0 <= 0)
        let cells = compute_pile_cells(UVec3::new(5, 5, 0), 0.0, 10, 10);
        assert_eq!(cells.len(), 1);
        assert!(cells.contains(&UVec3::new(5, 5, 0)));
    }

    #[test]
    fn compute_pile_cells_small_radius_returns_center() {
        let cells = compute_pile_cells(UVec3::new(5, 5, 0), 0.5, 10, 10);
        assert!(cells.contains(&UVec3::new(5, 5, 0)));
        assert_eq!(cells.len(), 1);
    }

    #[test]
    fn compute_pile_cells_grows_with_radius() {
        let small = compute_pile_cells(UVec3::new(10, 10, 0), 1.0, 20, 20);
        let large = compute_pile_cells(UVec3::new(10, 10, 0), 3.0, 20, 20);
        assert!(large.len() > small.len());
    }

    #[test]
    fn compute_pile_cells_clamps_at_grid_boundaries() {
        // Center at corner — cells should not go negative
        let cells = compute_pile_cells(UVec3::new(0, 0, 0), 3.0, 10, 10);
        for cell in &cells {
            assert!(cell.x < 10);
            assert!(cell.y < 10);
        }
    }

    #[test]
    fn compute_pile_cells_all_within_bounds() {
        let cells = compute_pile_cells(UVec3::new(5, 5, 0), 4.0, 8, 8);
        for cell in &cells {
            assert!(cell.x < 8, "x={} out of bounds", cell.x);
            assert!(cell.y < 8, "y={} out of bounds", cell.y);
        }
    }

    // -----------------------------------------------------------------------
    // nearest_pile_cell
    // -----------------------------------------------------------------------

    #[test]
    fn nearest_pile_cell_returns_closest() {
        let mut cells = HashSet::new();
        cells.insert(UVec3::new(0, 0, 0));
        cells.insert(UVec3::new(10, 10, 0));
        cells.insert(UVec3::new(5, 5, 0));
        let state = PileState {
            cells,
            center: UVec3::new(5, 5, 0),
            radius_tiles: 5.0,
            last_radius_int: 5,
        };
        let result = nearest_pile_cell(UVec3::new(4, 4, 0), &state);
        assert_eq!(result, UVec3::new(5, 5, 0));
    }

    #[test]
    fn nearest_pile_cell_empty_returns_center() {
        let state = PileState {
            cells: HashSet::new(),
            center: UVec3::new(20, 16, 0),
            radius_tiles: 0.0,
            last_radius_int: 0,
        };
        let result = nearest_pile_cell(UVec3::new(0, 0, 0), &state);
        assert_eq!(result, UVec3::new(20, 16, 0));
    }

    // -----------------------------------------------------------------------
    // nearest_edge_cell
    // -----------------------------------------------------------------------

    #[test]
    fn nearest_edge_cell_returns_closest() {
        let edges = vec![
            UVec3::new(0, 0, 0),
            UVec3::new(39, 0, 0),
            UVec3::new(0, 31, 0),
        ];
        let result = nearest_edge_cell(UVec3::new(38, 1, 0), &edges);
        assert_eq!(result, UVec3::new(39, 0, 0));
    }

    #[test]
    fn nearest_edge_cell_empty_returns_pos() {
        let result = nearest_edge_cell(UVec3::new(5, 5, 0), &[]);
        assert_eq!(result, UVec3::new(5, 5, 0));
    }
}
