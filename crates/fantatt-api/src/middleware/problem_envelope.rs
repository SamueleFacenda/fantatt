//! Converts every error response into problem+json, whatever layer produced it.

use axum::extract::Request;
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::{PROBLEM_MEDIA_TYPE, Problem};

pub async fn enforce_problem_envelope(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    if !response.status().is_client_error() && !response.status().is_server_error() {
        return response;
    }
    if is_problem(&response) {
        return response;
    }
    into_problem(response)
}

fn is_problem(response: &Response) -> bool {
    response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with(PROBLEM_MEDIA_TYPE))
}

fn into_problem(response: Response) -> Response {
    let (parts, _body) = response.into_parts();
    let problem = Problem::from_status(parts.status);
    let mut problem_response = problem.into_response();
    if let Some(retry_after) = parts.headers.get(header::RETRY_AFTER) {
        problem_response
            .headers_mut()
            .insert(header::RETRY_AFTER, retry_after.clone());
    }
    problem_response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ProblemType;
    use axum::http::StatusCode;

    #[test]
    fn a_problem_response_is_left_untouched() {
        let response = Problem::from_type(ProblemType::BodyTooLarge).into_response();
        assert!(is_problem(&response));
    }

    #[test]
    fn a_framework_error_becomes_a_problem_response() {
        let response = axum::http::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(axum::body::Body::from("no route"))
            .expect("build");
        assert!(!is_problem(&response));
        let problem = into_problem(response);
        assert_eq!(problem.status(), StatusCode::NOT_FOUND);
        assert!(is_problem(&problem));
    }
}
