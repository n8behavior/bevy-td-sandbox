//! Terrain generation and gameplay effect systems.
//!
//! Generation is split into pure helper functions ([`build_exclusion_set`],
//! [`grow_cluster`], [`generate_clusters`], [`validate_paths`]) orchestrated by
//! [`build_terrain_map`]. The ECS system [`generate_terrain`] is a thin wrapper
//! that calls `build_terrain_map` and applies visual results to sprites.
//!
//! Gameplay effects ([`apply_puddle_slow`], [`apply_radioactive_damage`]) use
//! the shared [`terrain_at`] helper to look up terrain at enemy positions.

use std::collections::HashSet;

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::{Rng, RngExt};

use crate::common::constants::*;
use crate::enemy::components::{Enemy, Health, MoveSpeed};
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
    let buffer = PILE_BUFFER_RADIUS;
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
        let growth_chance = CLUSTER_GROWTH_BASE
            - (cluster.len() as f32 / target_size as f32) * CLUSTER_GROWTH_DECAY;
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

// ANCHOR: build_terrain_map
/// Generate all terrain clusters and return a populated [`TerrainMap`].
///
/// Mutates `grid` to mark rubble cells as [`Nav::Impassable`] (with
/// path-validation backtracking), but performs no ECS entity work.
fn build_terrain_map(
    config: &GridConfig,
    pile_state: &PileState,
    edge_cells: &EdgeCells,
    grid: &mut OrdinalGrid,
    rng: &mut impl Rng,
) -> TerrainMap {
    // 1. Build exclusion set.
    let excluded = build_exclusion_set(config, pile_state, edge_cells);
    let mut occupied = excluded.clone();

    // 2. Compute per-type cluster counts.
    let target_cells = (config.width * config.height) as f32 * TERRAIN_COVERAGE;
    let avg_cluster = (TERRAIN_MIN_CLUSTER + TERRAIN_MAX_CLUSTER) / 2;

    let num_rubble = (target_cells * RUBBLE_WEIGHT) as u32 / avg_cluster;
    let num_puddle = (target_cells * PUDDLE_WEIGHT) as u32 / avg_cluster;
    let num_radioactive = (target_cells * RADIOACTIVE_WEIGHT) as u32 / avg_cluster;

    // 3. Generate rubble clusters first (impassable — need path validation).
    let mut rubble_clusters = generate_clusters(num_rubble, &mut occupied, config, rng);

    for cluster in &rubble_clusters {
        for &cell in cluster {
            grid.set_nav(UVec3::new(cell.x as u32, cell.y as u32, 0), Nav::Impassable);
        }
    }

    // 4. Validate paths — remove rubble clusters from the back until all paths work.
    grid.build();
    while !validate_paths(grid, config, pile_state.center) {
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
    let puddle_clusters = generate_clusters(num_puddle, &mut occupied, config, rng);
    let radioactive_clusters = generate_clusters(num_radioactive, &mut occupied, config, rng);

    // 6. Populate terrain map.
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

    terrain_map
}
// ANCHOR_END: build_terrain_map

// ANCHOR: generate_terrain
/// ECS system that generates terrain and applies visuals to grid cells.
///
/// Delegates data generation to [`build_terrain_map`], then inserts
/// [`Terrain`] components and tints sprites.
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

    let terrain_map = build_terrain_map(&config, &pile_state, &edge_cells, &mut grid, &mut rng);

    // Apply visuals.
    for (entity, grid_cell, mut sprite) in &mut cell_query {
        if let Some(&terrain) = terrain_map.cells.get(&grid_cell.coord) {
            commands.entity(entity).insert(terrain);
            sprite.color = terrain_color(terrain, &mut rng);
        }
    }

    // Log coverage stats.
    let total_cells = config.width * config.height;
    let rubble_count = terrain_map
        .cells
        .values()
        .filter(|t| **t == Terrain::Rubble)
        .count();
    let puddle_count = terrain_map
        .cells
        .values()
        .filter(|t| **t == Terrain::Puddle)
        .count();
    let radio_count = terrain_map
        .cells
        .values()
        .filter(|t| **t == Terrain::Radioactive)
        .count();
    let total = terrain_map.cells.len();

    info!(
        "Generated terrain: {} rubble, {} puddle, {} radioactive ({} cells, {:.1}% coverage)",
        rubble_count,
        puddle_count,
        radio_count,
        total,
        total as f32 / total_cells as f32 * 100.0,
    );

    commands.insert_resource(terrain_map);
}
// ANCHOR_END: generate_terrain

// ---------------------------------------------------------------------------
// Gameplay systems
// ---------------------------------------------------------------------------

// ANCHOR: terrain_at
/// Look up the terrain type at a world position.
///
/// Returns `None` if the position is off-grid or the cell has no terrain.
fn terrain_at(pos: Vec2, terrain_map: &TerrainMap, config: &GridConfig) -> Option<Terrain> {
    let grid_pos = world_to_grid(pos, config)?;
    terrain_map.cells.get(&grid_pos).copied()
}
// ANCHOR_END: terrain_at

/// Reduce movement speed for enemies standing on puddle cells.
pub fn apply_puddle_slow(
    mut enemies: Query<(&Transform, &mut MoveSpeed), With<Enemy>>,
    terrain_map: Res<TerrainMap>,
    config: Res<GridConfig>,
) {
    for (transform, mut speed) in &mut enemies {
        if terrain_at(transform.translation.truncate(), &terrain_map, &config)
            == Some(Terrain::Puddle)
        {
            speed.current = speed.current.min(speed.base * PUDDLE_SLOW_FACTOR);
        }
    }
}

/// Deal damage-over-time to enemies standing on radioactive cells.
pub fn apply_radioactive_damage(
    mut enemies: Query<(&Transform, &mut Health), With<Enemy>>,
    terrain_map: Res<TerrainMap>,
    config: Res<GridConfig>,
    time: Res<Time>,
) {
    for (transform, mut health) in &mut enemies {
        if terrain_at(transform.translation.truncate(), &terrain_map, &config)
            == Some(Terrain::Radioactive)
        {
            health.current -= RADIOACTIVE_DPS * time.delta_secs();
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::systems::grid_to_world_cfg;
    use crate::pile::resources::{EdgeCells, PileState};
    use crate::test_helpers::test_grid_config;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    /// Small grid config for focused tests (avoids slow cluster generation).
    fn small_config() -> GridConfig {
        GridConfig {
            width: 16,
            height: 16,
            pixel_scale: 1,
        }
    }

    /// Minimal pile state at grid center with no cells.
    fn test_pile(config: &GridConfig) -> PileState {
        PileState {
            cells: HashSet::new(),
            center: config.center(),
            radius_tiles: 0.0,
            last_radius_int: 0,
        }
    }

    // -- in_bounds --

    #[test]
    fn in_bounds_interior_cell() {
        let config = small_config();
        assert!(in_bounds(IVec2::new(5, 5), &config));
    }

    #[test]
    fn in_bounds_edge_cells() {
        let config = small_config();
        assert!(in_bounds(IVec2::new(0, 0), &config));
        assert!(in_bounds(
            IVec2::new(config.width as i32 - 1, config.height as i32 - 1),
            &config
        ));
    }

    #[test]
    fn in_bounds_negative_returns_false() {
        let config = small_config();
        assert!(!in_bounds(IVec2::new(-1, 0), &config));
        assert!(!in_bounds(IVec2::new(0, -1), &config));
    }

    #[test]
    fn in_bounds_beyond_dimensions_returns_false() {
        let config = small_config();
        assert!(!in_bounds(IVec2::new(config.width as i32, 0), &config));
        assert!(!in_bounds(IVec2::new(0, config.height as i32), &config));
    }

    // -- build_exclusion_set --

    #[test]
    fn exclusion_set_contains_pile_center() {
        let config = small_config();
        let pile = test_pile(&config);
        let excluded = build_exclusion_set(&config, &pile, &EdgeCells(Vec::new()));
        let center = IVec2::new(pile.center.x as i32, pile.center.y as i32);
        assert!(excluded.contains(&center));
    }

    #[test]
    fn exclusion_set_contains_pile_buffer() {
        let config = small_config();
        let pile = test_pile(&config);
        let excluded = build_exclusion_set(&config, &pile, &EdgeCells(Vec::new()));
        let cx = pile.center.x as i32;
        let cy = pile.center.y as i32;
        for dx in -PILE_BUFFER_RADIUS..=PILE_BUFFER_RADIUS {
            for dy in -PILE_BUFFER_RADIUS..=PILE_BUFFER_RADIUS {
                let cell = IVec2::new(cx + dx, cy + dy);
                if in_bounds(cell, &config) {
                    assert!(
                        excluded.contains(&cell),
                        "buffer cell ({}, {}) should be excluded",
                        cell.x,
                        cell.y
                    );
                }
            }
        }
    }

    #[test]
    fn exclusion_set_contains_edge_neighbors() {
        let config = small_config();
        let pile = test_pile(&config);
        let edge = UVec3::new(0, config.height / 2, 0);
        let excluded = build_exclusion_set(&config, &pile, &EdgeCells(vec![edge]));

        let ec = IVec2::new(edge.x as i32, edge.y as i32);
        assert!(
            excluded.contains(&ec),
            "edge cell itself should be excluded"
        );
        // Inward neighbor (1, height/2) should also be excluded.
        let inward = IVec2::new(1, edge.y as i32);
        assert!(
            excluded.contains(&inward),
            "inward neighbor should be excluded"
        );
    }

    #[test]
    fn exclusion_set_all_cells_in_bounds() {
        let config = small_config();
        let pile = test_pile(&config);
        let excluded = build_exclusion_set(&config, &pile, &EdgeCells(Vec::new()));
        for cell in &excluded {
            assert!(
                in_bounds(*cell, &config),
                "excluded cell ({}, {}) out of bounds",
                cell.x,
                cell.y
            );
        }
    }

    // -- pick_random_seed --

    #[test]
    fn pick_random_seed_avoids_occupied() {
        let config = small_config();
        let mut occupied = HashSet::new();
        occupied.insert(IVec2::new(5, 5));
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..50 {
            if let Some(cell) = pick_random_seed(&config, &occupied, &mut rng) {
                assert!(!occupied.contains(&cell));
            }
        }
    }

    #[test]
    fn pick_random_seed_returns_none_when_full() {
        let config = small_config();
        let mut occupied = HashSet::new();
        for x in 0..config.width as i32 {
            for y in 0..config.height as i32 {
                occupied.insert(IVec2::new(x, y));
            }
        }
        let mut rng = SmallRng::seed_from_u64(42);
        assert!(pick_random_seed(&config, &occupied, &mut rng).is_none());
    }

    // -- grow_cluster --

    #[test]
    fn grow_cluster_contains_seed() {
        let config = small_config();
        let seed = IVec2::new(8, 8);
        let mut rng = SmallRng::seed_from_u64(42);
        let cluster = grow_cluster(seed, 10, &HashSet::new(), &config, &mut rng);
        assert!(cluster.contains(&seed));
    }

    #[test]
    fn grow_cluster_respects_max_size() {
        let config = small_config();
        let mut rng = SmallRng::seed_from_u64(42);
        let target = 8;
        let cluster = grow_cluster(IVec2::new(8, 8), target, &HashSet::new(), &config, &mut rng);
        assert!(
            cluster.len() as u32 <= target,
            "cluster size {} exceeds target {}",
            cluster.len(),
            target
        );
    }

    #[test]
    fn grow_cluster_avoids_occupied() {
        let config = small_config();
        let mut occupied = HashSet::new();
        occupied.insert(IVec2::new(9, 8));
        occupied.insert(IVec2::new(8, 9));
        let mut rng = SmallRng::seed_from_u64(42);
        let cluster = grow_cluster(IVec2::new(8, 8), 10, &occupied, &config, &mut rng);
        for cell in &cluster {
            assert!(
                !occupied.contains(cell),
                "cluster contains occupied cell {cell}"
            );
        }
    }

    #[test]
    fn grow_cluster_all_in_bounds() {
        let config = small_config();
        let mut rng = SmallRng::seed_from_u64(42);
        let cluster = grow_cluster(IVec2::new(0, 0), 15, &HashSet::new(), &config, &mut rng);
        for cell in &cluster {
            assert!(
                in_bounds(*cell, &config),
                "cluster cell ({}, {}) out of bounds",
                cell.x,
                cell.y
            );
        }
    }

    #[test]
    fn grow_cluster_cells_contiguous() {
        let config = small_config();
        let seed = IVec2::new(8, 8);
        let mut rng = SmallRng::seed_from_u64(42);
        let cluster = grow_cluster(seed, 15, &HashSet::new(), &config, &mut rng);
        let cluster_set: HashSet<IVec2> = cluster.iter().copied().collect();
        for &cell in &cluster {
            if cell == seed {
                continue;
            }
            let has_neighbor = NEIGHBORS_4
                .iter()
                .any(|o| cluster_set.contains(&(cell + *o)));
            assert!(
                has_neighbor,
                "cluster cell ({}, {}) has no neighbor in cluster",
                cell.x, cell.y
            );
        }
    }

    // -- generate_clusters --

    #[test]
    fn generate_clusters_marks_occupied() {
        let config = small_config();
        let mut occupied = HashSet::new();
        let mut rng = SmallRng::seed_from_u64(42);
        let clusters = generate_clusters(3, &mut occupied, &config, &mut rng);
        for cluster in &clusters {
            for cell in cluster {
                assert!(occupied.contains(cell));
            }
        }
    }

    #[test]
    fn generate_clusters_respects_count() {
        let config = small_config();
        let mut occupied = HashSet::new();
        let mut rng = SmallRng::seed_from_u64(42);
        let clusters = generate_clusters(5, &mut occupied, &config, &mut rng);
        assert!(clusters.len() <= 5);
    }

    #[test]
    fn generate_clusters_non_overlapping() {
        let config = small_config();
        let mut occupied = HashSet::new();
        let mut rng = SmallRng::seed_from_u64(42);
        let clusters = generate_clusters(3, &mut occupied, &config, &mut rng);
        let mut seen = HashSet::new();
        for cluster in &clusters {
            for &cell in cluster {
                assert!(
                    seen.insert(cell),
                    "cell ({}, {}) appears in multiple clusters",
                    cell.x,
                    cell.y
                );
            }
        }
    }

    // -- terrain_at --

    #[test]
    fn terrain_at_returns_correct_type() {
        let config = test_grid_config();
        let mut terrain_map = TerrainMap::default();
        terrain_map.cells.insert(IVec2::new(5, 5), Terrain::Puddle);
        let world_pos = grid_to_world_cfg(UVec3::new(5, 5, 0), &config);
        assert_eq!(
            terrain_at(world_pos, &terrain_map, &config),
            Some(Terrain::Puddle)
        );
    }

    #[test]
    fn terrain_at_returns_none_for_empty_cell() {
        let config = test_grid_config();
        let terrain_map = TerrainMap::default();
        let world_pos = grid_to_world_cfg(UVec3::new(5, 5, 0), &config);
        assert_eq!(terrain_at(world_pos, &terrain_map, &config), None);
    }

    #[test]
    fn terrain_at_returns_none_for_off_grid() {
        let config = test_grid_config();
        let terrain_map = TerrainMap::default();
        assert_eq!(
            terrain_at(Vec2::new(10000.0, 10000.0), &terrain_map, &config),
            None
        );
    }

    // -- terrain_color --

    #[test]
    fn terrain_color_near_base() {
        let mut rng = SmallRng::seed_from_u64(42);
        for terrain in [Terrain::Rubble, Terrain::Puddle, Terrain::Radioactive] {
            let color = terrain_color(terrain, &mut rng);
            let base = match terrain {
                Terrain::Rubble => RUBBLE_COLOR,
                Terrain::Puddle => PUDDLE_COLOR,
                Terrain::Radioactive => RADIOACTIVE_COLOR,
            };
            let Srgba {
                red: cr,
                green: cg,
                blue: cb,
                ..
            } = Srgba::from(color);
            let Srgba {
                red: br,
                green: bg,
                blue: bb,
                ..
            } = Srgba::from(base);
            assert!((cr - br).abs() <= 0.03, "red channel off for {terrain:?}");
            assert!((cg - bg).abs() <= 0.03, "green channel off for {terrain:?}");
            assert!((cb - bb).abs() <= 0.03, "blue channel off for {terrain:?}");
        }
    }

    #[test]
    fn terrain_color_channels_share_offset() {
        let mut rng = SmallRng::seed_from_u64(99);
        let color = terrain_color(Terrain::Puddle, &mut rng);
        let Srgba {
            red: cr,
            green: cg,
            blue: cb,
            ..
        } = Srgba::from(color);
        let Srgba {
            red: br,
            green: bg,
            blue: bb,
            ..
        } = Srgba::from(PUDDLE_COLOR);
        let dr = cr - br;
        let dg = cg - bg;
        let db = cb - bb;
        // All channels should have the same offset (before clamping).
        assert!(
            (dr - dg).abs() < 1e-5 && (dg - db).abs() < 1e-5,
            "channel offsets should be equal: dr={dr}, dg={dg}, db={db}"
        );
    }

    // -- build_terrain_map --

    #[test]
    fn build_terrain_map_no_terrain_in_exclusion_set() {
        let config = small_config();
        let pile = test_pile(&config);
        let edge_cells = EdgeCells(Vec::new());
        let excluded = build_exclusion_set(&config, &pile, &edge_cells);

        let settings = GridSettingsBuilder::new_2d(config.width, config.height)
            .chunk_size(8)
            .build();
        let mut grid = OrdinalGrid::new(&settings);
        grid.build();
        let mut rng = SmallRng::seed_from_u64(42);

        let terrain_map = build_terrain_map(&config, &pile, &edge_cells, &mut grid, &mut rng);

        for cell in terrain_map.cells.keys() {
            assert!(
                !excluded.contains(cell),
                "terrain at ({}, {}) is in the exclusion set",
                cell.x,
                cell.y
            );
        }
    }

    #[test]
    fn build_terrain_map_deterministic() {
        let config = small_config();
        let pile = test_pile(&config);
        let edge_cells = EdgeCells(Vec::new());

        let settings = GridSettingsBuilder::new_2d(config.width, config.height)
            .chunk_size(8)
            .build();

        let mut grid1 = OrdinalGrid::new(&settings);
        grid1.build();
        let mut rng1 = SmallRng::seed_from_u64(123);
        let map1 = build_terrain_map(&config, &pile, &edge_cells, &mut grid1, &mut rng1);

        let mut grid2 = OrdinalGrid::new(&settings);
        grid2.build();
        let mut rng2 = SmallRng::seed_from_u64(123);
        let map2 = build_terrain_map(&config, &pile, &edge_cells, &mut grid2, &mut rng2);

        assert_eq!(
            map1.cells, map2.cells,
            "same seed should produce identical terrain"
        );
    }

    #[test]
    fn build_terrain_map_contains_all_types() {
        // Use a larger grid so there is room for all terrain types.
        let config = test_grid_config();
        let pile = test_pile(&config);
        let edge_cells = EdgeCells(Vec::new());

        let settings = GridSettingsBuilder::new_2d(config.width, config.height)
            .chunk_size(8)
            .build();
        let mut grid = OrdinalGrid::new(&settings);
        grid.build();
        let mut rng = SmallRng::seed_from_u64(42);

        let terrain_map = build_terrain_map(&config, &pile, &edge_cells, &mut grid, &mut rng);

        let has_rubble = terrain_map.cells.values().any(|t| *t == Terrain::Rubble);
        let has_puddle = terrain_map.cells.values().any(|t| *t == Terrain::Puddle);
        let has_radio = terrain_map
            .cells
            .values()
            .any(|t| *t == Terrain::Radioactive);
        assert!(has_rubble, "should generate rubble");
        assert!(has_puddle, "should generate puddles");
        assert!(has_radio, "should generate radioactive");
    }

    #[test]
    fn build_terrain_map_rubble_is_impassable() {
        let config = small_config();
        let pile = test_pile(&config);
        let edge_cells = EdgeCells(Vec::new());

        let settings = GridSettingsBuilder::new_2d(config.width, config.height)
            .chunk_size(8)
            .build();
        let mut grid = OrdinalGrid::new(&settings);
        grid.build();
        let mut rng = SmallRng::seed_from_u64(42);

        let terrain_map = build_terrain_map(&config, &pile, &edge_cells, &mut grid, &mut rng);

        for (&cell, &terrain) in &terrain_map.cells {
            let pos = UVec3::new(cell.x as u32, cell.y as u32, 0);
            if terrain == Terrain::Rubble {
                assert!(
                    !grid.is_passable(pos),
                    "rubble at ({}, {}) should be impassable",
                    cell.x,
                    cell.y
                );
            } else {
                assert!(
                    grid.is_passable(pos),
                    "non-rubble at ({}, {}) should be passable",
                    cell.x,
                    cell.y
                );
            }
        }
    }
}
