//! Idempotency-Key handling: a malformed header is refused, a well-formed one
//! rides to the handler in request extensions. Replay persistence lands with
//! the first creating endpoint (G2.4/G2.5); no POST that mutates exists yet.

use axum::extract::Request;
use axum::http::HeaderName;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::{Problem, ProblemType};

const IDEMPOTENCY_KEY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");
const MAXIMUM_LENGTH: usize = 128;

/// The well-formed key a creating POST carries.
#[derive(Clone, Debug)]
pub struct IdempotencyKey(pub String);

pub async fn normalize_idempotency_key(mut request: Request, next: Next) -> Response {
    let supplied = request
        .headers()
        .get(IDEMPOTENCY_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if let Some(value) = supplied {
        if !is_well_formed(&value) {
            return Problem::from_type(ProblemType::IdempotencyKeyInvalid).into_response();
        }
        request.extensions_mut().insert(IdempotencyKey(value));
    }
    next.run(request).await
}

fn is_well_formed(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed == value && !trimmed.is_empty() && trimmed.len() <= MAXIMUM_LENGTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_key_passes() {
        assert!(is_well_formed("01J8ZQ4V2K"));
        assert!(is_well_formed(&"k".repeat(MAXIMUM_LENGTH)));
    }

    #[test]
    fn a_malformed_key_is_refused() {
        assert!(!is_well_formed(""));
        assert!(!is_well_formed("  "));
        assert!(!is_well_formed(" padded "));
        assert!(!is_well_formed(&"k".repeat(MAXIMUM_LENGTH + 1)));
    }
}
