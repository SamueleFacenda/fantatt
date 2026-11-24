//! Scheduler and worker processes.
//!
//! The Postgres-backed queue carries the work list discovery plans: the sweep
//! walks the portal, records every page it fetched in the ingest ledger, and
//! enqueues the fetch jobs the worker drains. Job state lives in the database,
//! so a restart re-reads the queue and nothing is lost (docs/06-data-pipeline.md
//! §4).

pub mod config;
pub mod ids;
pub mod scheduler;
pub mod store;
pub mod sweep;
pub mod worker;

pub use store::{ClaimedJob, NewJob, Outcome, Store};
