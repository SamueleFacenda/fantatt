//! Per-host circuit breaker.
//!
//! Consecutive failures of one host open its circuit: further requests fail
//! fast without touching the source, which is what stops a portal outage from
//! turning into a retry storm (docs/06-data-pipeline.md §5). After the reset
//! window the circuit admits one probe; the probe's outcome closes the circuit
//! or opens it again.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::Instant;

/// The shared breaker state of every fetch.
#[derive(Debug)]
pub struct Breaker {
    hosts: Mutex<HashMap<String, HostBreaker>>,
    threshold: u32,
    reset: Duration,
}

#[derive(Debug)]
enum Circuit {
    Closed { failures: u32 },
    Open { since: Instant },
    HalfOpen,
}

#[derive(Debug)]
struct HostBreaker {
    circuit: Circuit,
}

impl Breaker {
    pub fn new(threshold: u32, reset: Duration) -> Self {
        Self {
            hosts: Mutex::new(HashMap::new()),
            threshold,
            reset,
        }
    }

    /// Whether the host may be requested now; an open circuit refuses.
    pub fn acquire(&self, host: &str) -> Result<(), BreakerOpen> {
        let mut hosts = self.hosts.lock().expect("hosts");
        let breaker = hosts.entry(host.to_owned()).or_insert(HostBreaker {
            circuit: Circuit::Closed { failures: 0 },
        });
        match breaker.circuit {
            Circuit::Closed { .. } => Ok(()),
            Circuit::Open { since } if since.elapsed() >= self.reset => {
                breaker.circuit = Circuit::HalfOpen;
                Ok(())
            }
            Circuit::Open { .. } | Circuit::HalfOpen => Err(BreakerOpen {
                host: host.to_owned(),
            }),
        }
    }

    /// A request to the host succeeded; the failure streak resets.
    pub fn record_success(&self, host: &str) {
        self.with(host, |breaker| {
            breaker.circuit = Circuit::Closed { failures: 0 };
        });
    }

    /// A request to the host failed; a streak of `threshold` opens the circuit,
    /// and a failing half-open probe opens it again at once.
    pub fn record_failure(&self, host: &str) {
        let threshold = self.threshold;
        self.with(host, move |breaker| {
            breaker.circuit = match breaker.circuit {
                Circuit::Closed { failures } if failures + 1 < threshold => Circuit::Closed {
                    failures: failures + 1,
                },
                _ => Circuit::Open {
                    since: Instant::now(),
                },
            };
        });
    }

    fn with(&self, host: &str, change: impl FnOnce(&mut HostBreaker)) {
        let mut hosts = self.hosts.lock().expect("hosts");
        let breaker = hosts.entry(host.to_owned()).or_insert(HostBreaker {
            circuit: Circuit::Closed { failures: 0 },
        });
        change(breaker);
    }
}

/// The circuit is open; the host is failing.
#[derive(Debug, thiserror::Error)]
#[error("the circuit breaker for {host} is open; the source keeps failing")]
pub struct BreakerOpen {
    pub host: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker() -> Breaker {
        Breaker::new(2, Duration::from_millis(1_000))
    }

    #[tokio::test(start_paused = true)]
    async fn a_streak_of_failures_opens_the_circuit() {
        let breaker = breaker();
        assert!(breaker.acquire("portale.fitet.org").is_ok());
        breaker.record_failure("portale.fitet.org");
        assert!(breaker.acquire("portale.fitet.org").is_ok());
        breaker.record_failure("portale.fitet.org");
        let open = breaker.acquire("portale.fitet.org").expect_err("open");
        assert!(open.to_string().contains("portale.fitet.org"));
    }

    #[tokio::test(start_paused = true)]
    async fn a_success_resets_the_streak() {
        let breaker = breaker();
        breaker.record_failure("portale.fitet.org");
        breaker.record_success("portale.fitet.org");
        breaker.record_failure("portale.fitet.org");
        assert!(breaker.acquire("portale.fitet.org").is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn a_probe_passes_after_the_reset_window() {
        let breaker = breaker();
        breaker.record_failure("portale.fitet.org");
        breaker.record_failure("portale.fitet.org");
        assert!(breaker.acquire("portale.fitet.org").is_err());
        tokio::time::advance(Duration::from_millis(1_001)).await;
        assert!(breaker.acquire("portale.fitet.org").is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn a_failing_probe_opens_the_circuit_again() {
        let breaker = breaker();
        breaker.record_failure("portale.fitet.org");
        breaker.record_failure("portale.fitet.org");
        tokio::time::advance(Duration::from_millis(1_001)).await;
        assert!(breaker.acquire("portale.fitet.org").is_ok());
        breaker.record_failure("portale.fitet.org");
        assert!(breaker.acquire("portale.fitet.org").is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn hosts_break_independently() {
        let breaker = breaker();
        breaker.record_failure("portale.fitet.org");
        breaker.record_failure("portale.fitet.org");
        assert!(breaker.acquire("portale.fitet.org").is_err());
        assert!(breaker.acquire("www.fitet.org").is_ok());
    }
}
