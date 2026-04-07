use std::collections::HashSet;

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use crate::common::constants::*;
use crate::enemy::components::{Enemy, EnemyState, Health, MoveSpeed};
use crate::grid::components::GridCell;
use crate::grid::systems::world_to_grid;
use crate::pile::resources::{EdgeCells, PileState};

use super::components::{Terrain, TerrainMap};

const NEIGHBORS_4: [IVec2; 4] = [
    IVec2::new(1, 0),
    IVec2::new(-1, 0),
    IVec2::new(0, 1),
    IVec2::new(0, -1),
];

fn in_bounds(cell: IVec2, config: &GridConfig) -> bool {
    cell.x >= 0 && cell.y >= 0 && (cell.x as u32) < config.width && (cell.y as u32) < config.height
}

/// Build set of cells that must never be terrain.
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

    edge_midpoints.iter().all(|edge| {
        grid.pathfind(&mut PathfindArgs::new(*edge, center).astar())
            .is_some()
    })
}

fn terrain_color(terrain: Terrain, rng: &mut impl Rng) -> Color {
    let base = match terrain {
        Terrain::Rubble => RUBBLE_COLOR,
        Terrain::Puddle => PUDDLE_COLOR,
        Terrain::Radioactive => RADIOACTIVE_COLOR,
    };
    let Srgba {
        red, green, blue, ..
    } = Srgba::from(base);
    let v = rng.random_range(-0.03..0.03);
    Color::srgb(
        (red + v).clamp(0.0, 1.0),
        (green + v).clamp(0.0, 1.0),
        (blue + v).clamp(0.0, 1.0),
    )
}

/// Generate clusters for a single terrain type.
fn generate_clusters(
    num_clusters: u32,
    occupied: &mut HashSet<IVec2>,
    config: &GridConfig,
    rng: &mut impl Rng,
) -> Vec<Vec<IVec2>> {
    let mut clusters = Vec::new();
    for _ in 0..num_clusters {
        let Some(seed) = pick_random_seed(config, occupied, rng) else {
            break;
        };
        let target_size = rng.random_range(TERRAIN_MIN_CLUSTER..=TERRAIN_MAX_CLUSTER);
        let cluster = grow_cluster(seed, target_size, occupied, config, rng);
        for &cell in &cluster {
            occupied.insert(cell);
        }
        clusters.push(cluster);
    }
    clusters
}

pub fn generate_terrain(
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

    // 2. Compute per-type cluster counts.
    let total_cells = config.width * config.height;
    let target_cells = (total_cells as f32 * TERRAIN_COVERAGE) as u32;
    let avg_cluster = (TERRAIN_MIN_CLUSTER + TERRAIN_MAX_CLUSTER) / 2;

    let num_rubble = ((target_cells as f32 * RUBBLE_WEIGHT) as u32) / avg_cluster;
    let num_puddle = ((target_cells as f32 * PUDDLE_WEIGHT) as u32) / avg_cluster;
    let num_radioactive = ((target_cells as f32 * RADIOACTIVE_WEIGHT) as u32) / avg_cluster;

    // 3. Generate rubble clusters first (impassable — need path validation).
    let mut rubble_clusters = generate_clusters(num_rubble, &mut occupied, &config, &mut rng);

    for cluster in &rubble_clusters {
        for &cell in cluster {
            grid.set_nav(UVec3::new(cell.x as u32, cell.y as u32, 0), Nav::Impassable);
        }
    }

    // 4. Validate paths — remove rubble clusters from the back until all paths work.
    grid.build();
    while !validate_paths(&grid, &config, pile_state.center) {
        if let Some(removed) = rubble_clusters.pop() {
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

    // 5. Generate passable terrain clusters (no path validation needed).
    let puddle_clusters = generate_clusters(num_puddle, &mut occupied, &config, &mut rng);
    let radioactive_clusters = generate_clusters(num_radioactive, &mut occupied, &config, &mut rng);

    // 6. Build terrain map.
    let mut terrain_map = TerrainMap::default();

    for cluster in &rubble_clusters {
        for &cell in cluster {
            terrain_map.cells.insert(cell, Terrain::Rubble);
        }
    }
    for cluster in &puddle_clusters {
        for &cell in cluster {
            terrain_map.cells.insert(cell, Terrain::Puddle);
        }
    }
    for cluster in &radioactive_clusters {
        for &cell in cluster {
            terrain_map.cells.insert(cell, Terrain::Radioactive);
        }
    }

    // 7. Update cell visuals and insert Terrain components.
    for (entity, grid_cell, mut sprite) in &mut cell_query {
        if let Some(&terrain) = terrain_map.cells.get(&grid_cell.coord) {
            commands.entity(entity).insert(terrain);
            sprite.color = terrain_color(terrain, &mut rng);
        }
    }

    commands.insert_resource(terrain_map);

    let rubble_count: usize = rubble_clusters.iter().map(|c| c.len()).sum();
    let puddle_count: usize = puddle_clusters.iter().map(|c| c.len()).sum();
    let radio_count: usize = radioactive_clusters.iter().map(|c| c.len()).sum();
    let total = rubble_count + puddle_count + radio_count;

    info!(
        "Generated terrain: {} rubble, {} puddle, {} radioactive ({} cells, {:.1}% coverage)",
        rubble_count,
        puddle_count,
        radio_count,
        total,
        total as f32 / total_cells as f32 * 100.0,
    );
}

// ---------------------------------------------------------------------------
// Gameplay systems
// ---------------------------------------------------------------------------

pub fn apply_puddle_slow(
    mut enemies: Query<(&Transform, &mut MoveSpeed, &EnemyState), With<Enemy>>,
    terrain_map: Res<TerrainMap>,
    config: Res<GridConfig>,
) {
    for (transform, mut speed, state) in &mut enemies {
        if !state.is_alive() {
            continue;
        }
        let pos = transform.translation.truncate();
        let Some(grid_pos) = world_to_grid(pos, &config) else {
            continue;
        };
        if terrain_map.cells.get(&grid_pos) == Some(&Terrain::Puddle) {
            speed.current = speed.current.min(speed.base * PUDDLE_SLOW_FACTOR);
        }
    }
}

pub fn apply_radioactive_damage(
    mut enemies: Query<(&Transform, &mut Health, &EnemyState), With<Enemy>>,
    terrain_map: Res<TerrainMap>,
    config: Res<GridConfig>,
    time: Res<Time>,
) {
    for (transform, mut health, state) in &mut enemies {
        if !state.is_alive() {
            continue;
        }
        let pos = transform.translation.truncate();
        let Some(grid_pos) = world_to_grid(pos, &config) else {
            continue;
        };
        if terrain_map.cells.get(&grid_pos) == Some(&Terrain::Radioactive) {
            health.current -= RADIOACTIVE_DPS * time.delta_secs();
        }
    }
}
