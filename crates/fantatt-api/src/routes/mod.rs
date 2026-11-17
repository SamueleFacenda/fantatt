//! The router: routes first, route-level middleware inside the session layer,
//! then the global layers outward.

use axum::Router;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{delete, get, post};

use crate::error::{Problem, ProblemType};
use crate::middleware;
use crate::sessions;
use crate::state::AppState;

pub mod auth;
pub mod csp_report;
pub mod health;

pub fn build_router(state: AppState) -> Router {
    let routes = shell_routes(state.clone());
    let routes = with_route_middleware(routes, &state);
    harden(routes, state)
}

fn shell_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/auth/csrf", get(auth::issue_csrf_token))
        .route(
            "/auth/session",
            delete(auth::revoke_session)
                .layer(from_fn_with_state(state, middleware::csrf::enforce_csrf)),
        )
        .route("/api/v1/csp-report", post(csp_report::report_violation))
        .route("/openapi.json", get(crate::openapi::openapi_document))
}

fn with_route_middleware(router: Router<AppState>, state: &AppState) -> Router<AppState> {
    router
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::session_lifetime::enforce_absolute_lifetime,
        ))
        .route_layer(from_fn_with_state(
            state.clone(),
            crate::rate_limit::enforce_rate_limits,
        ))
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::body_limit::reject_oversized_bodies,
        ))
        .route_layer(from_fn(middleware::idempotency::normalize_idempotency_key))
        .route_layer(middleware::body_limit::body_limit_layer(state))
}

fn harden(router: Router<AppState>, state: AppState) -> Router {
    router
        .layer(sessions::session_layer(&state))
        .fallback(route_not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(from_fn(
            middleware::problem_envelope::enforce_problem_envelope,
        ))
        .layer(from_fn(
            middleware::security_headers::attach_security_headers,
        ))
        .layer(from_fn(middleware::request_id::attach_request_id))
        .with_state(state)
}

async fn route_not_found() -> Problem {
    Problem::from_type(ProblemType::RouteNotFound)
}

async fn method_not_allowed() -> Problem {
    Problem::from_type(ProblemType::MethodNotAllowed)
}
