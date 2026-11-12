//! Domain core: value objects, aggregates, and typed invariants with no I/O.
//!
//! fantatt-core depends only on serde, thiserror, and time. No HTTP, database,
//! clock, or async runtime enters this crate (docs/11-repo-structure.md §3).
#![forbid(unsafe_code)]

pub mod division;
pub mod ids;
pub mod money;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {}
}
