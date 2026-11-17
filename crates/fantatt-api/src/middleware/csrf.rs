//! CSRF double-submit: a token bound to the session id by HMAC-SHA256 with the
//! configured secret (docs/10-security.md §2).

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::error::{Problem, ProblemType};
use crate::state::AppState;

pub async fn enforce_csrf(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(session) = request
        .extensions()
        .get::<tower_sessions::Session>()
        .cloned()
    else {
        return Problem::from_type(ProblemType::Internal).into_response();
    };
    let supplied = supplied_token(&request, &state.settings().csrf.header);
    match expected_token(&session, state.csrf_key()) {
        Some(expected) if supplied.as_deref() == Some(expected.as_str()) => next.run(request).await,
        Some(_) => Problem::from_type(ProblemType::CsrfTokenMismatch).into_response(),
        None => Problem::from_type(ProblemType::CsrfTokenMissing).into_response(),
    }
}

fn supplied_token(request: &Request, header_name: &str) -> Option<String> {
    let value = request
        .headers()
        .get(header_name)
        .and_then(|value| value.to_str().ok())?;
    Some(value.to_owned())
}

/// why: the token recomputes per request instead of being stored in the
/// session, so every session rotation invalidates outstanding tokens without
/// any cleanup.
pub fn expected_token(session: &tower_sessions::Session, key: &[u8]) -> Option<String> {
    let id = session.id()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).ok()?;
    mac.update(id.to_string().as_bytes());
    Some(super::security_headers::hex(&mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::http::HeaderValue;

    fn request(header_name: &str, token: &str) -> Request {
        let mut request = Request::builder()
            .uri("https://fantatt.example.org/auth/session")
            .body(axum::body::Body::empty())
            .expect("build");
        let name = axum::http::HeaderName::try_from(header_name.to_owned())
            .expect("a test name is a valid header name");
        let value = HeaderValue::from_str(token).expect("a token is a header value");
        request.headers_mut().insert(name, value);
        request
    }

    #[test]
    fn a_supplied_token_is_read_from_the_configured_header() {
        let request = request("x-csrf-token", "abc123");
        assert_eq!(
            supplied_token(&request, "x-csrf-token").as_deref(),
            Some("abc123")
        );
    }

    #[test]
    fn an_absent_header_yields_no_token() {
        let request = request("x-other", "abc123");
        assert!(supplied_token(&request, "x-csrf-token").is_none());
    }
}
