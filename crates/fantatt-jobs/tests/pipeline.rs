//! Database-backed pipeline tests: the sweep walks the recorded portal tree
//! into the ledger and the queue, the queue dedupes what a re-sweep re-plans,
//! and the worker drains a job and records its page (goal G1.3).

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use fantatt_fixtures::FixtureSet;
use fantatt_ingest::discover::{DEFAULT_TTL, Page, PageReply, PageSource, Walk};
use fantatt_ingest::uri::Uri;
use fantatt_jobs::config::Runtime;
use fantatt_jobs::scheduler;
use fantatt_jobs::store::Outcome;
use fantatt_jobs::store::Record;
use fantatt_jobs::{NewJob, Store, sweep, worker};
use fantatt_test_support::TestDatabase;

/// The recorded tree as a page source: every URL the capture index names, and
/// nothing else.
struct RecordedTree(BTreeMap<String, Vec<u8>>);

impl RecordedTree {
    fn load() -> Self {
        let set = FixtureSet::discover().expect("fixture tree");
        let mut pages = BTreeMap::new();
        for surface in set.html_surfaces().expect("surfaces") {
            let bytes = std::fs::read(set.file_path(&surface.file)).expect("page");
            let key = Uri::parse(&surface.url).expect("recorded url");
            pages.insert(key.as_url(), bytes);
        }
        Self(pages)
    }
}

#[async_trait]
impl PageSource for RecordedTree {
    async fn page(
        &self,
        url: &Uri,
        _validators: &fantatt_ingest::conditional::CacheValidators,
    ) -> Result<PageReply, fantatt_ingest::FetchError> {
        Ok(match self.0.get(&url.as_url()) {
            Some(bytes) => PageReply::Fresh(Page {
                bytes: bytes.clone(),
                validators: fantatt_ingest::conditional::CacheValidators::default(),
            }),
            None => PageReply::Missing,
        })
    }
}

fn runtime() -> Runtime {
    Runtime::from_config(&ingest()).expect("runtime")
}

fn ingest() -> fantatt_config::settings::ingest::Ingest {
    fantatt_config::settings::ingest::Ingest {
        enabled: true,
        concurrency: 2,
        min_request_interval_ms: 750,
        source_base_url: "https://portale.fitet.org/".to_owned(),
        regions: vec!["Valle d'Aosta".to_owned()],
        discovery_interval_minutes: 30,
        matchday_interval_minutes: 5,
        classifica_interval_hours: 24,
        respect_robots: true,
    }
}

async fn store() -> (TestDatabase, Store) {
    let database = TestDatabase::new().await.expect("database");
    let store = Store::new(database.pool().clone());
    (database, store)
}

async fn sweep(store: &Store, tree: &RecordedTree) -> sweep::SweepReport {
    scheduler::run_sweep(store, &runtime(), tree)
        .await
        .expect("sweep")
}

async fn ledger_rows(store: &Store) -> i64 {
    sqlx::query_scalar!("SELECT count(*) FROM fantatt.ingest_ledger WHERE source = 'fitet'")
        .fetch_one(store.pool())
        .await
        .expect("count")
        .expect("count")
}

async fn job_rows(store: &Store) -> i64 {
    sqlx::query_scalar!("SELECT count(*) FROM fantatt.ingest_jobs WHERE source = 'fitet'")
        .fetch_one(store.pool())
        .await
        .expect("count")
        .expect("count")
}

async fn jobs_in(store: &Store, state: &str) -> i64 {
    sqlx::query_scalar!(
        "SELECT count(*) FROM fantatt.ingest_jobs WHERE source = 'fitet' AND state = $1",
        state
    )
    .fetch_one(store.pool())
    .await
    .expect("count")
    .expect("count")
}

#[tokio::test]
async fn a_sweep_records_the_walk_and_queues_the_work_list() {
    let (_database, store) = store().await;
    let tree = RecordedTree::load();
    let report = sweep(&store, &tree).await;

    // why: the tournament indexes are walked now, so one frameset, its
    // header and its list join the tenth page the G1.3 chain walked, and the
    // four classifica headers the menu links join the jobs
    assert_eq!(report.walked, 13, "{report:?}");
    assert_eq!(report.jobs, 6, "{report:?}");
    assert_eq!(report.missing, 113, "{report:?}");
    assert_eq!(ledger_rows(&store).await, 13);
    assert_eq!(job_rows(&store).await, 6);
}

#[tokio::test]
async fn a_second_sweep_records_nothing_new_and_queues_nothing_new() {
    let (_database, store) = store().await;
    let tree = RecordedTree::load();
    let first = sweep(&store, &tree).await;
    let second = sweep(&store, &tree).await;

    assert_eq!(first.jobs, 6);
    assert_eq!(second.walked, 6, "{second:?}");
    assert_eq!(second.jobs, 0, "{second:?}");
    assert_eq!(ledger_rows(&store).await, 13);
    assert_eq!(job_rows(&store).await, 6);
}

#[tokio::test]
async fn the_worker_drains_the_whole_work_list() {
    let (_database, store) = store().await;
    let tree = RecordedTree::load();
    sweep(&store, &tree).await;

    let drained = scheduler::drain(&store, &tree).await.expect("drain");
    assert_eq!(jobs_in(&store, "PENDING").await, 0);
    // why: the recorded leaves fetched — the classifica header, the giornata,
    // and the match detail — while the three unrecorded classifica headers skip
    assert_eq!(jobs_in(&store, "SUCCEEDED").await, 3);
    assert_eq!(jobs_in(&store, "SKIPPED").await, 3);
    assert_eq!(drained, 6);
    assert_eq!(ledger_rows(&store).await, 16);
}

#[tokio::test]
async fn enqueue_dedupes_the_active_key_and_a_finished_job_frees_it() {
    let (_database, store) = store().await;
    let job = NewJob {
        job_type: "fetch-giornata".to_owned(),
        source_id: "https://portale.fitet.org/x.php".to_owned(),
        payload: serde_json::json!({"url": "https://portale.fitet.org/x.php"}),
    };
    assert!(store.enqueue(&job).await.expect("first"));
    assert!(!store.enqueue(&job).await.expect("second"));
    assert_eq!(job_rows(&store).await, 1);

    let claimed = store.claim().await.expect("claim").expect("one job");
    store
        .finish(&claimed, &Outcome::Succeeded)
        .await
        .expect("finish");
    // a finished row no longer occupies the active key
    assert!(store.enqueue(&job).await.expect("third"));
    assert_eq!(job_rows(&store).await, 2);
}

#[tokio::test]
async fn a_missing_page_skips_its_job() {
    let (_database, store) = store().await;
    let tree = RecordedTree::load();
    let job = NewJob {
        job_type: "fetch-giornata".to_owned(),
        source_id: "https://portale.fitet.org/never-recorded.php".to_owned(),
        payload: serde_json::json!({"url": "https://portale.fitet.org/never-recorded.php"}),
    };
    store.enqueue(&job).await.expect("queue");
    assert!(worker::drain_once(&store, &tree).await.expect("drain"));
    let row = sqlx::query!(
        "SELECT state, last_error FROM fantatt.ingest_jobs WHERE source_id = $1",
        job.source_id
    )
    .fetch_one(store.pool())
    .await
    .expect("row");
    assert_eq!(row.state, "SKIPPED");
}

#[tokio::test]
async fn a_claim_marks_the_job_running_and_counts_the_attempt() {
    let (_database, store) = store().await;
    let job = NewJob {
        job_type: "fetch-giornata".to_owned(),
        source_id: "https://portale.fitet.org/y.php".to_owned(),
        payload: serde_json::json!({"url": "https://portale.fitet.org/y.php"}),
    };
    store.enqueue(&job).await.expect("queue");
    let claimed = store.claim().await.expect("claim").expect("one job");
    assert_eq!(claimed.job_type, "fetch-giornata");
    let row = sqlx::query!(
        "SELECT state, attempts FROM fantatt.ingest_jobs WHERE job_id = $1",
        claimed.job_id
    )
    .fetch_one(store.pool())
    .await
    .expect("row");
    assert_eq!(row.state, "RUNNING");
    assert_eq!(row.attempts, 1);
    // the queue is empty now
    assert!(store.claim().await.expect("claim").is_none());
}

#[tokio::test]
async fn the_ledger_keeps_the_newest_hash_per_url() {
    let (_database, store) = store().await;
    let source = "fitet";
    let url = "https://portale.fitet.org/z.php";
    let empty = fantatt_ingest::conditional::CacheValidators::default();
    let write = |hash| Record {
        source,
        source_id: url,
        content_hash: hash,
        validators: &empty,
        state: "FETCHED",
    };
    store.record(write("hash-one")).await.expect("one");
    store.record(write("hash-two")).await.expect("two");
    // a known hash refreshes instead of appending
    store.record(write("hash-two")).await.expect("again");

    let ledger = store.ledger(source).await.expect("ledger");
    let entry = ledger.get(&Uri::parse(url).expect("url")).expect("entry");
    assert_eq!(entry.content_hash, "hash-two");
    assert_eq!(ledger_rows(&store).await, 2);
}

#[tokio::test]
async fn the_scheduler_cron_follows_the_config() {
    assert_eq!(scheduler::discovery_cron(&runtime()), "0 */30 * * * *");
    assert_eq!(DEFAULT_TTL, Duration::from_secs(7 * 24 * 3600));
    let _ = |pages: &dyn PageSource, ledger: &fantatt_ingest::discover::Ledger| {
        let _ = Walk {
            pages,
            ledger,
            now: std::time::SystemTime::now(),
        };
    };
}
