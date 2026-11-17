//! Header flags on every response, read from the wire (docs/10-security.md
//! §3, §1.4).

mod common;

use common::{cookie_attributes, spawn_shell};

#[tokio::test]
async fn every_response_carries_the_security_headers() {
    let shell = spawn_shell().await;
    let response = reqwest::get(format!("{}/healthz", shell.base_url))
        .await
        .expect("get");
    let csp = header(&response, "content-security-policy");
    assert!(csp.contains("script-src 'nonce-"), "{csp}");
    assert!(csp.contains("style-src 'nonce-"), "{csp}");
    assert!(csp.contains("frame-ancestors 'none'"), "{csp}");
    assert!(csp.contains("base-uri 'none'"), "{csp}");
    assert!(csp.contains("require-trusted-types-for 'script'"), "{csp}");
    assert!(!csp.contains("{nonce}"), "{csp}");
    assert_eq!(
        header(&response, "strict-transport-security"),
        "max-age=63072000; includeSubDomains"
    );
    assert_eq!(header(&response, "x-content-type-options"), "nosniff");
    assert!(response.headers().get("x-request-id").is_some());
}

#[tokio::test]
async fn an_error_response_carries_the_security_headers_too() {
    let shell = spawn_shell().await;
    let response = reqwest::get(format!("{}/nope", shell.base_url))
        .await
        .expect("get");
    assert_eq!(response.status(), 404);
    assert!(header(&response, "content-security-policy").contains("default-src 'none'"));
    assert_eq!(header(&response, "x-content-type-options"), "nosniff");
}

#[tokio::test]
async fn each_response_gets_its_own_nonce() {
    let shell = spawn_shell().await;
    let first = nonce(
        reqwest::get(format!("{}/healthz", shell.base_url))
            .await
            .expect("get"),
    );
    let second = nonce(
        reqwest::get(format!("{}/healthz", shell.base_url))
            .await
            .expect("get"),
    );
    assert_ne!(first, second);
    assert_eq!(first.len(), 32);
}

#[tokio::test]
async fn the_session_cookie_carries_every_documented_flag() {
    let shell = spawn_shell().await;
    let response = reqwest::get(format!("{}/auth/csrf", shell.base_url))
        .await
        .expect("get");
    let cookie = cookie_attributes(&response);
    assert!(cookie.starts_with("__Host-session="), "{cookie}");
    assert!(cookie.contains("HttpOnly"), "{cookie}");
    assert!(cookie.contains("Secure"), "{cookie}");
    assert!(cookie.contains("SameSite=Lax"), "{cookie}");
    assert!(cookie.contains("Path=/"), "{cookie}");
}

fn header(response: &reqwest::Response, name: &str) -> String {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

fn nonce(response: reqwest::Response) -> String {
    let csp = header(&response, "content-security-policy");
    let start = csp.find("nonce-").expect("the policy carries a nonce") + "nonce-".len();
    let end = csp[start..]
        .find('\'')
        .expect("the nonce closes with a quote")
        + start;
    csp[start..end].to_owned()
}
