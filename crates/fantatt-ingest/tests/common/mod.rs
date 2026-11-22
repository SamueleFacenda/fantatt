//! A loopback mock of the FITET portal.
//!
//! The integration suite drives the real reqwest transport against this mock:
//! it records in-flight concurrency, per-route hit counts, and conditional
//! requests, and it script-fails, redirects, and tags pages on demand. Every
//! listener binds a loopback address, so the suite needs no network.
#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect};
use tokio::net::TcpListener;

/// The observed behaviour of one or more mock portals.
#[derive(Clone, Default)]
pub struct Counters {
    in_flight: Arc<AtomicUsize>,
    peak: Arc<AtomicUsize>,
    hits: Arc<Mutex<HashMap<String, usize>>>,
    flaky: Arc<AtomicUsize>,
    conditional: Arc<AtomicUsize>,
}

impl Counters {
    /// Registers one in-flight request and returns the guard that releases it.
    pub fn enter(&self) -> InFlight {
        let active = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        InFlight {
            in_flight: Arc::clone(&self.in_flight),
        }
    }

    pub fn record(&self, route: &str) {
        let mut hits = self.hits.lock().expect("hits");
        *hits.entry(route.to_owned()).or_insert(0) += 1;
    }

    pub fn hits(&self, route: &str) -> usize {
        self.hits
            .lock()
            .expect("hits")
            .get(route)
            .copied()
            .unwrap_or(0)
    }

    pub fn peak(&self) -> usize {
        self.peak.load(Ordering::SeqCst)
    }

    pub fn conditional(&self) -> usize {
        self.conditional.load(Ordering::SeqCst)
    }

    fn flaky_step(&self) -> usize {
        self.flaky.fetch_add(1, Ordering::SeqCst)
    }
}

/// Held while one request is in flight at the mock.
pub struct InFlight {
    in_flight: Arc<AtomicUsize>,
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
    }
}

/// A running mock portal.
pub struct MockPortal {
    pub base_url: String,
    counters: Counters,
    _server: tokio::task::JoinHandle<()>,
}

impl MockPortal {
    pub fn counters(&self) -> &Counters {
        &self.counters
    }

    /// Binds a portal on a loopback address; `/away` redirects to the given
    /// base when one is supplied.
    pub async fn spawn(host: IpAddr, away_base: Option<String>, counters: Counters) -> Self {
        let listener = TcpListener::bind(SocketAddr::from((host, 0)))
            .await
            .expect("bind a loopback port");
        let address = listener.local_addr().expect("local address");
        let state = PortalState {
            counters: counters.clone(),
            away_base,
        };
        Self {
            base_url: format!("http://{address}"),
            counters,
            _server: serve(listener, portal_routes(state)),
        }
    }
}

#[derive(Clone)]
struct PortalState {
    counters: Counters,
    away_base: Option<String>,
}

/// Serves the mock on a loopback listener for the lifetime of the suite.
fn serve(listener: TcpListener, app: Router) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("serve the mock portal");
    })
}

/// The routes of the mock portal and the handlers behind them.
fn portal_routes(state: PortalState) -> Router {
    Router::new()
        .route("/slow", axum::routing::get(slow))
        .route("/flaky", axum::routing::get(flaky))
        .route("/always-503", axum::routing::get(always_503))
        .route("/etag", axum::routing::get(etag))
        .route("/ua", axum::routing::get(ua))
        .route("/loop", axum::routing::get(loop_target))
        .route("/away", axum::routing::get(away))
        .route("/landing", axum::routing::get(landing))
        .with_state(state)
}

async fn slow(State(state): State<PortalState>) -> impl IntoResponse {
    state.counters.record("slow");
    let _in_flight = state.counters.enter();
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    "slow body"
}

/// The first two requests fail with 503; the third succeeds.
async fn flaky(State(state): State<PortalState>) -> impl IntoResponse {
    state.counters.record("flaky");
    match state.counters.flaky_step() {
        0 | 1 => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        _ => "flaky body".into_response(),
    }
}

async fn always_503(State(state): State<PortalState>) -> impl IntoResponse {
    state.counters.record("always-503");
    StatusCode::SERVICE_UNAVAILABLE.into_response()
}

async fn etag(State(state): State<PortalState>, headers: HeaderMap) -> impl IntoResponse {
    state.counters.record("etag");
    if headers
        .get("if-none-match")
        .and_then(|value| value.to_str().ok())
        == Some("\"tag-1\"")
    {
        state.counters.conditional.fetch_add(1, Ordering::SeqCst);
        return StatusCode::NOT_MODIFIED.into_response();
    }
    ([(header::ETAG, "\"tag-1\"")], "etag body").into_response()
}

async fn ua(State(state): State<PortalState>, headers: HeaderMap) -> String {
    state.counters.record("ua");
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("none")
        .to_owned()
}

async fn loop_target(State(state): State<PortalState>) -> impl IntoResponse {
    state.counters.record("loop");
    Redirect::to("/ua")
}

async fn away(State(state): State<PortalState>) -> impl IntoResponse {
    state.counters.record("away");
    let away = state.away_base.clone().unwrap_or_default();
    Redirect::temporary(&format!("{away}/landing"))
}

async fn landing(State(state): State<PortalState>) -> &'static str {
    state.counters.record("landing");
    "the other origin"
}
