use std::collections::HashSet;

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use crate::common::constants::*;
use crate::grid::components::GridCell;
use crate::pile::resources::{EdgeCells, PileState};

use super::components::Obstacle;

const NEIGHBORS_4: [IVec2; 4] = [
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
];

fn in_bounds(cell: IVec2, config: &GridConfig) -> bool {
    cell.x >= 0
        && cell.y >= 0
        && (cell.x as u32) < config.width
        && (cell.y as u32) < config.height
}

/// Build set of cells that must never be obstacles.
fn build_exclusion_set(
    config: &GridConfig,
    pile_state: &PileState,
    edge_cells: &EdgeCells,
) -> HashSet<IVec2> {
    let mut excluded = HashSet::new();

    // Edge cells + 1-cell inward buffer.
    for &edge in &edge_cells.0 {
        let ec = IVec2::new(edge.x as i32, edge.y as i32);
        excluded.insert(ec);
        for offset in &NEIGHBORS_4 {
            let n = ec + *offset;
            if in_bounds(n, config) {
                excluded.insert(n);
            }
        }
    }

    // Pile center + buffer.
    let cx = pile_state.center.x as i32;
    let cy = pile_state.center.y as i32;
    let buffer = 3i32;
    for dx in -buffer..=buffer {
        for dy in -buffer..=buffer {
            let cell = IVec2::new(cx + dx, cy + dy);
            if in_bounds(cell, config) {
                excluded.insert(cell);
            }
        }
    }

    excluded
}

/// Try to pick a random cell not in the occupied set.
fn pick_random_seed(
    config: &GridConfig,
    occupied: &HashSet<IVec2>,
    rng: &mut impl Rng,
) -> Option<IVec2> {
    for _ in 0..100 {
        let cell = IVec2::new(
            rng.random_range(0..config.width as i32),
            rng.random_range(0..config.height as i32),
        );
        if !occupied.contains(&cell) {
            return Some(cell);
        }
    }
    None
}

/// Grow an irregular blob from a seed using frontier expansion.
fn grow_cluster(
    seed: IVec2,
    target_size: u32,
    occupied: &HashSet<IVec2>,
    config: &GridConfig,
    rng: &mut impl Rng,
) -> Vec<IVec2> {
    let mut cluster = vec![seed];
    let mut in_cluster: HashSet<IVec2> = HashSet::from([seed]);
    let mut frontier: Vec<IVec2> = NEIGHBORS_4
        .iter()
        .map(|o| seed + *o)
        .filter(|n| in_bounds(*n, config) && !occupied.contains(n))
        .collect();

    while (cluster.len() as u32) < target_size && !frontier.is_empty() {
        let idx = rng.random_range(0..frontier.len());
        let cell = frontier.swap_remove(idx);

        if in_cluster.contains(&cell) || occupied.contains(&cell) {
            continue;
        }

        // Growth chance decreases with size for irregular edges.
        let growth_chance = 0.85 - (cluster.len() as f32 / target_size as f32) * 0.3;
        if rng.random::<f32>() > growth_chance {
            continue;
        }

        cluster.push(cell);
        in_cluster.insert(cell);

        for offset in &NEIGHBORS_4 {
            let n = cell + *offset;
            if in_bounds(n, config) && !in_cluster.contains(&n) && !occupied.contains(&n) {
                frontier.push(n);
            }
        }
    }

    cluster
}

/// Validate that all 4 edge midpoints can reach the pile center.
fn validate_paths(grid: &OrdinalGrid, config: &GridConfig, center: UVec3) -> bool {
    let edge_midpoints = [
        UVec3::new(0, config.height / 2, 0),
        UVec3::new(config.width - 1, config.height / 2, 0),
        UVec3::new(config.width / 2, 0, 0),
        UVec3::new(config.width / 2, config.height - 1, 0),
    ];

    edge_midpoints
        .iter()
        .all(|edge| grid.pathfind(&mut PathfindArgs::new(*edge, center).astar()).is_some())
}

pub fn generate_obstacles(
    mut commands: Commands,
    config: Res<GridConfig>,
    pile_state: Res<PileState>,
    edge_cells: Res<EdgeCells>,
    mut grid_query: Query<&mut OrdinalGrid>,
    mut cell_query: Query<(Entity, &GridCell, &mut Sprite)>,
) {
    let Ok(mut grid) = grid_query.single_mut() else {
        return;
    };
    let mut rng = rand::rng();

    // 1. Build exclusion set.
    let excluded = build_exclusion_set(&config, &pile_state, &edge_cells);
    let mut occupied = excluded.clone();

    // 2. Compute cluster parameters.
    let total_cells = config.width * config.height;
    let target_cells = (total_cells as f32 * OBSTACLE_COVERAGE) as u32;
    let avg_cluster = (OBSTACLE_MIN_CLUSTER + OBSTACLE_MAX_CLUSTER) / 2;
    let num_clusters = target_cells / avg_cluster;

    // 3. Generate clusters.
    let mut placed_clusters: Vec<Vec<IVec2>> = Vec::new();

    for _ in 0..num_clusters {
        let Some(seed) = pick_random_seed(&config, &occupied, &mut rng) else {
            break;
        };

        let target_size = rng.random_range(OBSTACLE_MIN_CLUSTER..=OBSTACLE_MAX_CLUSTER);
        let cluster = grow_cluster(seed, target_size, &occupied, &config, &mut rng);

        for &cell in &cluster {
            grid.set_nav(UVec3::new(cell.x as u32, cell.y as u32, 0), Nav::Impassable);
            occupied.insert(cell);
        }
        placed_clusters.push(cluster);
    }

    // 4. Build and validate — remove clusters from the back until all paths work.
    grid.build();
    while !validate_paths(&grid, &config, pile_state.center) {
        if let Some(removed) = placed_clusters.pop() {
            for &cell in &removed {
                grid.set_nav(
                    UVec3::new(cell.x as u32, cell.y as u32, 0),
                    Nav::Passable(1),
                );
                occupied.remove(&cell);
            }
            grid.build();
        } else {
            break;
        }
    }

    // 5. Collect final obstacle set and update visuals.
    let obstacle_set: HashSet<IVec2> = placed_clusters.iter().flatten().copied().collect();

    for (entity, grid_cell, mut sprite) in &mut cell_query {
        if obstacle_set.contains(&grid_cell.coord) {
            commands.entity(entity).insert(Obstacle);
            let Srgba { red, green, blue, .. } = Srgba::from(OBSTACLE_COLOR);
            let v = rng.random_range(-0.03..0.03);
            sprite.color = Color::srgb(
                (red + v).clamp(0.0, 1.0),
                (green + v).clamp(0.0, 1.0),
                (blue + v).clamp(0.0, 1.0),
            );
        }
    }

    info!(
        "Generated {} obstacle clusters ({} cells, {:.1}% coverage)",
        placed_clusters.len(),
        obstacle_set.len(),
        obstacle_set.len() as f32 / total_cells as f32 * 100.0,
    );
}
