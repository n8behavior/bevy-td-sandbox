use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Cumulative statistics for a single play session.
#[derive(Resource)]
pub struct RunStats {
    /// `Time::elapsed_secs()` captured when the run begins.
    pub start_time: f32,
    /// Updated each frame: current elapsed minus `start_time`.
    pub survival_time_secs: f32,
    /// Per-blueprint kill counts. Keys are `EnemyBlueprint::name` values
    /// (e.g. `"Shambler"`, `"Boss"`); a new enemy type adds a new map
    /// entry without touching this struct.
    pub kills: HashMap<&'static str, u32>,
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
            kills: HashMap::new(),
            scrap_collected: 0,
            scrap_spent: 0,
            towers_placed: 0,
            towers_sold: 0,
        }
    }

    /// Increment the kill counter for the given blueprint name.
    pub fn record_kill(&mut self, blueprint_name: &'static str) {
        *self.kills.entry(blueprint_name).or_insert(0) += 1;
    }

    /// Total kills across every enemy type.
    pub fn total_kills(&self) -> u32 {
        self.kills.values().sum()
    }

    /// Lookup helper for UIs and tests.
    pub fn kills_of(&self, blueprint_name: &str) -> u32 {
        self.kills.get(blueprint_name).copied().unwrap_or(0)
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
        assert_eq!(stats.total_kills(), 0);
        assert_eq!(stats.scrap_collected, 0);
        assert_eq!(stats.scrap_spent, 0);
        assert_eq!(stats.towers_placed, 0);
        assert_eq!(stats.towers_sold, 0);
    }

    // -- record_kill --

    #[test]
    fn record_kill_shambler() {
        let mut stats = make_test_stats();
        stats.record_kill("Shambler");
        assert_eq!(stats.kills_of("Shambler"), 1);
        assert_eq!(stats.kills_of("Runner"), 0);
        assert_eq!(stats.kills_of("Brute"), 0);
        assert_eq!(stats.kills_of("Boss"), 0);
    }

    #[test]
    fn record_kill_runner() {
        let mut stats = make_test_stats();
        stats.record_kill("Runner");
        assert_eq!(stats.kills_of("Runner"), 1);
        assert_eq!(stats.kills_of("Shambler"), 0);
    }

    #[test]
    fn record_kill_brute() {
        let mut stats = make_test_stats();
        stats.record_kill("Brute");
        assert_eq!(stats.kills_of("Brute"), 1);
    }

    #[test]
    fn record_kill_boss() {
        let mut stats = make_test_stats();
        stats.record_kill("Boss");
        assert_eq!(stats.kills_of("Boss"), 1);
    }

    #[test]
    fn record_kill_unknown_blueprint_still_counts() {
        // The HashMap-backed counter accepts any &'static str — adding a
        // new enemy type doesn't require touching RunStats.
        let mut stats = make_test_stats();
        stats.record_kill("Raider");
        assert_eq!(stats.kills_of("Raider"), 1);
    }

    #[test]
    fn record_kill_multiple_types() {
        let mut stats = make_test_stats();
        stats.record_kill("Shambler");
        stats.record_kill("Shambler");
        stats.record_kill("Boss");
        assert_eq!(stats.kills_of("Shambler"), 2);
        assert_eq!(stats.kills_of("Boss"), 1);
    }

    // -- total_kills --

    #[test]
    fn total_kills_sums_all_types() {
        let mut stats = make_test_stats();
        for _ in 0..3 {
            stats.record_kill("Shambler");
        }
        for _ in 0..5 {
            stats.record_kill("Runner");
        }
        for _ in 0..2 {
            stats.record_kill("Brute");
        }
        stats.record_kill("Boss");
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
