//! The hardened shell: health contract, default-deny fallbacks, the error
//! envelope, and the OpenAPI snapshot.

mod common;

use common::{Shell, header, spawn_shell};

use fantatt_api::openapi::ApiDoc;
use serde_json::{Value, json};
use utoipa::OpenApi;

async fn shell() -> Shell {
    spawn_shell().await
}

#[tokio::test]
async fn healthz_answers_the_contract() {
    let shell = shell().await;
    let response = reqwest::get(format!("{}/healthz", shell.base_url))
        .await
        .expect("get");
    assert_eq!(response.status(), 200);
    assert!(header(&response, "content-type").starts_with("application/json"));
    let body: Value = response.json().await.expect("json");
    assert_eq!(body, json!({ "status": "ok" }));
}

#[tokio::test]
async fn a_csp_report_is_accepted() {
    let shell = shell().await;
    let report = json!({ "csp-report": { "blocked-uri": "https://evil.example.org/", "effective-directive": "script-src" } });
    let response = reqwest::Client::new()
        .post(format!("{}/api/v1/csp-report", shell.base_url))
        .json(&report)
        .send()
        .await
        .expect("post");
    assert_eq!(response.status(), 204);
}

#[tokio::test]
async fn an_unknown_path_answers_a_problem() {
    let shell = shell().await;
    let response = reqwest::get(format!("{}/nope", shell.base_url))
        .await
        .expect("get");
    assert_eq!(response.status(), 404);
    assert_problem("unknown path", response).await;
}

#[tokio::test]
async fn a_known_path_with_the_wrong_method_answers_a_problem() {
    let shell = shell().await;
    let response = wrong_method(&shell).await;
    assert_eq!(response.status(), 405);
    assert_problem("wrong method", response).await;
}

#[tokio::test]
async fn every_error_response_carries_the_envelope() {
    let shell = shell().await;
    for (case, response) in error_battery(&shell).await {
        assert_problem(case, response).await;
    }
}

#[tokio::test]
async fn openapi_is_served_and_matches_the_committed_snapshot() {
    let shell = shell().await;
    let response = reqwest::get(format!("{}/openapi.json", shell.base_url))
        .await
        .expect("get");
    assert_eq!(response.status(), 200);
    let served: Value = response.json().await.expect("json");
    let committed: Value = serde_json::from_str(include_str!("../openapi.json"))
        .expect("the committed snapshot parses");
    let derived: Value = serde_json::to_value(ApiDoc::openapi()).expect("serialize");
    assert_eq!(served, committed);
    assert_eq!(derived, committed);
}

async fn wrong_method(shell: &Shell) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{}/healthz", shell.base_url))
        .send()
        .await
        .expect("post")
}

async fn malformed_body(shell: &Shell) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{}/api/v1/csp-report", shell.base_url))
        .header(reqwest::header::CONTENT_TYPE, "application/csp-report")
        .body("not json")
        .send()
        .await
        .expect("post")
}

async fn malformed_idempotency_key(shell: &Shell) -> reqwest::Response {
    reqwest::Client::new()
        .get(format!("{}/healthz", shell.base_url))
        .header("idempotency-key", "bad key")
        .send()
        .await
        .expect("get")
}

async fn oversized_body(shell: &Shell) -> reqwest::Response {
    let body = "x".repeat(2_000_000);
    let client = reqwest::Client::new();
    client
        .post(format!("{}/api/v1/csp-report", shell.base_url))
        .header(reqwest::header::CONTENT_LENGTH, body.len())
        .header(reqwest::header::CONTENT_TYPE, "application/csp-report")
        .body(body)
        .send()
        .await
        .expect("post")
}

async fn error_battery(shell: &Shell) -> Vec<(&'static str, reqwest::Response)> {
    vec![
        (
            "unknown path",
            reqwest::get(format!("{}/nope", shell.base_url))
                .await
                .expect("get"),
        ),
        ("wrong method", wrong_method(shell).await),
        ("malformed body", malformed_body(shell).await),
        (
            "malformed idempotency key",
            malformed_idempotency_key(shell).await,
        ),
        ("oversized body", oversized_body(shell).await),
    ]
}

async fn assert_problem(case: &str, response: reqwest::Response) {
    let status = response.status();
    let content_type = header(&response, "content-type");
    assert!(
        content_type.starts_with("application/problem+json"),
        "{case}: status {status}, content-type {content_type}"
    );
    let body: Value = response.json().await.expect("json");
    let type_uri = body["type"].as_str().expect("type is a string");
    assert!(
        type_uri.starts_with("https://fantatt.org/problems/"),
        "{type_uri}"
    );
    assert!(body["title"].is_string());
    assert_eq!(body["status"].as_u64(), Some(status.as_u16().into()));
}
