use std::collections::HashMap;

use bevy::prelude::*;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Terrain {
    Rubble,
    Puddle,
    Radioactive,
}

/// O(1) lookup of terrain type by grid coordinate.
#[derive(Resource, Default)]
pub struct TerrainMap {
    pub cells: HashMap<IVec2, Terrain>,
}
