//! A live discovery sweep against the portal: the smoke check for G1.3.
//!
//! Walks `portale.fitet.org` through the polite fetch client, records every
//! page it fetches in the dev database named by `DATABASE_URL`, and queues
//! the work list. Assertions stay structural — the walk shape, the host
//! allow-list, the second sweep pruning its cache — because live content
//! moves, while the golden expectations live against the recorded fixtures.

use std::time::Instant;

use fantatt_ingest::{FetchClient, discover::ClientSource, discover::SOURCE};
use fantatt_jobs::config::Runtime;
use fantatt_jobs::scheduler;
use fantatt_jobs::store::Store;

/// The documented `[ingest]` defaults (docs/12-configuration.md §2).
fn ingest() -> fantatt_config::settings::ingest::Ingest {
    fantatt_config::settings::ingest::Ingest {
        enabled: true,
        concurrency: 4,
        min_request_interval_ms: 750,
        source_base_url: "https://portale.fitet.org/".to_owned(),
        regions: vec!["Trentino".to_owned()],
        discovery_interval_minutes: 30,
        matchday_interval_minutes: 5,
        classifica_interval_hours: 24,
        respect_robots: true,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = connect().await?;
    let runtime = Runtime::from_config(&ingest())?;
    let client = FetchClient::from_settings(runtime.settings.clone())?;
    let source = ClientSource::new(&client);
    let first = sweep_once(&store, &runtime, &source).await?;
    let second = sweep_once(&store, &runtime, &source).await?;
    assert!(
        second.walked <= first.walked,
        "the ledger cache did not prune the second sweep"
    );
    let ledger = store.ledger(SOURCE).await?;
    let allowed = "https://portale.fitet.org/";
    assert!(
        ledger.urls().iter().all(|url| url.starts_with(allowed)),
        "the walk followed a host outside the allow-list"
    );
    println!("ledger holds {} portal urls", ledger.urls().len());
    Ok(())
}

/// The dev database the `discover-smoke` recipe migrated.
async fn connect() -> Result<Store, sqlx::Error> {
    let database = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres@127.0.0.1:5433/fantatt_dev".to_owned());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(&database)
        .await?;
    Ok(Store::new(pool))
}

async fn sweep_once(
    store: &Store,
    runtime: &Runtime,
    source: &ClientSource<'_>,
) -> Result<fantatt_jobs::sweep::SweepReport, fantatt_jobs::sweep::SweepError> {
    let started = Instant::now();
    let report = scheduler::run_sweep(store, runtime, source).await?;
    println!("sweep: {report:?} in {:?}", started.elapsed());
    assert!(report.walked > 0, "the sweep walked no page");
    assert!(report.jobs > 0, "the sweep planned no job");
    Ok(report)
}
