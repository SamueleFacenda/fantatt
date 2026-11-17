//! The OpenAPI 3.1 document, derived from the annotated handlers.

use axum::Json;
use utoipa::OpenApi;

use crate::error::Problem;
use crate::routes::auth::CsrfToken;
use crate::routes::csp_report::{CspReport, ViolationReport};
use crate::routes::health::HealthStatus;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Fantatt API",
        description = "Fantatt fantasy table tennis",
        version = "0.1.0"
    ),
    paths(
        crate::routes::health::healthz,
        crate::routes::auth::issue_csrf_token,
        crate::routes::auth::revoke_session,
        crate::routes::csp_report::report_violation,
        openapi_document,
    ),
    components(schemas(Problem, HealthStatus, CsrfToken, CspReport, ViolationReport))
)]
pub struct ApiDoc;

/// The document itself, served as JSON.
#[utoipa::path(get, path = "/openapi.json", responses(
    (status = 200, description = "The OpenAPI 3.1 document"),
))]
pub async fn openapi_document() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
