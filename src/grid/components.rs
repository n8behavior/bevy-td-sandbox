use bevy::prelude::*;

#[derive(Component)]
pub struct GridCell {
    pub coord: IVec2,
}

#[derive(Component)]
pub struct SpawnPoint;

#[derive(Component)]
pub struct GoalPoint;
