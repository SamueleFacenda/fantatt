//! HTTP surface: axum handlers, security middleware, and OpenAPI document.
//!
//! This crate (with fantatt-jobs and fantatt-notify) is the only place that
//! knows about HTTP, the database, and the wall clock.

pub mod authz;
pub mod error;
pub mod middleware;
pub mod openapi;
pub mod rate_limit;
pub mod routes;
pub mod sessions;
pub mod state;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
