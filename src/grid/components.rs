use bevy::prelude::*;

#[derive(Component)]
pub struct GridCell {
    pub coord: IVec2,
}

/// Marker for cells on the map edges (potential enemy spawn points).
#[derive(Component)]
pub struct EdgeCell;
