//! Hotness scoring for memory lifecycle management (formula design borrowed
//! from OpenViking's `memory_lifecycle` concept).
//!
//! Computes a 0.0–1.0 hotness score blending access frequency and recency:
//!
//! ```text
//! score = sigmoid(log1p(active_count)) * time_decay(updated_at)
//! ```
//!
//! - **sigmoid** maps `log1p(active_count)` into (0, 1)
//! - **time_decay** is exponential decay with a configurable half-life
//!   (default 7 days); returns 0.0 when `updated_at` is unknown
//!
//! Intended to be blended with semantic similarity to boost frequently-used,
//! recently-updated memories during ranking, or consumed as a feature by the
//! online relevance learner.

use chrono::{DateTime, Utc};

/// Default half-life in days for the exponential time-decay component
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 7.0;

/// Sigmoid function with overflow-safe bounds.
fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        1.0 / (1.0 + (-x).exp())
    } else {
        let e = x.exp();
        e / (1.0 + e)
    }
}

/// Exponential time decay with configurable half-life; 0.0 for unknown times.
pub fn time_decay(
    updated_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    half_life_days: f64,
) -> f64 {
    match updated_at {
        None => 0.0,
        Some(t) => {
            let age_days = (now - t).num_seconds().max(0) as f64 / 86_400.0;
            let half_life_secs = half_life_days * 86_400.0;
            0.5f64.powf(age_days * 86_400.0 / half_life_secs)
        }
    }
}

/// Compute a 0.0–1.0 hotness score.
///
/// * `active_count` — number of times the memory was retrieved/accessed
/// * `updated_at` — last update/access timestamp (UTC preferred)
/// * `now` — override for deterministic tests
/// * `half_life_days` — recency decay half-life
pub fn hotness_score(
    active_count: u32,
    updated_at: Option<DateTime<Utc>>,
    now: impl Into<Option<DateTime<Utc>>>,
    half_life_days: f64,
) -> f64 {
    let now = now.into().unwrap_or_else(Utc::now);
    let frequency = sigmoid((active_count as f64).ln_1p());
    frequency * time_decay(updated_at, now, half_life_days)
}

/// Convenience wrapper using defaults (`now = Utc::now()`, 7-day half-life)
pub fn hotness(active_count: u32, updated_at: Option<DateTime<Utc>>) -> f64 {
    hotness_score(active_count, updated_at, None, DEFAULT_HALF_LIFE_DAYS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_sigmoid_bounds_and_symmetry() {
        assert!((sigmoid(0.0) - 0.5).abs() < 1e-9);
        assert!(sigmoid(1000.0) <= 1.0);
        assert!(sigmoid(-1000.0) >= 0.0);
        assert!((sigmoid(2.0) + sigmoid(-2.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_zero_access_is_low_hotness() {
        let now = Utc::now();
        // Fresh but never accessed: frequency component ~ sigmoid(0)=0.5
        let h = hotness_score(0, Some(now), now, 7.0);
        assert!((h - 0.5).abs() < 1e-9);
        // Never accessed and old: near zero (sigmoid(0)*0.5^(30/7) ≈ 0.026)
        let h2 = hotness_score(0, Some(now - Duration::days(30)), now, 7.0);
        assert!(h2 < 0.03);
        assert!(h2 < h);
    }

    #[test]
    fn test_unknown_timestamp_scores_zero() {
        assert_eq!(hotness_score(10, None, Utc::now(), 7.0), 0.0);
    }

    #[test]
    fn test_half_life_decay_values() {
        let now = Utc::now();
        // Exactly one half-life old => decay = 0.5
        let d = time_decay(Some(now), now, 7.0);
        assert!((d - 1.0).abs() < 1e-9); // age 0 => 1.0
        let later = now + Duration::days(7);
        let d2 = time_decay(Some(now), later, 7.0);
        assert!((d2 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_more_accesses_increase_hotness() {
        let now = Utc::now();
        let low = hotness_score(1, Some(now), now, 7.0);
        let mid = hotness_score(10, Some(now), now, 7.0);
        let high = hotness_score(100, Some(now), now, 7.0);
        assert!(low < mid && mid < high && high <= 1.0);
    }

    #[test]
    fn test_monotonic_in_recency() {
        let now = Utc::now();
        let fresh = hotness_score(5, Some(now - Duration::hours(1)), now, 7.0);
        let stale = hotness_score(5, Some(now - Duration::days(60)), now, 7.0);
        assert!(fresh > stale);
    }

    #[test]
    fn test_future_timestamp_clamped() {
        let now = Utc::now();
        let h = hotness_score(3, Some(now + Duration::days(30)), now, 7.0);
        // Should not exceed 1.0 or panic on negative age
        assert!(h <= 1.0);
    }

    #[test]
    fn test_convenience_wrapper() {
        let h = hotness(20, Some(Utc::now()));
        assert!((0.0..=1.0).contains(&h));
    }
}
