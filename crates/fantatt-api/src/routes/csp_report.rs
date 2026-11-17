//! The browser CSP report endpoint.

use axum::extract::Json;
use axum::http::StatusCode;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::Problem;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CspReport {
    #[serde(rename = "csp-report")]
    pub report: ViolationReport,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ViolationReport {
    #[serde(rename = "blocked-uri")]
    pub blocked_uri: Option<String>,
    #[serde(rename = "document-uri")]
    pub document_uri: Option<String>,
    #[serde(rename = "effective-directive")]
    pub effective_directive: Option<String>,
    #[serde(rename = "original-policy")]
    pub original_policy: Option<String>,
    #[serde(rename = "violated-directive")]
    pub violated_directive: Option<String>,
}

/// Receives a browser CSP violation report. Reports arrive from browsers
/// without a CSRF token and change no state, so the mutating-route CSRF guard
/// does not apply here.
#[utoipa::path(post, path = "/api/v1/csp-report", request_body = CspReport, responses(
    (status = 204, description = "The report is accepted"),
    (status = 400, description = "The report is malformed", body = Problem),
))]
pub async fn report_violation(Json(report): Json<CspReport>) -> Result<StatusCode, Problem> {
    tracing::info!(
        effective_directive = ?report.report.effective_directive,
        blocked_uri = ?report.report.blocked_uri,
        "csp violation reported"
    );
    Ok(StatusCode::NO_CONTENT)
}
