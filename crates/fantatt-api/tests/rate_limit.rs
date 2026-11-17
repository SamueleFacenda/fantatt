//! Rate limits: the auth budget exhausts per key and answers 429 with a
//! problem body and a Retry-After header.

mod common;

use common::{header, spawn_shell};

#[tokio::test]
async fn the_auth_budget_exhausts_after_ten_requests_per_key() {
    let shell = spawn_shell().await;
    let client = reqwest::Client::new();
    exhaust(&client, &shell.base_url).await;
    assert_limited(&client, &shell.base_url).await;
}

async fn exhaust(client: &reqwest::Client, base_url: &str) {
    for attempt in 0..10 {
        let response = client
            .get(format!("{base_url}/auth/csrf"))
            .send()
            .await
            .expect("get");
        assert_eq!(response.status(), 200, "attempt {attempt} must pass");
    }
}

async fn assert_limited(client: &reqwest::Client, base_url: &str) {
    let limited = client
        .get(format!("{base_url}/auth/csrf"))
        .send()
        .await
        .expect("get");
    assert_eq!(limited.status(), 429);
    assert!(header(&limited, "content-type").starts_with("application/problem+json"));
    let retry_after: u64 = header(&limited, "retry-after")
        .parse()
        .expect("retry after is seconds");
    assert!((1..=60).contains(&retry_after));
}
