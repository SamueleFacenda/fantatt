//! Layered configuration loading and startup validation.
//!
//! Loads defaults, file, environment, and CLI layers into typed settings and
//! fails fast on missing secrets or invalid combinations.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
