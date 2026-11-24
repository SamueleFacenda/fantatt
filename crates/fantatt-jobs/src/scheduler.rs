//! The scheduler skeleton: the cron expressions the cadences compile to, the
//! sweep loop that re-plans discovery, and the worker loop that drains the
//! queue. Job state lives in Postgres, so a restart re-reads the queue and
//! nothing is lost (docs/06-data-pipeline.md §4).
//!
//! The loop bodies are the process entry points; the pieces they call — one
//! sweep, one drain — are the tested units in `sweep` and `worker`.

use std::time::Duration;

use crate::config::Runtime;
use crate::store::Store;
use crate::sweep;
use crate::worker;

/// The channel a sweep notifies a listening worker on.
pub const CHANNEL: &str = "fantatt_jobs";

/// The poll interval that backs the notify wake-up.
pub const POLL: Duration = Duration::from_secs(30);

/// A cron expression firing every `minutes`, on the minute.
/// why: tokio-cron-scheduler speaks the six-field cron of Spring; the sweep
/// does not need second precision.
pub fn every_minutes(minutes: u32) -> String {
    let minutes = minutes.max(1);
    if minutes.is_multiple_of(60) {
        let hours = minutes / 60;
        return format!("0 0 */{hours} * * *");
    }
    if minutes < 60 {
        return format!("0 */{minutes} * * * *");
    }
    let hours = minutes / 60;
    let rest = minutes % 60;
    format!("0 {rest} */{hours} * * *")
}

/// Runs one sweep through one page source; the loop wraps it on the cadence.
pub async fn run_sweep(
    store: &Store,
    runtime: &Runtime,
    pages: &dyn fantatt_ingest::discover::PageSource,
) -> Result<sweep::SweepReport, sweep::SweepError> {
    let plan = sweep::Sweep {
        seed: &runtime.seed,
        ttl: runtime.ttl,
    };
    sweep::run(store, pages, &plan).await
}

/// Drains the queue until it is empty; the loop wraps it behind the notify
/// wake-up.
pub async fn drain(
    store: &Store,
    pages: &dyn fantatt_ingest::discover::PageSource,
) -> Result<usize, worker::WorkerError> {
    let mut drained = 0;
    while worker::drain_once(store, pages).await? {
        drained += 1;
    }
    Ok(drained)
}

/// The cron expression one scheduler process runs discovery on.
pub fn discovery_cron(runtime: &Runtime) -> String {
    every_minutes((runtime.cadence.as_secs() / 60) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(minutes: u32) -> Runtime {
        let config = fantatt_config::settings::ingest::Ingest {
            enabled: true,
            concurrency: 1,
            min_request_interval_ms: 750,
            source_base_url: "https://portale.fitet.org/".to_owned(),
            regions: vec!["Valle d'Aosta".to_owned()],
            discovery_interval_minutes: minutes,
            matchday_interval_minutes: 5,
            classifica_interval_hours: 24,
            respect_robots: true,
        };
        Runtime::from_config(&config).expect("runtime")
    }

    #[test]
    fn a_sweep_cadence_compiles_to_a_cron_expression() {
        assert_eq!(every_minutes(30), "0 */30 * * * *");
        assert_eq!(every_minutes(90), "0 30 */1 * * *");
        assert_eq!(every_minutes(120), "0 0 */2 * * *");
        assert_eq!(every_minutes(0), "0 */1 * * * *");
    }

    #[test]
    fn the_discovery_cron_follows_the_config() {
        assert_eq!(discovery_cron(&runtime(30)), "0 */30 * * * *");
    }
}
