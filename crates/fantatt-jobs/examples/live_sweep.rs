//! A live discovery sweep against the portal: the smoke check for G1.3.
//!
//! Walks `portale.fitet.org` through the polite fetch client, records every
//! page it fetches in the dev database named by `DATABASE_URL`, and queues
//! the work list. The source serves the cheap topology only — the menu, the
//! framesets, the menus, the indexes and lists — because the full expansion
//! is tens of thousands of pages that the scheduler walks incrementally, and
//! a smoke run stays a smoke run. Assertions stay structural, because live
//! content moves, while the golden expectations live against the recorded
//! fixtures.

use std::time::Instant;

use async_trait::async_trait;
use fantatt_ingest::conditional::CacheValidators;
use fantatt_ingest::discover::{ClientSource, PageReply, PageSource, SOURCE, Surface};
use fantatt_ingest::uri::Uri;
use fantatt_ingest::{FetchClient, FetchError};
use fantatt_jobs::config::Runtime;
use fantatt_jobs::scheduler;
use fantatt_jobs::store::Store;

/// The base URL the smoke walks: the documented default, or the plain-HTTP
/// override the recipe points at. why: the portal's edge resets non-browser
/// TLS clients with a connection reset (os error 104) before any HTTP
/// exchange while the same client reads the plain-HTTP origin fine, so `just
/// discover-smoke` walks over http:// until the edge accepts the rustls client.
fn smoke_base_url() -> String {
    std::env::var("FANTATT_SMOKE_BASE_URL")
        .unwrap_or_else(|_| "https://portale.fitet.org/".to_owned())
}

/// The documented `[ingest]` defaults (docs/12-configuration.md §2) over the
/// given base URL.
fn ingest(base: String) -> fantatt_config::settings::ingest::Ingest {
    fantatt_config::settings::ingest::Ingest {
        enabled: true,
        concurrency: 4,
        min_request_interval_ms: 750,
        source_base_url: base,
        regions: vec!["Trentino".to_owned()],
        discovery_interval_minutes: 30,
        matchday_interval_minutes: 5,
        classifica_interval_hours: 24,
        respect_robots: true,
    }
}

/// The live source bounded to the cheap topology.
struct TopologySource<'a> {
    inner: ClientSource<'a>,
}

#[async_trait]
impl PageSource for TopologySource<'_> {
    async fn page(&self, url: &Uri, validators: &CacheValidators) -> Result<PageReply, FetchError> {
        let topological = Surface::classify(url).is_some_and(Surface::is_topology);
        if topological {
            self.inner.page(url, validators).await
        } else {
            Ok(PageReply::Missing)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = smoke_base_url();
    let allowed = Uri::parse(&base)?.as_url();
    let store = connect().await?;
    let runtime = Runtime::from_config(&ingest(base))?;
    let client = FetchClient::from_settings(runtime.settings.clone())?;
    let source = TopologySource {
        inner: ClientSource::new(&client),
    };
    let first = sweep_once(&store, &runtime, &source).await?;
    let second = sweep_once(&store, &runtime, &source).await?;
    assert!(
        second.walked <= first.walked,
        "the ledger cache did not prune the second sweep"
    );
    check_host(&store, &allowed).await?;
    Ok(())
}

/// The walk stayed on the configured portal origin, reporting the ledger size.
async fn check_host(store: &Store, origin: &str) -> Result<(), sqlx::Error> {
    let ledger = store.ledger(SOURCE).await?;
    assert!(
        ledger.urls().iter().all(|url| url.starts_with(origin)),
        "the walk followed a host outside the allow-list"
    );
    println!("ledger holds {} portal urls", ledger.urls().len());
    Ok(())
}

async fn sweep_once(
    store: &Store,
    runtime: &Runtime,
    source: &TopologySource<'_>,
) -> Result<fantatt_jobs::sweep::SweepReport, fantatt_jobs::sweep::SweepError> {
    let started = Instant::now();
    let report = scheduler::run_sweep(store, runtime, source).await?;
    println!("sweep: {report:?} in {:?}", started.elapsed());
    assert!(report.walked > 0, "the sweep walked no page");
    Ok(report)
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
