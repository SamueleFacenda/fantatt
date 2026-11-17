//! The liveness probe the whole stack answers with.

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthStatus {
    status: &'static str,
}

/// Liveness probe: the hardened shell answers this and nothing else without
/// credentials.
#[utoipa::path(get, path = "/healthz", responses(
    (status = 200, description = "The service is up", body = HealthStatus),
))]
pub async fn healthz() -> Json<HealthStatus> {
    Json(HealthStatus { status: "ok" })
}
