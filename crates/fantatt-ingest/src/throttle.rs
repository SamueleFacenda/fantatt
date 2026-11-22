//! Politeness throttle: a global concurrency semaphore plus a per-host token
//! bucket with jitter.
//!
//! The portal collapses above ~20 concurrent requests
//! (docs/06-data-pipeline.md §2), so the semaphore caps in-flight requests
//! worldwide while the bucket spaces requests to one host. Waits happen before
//! the permit is taken: a sleeping request holds no concurrency slot.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, sleep, sleep_until};

/// The shared politeness state of every fetch.
#[derive(Debug)]
pub struct Throttle {
    permits: std::sync::Arc<Semaphore>,
    slots: Mutex<HashMap<String, Instant>>,
    interval: Duration,
    jitter: Duration,
}

impl Throttle {
    /// `concurrency` caps in-flight requests, `interval` spaces requests to one
    /// host, and `jitter` randomizes the pre-send delay.
    pub fn new(concurrency: usize, interval: Duration, jitter: Duration) -> Self {
        Self {
            permits: std::sync::Arc::new(Semaphore::new(concurrency)),
            slots: Mutex::new(HashMap::new()),
            interval,
            jitter,
        }
    }

    /// Waits out the jitter and the host's token slot, then holds a global
    /// permit until the returned guard is dropped.
    pub async fn hold(self: &std::sync::Arc<Self>, host: &str) -> Hold {
        sleep(random_delay(self.jitter)).await;
        let slot = self.reserve(host);
        sleep_until(slot).await;
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("the semaphore is never closed");
        Hold { _permit: permit }
    }

    /// Reserves this host's next slot and returns the instant the caller may
    /// wait for; every reservation pushes the following one one interval out.
    fn reserve(&self, host: &str) -> Instant {
        let now = Instant::now();
        let mut slots = self.slots.lock().expect("slots");
        let slot = slots
            .get(host)
            .map_or(now, |reserved| std::cmp::max(*reserved, now));
        slots.insert(host.to_owned(), slot + self.interval);
        slot
    }
}

/// A global concurrency permit held across one transport attempt.
#[derive(Debug)]
pub struct Hold {
    _permit: OwnedSemaphorePermit,
}

/// A random delay in `[0, bound]`, the politeness jitter of both the throttle
/// and the retry backoff.
pub(crate) fn random_delay(bound: Duration) -> Duration {
    if bound.is_zero() {
        return Duration::ZERO;
    }
    let millis = bound.as_millis().min(u64::MAX as u128) as u64;
    Duration::from_millis(rand::random_range(0..=millis))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[tokio::test(start_paused = true)]
    async fn requests_to_one_host_are_spaced_by_the_interval() {
        let throttle = Arc::new(Throttle::new(4, Duration::from_millis(100), Duration::ZERO));
        let start = Instant::now();
        let first = throttle.hold("portale.fitet.org").await;
        let first_at = Instant::now();
        drop(first);
        assert!(first_at - start < Duration::from_millis(100));

        let second = throttle.hold("portale.fitet.org").await;
        let second_at = Instant::now();
        drop(second);
        assert!(second_at - first_at >= Duration::from_millis(100));
    }

    #[tokio::test(start_paused = true)]
    async fn different_hosts_have_independent_buckets() {
        let throttle = Arc::new(Throttle::new(4, Duration::from_millis(100), Duration::ZERO));
        throttle.hold("portale.fitet.org").await;
        let start = Instant::now();
        let other = throttle.hold("www.fitet.org").await;
        let waited = Instant::now() - start;
        drop(other);
        assert!(waited < Duration::from_millis(100), "waited {waited:?}");
    }

    #[tokio::test(start_paused = true)]
    async fn jitter_stays_within_its_bound() {
        let throttle = Arc::new(Throttle::new(4, Duration::ZERO, Duration::from_millis(50)));
        let start = Instant::now();
        let hold = throttle.hold("portale.fitet.org").await;
        let waited = Instant::now() - start;
        drop(hold);
        assert!(waited <= Duration::from_millis(50), "waited {waited:?}");
    }

    /// Tracks in-flight work and its peak, shared by the spawned tasks.
    #[derive(Clone, Default)]
    struct Tracker {
        in_flight: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
    }

    impl Tracker {
        fn occupy(&self) {
            let active = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
        }

        fn release(&self) {
            self.in_flight.fetch_sub(1, Ordering::SeqCst);
        }

        fn peak(&self) -> usize {
            self.peak.load(Ordering::SeqCst)
        }
    }

    async fn hold_and_occupy(throttle: &Arc<Throttle>, tracker: &Tracker) {
        let hold = throttle.hold("portale.fitet.org").await;
        tracker.occupy();
        sleep(Duration::from_millis(20)).await;
        tracker.release();
        drop(hold);
    }

    #[tokio::test(start_paused = true)]
    async fn the_global_semaphore_caps_concurrent_requests() {
        let throttle = Arc::new(Throttle::new(2, Duration::ZERO, Duration::ZERO));
        let tracker = Tracker::default();
        let mut tasks = Vec::new();
        for _ in 0..6 {
            let throttle = Arc::clone(&throttle);
            let tracker = tracker.clone();
            tasks.push(tokio::spawn(async move {
                hold_and_occupy(&throttle, &tracker).await
            }));
        }
        for task in tasks {
            task.await.expect("task");
        }
        assert_eq!(tracker.peak(), 2);
    }
}
