//! The twelve-hour absolute session cap (docs/10-security.md §1.4).
//!
//! tower-sessions has no absolute-expiry variant, so the creation time rides
//! inside the session data and every request with a live session past the cap
//! is destroyed. The thirty-day sliding window is the layer's
//! `OnInactivity` expiry with `always_save`.

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use time::OffsetDateTime;

use crate::error::{Problem, ProblemType};
use crate::state::AppState;

pub const SESSION_STARTED_AT_KEY: &str = "started_at";

pub async fn enforce_absolute_lifetime(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(session) = live_session(&request) else {
        return next.run(request).await;
    };
    let started_at: Option<i64> = session.get(SESSION_STARTED_AT_KEY).await.unwrap_or(None);
    let cap = i64::from(state.settings().session.absolute_seconds);
    if started_at.is_some_and(|started| unix_now() - started > cap) {
        session.flush().await.ok();
        return Problem::from_type(ProblemType::SessionExpired).into_response();
    }
    if started_at.is_none() {
        session
            .insert(SESSION_STARTED_AT_KEY, unix_now())
            .await
            .ok();
    }
    next.run(request).await
}

/// why: a session without an id belongs to a first-time visitor; acting on it
/// would mint a server-side session for every anonymous request.
fn live_session(request: &Request) -> Option<tower_sessions::Session> {
    let session = request
        .extensions()
        .get::<tower_sessions::Session>()
        .cloned()?;
    session.id().is_some().then_some(session)
}

pub fn unix_now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
