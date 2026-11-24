//! One discovery sweep: read the ledger, walk the portal, record what was
//! fetched, and queue the work list. Live runs walk through the polite fetch
//! client; tests walk the recorded fixture tree.

use std::time::Duration;

use fantatt_ingest::discover::{Discovery, PageSource, Plan, Walk};
use fantatt_ingest::uri::Uri;

use crate::store::{NewJob, Record, Store};

/// What one sweep changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepReport {
    pub walked: usize,
    pub jobs: usize,
    pub missing: usize,
}

/// Why a sweep failed: the walk itself, or the store refusing its writes.
#[derive(Debug, thiserror::Error)]
pub enum SweepError {
    #[error("the sweep could not walk the portal: {0}")]
    Walk(#[from] fantatt_ingest::discover::DiscoveryError),

    #[error("the sweep could not write the ledger: {0}")]
    Store(#[from] sqlx::Error),
}

/// Where a sweep walks: its seed and its cache window.
pub struct Sweep<'a> {
    pub seed: &'a Uri,
    pub ttl: Duration,
}

/// Runs one sweep and persists its plan.
pub async fn run(
    store: &Store,
    pages: &dyn PageSource,
    sweep: &Sweep<'_>,
) -> Result<SweepReport, SweepError> {
    let ledger = store.ledger(fantatt_ingest::discover::SOURCE).await?;
    let plan = Discovery::new(sweep.seed.clone())
        .with_ttl(sweep.ttl)
        .plan(Walk {
            pages,
            ledger: &ledger,
            now: std::time::SystemTime::now(),
        })
        .await?;
    record_walked(store, &plan).await?;
    let jobs = queue_jobs(store, &plan).await?;
    store.notify(crate::scheduler::CHANNEL).await.ok();
    Ok(SweepReport {
        walked: plan.walked.len(),
        jobs,
        missing: plan.missing.len(),
    })
}

async fn record_walked(store: &Store, plan: &Plan) -> Result<(), sqlx::Error> {
    for page in &plan.walked {
        store
            .record(Record {
                source: fantatt_ingest::discover::SOURCE,
                source_id: &page.url.as_url(),
                content_hash: &page.content_hash,
                validators: &page.validators,
                state: "FETCHED",
            })
            .await?;
    }
    Ok(())
}

async fn queue_jobs(store: &Store, plan: &Plan) -> Result<usize, sqlx::Error> {
    let mut queued = 0;
    for job in &plan.jobs {
        let payload = job_payload(job);
        let new_job = NewJob {
            job_type: format!("fetch-{}", job.surface.slug()),
            source_id: job.url.as_url(),
            payload,
        };
        if store.enqueue(&new_job).await? {
            queued += 1;
        }
    }
    Ok(queued)
}

/// The payload a parse stage dispatches on: the target URL, its surface, and
/// every query parameter the URL carried.
fn job_payload(job: &fantatt_ingest::discover::Job) -> serde_json::Value {
    let params: serde_json::Map<String, serde_json::Value> = job
        .url
        .params()
        .into_iter()
        .map(|(key, value)| (key, serde_json::Value::String(value)))
        .collect();
    serde_json::json!({
        "url": job.url.as_url(),
        "surface": job.surface.slug(),
        "params": params,
    })
}
