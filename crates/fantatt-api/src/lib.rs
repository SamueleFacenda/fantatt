//! HTTP surface: axum handlers, security middleware, and OpenAPI document.
//!
//! This crate (with fantatt-jobs and fantatt-notify) is the only place that
//! knows about HTTP, the database, and the wall clock.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
