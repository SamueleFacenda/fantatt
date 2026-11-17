//! The two session routes the shell answers before authentication exists.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use tower_sessions::Session;
use utoipa::ToSchema;

use crate::error::{Problem, ProblemType};
use crate::middleware::csrf;
use crate::middleware::session_lifetime::{self, SESSION_STARTED_AT_KEY};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct CsrfToken {
    token: String,
}

/// Issues a CSRF token bound to the session, creating the session when the
/// request carries none.
#[utoipa::path(get, path = "/auth/csrf", responses(
    (status = 200, description = "A token bound to the session", body = CsrfToken),
    (status = 500, description = "The session layer is unavailable", body = Problem),
))]
pub async fn issue_csrf_token(
    session: Session,
    State(state): State<AppState>,
) -> Result<Json<CsrfToken>, Problem> {
    session
        .insert(SESSION_STARTED_AT_KEY, session_lifetime::unix_now())
        .await
        .map_err(|_| store_failure())?;
    session.save().await.map_err(|_| store_failure())?;
    let token = csrf::expected_token(&session, state.csrf_key()).ok_or_else(store_failure)?;
    Ok(Json(CsrfToken { token }))
}

/// Logout: destroys the session and its CSRF token.
#[utoipa::path(delete, path = "/auth/session", responses(
    (status = 204, description = "The session is revoked"),
    (status = 403, description = "The CSRF token is missing or wrong", body = Problem),
    (status = 500, description = "The session layer is unavailable", body = Problem),
))]
pub async fn revoke_session(session: Session) -> Result<StatusCode, Problem> {
    session.flush().await.map_err(|_| store_failure())?;
    Ok(StatusCode::NO_CONTENT)
}

fn store_failure() -> Problem {
    Problem::from_type(ProblemType::Internal).with_code("E_SessionStore")
}
