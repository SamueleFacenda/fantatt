//! Integration suite: the real reqwest transport against the loopback mock
//! portal, asserting the politeness contract of docs/06-data-pipeline.md §3.3 —
//! the concurrency cap, retries, the circuit breaker, the User-Agent, the
//! redirect policy, and conditional requests.

mod common;

use std::net::IpAddr;
use std::sync::Arc;

use fantatt_ingest::archive::{Archive, FilesystemArchive, content_key};
use fantatt_ingest::{FetchClient, FetchError, FetchSettings, Fetched};

use common::{Counters, MockPortal};

/// Settings aimed at the mock: tight politeness so the suite stays fast.
fn settings(base_url: &str) -> FetchSettings {
    FetchSettings {
        concurrency: 4,
        min_interval_ms: 10,
        jitter_ms: 0,
        max_attempts: 4,
        backoff_base_ms: 5,
        backoff_cap_ms: 10,
        breaker_threshold: 10,
        breaker_reset_ms: 60_000,
        timeout_ms: 5_000,
        max_redirects: 5,
        source_base_url: base_url.to_owned(),
        allowed_hosts: vec!["127.0.0.1".to_owned()],
        user_agent: "fantatt-ingest-test/0.1.0 (+https://example.invalid)".to_owned(),
        archive_path: None,
    }
}

async fn portal() -> MockPortal {
    MockPortal::spawn(IpAddr::from([127, 0, 0, 1]), None, Counters::default()).await
}

async fn client_for(base_url: &str) -> Arc<FetchClient> {
    Arc::new(FetchClient::from_settings(settings(base_url)).expect("client"))
}

fn path(base_url: &str, route: &str) -> reqwest::Url {
    format!("{base_url}{route}").parse().expect("url")
}

/// Concurrent fetches of the slow route, to watch the semaphore cap from the
/// server side.
fn spawn_slow_fetches(
    client: &Arc<FetchClient>,
    base_url: &str,
    count: usize,
) -> Vec<tokio::task::JoinHandle<Result<Fetched, FetchError>>> {
    let mut tasks = Vec::new();
    for _ in 0..count {
        let client = Arc::clone(client);
        let url = path(base_url, "/slow");
        tasks.push(tokio::spawn(async move {
            client.fetch(&url, Default::default()).await
        }));
    }
    tasks
}

#[tokio::test]
async fn the_global_semaphore_caps_concurrent_requests() {
    let portal = portal().await;
    let mut settings = settings(&portal.base_url);
    settings.concurrency = 2;
    let client = Arc::new(FetchClient::from_settings(settings).expect("client"));
    let tasks = spawn_slow_fetches(&client, &portal.base_url, 6);
    for task in tasks {
        task.await
            .expect("task")
            .expect("the semaphore serializes, it does not fail");
    }
    assert!(
        portal.counters().peak() <= 2,
        "peak was {}",
        portal.counters().peak()
    );
    assert_eq!(portal.counters().hits("slow"), 6);
}

#[tokio::test]
async fn a_transient_failure_retries_then_succeeds() {
    let portal = portal().await;
    let client = client_for(&portal.base_url).await;
    let url = path(&portal.base_url, "/flaky");
    let fetched = client.fetch(&url, Default::default()).await.expect("fresh");
    assert!(matches!(fetched, Fetched::Fresh(_)));
    assert_eq!(portal.counters().hits("flaky"), 3);
}

#[tokio::test]
async fn the_circuit_opens_after_consecutive_failures() {
    let portal = portal().await;
    let mut settings = settings(&portal.base_url);
    settings.breaker_threshold = 2;
    settings.backoff_base_ms = 0;
    settings.backoff_cap_ms = 0;
    let client = FetchClient::from_settings(settings).expect("client");
    let url = path(&portal.base_url, "/always-503");

    let error = client
        .fetch(&url, Default::default())
        .await
        .expect_err("open");
    assert!(matches!(error, FetchError::CircuitOpen { .. }), "{error}");
    assert_eq!(portal.counters().hits("always-503"), 2);

    let second = client
        .fetch(&url, Default::default())
        .await
        .expect_err("still open");
    assert!(matches!(second, FetchError::CircuitOpen { .. }), "{second}");
    assert_eq!(portal.counters().hits("always-503"), 2);
}

#[tokio::test]
async fn the_user_agent_identifies_the_client() {
    let portal = portal().await;
    let client = client_for(&portal.base_url).await;
    let url = path(&portal.base_url, "/ua");
    let fetched = client.fetch(&url, Default::default()).await.expect("fresh");
    let Fetched::Fresh(page) = fetched else {
        panic!("expected a fresh page");
    };
    let body = String::from_utf8(page.body).expect("utf8 body");
    assert_eq!(body, "fantatt-ingest-test/0.1.0 (+https://example.invalid)");
}

#[tokio::test]
async fn a_cross_host_redirect_is_rejected() {
    let counters = Counters::default();
    let away_portal = MockPortal::spawn(IpAddr::from([127, 0, 0, 2]), None, counters.clone()).await;
    let origin_portal = MockPortal::spawn(
        IpAddr::from([127, 0, 0, 1]),
        Some(away_portal.base_url.clone()),
        counters.clone(),
    )
    .await;
    let client = FetchClient::from_settings(settings(&origin_portal.base_url)).expect("client");
    let url = path(&origin_portal.base_url, "/away");
    let error = client
        .fetch(&url, Default::default())
        .await
        .expect_err("blocked");
    assert!(matches!(error, FetchError::Blocked { .. }), "{error}");
    assert_eq!(counters.hits("away"), 1);
    assert_eq!(counters.hits("landing"), 0);
}

#[tokio::test]
async fn a_same_host_redirect_is_followed() {
    let portal = portal().await;
    let client = client_for(&portal.base_url).await;
    let url = path(&portal.base_url, "/loop");
    let fetched = client
        .fetch(&url, Default::default())
        .await
        .expect("followed");
    let Fetched::Fresh(page) = fetched else {
        panic!("expected a fresh page");
    };
    assert!(
        page.final_url.as_str().ends_with("/ua"),
        "{}",
        page.final_url
    );
    assert_eq!(page.status, 200);
    assert_eq!(portal.counters().hits("loop"), 1);
    assert_eq!(portal.counters().hits("ua"), 1);
}

#[tokio::test]
async fn a_conditional_request_answers_not_modified() {
    let portal = portal().await;
    let client = client_for(&portal.base_url).await;
    let url = path(&portal.base_url, "/etag");
    let first = client.fetch(&url, Default::default()).await.expect("fresh");
    let Fetched::Fresh(page) = first else {
        panic!("expected a fresh page");
    };
    assert_eq!(page.body, b"etag body");
    assert_eq!(page.validators.etag.as_deref(), Some("\"tag-1\""));

    let second = client
        .fetch(&url, page.validators.clone())
        .await
        .expect("not modified");
    assert!(matches!(second, Fetched::NotModified));
    assert_eq!(portal.counters().hits("etag"), 2);
    assert_eq!(portal.counters().conditional(), 1);
}

#[tokio::test]
async fn a_fresh_body_is_archived_under_its_content_key() {
    let portal = portal().await;
    let root = std::env::temp_dir().join(format!("fantatt-ingest-archive-{}", std::process::id()));
    let mut settings = settings(&portal.base_url);
    settings.archive_path = Some(root.clone());
    let client = FetchClient::from_settings(settings).expect("client");
    let url = path(&portal.base_url, "/etag");
    let fetched = client.fetch(&url, Default::default()).await.expect("fresh");
    let Fetched::Fresh(page) = fetched else {
        panic!("expected a fresh page");
    };
    let archive = FilesystemArchive::new(&root);
    let stored = archive
        .load(&content_key(&page.body))
        .await
        .expect("stored");
    assert_eq!(stored, page.body);
    std::fs::remove_dir_all(&root).ok();
}
