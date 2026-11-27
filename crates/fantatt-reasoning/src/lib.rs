//! Knowledge-graph runtime: the graph is a derived projection of the Postgres
//! ledger (docs/adr/0003) held by an embedded Oxigraph store (docs/adr/0002).
//!
//! An asserted matchday document lands as one named-graph commit: the commit
//! id is the SHA-256 of the canonical serialisation of the document, and the
//! whole write is one storage transaction, so a document that fails
//! validation leaves the graph exactly as it was. The store snapshots as
//! canonical N-Quads behind a storage facade and rebuilds byte-identically
//! from the assertion log.

pub mod assert;
pub mod commit;
pub mod error;
pub mod graph;
pub mod rebuild;
pub mod snapshot;
pub mod vocabulary;

pub use assert::{Asserted, MatchdayFacts, TieFacts, assert_matchday};
pub use commit::{Commit, CommitId};
pub use error::GraphError;
pub use graph::{Answers, Graph};
pub use rebuild::{Ledger, RebuildReport, rebuild};
pub use snapshot::{MemorySnapshotStore, SnapshotStore, canonical_snapshot, hydrate, save};
