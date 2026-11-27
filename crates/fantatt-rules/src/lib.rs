//! Rule bundles: game logic as immutable, content-addressed artifacts.
//!
//! A bundle is a directory `rules/<name>/<semver>/` whose `manifest.toml`
//! declares the entry points an engine may call. Loading a bundle hashes its
//! manifest and every file it carries into one ruleset version, so a derived
//! score always names the exact rules that produced it and an edited rule
//! reads as a new version rather than a silent change
//! (docs/04-reasoning-rules.md §3).

pub mod error;
pub mod manifest;

pub use error::BundleError;
pub use manifest::{Evidence, Manifest};
