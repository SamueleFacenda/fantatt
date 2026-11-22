//! The transport seam: one trait, one method.
//!
//! Unit tests inject a stub that records requests and answers canned
//! responses; the production path is reqwest with rustls and HTTP/2
//! (docs/06-data-pipeline.md §3.3). The trait owns no politeness policy: the
//! caller throttles, retries, and breaks circuits around it.

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Method;
use reqwest::StatusCode;
use reqwest::Url;
use reqwest::header::HeaderMap;
use reqwest::redirect::Policy;

/// One outbound request, fully prepared with headers and a per-attempt
/// timeout.
#[derive(Debug, Clone)]
pub struct PreparedRequest {
    pub method: Method,
    pub url: Url,
    pub headers: HeaderMap,
    pub timeout: Duration,
}

impl PreparedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|value| value.to_str().ok())
    }
}

/// One response: status, headers, body, and the URL the response came from
/// (redirects can move it).
#[derive(Debug, Clone)]
pub struct RawResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub final_url: Url,
}

/// The seam every fetch goes through.
#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, request: &PreparedRequest) -> Result<RawResponse, TransportError>;
}

/// Why a single attempt failed, classified for the retry policy.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    #[error("the request to {url} timed out after {timeout_ms} ms")]
    Timeout { url: String, timeout_ms: u64 },

    #[error("cannot reach {url}: {reason}")]
    Connect { url: String, reason: String },

    #[error("the transport refused {url}: {reason}")]
    Request { url: String, reason: String },

    #[error("a redirect left the allowed origins for {url}: {reason}")]
    Blocked { url: String, reason: String },
}

impl TransportError {
    /// A timeout or an unreachable host can succeed later; a refused or
    /// policy-blocked request cannot.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::Connect { .. })
    }

    pub fn url(&self) -> &str {
        match self {
            Self::Timeout { url, .. }
            | Self::Connect { url, .. }
            | Self::Request { url, .. }
            | Self::Blocked { url, .. } => url,
        }
    }
}

impl From<TransportError> for crate::error::FetchError {
    fn from(error: TransportError) -> Self {
        use crate::error::FetchError;
        match error {
            TransportError::Blocked { url, reason } => FetchError::Blocked { url, reason },
            TransportError::Timeout { url, timeout_ms } => FetchError::Timeout { url, timeout_ms },
            TransportError::Connect { url, reason } => FetchError::Connect { url, reason },
            TransportError::Request { url, reason } => FetchError::Refused { url, reason },
        }
    }
}

/// The reqwest transport: one pooled client, rustls, HTTP/2, gzip.
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new(redirects: Policy) -> Result<Self, TransportError> {
        let client = reqwest::Client::builder()
            .redirect(redirects)
            .build()
            .map_err(|reason| TransportError::Request {
                url: "client".to_owned(),
                reason: reason.to_string(),
            })?;
        Ok(Self { client })
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn send(&self, request: &PreparedRequest) -> Result<RawResponse, TransportError> {
        let response = self
            .client
            .request(request.method.clone(), request.url.clone())
            .headers(request.headers.clone())
            .timeout(request.timeout)
            .send()
            .await
            .map_err(|error| map_error(error, request))?;
        Ok(RawResponse {
            status: response.status(),
            headers: response.headers().clone(),
            final_url: response.url().clone(),
            body: read_body(response, request).await?,
        })
    }
}

/// Reads the body of a response whose status already succeeded.
async fn read_body(
    response: reqwest::Response,
    request: &PreparedRequest,
) -> Result<Vec<u8>, TransportError> {
    response
        .bytes()
        .await
        .map_err(|error| map_error(error, request))
        .map(|body| body.to_vec())
}

fn map_error(error: reqwest::Error, request: &PreparedRequest) -> TransportError {
    let url = request.url.to_string();
    if error.is_timeout() {
        let timeout_ms = request.timeout.as_millis() as u64;
        return TransportError::Timeout { url, timeout_ms };
    }
    permanent(error, url)
}

/// Every non-timeout failure is permanent for the retry policy.
fn permanent(error: reqwest::Error, url: String) -> TransportError {
    let reason = error.to_string();
    if error.is_redirect() {
        return TransportError::Blocked { url, reason };
    }
    if error.is_request() {
        return TransportError::Request { url, reason };
    }
    TransportError::Connect { url, reason }
}

/// A stub transport for unit tests: records every request and answers from a
/// scripted queue, or a canned failure when the queue runs dry.
#[cfg(test)]
pub mod stub {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct StubTransport {
        script: Arc<Mutex<Vec<Result<RawResponse, TransportError>>>>,
        requests: Arc<Mutex<Vec<PreparedRequest>>>,
    }

    impl StubTransport {
        pub fn answering(script: Vec<Result<RawResponse, TransportError>>) -> Self {
            Self {
                script: Arc::new(Mutex::new(script)),
                requests: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn requests(&self) -> Vec<PreparedRequest> {
            self.requests.lock().expect("requests").clone()
        }

        pub fn call_count(&self) -> usize {
            self.requests.lock().expect("requests").len()
        }
    }

    #[async_trait]
    impl Transport for StubTransport {
        async fn send(&self, request: &PreparedRequest) -> Result<RawResponse, TransportError> {
            self.requests
                .lock()
                .expect("requests")
                .push(request.clone());
            let mut script = self.script.lock().expect("script");
            if script.is_empty() {
                return Err(TransportError::Connect {
                    url: request.url.to_string(),
                    reason: "the script ran dry".to_owned(),
                });
            }
            script.remove(0)
        }
    }

    pub fn response(status: u16) -> RawResponse {
        RawResponse {
            status: StatusCode::from_u16(status).expect("status"),
            headers: HeaderMap::new(),
            body: b"ok".to_vec(),
            final_url: Url::parse("https://portale.fitet.org/stub").expect("url"),
        }
    }

    /// A response carrying the given body and header pairs.
    pub fn page(status: u16, body: &[u8], headers: &[(&str, &str)]) -> RawResponse {
        let mut header_map = HeaderMap::new();
        for (name, value) in headers {
            header_map.insert(
                HeaderName::from_bytes(name.as_bytes()).expect("name"),
                HeaderValue::from_str(value).expect("value"),
            );
        }
        RawResponse {
            status: StatusCode::from_u16(status).expect("status"),
            headers: header_map,
            body: body.to_vec(),
            final_url: Url::parse("https://portale.fitet.org/stub").expect("url"),
        }
    }

    pub fn error() -> TransportError {
        TransportError::Connect {
            url: "https://portale.fitet.org/x".to_owned(),
            reason: "connection reset".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};

    fn request() -> PreparedRequest {
        PreparedRequest {
            method: Method::GET,
            url: Url::parse("https://portale.fitet.org/menu.php").expect("url"),
            headers: HeaderMap::new(),
            timeout: Duration::from_millis(1_000),
        }
    }

    #[test]
    fn a_timeout_and_a_connect_failure_are_retryable() {
        let timeout = TransportError::Timeout {
            url: "https://portale.fitet.org/x".to_owned(),
            timeout_ms: 1_000,
        };
        let refused = TransportError::Request {
            url: "https://portale.fitet.org/x".to_owned(),
            reason: "bad request".to_owned(),
        };
        assert!(timeout.is_retryable());
        assert!(!refused.is_retryable());
    }

    #[test]
    fn a_header_can_be_read_back_for_assertions() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("user-agent"),
            HeaderValue::from_static("fantatt-ingest/0.1.0"),
        );
        let prepared = PreparedRequest {
            headers,
            ..request()
        };
        assert_eq!(prepared.header("user-agent"), Some("fantatt-ingest/0.1.0"));
    }

    #[test]
    fn a_transport_block_surfaces_as_a_policy_block() {
        let blocked = TransportError::Blocked {
            url: "https://portale.fitet.org/x".to_owned(),
            reason: "not an allowed host".to_owned(),
        };
        let error = crate::error::FetchError::from(blocked);
        assert!(
            matches!(error, crate::error::FetchError::Blocked { .. }),
            "{error}"
        );
    }

    #[tokio::test]
    async fn the_stub_records_every_request_and_answers_in_order() {
        let stub =
            stub::StubTransport::answering(vec![Ok(stub::response(503)), Ok(stub::response(200))]);
        assert_eq!(stub.send(&request()).await.expect("first").status, 503);
        assert_eq!(stub.send(&request()).await.expect("second").status, 200);
        assert_eq!(stub.call_count(), 2);
        assert_eq!(stub.requests().len(), 2);
        assert!(stub.send(&request()).await.is_err());
        assert_eq!(stub.call_count(), 3);
    }
}
