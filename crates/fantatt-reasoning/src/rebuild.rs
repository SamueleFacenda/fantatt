use crate::assert::{MatchdayFacts, assert_matchday};
use crate::error::GraphError;
use crate::graph::Graph;

/// The ledger view the projection rebuilds from: the asserted documents, in
/// the order the ledger recorded them.
///
/// Postgres stays the system of record (docs/adr/0003); the graph is a
/// projection, so any store loss is repaired by replaying this log.
pub trait Ledger {
    fn documents(&self) -> Vec<MatchdayFacts>;
}

/// What one rebuild committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RebuildReport {
    pub documents: usize,
    pub commits: usize,
}

/// Clears the graph and re-asserts every ledger document, leaving the store
/// byte-identical to the run that asserted them one by one.
pub fn rebuild(graph: &Graph, ledger: &impl Ledger) -> Result<RebuildReport, GraphError> {
    graph.clear()?;
    let documents = ledger.documents();
    for facts in &documents {
        assert_matchday(graph, facts)?;
    }
    Ok(RebuildReport {
        documents: documents.len(),
        commits: graph.named_graphs()?.len(),
    })
}
