//! Bounded retries with exponential backoff and jitter.
//!
//! Only transient failures retry: transport errors, timeouts, and the statuses
//! that mean "ask again later" (408, 429, 500, 502, 503, 504). A definitive
//! answer such as 404 returns to the caller untouched; the job layer
//! classifies a missing page (docs/06-data-pipeline.md §5).

use std::time::Duration;

use reqwest::StatusCode;

use crate::throttle::random_delay;

/// The growth of the delay between attempts.
#[derive(Debug, Clone, Copy)]
pub struct Backoff {
    base: Duration,
    cap: Duration,
    jitter: Duration,
}

impl Backoff {
    pub fn new(base: Duration, cap: Duration, jitter: Duration) -> Self {
        Self { base, cap, jitter }
    }

    /// The delay before the attempt following the 1-based `attempt` that just
    /// failed: `base` doubled per attempt, capped, plus jitter.
    pub fn delay(&self, attempt: u32) -> Duration {
        let growths = attempt.saturating_sub(1).min(30);
        let growth = self
            .base
            .checked_mul(1u32 << growths)
            .unwrap_or(self.cap)
            .min(self.cap);
        growth + random_delay(self.jitter)
    }
}

/// A status the source uses to say "not now, ask later".
pub fn is_retryable_status(status: StatusCode) -> bool {
    matches!(status.as_u16(), 408 | 429 | 500 | 502 | 503 | 504)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backoff() -> Backoff {
        Backoff::new(
            Duration::from_millis(100),
            Duration::from_millis(1_000),
            Duration::ZERO,
        )
    }

    #[test]
    fn the_delay_doubles_per_attempt() {
        assert_eq!(backoff().delay(1), Duration::from_millis(100));
        assert_eq!(backoff().delay(2), Duration::from_millis(200));
        assert_eq!(backoff().delay(3), Duration::from_millis(400));
    }

    #[test]
    fn the_delay_never_exceeds_the_cap() {
        assert_eq!(backoff().delay(4), Duration::from_millis(800));
        assert_eq!(backoff().delay(5), Duration::from_millis(1_000));
        assert_eq!(backoff().delay(40), Duration::from_millis(1_000));
    }

    #[test]
    fn transient_statuses_retry_and_definitive_ones_do_not() {
        for status in [408u16, 429, 500, 502, 503, 504] {
            assert!(
                is_retryable_status(StatusCode::from_u16(status).expect("status")),
                "{status} should retry"
            );
        }
        for status in [200u16, 304, 400, 404, 410] {
            assert!(
                !is_retryable_status(StatusCode::from_u16(status).expect("status")),
                "{status} should not retry"
            );
        }
    }

    #[test]
    fn jitter_stays_within_its_bound() {
        let jittering = Backoff::new(
            Duration::from_millis(100),
            Duration::from_millis(1_000),
            Duration::from_millis(50),
        );
        for attempt in 1..=6 {
            let delay = jittering.delay(attempt);
            let doubled = Duration::from_millis(100) * 2u32.pow(attempt - 1);
            let floor = doubled.min(Duration::from_millis(1_000));
            assert!(
                delay >= floor && delay <= floor + Duration::from_millis(50),
                "attempt {attempt} waited {delay:?}"
            );
        }
    }
}
