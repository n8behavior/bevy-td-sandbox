use bevy::prelude::*;

#[derive(Component)]
pub struct ScrapDrop {
    pub value: u32,
    pub lifetime: Timer,
}
