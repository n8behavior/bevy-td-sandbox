use bevy::prelude::*;

use crate::enemy::components::EnemyType;

/// Cumulative statistics for a single play session.
#[derive(Resource)]
pub struct RunStats {
    /// `Time::elapsed_secs()` captured when the run begins.
    pub start_time: f32,
    /// Updated each frame: current elapsed minus `start_time`.
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
    /// Creates a fresh stats snapshot with all counters zeroed.
    pub fn new(start_time: f32) -> Self {
        Self {
            start_time,
            survival_time_secs: 0.0,
            kills_shambler: 0,
            kills_runner: 0,
            kills_brute: 0,
            kills_boss: 0,
            scrap_collected: 0,
            scrap_spent: 0,
            towers_placed: 0,
            towers_sold: 0,
        }
    }

    /// Increments the kill counter for the given enemy type.
    pub fn record_kill(&mut self, enemy_type: EnemyType) {
        match enemy_type {
            EnemyType::Shambler => self.kills_shambler += 1,
            EnemyType::Runner => self.kills_runner += 1,
            EnemyType::Brute => self.kills_brute += 1,
            EnemyType::Boss => self.kills_boss += 1,
        }
    }

    pub fn total_kills(&self) -> u32 {
        self.kills_shambler + self.kills_runner + self.kills_brute + self.kills_boss
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::make_test_stats;

    // -- RunStats::new --

    #[test]
    fn new_captures_start_time() {
        let stats = RunStats::new(42.5);
        assert_eq!(stats.start_time, 42.5);
    }

    #[test]
    fn new_zeroes_all_counters() {
        let stats = make_test_stats();
        assert_eq!(stats.survival_time_secs, 0.0);
        assert_eq!(stats.kills_shambler, 0);
        assert_eq!(stats.kills_runner, 0);
        assert_eq!(stats.kills_brute, 0);
        assert_eq!(stats.kills_boss, 0);
        assert_eq!(stats.scrap_collected, 0);
        assert_eq!(stats.scrap_spent, 0);
        assert_eq!(stats.towers_placed, 0);
        assert_eq!(stats.towers_sold, 0);
    }

    // -- record_kill --

    #[test]
    fn record_kill_shambler() {
        let mut stats = make_test_stats();
        stats.record_kill(EnemyType::Shambler);
        assert_eq!(stats.kills_shambler, 1);
        assert_eq!(stats.kills_runner, 0);
        assert_eq!(stats.kills_brute, 0);
        assert_eq!(stats.kills_boss, 0);
    }

    #[test]
    fn record_kill_runner() {
        let mut stats = make_test_stats();
        stats.record_kill(EnemyType::Runner);
        assert_eq!(stats.kills_runner, 1);
        assert_eq!(stats.kills_shambler, 0);
        assert_eq!(stats.kills_brute, 0);
        assert_eq!(stats.kills_boss, 0);
    }

    #[test]
    fn record_kill_brute() {
        let mut stats = make_test_stats();
        stats.record_kill(EnemyType::Brute);
        assert_eq!(stats.kills_brute, 1);
        assert_eq!(stats.kills_shambler, 0);
        assert_eq!(stats.kills_runner, 0);
        assert_eq!(stats.kills_boss, 0);
    }

    #[test]
    fn record_kill_boss() {
        let mut stats = make_test_stats();
        stats.record_kill(EnemyType::Boss);
        assert_eq!(stats.kills_boss, 1);
        assert_eq!(stats.kills_shambler, 0);
        assert_eq!(stats.kills_runner, 0);
        assert_eq!(stats.kills_brute, 0);
    }

    #[test]
    fn record_kill_multiple_types() {
        let mut stats = make_test_stats();
        stats.record_kill(EnemyType::Shambler);
        stats.record_kill(EnemyType::Shambler);
        stats.record_kill(EnemyType::Boss);
        assert_eq!(stats.kills_shambler, 2);
        assert_eq!(stats.kills_boss, 1);
        assert_eq!(stats.kills_runner, 0);
        assert_eq!(stats.kills_brute, 0);
    }

    // -- total_kills --

    #[test]
    fn total_kills_sums_all_types() {
        let mut stats = make_test_stats();
        stats.kills_shambler = 3;
        stats.kills_runner = 5;
        stats.kills_brute = 2;
        stats.kills_boss = 1;
        assert_eq!(stats.total_kills(), 11);
    }

    #[test]
    fn total_kills_zero_when_fresh() {
        assert_eq!(make_test_stats().total_kills(), 0);
    }

    // -- survival time arithmetic --

    #[test]
    fn survival_time_zero_at_start() {
        let mut stats = RunStats::new(10.0);
        stats.survival_time_secs = 10.0 - stats.start_time;
        assert_eq!(stats.survival_time_secs, 0.0);
    }

    #[test]
    fn survival_time_advances() {
        let mut stats = RunStats::new(5.0);
        stats.survival_time_secs = 12.0 - stats.start_time;
        assert_eq!(stats.survival_time_secs, 7.0);
    }
}
