//! The worker: claim one job from the queue, run it, and record the outcome.
//! Every job type is a page fetch through the same source the sweep walked,
//! so a fetch job re-reads its page, archives it behind the client, and
//! refreshes the ledger row (docs/06-data-pipeline.md §3.3).

use crate::store::Record;
use fantatt_ingest::archive::content_key;
use fantatt_ingest::discover::{Page, PageReply, PageSource, SOURCE};
use fantatt_ingest::uri::Uri;

use crate::store::{ClaimedJob, Outcome, Store};

/// Why a claimed job could not run.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("the job could not be read: {0}")]
    Store(#[from] sqlx::Error),

    #[error("the job could not fetch its page: {0}")]
    Fetch(#[from] fantatt_ingest::FetchError),

    #[error("the job carries no target URL")]
    MissingTarget,
}

/// How the page answered.
enum Ran {
    Fetched,
    Unchanged,
    Missing,
}

/// Claims and runs the oldest pending job; false when the queue was empty.
pub async fn drain_once(store: &Store, pages: &dyn PageSource) -> Result<bool, WorkerError> {
    let Some(job) = store.claim().await? else {
        return Ok(false);
    };
    let outcome = run_job(store, pages, &job).await;
    store.finish(&job, &outcome).await?;
    Ok(true)
}

async fn run_job(store: &Store, pages: &dyn PageSource, job: &ClaimedJob) -> Outcome {
    match fetch_page(store, pages, job).await {
        Ok(Ran::Fetched) => Outcome::Succeeded,
        Ok(Ran::Unchanged) => Outcome::Succeeded,
        Ok(Ran::Missing) => Outcome::Skipped,
        Err(WorkerError::MissingTarget) => Outcome::Failed("the job names no page".to_owned()),
        Err(WorkerError::Fetch(error)) => Outcome::Failed(error.to_string()),
        Err(WorkerError::Store(error)) => Outcome::Failed(error.to_string()),
    }
}

async fn fetch_page(
    store: &Store,
    pages: &dyn PageSource,
    job: &ClaimedJob,
) -> Result<Ran, WorkerError> {
    let Some(url) = job_url(job) else {
        return Err(WorkerError::MissingTarget);
    };
    let validators = store
        .validators(SOURCE, &url.as_url())
        .await?
        .unwrap_or_default();
    match pages.page(&url, &validators).await? {
        PageReply::Fresh(page) => {
            record(store, &url, page).await?;
            Ok(Ran::Fetched)
        }
        PageReply::Unchanged => {
            store.touch(SOURCE, &url.as_url()).await?;
            Ok(Ran::Unchanged)
        }
        PageReply::Missing => Ok(Ran::Missing),
    }
}

async fn record(store: &Store, url: &Uri, page: Page) -> Result<(), WorkerError> {
    store
        .record(Record {
            source: SOURCE,
            source_id: &url.as_url(),
            content_hash: &content_key(&page.bytes),
            validators: &page.validators,
            state: "FETCHED",
        })
        .await?;
    Ok(())
}

/// The URL one job's payload names.
fn job_url(job: &ClaimedJob) -> Option<Uri> {
    job.payload
        .get("url")
        .and_then(|value| value.as_str())
        .and_then(|text| Uri::parse(text).ok())
}
