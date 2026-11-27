use std::collections::HashMap;
use std::sync::Mutex;

use oxigraph::io::{RdfFormat, RdfParser, RdfSerializer};
use oxigraph::model::{Quad, QuadRef};

use crate::error::GraphError;
use crate::graph::Graph;

/// Where a snapshot lives: the storage facade the graph writes N-Quads to.
pub trait SnapshotStore {
    fn save(&self, key: &str, bytes: &[u8]) -> Result<(), GraphError>;
    fn load(&self, key: &str) -> Result<Vec<u8>, GraphError>;
    fn exists(&self, key: &str) -> Result<bool, GraphError>;
}

/// A snapshot store holding every snapshot in process memory.
#[derive(Debug, Default)]
pub struct MemorySnapshotStore {
    snapshots: Mutex<HashMap<String, Vec<u8>>>,
}

impl SnapshotStore for MemorySnapshotStore {
    fn save(&self, key: &str, bytes: &[u8]) -> Result<(), GraphError> {
        self.snapshots
            .lock()
            .map_err(|_| poisoned())?
            .insert(key.to_owned(), bytes.to_vec());
        Ok(())
    }

    fn load(&self, key: &str) -> Result<Vec<u8>, GraphError> {
        self.snapshots
            .lock()
            .map_err(|_| poisoned())?
            .get(key)
            .cloned()
            .ok_or_else(|| GraphError::Missing {
                key: key.to_owned(),
            })
    }

    fn exists(&self, key: &str) -> Result<bool, GraphError> {
        Ok(self
            .snapshots
            .lock()
            .map_err(|_| poisoned())?
            .contains_key(key))
    }
}

fn poisoned() -> GraphError {
    GraphError::Read {
        key: String::new(),
        message: "the snapshot store lock was poisoned".to_owned(),
    }
}

/// The whole store as canonical N-Quads: one sorted line per quad, so the
/// same graph state always serializes to identical bytes.
pub fn canonical_snapshot(graph: &Graph) -> Result<Vec<u8>, GraphError> {
    let mut lines: Vec<String> = Vec::new();
    for quad in graph.quads()? {
        let mut serializer = RdfSerializer::from_format(RdfFormat::NQuads).for_writer(Vec::new());
        serializer.serialize_quad(QuadRef::from(&quad))?;
        let bytes = serializer.finish()?;
        lines.push(String::from_utf8(bytes).map_err(|_| GraphError::NotSolutions)?);
    }
    lines.sort();
    Ok(lines.concat().into_bytes())
}

pub fn save(graph: &Graph, store: &impl SnapshotStore, key: &str) -> Result<(), GraphError> {
    let bytes = canonical_snapshot(graph)?;
    store.save(key, &bytes)
}

pub fn hydrate(graph: &Graph, store: &impl SnapshotStore, key: &str) -> Result<(), GraphError> {
    let bytes = store.load(key)?;
    graph.clear()?;
    let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::NQuads)
        .rename_blank_nodes()
        .for_reader(bytes.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(GraphError::RdfParse)?;
    graph.insert_quads(&quads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{ontology, rdf_term};
    use oxigraph::model::{GraphName, NamedNode, Term};

    fn store() -> MemorySnapshotStore {
        MemorySnapshotStore::default()
    }

    fn quad() -> Quad {
        Quad::new(
            NamedNode::new("https://fantatt.org/data/matchday/md_01").expect("iri"),
            rdf_term("type").expect("predicate"),
            Term::from(ontology("Matchday").expect("class")),
            GraphName::DefaultGraph,
        )
    }

    #[test]
    fn a_missing_snapshot_names_its_key() {
        let error = store().load("absent").expect_err("missing snapshot");
        assert!(matches!(error, GraphError::Missing { .. }));
    }

    #[test]
    fn saving_and_hydrating_round_trips_a_snapshot() {
        let graph = Graph::in_memory().expect("graph");
        let snapshot = store();
        graph.insert_quads(&[quad()]).expect("insert");
        save(&graph, &snapshot, "kg/v1").expect("save");
        assert!(snapshot.exists("kg/v1").expect("exists"));
        let bytes = canonical_snapshot(&graph).expect("canonical");
        assert_eq!(snapshot.load("kg/v1").expect("load"), bytes);

        graph.clear().expect("clear");
        hydrate(&graph, &snapshot, "kg/v1").expect("hydrate");
        assert_eq!(canonical_snapshot(&graph).expect("again"), bytes);
    }
}
