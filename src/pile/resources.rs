use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource)]
pub struct PileScrap {
    pub amount: u32,
}

#[derive(Resource)]
pub struct PileState {
    pub cells: HashSet<UVec3>,
    pub center: UVec3,
    pub radius_tiles: f32,
    /// Cached integer radius to throttle recalculations.
    pub last_radius_int: u32,
}

#[derive(Resource)]
pub struct EdgeCells(pub Vec<UVec3>);
