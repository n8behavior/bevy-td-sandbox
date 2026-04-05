use bevy::prelude::*;

#[derive(Resource)]
pub struct RunStats {
    /// Time::elapsed_secs() at game start.
    pub start_time: f32,
    /// Updated each frame: current elapsed - start_time.
    pub survival_time_secs: f32,
    pub kills_shambler: u32,
    pub kills_runner: u32,
    pub kills_brute: u32,
    pub kills_boss: u32,
    pub scrap_collected: u32,
    pub scrap_spent: u32,
    pub towers_placed: u32,
    pub towers_sold: u32,
}

impl RunStats {
    pub fn total_kills(&self) -> u32 {
        self.kills_shambler + self.kills_runner + self.kills_brute + self.kills_boss
    }
}
