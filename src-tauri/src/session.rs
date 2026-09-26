use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub daimoku_count: u32,
}

impl Session {
    pub fn duration_seconds(&self) -> i64 {
        (self.ended_at - self.started_at).num_seconds()
    }

    /// Returns `None` when duration is zero, to avoid division by zero.
    pub fn daimoku_per_minute(&self) -> Option<f64> {
        let secs = self.duration_seconds();
        if secs <= 0 {
            return None;
        }
        Some(self.daimoku_count as f64 * 60.0 / secs as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn session(start_sec: i64, end_sec: i64, count: u32) -> Session {
        Session {
            started_at: Utc.timestamp_opt(start_sec, 0).unwrap(),
            ended_at: Utc.timestamp_opt(end_sec, 0).unwrap(),
            daimoku_count: count,
        }
    }

    #[test]
    fn duration_is_correct() {
        let s = session(1_700_000_000, 1_700_000_060, 30);
        assert_eq!(s.duration_seconds(), 60);
    }

    #[test]
    fn frequency_is_correct() {
        // 60 daimoku in 60 seconds = 60 per minute
        let s = session(1_700_000_000, 1_700_000_060, 60);
        assert_eq!(s.daimoku_per_minute(), Some(60.0));
    }

    #[test]
    fn frequency_none_on_zero_duration() {
        let s = session(1_700_000_000, 1_700_000_000, 10);
        assert_eq!(s.daimoku_per_minute(), None);
    }
}