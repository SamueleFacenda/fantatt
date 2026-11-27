//! Knowledge-graph runtime: the graph is a derived projection of the Postgres
//! ledger (docs/adr/0003) held by an embedded Oxigraph store (docs/adr/0002).

pub mod error;
pub mod graph;
pub mod vocabulary;

pub use error::GraphError;
pub use graph::{Answers, Graph};