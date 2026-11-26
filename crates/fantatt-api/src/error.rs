//! RFC 9457 problem+json responses.

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

pub const PROBLEM_MEDIA_TYPE: &str = "application/problem+json";
const PROBLEM_TYPE_URI_PREFIX: &str = "https://fantatt.org/problems/";

/// Every error response the shell answers with is one of these named kinds, so
/// the error-shape conformance test enumerates them exhaustively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemType {
    RouteNotFound,
    MethodNotAllowed,
    InvalidRequest,
    BadRequest,
    MalformedJson,
    BodyTooLarge,
    IdempotencyKeyInvalid,
    CsrfTokenMissing,
    CsrfTokenMismatch,
    TooManyRequests,
    SessionExpired,
    CapabilityMissing,
    Internal,
}

impl ProblemType {
    fn slug(self) -> &'static str {
        match self {
            Self::RouteNotFound => "route-not-found",
            Self::MethodNotAllowed => "method-not-allowed",
            Self::InvalidRequest => "invalid-request",
            Self::BadRequest => "bad-request",
            Self::MalformedJson => "malformed-json",
            Self::BodyTooLarge => "body-too-large",
            Self::IdempotencyKeyInvalid => "idempotency-key-invalid",
            Self::CsrfTokenMissing => "csrf-token-missing",
            Self::CsrfTokenMismatch => "csrf-token-mismatch",
            Self::TooManyRequests => "too-many-requests",
            Self::SessionExpired => "session-expired",
            Self::CapabilityMissing => "capability-missing",
            Self::Internal => "internal",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::RouteNotFound => "Route not found",
            Self::MethodNotAllowed => "Method not allowed",
            Self::InvalidRequest => "The request could not be processed",
            Self::BadRequest => "The request could not be processed",
            Self::MalformedJson => "The request body is not valid JSON",
            Self::BodyTooLarge => "The request body exceeds the size limit",
            Self::IdempotencyKeyInvalid => "The Idempotency-Key header is malformed",
            Self::CsrfTokenMissing => "The request carries no CSRF token",
            Self::CsrfTokenMismatch => "The CSRF token does not match the session",
            Self::TooManyRequests => "The rate limit for this route is exhausted",
            Self::SessionExpired => "The session reached its absolute lifetime",
            Self::CapabilityMissing => "The session lacks the required capability",
            Self::Internal => "The server failed to handle the request",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            Self::RouteNotFound => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Self::InvalidRequest | Self::BadRequest | Self::MalformedJson => {
                StatusCode::BAD_REQUEST
            }
            Self::BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::IdempotencyKeyInvalid => StatusCode::BAD_REQUEST,
            Self::CsrfTokenMissing | Self::CsrfTokenMismatch => StatusCode::FORBIDDEN,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::SessionExpired => StatusCode::UNAUTHORIZED,
            Self::CapabilityMissing => StatusCode::FORBIDDEN,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// The extra fields a rule verdict adds to an error response
/// (docs/07-api-spec.md §6).
#[derive(Debug, Clone, Default, Serialize, ToSchema)]
pub struct ProblemExtras {
    pub code: Option<String>,
    pub rule: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// A problem detail with the fields every error response carries; `code`,
/// `rule`, and `details` stay absent until a rule verdict needs them
/// (docs/07-api-spec.md §6).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Problem {
    #[serde(rename = "type")]
    pub type_uri: String,
    pub title: String,
    pub status: u16,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub extras: Option<Box<ProblemExtras>>,
}

impl Problem {
    pub fn from_type(problem_type: ProblemType) -> Self {
        Self {
            type_uri: format!("{PROBLEM_TYPE_URI_PREFIX}{}", problem_type.slug()),
            title: problem_type.title().to_owned(),
            status: problem_type.status().as_u16(),
            extras: None,
        }
    }

    /// The safety net for any framework-generated error body: pick the named
    /// kind that best describes the status.
    pub fn from_status(status: StatusCode) -> Self {
        Self::from_type(problem_type_for_status(status))
    }

    /// The status this problem answers with, which a rule verdict may choose
    /// freely while the envelope keeps its shape.
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status.as_u16();
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.extras_mut().code = Some(code.into());
        self
    }

    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.extras_mut().rule = Some(rule.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.extras_mut().details = Some(details);
        self
    }

    fn extras_mut(&mut self) -> &mut ProblemExtras {
        self.extras.get_or_insert_with(Box::default)
    }
}

impl IntoResponse for Problem {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let media_type = HeaderValue::from_str(PROBLEM_MEDIA_TYPE)
            .expect("problem+json is a valid header value");
        (status, [(header::CONTENT_TYPE, media_type)], Json(self)).into_response()
    }
}

fn problem_type_for_status(status: StatusCode) -> ProblemType {
    match status {
        StatusCode::NOT_FOUND => ProblemType::RouteNotFound,
        StatusCode::METHOD_NOT_ALLOWED => ProblemType::MethodNotAllowed,
        StatusCode::PAYLOAD_TOO_LARGE => ProblemType::BodyTooLarge,
        StatusCode::TOO_MANY_REQUESTS => ProblemType::TooManyRequests,
        StatusCode::BAD_REQUEST => ProblemType::BadRequest,
        StatusCode::UNAUTHORIZED => ProblemType::SessionExpired,
        StatusCode::FORBIDDEN => ProblemType::CapabilityMissing,
        _ if status.is_server_error() => ProblemType::Internal,
        _ => ProblemType::InvalidRequest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_kinds() -> [ProblemType; 13] {
        [
            ProblemType::RouteNotFound,
            ProblemType::MethodNotAllowed,
            ProblemType::InvalidRequest,
            ProblemType::BadRequest,
            ProblemType::MalformedJson,
            ProblemType::BodyTooLarge,
            ProblemType::IdempotencyKeyInvalid,
            ProblemType::CsrfTokenMissing,
            ProblemType::CsrfTokenMismatch,
            ProblemType::TooManyRequests,
            ProblemType::SessionExpired,
            ProblemType::CapabilityMissing,
            ProblemType::Internal,
        ]
    }

    #[test]
    fn every_kind_carries_the_three_required_fields() {
        for problem_type in all_kinds() {
            let problem = Problem::from_type(problem_type);
            let json = serde_json::to_value(&problem).expect("serialize");
            assert_eq!(
                json["type"],
                format!("https://fantatt.org/problems/{}", problem_type.slug())
            );
            assert!(json["title"].is_string());
            assert_eq!(json["status"], problem_type.status().as_u16());
            assert!(json.get("code").is_none());
            assert!(json.get("rule").is_none());
            assert!(json.get("details").is_none());
        }
    }

    #[test]
    fn optional_fields_join_the_envelope_when_a_rule_verdict_needs_them() {
        let problem = Problem::from_type(ProblemType::Internal)
            .with_code("E_IneligiblePlayer")
            .with_rule("https://fantatt.org/rules/roster/1.4.0#slotTierMatchesPlayer")
            .with_details(serde_json::json!([{ "slot": "d1-2" }]));
        let json = serde_json::to_value(&problem).expect("serialize");
        assert_eq!(json["code"], "E_IneligiblePlayer");
        assert_eq!(
            json["rule"],
            "https://fantatt.org/rules/roster/1.4.0#slotTierMatchesPlayer"
        );
        assert_eq!(json["details"][0]["slot"], "d1-2");
    }

    #[test]
    fn status_maps_back_to_the_named_kind() {
        let expectations = [
            (StatusCode::NOT_FOUND, ProblemType::RouteNotFound),
            (
                StatusCode::METHOD_NOT_ALLOWED,
                ProblemType::MethodNotAllowed,
            ),
            (StatusCode::PAYLOAD_TOO_LARGE, ProblemType::BodyTooLarge),
            (StatusCode::TOO_MANY_REQUESTS, ProblemType::TooManyRequests),
            (StatusCode::BAD_REQUEST, ProblemType::BadRequest),
            (StatusCode::UNAUTHORIZED, ProblemType::SessionExpired),
            (StatusCode::FORBIDDEN, ProblemType::CapabilityMissing),
            (StatusCode::INTERNAL_SERVER_ERROR, ProblemType::Internal),
            (StatusCode::CONFLICT, ProblemType::InvalidRequest),
        ];
        for (status, expected_kind) in expectations {
            assert_eq!(
                Problem::from_status(status).status,
                expected_kind.status().as_u16()
            );
        }
    }

    #[tokio::test]
    async fn into_response_sets_the_problem_media_type_and_status() {
        let response = Problem::from_type(ProblemType::BodyTooLarge).into_response();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some(PROBLEM_MEDIA_TYPE)
        );
    }
}
