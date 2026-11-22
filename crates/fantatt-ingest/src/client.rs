//! The polite fetch client: allow-list, throttle, retry, breaker, and archive
//! wired around the transport seam.
//!
//! One `fetch` call walks the whole chain: the host policy rejects an unknown
//! source before any socket opens, the breaker fails fast while the source is
//! down, the throttle spaces the attempt, and a transient failure retries with
//! exponential backoff while the circuit keeps score.

use std::sync::Arc;

use reqwest::Method;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use tokio::time::sleep;

use crate::archive::{Archive, FilesystemArchive, content_key};
use crate::breaker::Breaker;
use crate::conditional::CacheValidators;
use crate::error::FetchError;
use crate::policy::{HostPolicy, host_of};
use crate::retry::{Backoff, is_retryable_status};
use crate::settings::FetchSettings;
use crate::throttle::Throttle;
use crate::transport::{PreparedRequest, RawResponse, ReqwestTransport, Transport, TransportError};

/// What a fetch delivered.
#[derive(Debug, Clone)]
pub enum Fetched {
    Fresh(FetchedPage),
    NotModified,
}

/// One fetched page, with the validators the next request needs.
#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub final_url: Url,
    pub status: u16,
    pub body: Vec<u8>,
    pub validators: CacheValidators,
}

/// The fetch facade.
pub struct FetchClient {
    settings: FetchSettings,
    policy: HostPolicy,
    transport: Box<dyn Transport>,
    archive: Option<Box<dyn Archive>>,
    throttle: Arc<Throttle>,
    breaker: Breaker,
    backoff: Backoff,
}

/// What one attempt produced, once the breaker scored it.
enum Outcome {
    Fresh(FetchedPage),
    NotModified,
    Failed {
        reason: FailureReason,
        retryable: bool,
    },
}

/// Why an attempt did not produce a page.
#[derive(Debug)]
enum FailureReason {
    Status(StatusCode),
    Transport(TransportError),
}

impl FailureReason {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Status(status) => is_retryable_status(*status),
            Self::Transport(error) => error.is_retryable(),
        }
    }
}

impl std::fmt::Display for FailureReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status(status) => write!(f, "{status}"),
            Self::Transport(error) => write!(f, "{error}"),
        }
    }
}

impl FetchClient {
    /// A client with no archive; attach one with `with_archive`.
    pub fn new(settings: FetchSettings, transport: Box<dyn Transport>) -> Result<Self, FetchError> {
        settings.validate()?;
        let policy = settings.host_policy()?;
        Ok(Self::assemble(settings, policy, transport))
    }

    /// The production client: the real reqwest transport, and a filesystem
    /// archive when the settings name a path.
    pub fn from_settings(settings: FetchSettings) -> Result<Self, FetchError> {
        settings.validate()?;
        let policy = settings.host_policy()?;
        let transport = ReqwestTransport::new(policy.redirect_policy(settings.max_redirects))?;
        let mut client = Self::assemble(settings.clone(), policy, Box::new(transport));
        if let Some(root) = settings.archive_path {
            client = client.with_archive(Box::new(FilesystemArchive::new(root)));
        }
        Ok(client)
    }

    /// Attaches an archive; fetched bodies land in it under their content key.
    pub fn with_archive(mut self, archive: Box<dyn Archive>) -> Self {
        self.archive = Some(archive);
        self
    }

    /// Fetches one page, honoring the validators the caller stored for it.
    pub async fn fetch(
        &self,
        url: &Url,
        validators: CacheValidators,
    ) -> Result<Fetched, FetchError> {
        self.policy.check(url)?;
        self.run(url, &self.prepare(url, &validators)).await
    }

    /// The attempt loop: every attempt is breaker-gated, throttled, and
    /// scored; a transient failure backs off and retries within the ceiling.
    async fn run(&self, url: &Url, request: &PreparedRequest) -> Result<Fetched, FetchError> {
        let host = host_of(url).unwrap_or_default().to_owned();
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match self.attempt(&host, request).await? {
                Outcome::Fresh(page) => {
                    self.archive_page(&page).await?;
                    return Ok(Fetched::Fresh(page));
                }
                Outcome::NotModified => return Ok(Fetched::NotModified),
                Outcome::Failed { reason, retryable } => {
                    if !retryable || attempt >= self.settings.max_attempts {
                        return Err(failure_error(url, attempt, &reason));
                    }
                    self.back_off(&host, attempt).await;
                }
            }
        }
    }

    /// One attempt: the breaker gate, the throttle wait, the transport call.
    async fn attempt(&self, host: &str, request: &PreparedRequest) -> Result<Outcome, FetchError> {
        self.breaker
            .acquire(host)
            .map_err(|_| FetchError::CircuitOpen {
                host: host.to_owned(),
            })?;
        let hold = self.throttle.hold(host).await;
        let response = self.transport.send(request).await;
        drop(hold);
        Ok(self.classify(host, response))
    }

    fn classify(&self, host: &str, response: Result<RawResponse, TransportError>) -> Outcome {
        match response {
            Ok(response) if response.status == StatusCode::NOT_MODIFIED => {
                self.breaker.record_success(host);
                Outcome::NotModified
            }
            Ok(response) if response.status.is_success() => {
                self.breaker.record_success(host);
                Outcome::Fresh(FetchedPage {
                    final_url: response.final_url,
                    status: response.status.as_u16(),
                    validators: CacheValidators::from_headers(&response.headers),
                    body: response.body,
                })
            }
            Ok(response) => self.failed(host, FailureReason::Status(response.status)),
            Err(error) => self.failed(host, FailureReason::Transport(error)),
        }
    }

    fn failed(&self, host: &str, reason: FailureReason) -> Outcome {
        self.breaker.record_failure(host);
        Outcome::Failed {
            retryable: reason.is_retryable(),
            reason,
        }
    }

    async fn back_off(&self, host: &str, attempt: u32) {
        let delay = self.backoff.delay(attempt);
        tracing::debug!(
            host = %host,
            attempt,
            delay_ms = delay.as_millis(),
            "transient failure; backing off"
        );
        sleep(delay).await;
    }

    fn prepare(&self, url: &Url, validators: &CacheValidators) -> PreparedRequest {
        let mut headers = HeaderMap::new();
        let user_agent = HeaderValue::from_str(&self.settings.user_agent)
            .unwrap_or_else(|_| HeaderValue::from_static("fantatt-ingest"));
        headers.insert(USER_AGENT, user_agent);
        validators.insert_into(&mut headers);
        PreparedRequest {
            method: Method::GET,
            url: url.clone(),
            headers,
            timeout: self.settings.timeout(),
        }
    }

    async fn archive_page(&self, page: &FetchedPage) -> Result<(), FetchError> {
        let Some(archive) = &self.archive else {
            return Ok(());
        };
        let key = content_key(&page.body);
        archive.store(&key, &page.body).await?;
        tracing::debug!(key = %key, bytes = page.body.len(), "archived the body");
        Ok(())
    }

    fn assemble(
        settings: FetchSettings,
        policy: HostPolicy,
        transport: Box<dyn Transport>,
    ) -> Self {
        let throttle = Throttle::new(settings.concurrency, settings.interval(), settings.jitter());
        let backoff = Backoff::new(
            settings.backoff_base(),
            settings.backoff_cap(),
            settings.jitter(),
        );
        Self {
            throttle: Arc::new(throttle),
            breaker: Breaker::new(settings.breaker_threshold, settings.breaker_reset()),
            backoff,
            policy,
            transport,
            archive: None,
            settings,
        }
    }
}

/// The error a failed fetch reports: the status or transport error itself when
/// it is definitive, a retry-exhaustion when the transient failure persisted.
fn failure_error(url: &Url, attempt: u32, reason: &FailureReason) -> FetchError {
    match reason {
        FailureReason::Status(status) => status_error(url, attempt, *status),
        FailureReason::Transport(TransportError::Blocked { reason, .. }) => FetchError::Blocked {
            url: url.to_string(),
            reason: reason.clone(),
        },
        FailureReason::Transport(error) if error.is_retryable() => {
            exhausted(url, attempt, error.to_string())
        }
        FailureReason::Transport(error) => FetchError::from(error.clone()),
    }
}

/// A non-2xx answer: the status itself, or an exhaustion when it was the
/// transient kind.
fn status_error(url: &Url, attempt: u32, status: StatusCode) -> FetchError {
    if is_retryable_status(status) {
        return exhausted(url, attempt, format!("status {status}"));
    }
    FetchError::Status {
        url: url.to_string(),
        status: status.as_u16(),
    }
}

fn exhausted(url: &Url, attempt: u32, reason: String) -> FetchError {
    FetchError::RetriesExhausted {
        url: url.to_string(),
        attempts: attempt,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::Instant;

    use super::*;
    use crate::archive::memory::MemoryArchive;
    use crate::settings::DEFAULT_USER_AGENT;
    use crate::transport::stub::{self, StubTransport};

    fn url() -> Url {
        Url::parse("https://portale.fitet.org/menu.php").expect("url")
    }

    fn validators() -> CacheValidators {
        CacheValidators {
            etag: Some("\"abc123\"".to_owned()),
            last_modified: None,
        }
    }

    /// A client over a scripted stub; interval, jitter, and backoff are zero so
    /// tests do not wait on politeness they do not exercise.
    fn client_with(
        script: Vec<Result<RawResponse, TransportError>>,
        interval_ms: u64,
    ) -> (FetchClient, StubTransport) {
        let transport = StubTransport::answering(script);
        let settings = FetchSettings {
            min_interval_ms: interval_ms,
            jitter_ms: 0,
            backoff_base_ms: 0,
            backoff_cap_ms: 0,
            ..FetchSettings::default()
        };
        let client = FetchClient::new(settings, Box::new(transport.clone())).expect("client");
        (client, transport)
    }

    fn client(script: Vec<Result<RawResponse, TransportError>>) -> (FetchClient, StubTransport) {
        client_with(script, 1)
    }

    #[tokio::test]
    async fn a_fresh_fetch_sends_the_user_agent_and_the_validators() {
        let (client, transport) = client(vec![Ok(stub::page(
            200,
            b"<html>menu</html>",
            &[("etag", "\"new-tag\"")],
        ))]);
        let fetched = client.fetch(&url(), validators()).await.expect("fresh");
        let Fetched::Fresh(page) = fetched else {
            panic!("expected a fresh page");
        };
        assert_eq!(page.body, b"<html>menu</html>");
        assert_eq!(page.status, 200);
        assert_eq!(page.validators.etag.as_deref(), Some("\"new-tag\""));
        let request = &transport.requests()[0];
        assert_eq!(request.header("user-agent"), Some(DEFAULT_USER_AGENT));
        assert_eq!(request.header("if-none-match"), Some("\"abc123\""));
    }

    #[tokio::test]
    async fn a_transient_failure_retries_then_succeeds() {
        let (client, transport) = client(vec![
            Ok(stub::response(503)),
            Ok(stub::response(503)),
            Ok(stub::response(200)),
        ]);
        let fetched = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect("fresh");
        assert!(matches!(fetched, Fetched::Fresh(_)));
        assert_eq!(transport.call_count(), 3);
    }

    #[tokio::test]
    async fn a_definitive_status_does_not_retry() {
        let (client, transport) = client(vec![Ok(stub::response(404))]);
        let error = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect_err("missing page");
        assert!(
            matches!(error, FetchError::Status { status: 404, .. }),
            "{error}"
        );
        assert_eq!(transport.call_count(), 1);
    }

    #[tokio::test]
    async fn persistent_transient_failures_exhaust_the_attempts() {
        let transport =
            StubTransport::answering(vec![Ok(stub::response(503)), Ok(stub::response(503))]);
        let settings = FetchSettings {
            min_interval_ms: 1,
            jitter_ms: 0,
            backoff_base_ms: 0,
            backoff_cap_ms: 0,
            max_attempts: 2,
            ..FetchSettings::default()
        };
        let client = FetchClient::new(settings, Box::new(transport.clone())).expect("client");
        let error = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect_err("exhausted");
        assert!(
            matches!(error, FetchError::RetriesExhausted { attempts: 2, .. }),
            "{error}"
        );
        assert_eq!(transport.call_count(), 2);
    }

    #[tokio::test]
    async fn an_open_circuit_stops_the_transport_calls() {
        let (client, transport) = circuit_client();
        let error = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect_err("circuit open");
        assert!(matches!(error, FetchError::CircuitOpen { .. }), "{error}");
        assert_eq!(transport.call_count(), 2);
    }

    #[tokio::test]
    async fn a_second_fetch_fails_fast_while_the_circuit_is_open() {
        let (client, transport) = circuit_client();
        let _ = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect_err("first trip");
        let second = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect_err("still open");
        assert!(matches!(second, FetchError::CircuitOpen { .. }), "{second}");
        assert_eq!(transport.call_count(), 2);
    }

    /// A client whose circuit trips after two failures, over a stub that fails
    /// exactly twice.
    fn circuit_client() -> (FetchClient, StubTransport) {
        let transport =
            StubTransport::answering(vec![Ok(stub::response(503)), Ok(stub::response(503))]);
        let settings = FetchSettings {
            min_interval_ms: 1,
            jitter_ms: 0,
            backoff_base_ms: 0,
            backoff_cap_ms: 0,
            breaker_threshold: 2,
            ..FetchSettings::default()
        };
        let client = FetchClient::new(settings, Box::new(transport.clone())).expect("client");
        (client, transport)
    }

    #[tokio::test]
    async fn a_not_modified_answer_carries_no_body() {
        let (client, transport) = client(vec![Ok(stub::response(304))]);
        let fetched = client
            .fetch(&url(), validators())
            .await
            .expect("not modified");
        assert!(matches!(fetched, Fetched::NotModified));
        assert_eq!(transport.call_count(), 1);
    }

    #[tokio::test]
    async fn an_unknown_host_never_reaches_the_transport() {
        let (client, transport) = client(vec![Ok(stub::response(200))]);
        let other = Url::parse("https://example.org/menu.php").expect("url");
        let error = client
            .fetch(&other, CacheValidators::default())
            .await
            .expect_err("blocked");
        assert!(matches!(error, FetchError::Blocked { .. }), "{error}");
        assert_eq!(transport.call_count(), 0);
    }

    #[tokio::test]
    async fn a_timeout_is_retried() {
        let (client, transport) = client(vec![Err(stub::error()), Ok(stub::response(200))]);
        let fetched = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect("fresh after a timeout");
        assert!(matches!(fetched, Fetched::Fresh(_)));
        assert_eq!(transport.call_count(), 2);
    }

    #[tokio::test]
    async fn a_fresh_body_is_archived_under_its_content_key() {
        let transport =
            StubTransport::answering(vec![Ok(stub::page(200, b"<html>giornata</html>", &[]))]);
        let archive = MemoryArchive::default();
        let settings = FetchSettings {
            min_interval_ms: 1,
            jitter_ms: 0,
            backoff_base_ms: 0,
            backoff_cap_ms: 0,
            ..FetchSettings::default()
        };
        let client = FetchClient::new(settings, Box::new(transport))
            .expect("client")
            .with_archive(Box::new(archive.clone()));
        let fetched = client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect("fresh");
        assert!(matches!(fetched, Fetched::Fresh(_)));
    }

    #[tokio::test(start_paused = true)]
    async fn two_fetches_to_one_host_are_spaced_by_the_interval() {
        let (client, transport) =
            client_with(vec![Ok(stub::response(200)), Ok(stub::response(200))], 750);
        let start = Instant::now();
        client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect("first");
        client
            .fetch(&url(), CacheValidators::default())
            .await
            .expect("second");
        assert!(Instant::now() - start >= Duration::from_millis(750));
        assert_eq!(transport.call_count(), 2);
    }
}
