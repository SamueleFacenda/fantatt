//! The router: declared routes first, route-level middleware inside the
//! session layer, then the global layers outward.

use std::sync::Arc;

use axum::Extension;
use axum::Router;
use axum::http::Method;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::delete;

use fantatt_core::authz::Capability;

use crate::authz::{self, DeclaredRouter, Guard, guard};
use crate::error::{Problem, ProblemType};
use crate::middleware;
use crate::sessions;
use crate::state::AppState;

pub mod auth;
pub mod csp_report;
pub mod health;

pub fn build_router(state: AppState) -> Router {
    let (routes, table) = shell_declarations(state.clone()).build();
    protected(routes, &table, state)
}

/// Applies the route-level middleware stack, authorization first among them,
/// then the global layers.
pub fn protected(router: Router<AppState>, table: &authz::RouteTable, state: AppState) -> Router {
    let routes = with_authorization(router, table, state.clone());
    harden(routes, state)
}

/// why: axum wraps each route as the layers are applied, so the declaration
/// table rides the last (outermost) layer and every inner middleware sees it.
fn with_authorization(
    router: Router<AppState>,
    table: &authz::RouteTable,
    state: AppState,
) -> Router<AppState> {
    router
        .route_layer(from_fn_with_state(
            state.clone(),
            middleware::session_lifetime::enforce_absolute_lifetime,
        ))
        .route_layer(from_fn_with_state(
            state.clone(),
            authz::enforce_authorization,
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
        .route_layer(middleware::body_limit::body_limit_layer(&state))
        .layer(Extension(Arc::new(table.clone())))
}

fn shell_declarations(state: AppState) -> DeclaredRouter<AppState> {
    let revoke = delete(auth::revoke_session)
        .layer(from_fn_with_state(state, middleware::csrf::enforce_csrf));
    DeclaredRouter::new()
        .route("/healthz", authz::get(health::healthz, public_guard()))
        .route(
            "/auth/csrf",
            authz::get(auth::issue_csrf_token, public_guard()),
        )
        .route(
            "/auth/session",
            authz::DeclaredRoute::new(Method::DELETE, revoke, public_guard()),
        )
        .route(
            "/api/v1/csp-report",
            authz::post(csp_report::report_violation, public_guard()),
        )
        .route(
            "/openapi.json",
            authz::get(crate::openapi::openapi_document, public_guard()),
        )
}

/// why: the shell routes serve anonymous visitors, so every one of them
/// declares the `public` capability until the first gated route lands.
fn public_guard() -> Guard {
    guard(Capability::Public, authz::platform())
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
