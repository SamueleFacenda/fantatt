//! Domain core: value objects, aggregates, and typed invariants with no I/O.
//!
//! fantatt-core depends only on serde, thiserror, and time. No HTTP, database,
//! clock, or async runtime enters this crate (docs/11-repo-structure.md §3).
#![forbid(unsafe_code)]

pub mod appearance;
pub mod auction;
pub mod calendar;
pub mod derivation;
pub mod division;
pub mod error;
pub mod identity;
pub mod idempotency;
pub mod ids;
pub mod league;
pub mod matchday;
pub mod membership;
pub mod money;
pub mod player;
pub mod purge;
pub mod rating;
pub mod squad;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
