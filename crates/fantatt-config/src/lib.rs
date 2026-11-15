//! Layered configuration loading and startup validation.
//!
//! Loads defaults, file, environment, and CLI layers into typed settings and
//! fails fast on missing secrets or invalid combinations.

pub mod cli;
pub mod environment;
pub mod error;
pub mod loader;
pub mod overrides;
pub mod secret;
pub mod settings;
pub mod validation;
