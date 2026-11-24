//! The database side of the pipeline: the ingest ledger that diffing reads
//! and the job queue the worker drains (docs/06-data-pipeline.md §3.2, §4).
//!
//! The ledger keeps one row per (source, url, content hash): a new hash
//! appends history while a known hash refreshes its fetched_at, so the
//! cache window never expires for a page that keeps answering 304. The queue
//! dedupes by the partial unique index on the active states, so a sweep that
//! re-plans an unchanged page inserts nothing.

use sqlx::PgPool;
use time::OffsetDateTime;

use fantatt_ingest::conditional::CacheValidators;
use fantatt_ingest::discover::{Ledger, LedgerEntry, LedgerState, SOURCE};

use crate::ids::ulid;

/// One row the snapshot reads for a URL: the newest fetch the ledger holds.
#[derive(Debug)]
struct LedgerRow {
    source_id: String,
    content_hash: String,
    etag: Option<String>,
    last_modified: Option<OffsetDateTime>,
    state: String,
    fetched_at: OffsetDateTime,
}

impl LedgerRow {
    fn entry(self) -> LedgerEntry {
        LedgerEntry {
            content_hash: self.content_hash,
            validators: CacheValidators::stored(self.etag, self.last_modified),
            state: ledger_state(&self.state),
            fetched_at: self.fetched_at.into(),
        }
    }
}

fn ledger_state(state: &str) -> LedgerState {
    match state {
        "SKIPPED" => LedgerState::Skipped,
        "INCOMPLETE" => LedgerState::Incomplete,
        _ => LedgerState::Fetched,
    }
}

/// One job waiting to be queued.
pub struct NewJob {
    pub job_type: String,
    pub source_id: String,
    pub payload: serde_json::Value,
}

/// One job a worker claimed.
pub struct ClaimedJob {
    pub job_id: String,
    pub job_type: String,
    pub source_id: Option<String>,
    pub payload: serde_json::Value,
}

/// How a claimed job ended.
pub enum Outcome {
    Succeeded,
    Skipped,
    Failed(String),
}

impl Outcome {
    fn state(&self) -> &'static str {
        match self {
            Self::Succeeded => "SUCCEEDED",
            Self::Skipped => "SKIPPED",
            Self::Failed(_) => "FAILED",
        }
    }

    fn error(&self) -> Option<String> {
        match self {
            Self::Failed(reason) => Some(reason.clone()),
            _ => None,
        }
    }
}

/// One page the ledger is asked to remember.
pub struct Record<'a> {
    pub source: &'a str,
    pub source_id: &'a str,
    pub content_hash: &'a str,
    pub validators: &'a CacheValidators,
    pub state: &'a str,
}

/// The ledger and the queue over one pool.
pub struct Store {
    pool: PgPool,
}

impl Store {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The pool the ledger and the queue live on.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// The newest ledger row per URL, as the diff snapshot.
    pub async fn ledger(&self, source: &str) -> Result<Ledger, sqlx::Error> {
        let rows = sqlx::query_as!(
            LedgerRow,
            "SELECT DISTINCT ON (source_id)
                source_id, content_hash, etag, last_modified, state, fetched_at
             FROM fantatt.ingest_ledger
             WHERE source = $1
             ORDER BY source_id, fetched_at DESC",
            source
        )
        .fetch_all(&self.pool)
        .await?;
        let mut ledger = Ledger::new();
        for row in rows {
            let url = fantatt_ingest::uri::Uri::parse(&row.source_id)
                .map_err(|_| sqlx::Error::Protocol("a stored source_id is a URL".to_owned()))?;
            ledger.insert(&url, row.entry());
        }
        Ok(ledger)
    }

    /// Records one fetched page: a new content hash appends history, a known
    /// hash refreshes its fetched_at.
    pub async fn record(&self, row: Record<'_>) -> Result<(), sqlx::Error> {
        let modified = row.validators.modified_at();
        sqlx::query!(
            "INSERT INTO fantatt.ingest_ledger
            (ledger_id, source, source_id, content_hash, etag, last_modified, state)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         ON CONFLICT (source, source_id, content_hash)
         DO UPDATE SET fetched_at = now()",
            ulid(),
            row.source,
            row.source_id,
            row.content_hash,
            row.validators.etag,
            modified,
            row.state
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Queues one job; a PENDING or RUNNING row with the same key dedupes it.
    pub async fn enqueue(&self, job: &NewJob) -> Result<bool, sqlx::Error> {
        let inserted = sqlx::query!(
            "INSERT INTO fantatt.ingest_jobs (job_id, job_type, source, source_id, payload)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (job_type, source, source_id)
             WHERE state IN ('PENDING', 'RUNNING')
             DO NOTHING",
            ulid(),
            job.job_type,
            SOURCE,
            job.source_id,
            job.payload
        )
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(inserted > 0)
    }

    /// Claims the oldest PENDING job, marking it RUNNING.
    pub async fn claim(&self) -> Result<Option<ClaimedJob>, sqlx::Error> {
        sqlx::query_as!(
            ClaimedJob,
            "UPDATE fantatt.ingest_jobs
             SET state = 'RUNNING', started_at = now(), attempts = attempts + 1
             WHERE job_id = (
                 SELECT job_id FROM fantatt.ingest_jobs
                 WHERE state = 'PENDING' AND (next_run_at IS NULL OR next_run_at <= now())
                 ORDER BY scheduled_at
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1
             )
             RETURNING job_id, job_type, source_id, payload"
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Records the end of a claimed job.
    pub async fn finish(&self, job: &ClaimedJob, outcome: &Outcome) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE fantatt.ingest_jobs
             SET state = $2, finished_at = now(), last_error = $3
             WHERE job_id = $1",
            job.job_id,
            outcome.state(),
            outcome.error()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Wakes a listening worker without writing a row.
    pub async fn notify(&self, channel: &str) -> Result<(), sqlx::Error> {
        sqlx::query!("SELECT pg_notify($1, '')", channel)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// The validators the newest row of one URL holds.
    pub async fn validators(
        &self,
        source: &str,
        source_id: &str,
    ) -> Result<Option<CacheValidators>, sqlx::Error> {
        let row = sqlx::query!(
            "SELECT etag, last_modified
             FROM fantatt.ingest_ledger
             WHERE source = $1 AND source_id = $2
             ORDER BY fetched_at DESC
             LIMIT 1",
            source,
            source_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|row| CacheValidators::stored(row.etag, row.last_modified)))
    }

    /// Refreshes the cache window of every row one URL holds, the way a 304
    /// answer leaves the ledger untouched but the page re-verified.
    pub async fn touch(&self, source: &str, source_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE fantatt.ingest_ledger
             SET fetched_at = now()
             WHERE source = $1 AND source_id = $2",
            source,
            source_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
