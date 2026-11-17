//! A body that declares more than the configured limit is refused before it
//! is read; a streamed body without a declared length hits the extractor cap.

use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::{Problem, ProblemType};
use crate::state::AppState;

pub async fn reject_oversized_bodies(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let limit = state.settings().server.request_body_limit_bytes;
    if declared_length(&request).is_some_and(|length| length > limit) {
        return Problem::from_type(ProblemType::BodyTooLarge).into_response();
    }
    next.run(request).await
}

/// The hard cap applied to every extractor, for bodies that stream without a
/// declared length.
pub fn body_limit_layer(state: &AppState) -> DefaultBodyLimit {
    let bytes =
        usize::try_from(state.settings().server.request_body_limit_bytes).unwrap_or(usize::MAX);
    DefaultBodyLimit::max(bytes)
}

fn declared_length(request: &Request) -> Option<u64> {
    let value = request
        .headers()
        .get(header::CONTENT_LENGTH)?
        .to_str()
        .ok()?;
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_declared_length_is_read_from_the_header() {
        let request = Request::builder()
            .header(header::CONTENT_LENGTH, "42")
            .body(axum::body::Body::empty())
            .expect("build");
        assert_eq!(declared_length(&request), Some(42));
    }

    #[test]
    fn an_absent_or_absurd_length_reads_as_none() {
        let absent = Request::builder()
            .body(axum::body::Body::empty())
            .expect("build");
        assert_eq!(declared_length(&absent), None);
        let absurd = Request::builder()
            .header(header::CONTENT_LENGTH, "many")
            .body(axum::body::Body::empty())
            .expect("build");
        assert_eq!(declared_length(&absurd), None);
    }
}
