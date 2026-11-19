//! Shared harness: a migrated ephemeral Postgres, a built state, and a live
//! server on an ephemeral port. Each test gets its own instance, so limiters
//! and sessions never bleed between tests.
//!
//! why: every test binary includes this module, so helpers another binary uses
//! would otherwise read as dead code here.
#![allow(dead_code)]

use std::net::SocketAddr;

use fantatt_api::routes;
use fantatt_api::state::AppState;
use fantatt_config::{cli::Cli, environment::Environment, loader::load};
use fantatt_test_support::TestDatabase;
use sqlx::PgPool;

pub struct Shell {
    pub base_url: String,
    pub pool: PgPool,
}

pub async fn spawn_shell() -> Shell {
    spawn_with_router(routes::build_router).await
}

/// Boots a shell around a caller-built router, for suites that exercise a
/// route table of their own declarations.
pub async fn spawn_with_router<F>(build: F) -> Shell
where
    F: FnOnce(AppState) -> axum::Router + Send + 'static,
{
    spawn_with_env(&[], build).await
}

/// Boots a shell whose settings carry extra environment pairs on top of the
/// shared test pairs.
pub async fn spawn_with_env<F>(pairs: &[(&str, &str)], build: F) -> Shell
where
    F: FnOnce(AppState) -> axum::Router + Send + 'static,
{
    let database = TestDatabase::new().await.expect("boot");
    let pool = database.pool().clone();
    let state = build_state_with(&database, pairs);
    let (listener, address) = bind_listener().await;
    tokio::spawn(async move {
        let server = axum::serve(
            listener,
            build(state).into_make_service_with_connect_info::<SocketAddr>(),
        );
        let _ = server.await;
        drop(database);
    });
    Shell {
        base_url: format!("http://{address}"),
        pool,
    }
}

async fn bind_listener() -> (tokio::net::TcpListener, SocketAddr) {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let address = listener.local_addr().expect("address");
    (listener, address)
}

pub fn build_state(database: &TestDatabase) -> AppState {
    build_state_with(database, &[])
}

fn build_state_with(database: &TestDatabase, extra: &[(&str, &str)]) -> AppState {
    let mut pairs = vec![
        (
            "FANTATT__DATABASE__URL",
            "postgres://postgres@127.0.0.1:5432/unused",
        ),
        ("FANTATT__CRYPTO__SESSION_SECRET", "test-session-secret"),
    ];
    pairs.extend_from_slice(extra);
    let environment = Environment::from_pairs(pairs);
    let loaded = load(&environment, &Cli::default()).expect("the defaults and test pairs load");
    AppState::build(&loaded, database.pool().clone()).expect("the state builds")
}

/// The `name=value` pair of the session cookie, dropped from its attributes so
/// it can be echoed in a later `Cookie` header.
pub fn session_cookie(response: &reqwest::Response) -> String {
    let value = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("the response sets the session cookie");
    let text = value.to_str().expect("the cookie is ascii");
    text.split(';')
        .next()
        .expect("a cookie carries a pair")
        .to_owned()
}

pub fn cookie_attributes(response: &reqwest::Response) -> String {
    let value = response
        .headers()
        .get(reqwest::header::SET_COOKIE)
        .expect("the response sets the session cookie");
    value.to_str().expect("the cookie is ascii").to_owned()
}

pub fn json_string(body: &serde_json::Value, field: &str) -> String {
    body[field]
        .as_str()
        .expect("the field is a string")
        .to_owned()
}

/// A response header as an owned string, empty when absent.
pub fn header(response: &reqwest::Response, name: &str) -> String {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

/// Issues a CSRF token and returns the session cookie pair with the token.
pub async fn issue_session(base_url: &str) -> (String, String) {
    let issued = reqwest::get(format!("{base_url}/auth/csrf"))
        .await
        .expect("get");
    let cookie = session_cookie(&issued);
    let body: serde_json::Value = issued.json().await.expect("json");
    (cookie, json_string(&body, "token"))
}

/// The value of a `name=value` cookie pair.
pub fn cookie_value(cookie: &str) -> &str {
    cookie.split('=').nth(1).unwrap_or_default()
}
