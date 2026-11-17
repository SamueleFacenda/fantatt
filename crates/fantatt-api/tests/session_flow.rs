//! Session flow through the HTTP surface and the real store: the token stays
//! bound to the session across requests (ADR-0009 decision 3).

mod common;

use common::{issue_session, spawn_shell};

#[tokio::test]
async fn the_token_stays_bound_to_the_session_across_requests() {
    let shell = spawn_shell().await;
    let (cookie, first_token) = issue_session(&shell.base_url).await;
    let again = reqwest::Client::new()
        .get(format!("{}/auth/csrf", shell.base_url))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("get");
    let rewritten = common::session_cookie(&again);
    let body: serde_json::Value = again.json().await.expect("json");
    let second_token = common::json_string(&body, "token");
    assert_eq!(rewritten, cookie, "the sliding window keeps the session");
    assert_eq!(
        second_token, first_token,
        "the token is bound to the session id"
    );
}

#[tokio::test]
async fn a_visitor_without_a_cookie_gets_their_own_session() {
    let shell = spawn_shell().await;
    let (_, first_token) = issue_session(&shell.base_url).await;
    let (_, second_token) = issue_session(&shell.base_url).await;
    assert_ne!(first_token, second_token);
}

#[tokio::test]
async fn healthz_mints_no_session_for_anonymous_visitors() {
    let shell = spawn_shell().await;
    let response = reqwest::get(format!("{}/healthz", shell.base_url))
        .await
        .expect("get");
    assert_eq!(response.status(), 200);
    assert!(
        response
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .is_none()
    );
}
