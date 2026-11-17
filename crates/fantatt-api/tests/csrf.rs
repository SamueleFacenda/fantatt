//! CSRF double-submit on the mutating route (docs/10-security.md §2).

mod common;

use common::{cookie_value, issue_session, spawn_shell};
use serde_json::Value;

#[tokio::test]
async fn a_mutating_request_without_a_session_or_token_is_refused() {
    let shell = spawn_shell().await;
    let response = reqwest::Client::new()
        .delete(format!("{}/auth/session", shell.base_url))
        .send()
        .await
        .expect("delete");
    assert_eq!(response.status(), 403);
    let body: Value = response.json().await.expect("json");
    assert_eq!(
        body["type"],
        "https://fantatt.org/problems/csrf-token-missing"
    );
}

#[tokio::test]
async fn a_wrong_token_is_refused() {
    let shell = spawn_shell().await;
    let (cookie, _token) = issue_session(&shell.base_url).await;
    let response = reqwest::Client::new()
        .delete(format!("{}/auth/session", shell.base_url))
        .header(reqwest::header::COOKIE, &cookie)
        .header("x-csrf-token", "not-the-token")
        .send()
        .await
        .expect("delete");
    assert_eq!(response.status(), 403);
    let body: Value = response.json().await.expect("json");
    assert_eq!(
        body["type"],
        "https://fantatt.org/problems/csrf-token-mismatch"
    );
}

#[tokio::test]
async fn the_right_token_logs_out_and_the_session_dies() {
    let shell = spawn_shell().await;
    let (cookie, token) = issue_session(&shell.base_url).await;
    assert_eq!(token.len(), 64, "an HMAC-SHA256 tag is 64 hex characters");
    let response = reqwest::Client::new()
        .delete(format!("{}/auth/session", shell.base_url))
        .header(reqwest::header::COOKIE, &cookie)
        .header("x-csrf-token", &token)
        .send()
        .await
        .expect("delete");
    assert_eq!(response.status(), 204);
    let reused = reqwest::Client::new()
        .get(format!("{}/auth/csrf", shell.base_url))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("get");
    let rewritten = common::session_cookie(&reused);
    assert_ne!(cookie_value(&rewritten), cookie_value(&cookie));
}
