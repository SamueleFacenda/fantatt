//! Request identifiers: a well-formed client value is echoed, anything else is
//! replaced by a freshly generated one.

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use rand::RngCore;

use super::security_headers::hex;

const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");
const MINIMUM_LENGTH: usize = 8;
const MAXIMUM_LENGTH: usize = 64;

/// The trace identifier of one request, readable by handlers and emitted as a
/// response header so clients can correlate logs.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl std::fmt::Display for RequestId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

pub async fn attach_request_id(mut request: Request, next: Next) -> Response {
    let request_id = RequestId(resolve_id(request.headers()));
    request.extensions_mut().insert(request_id.clone());
    let method = request.method().clone();
    let uri = request.uri().clone();
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        REQUEST_ID_HEADER,
        HeaderValue::from_str(&request_id.0).expect("request id is a valid header value"),
    );
    tracing::debug!(%method, %uri, %request_id, status = response.status().as_u16(), "request completed");
    response
}

fn resolve_id(headers: &HeaderMap) -> String {
    let supplied = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_well_formed(value));
    match supplied {
        Some(value) => value.to_owned(),
        None => generate_id(),
    }
}

fn is_well_formed(value: &str) -> bool {
    (MINIMUM_LENGTH..=MAXIMUM_LENGTH).contains(&value.len())
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

fn generate_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    hex(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                HeaderName::try_from(*name).expect("a test name is a valid header name"),
                HeaderValue::try_from(value.to_owned()).expect("a test value is a valid header"),
            );
        }
        headers
    }

    #[test]
    fn a_well_formed_client_value_is_echoed() {
        let headers = headers(&[("x-request-id", "trace-abc_123")]);
        assert_eq!(resolve_id(&headers), "trace-abc_123");
    }

    #[test]
    fn a_malformed_client_value_is_replaced() {
        for value in ["short", &"x".repeat(65), "bad value", "value/with/slashes"] {
            let headers = headers(&[("x-request-id", value)]);
            let resolved = resolve_id(&headers);
            assert_eq!(resolved.len(), 32, "{value} must not pass through");
            assert!(
                resolved
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            );
        }
    }

    #[test]
    fn an_absent_header_still_yields_an_identifier() {
        let headers = headers(&[]);
        assert_eq!(resolve_id(&headers).len(), 32);
    }
}
