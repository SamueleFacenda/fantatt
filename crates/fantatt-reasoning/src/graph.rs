use std::path::Path;

use oxigraph::io::{RdfFormat, RdfSerializer};
use oxigraph::model::{NamedNode, NamedOrBlankNode, Quad, QuadRef, Term};
use oxigraph::sparql::{QueryResults, QuerySolution, SparqlEvaluator};
use oxigraph::store::Store;

use crate::error::GraphError;

/// The materialized answer of a SELECT query: one row per solution.
#[derive(Debug, Default)]
pub struct Answers {
    solutions: Vec<QuerySolution>,
}

impl Answers {
    pub fn len(&self) -> usize {
        self.solutions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.solutions.is_empty()
    }

    pub fn rows(&self) -> &[QuerySolution] {
        &self.solutions
    }

    pub fn first(&self) -> Option<&QuerySolution> {
        self.solutions.first()
    }

    pub fn terms(&self, variable: &str) -> Vec<&Term> {
        self.solutions
            .iter()
            .filter_map(|solution| solution.get(variable))
            .collect()
    }
}

/// One Oxigraph store: the in-process projection of the ledger.
pub struct Graph {
    store: Store,
}

impl Graph {
    pub fn in_memory() -> Result<Self, GraphError> {
        Store::new()
            .map_err(GraphError::Store)
            .map(|store| Self { store })
    }

    pub fn on_disk(path: impl AsRef<Path>) -> Result<Self, GraphError> {
        Store::open(path)
            .map_err(GraphError::Store)
            .map(|store| Self { store })
    }

    pub fn select(&self, sparql: &str) -> Result<Answers, GraphError> {
        let results = self.run_query(sparql)?;
        match results {
            QueryResults::Solutions(solutions) => {
                let rows = solutions
                    .map(|solution| solution.map_err(GraphError::Query))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Answers { solutions: rows })
            }
            _ => Err(GraphError::NotSolutions),
        }
    }

    pub fn ask(&self, sparql: &str) -> Result<bool, GraphError> {
        match self.run_query(sparql)? {
            QueryResults::Boolean(answer) => Ok(answer),
            _ => Err(GraphError::NotBoolean),
        }
    }

    pub fn quads(&self) -> Result<Vec<Quad>, GraphError> {
        self.store
            .quads_for_pattern(None, None, None, None)
            .collect::<Result<Vec<_>, _>>()
            .map_err(GraphError::Store)
    }

    pub fn quad_count(&self) -> Result<usize, GraphError> {
        Ok(self.quads()?.len())
    }

    pub fn named_graphs(&self) -> Result<Vec<NamedNode>, GraphError> {
        self.store
            .named_graphs()
            .collect::<Result<Vec<_>, _>>()
            .map_err(GraphError::Store)?
            .into_iter()
            .map(|graph| match graph {
                NamedOrBlankNode::NamedNode(named) => Ok(named),
                NamedOrBlankNode::BlankNode(_) => Err(GraphError::BlankGraph),
            })
            .collect()
    }

    pub fn has_graph(&self, name: &NamedNode) -> Result<bool, GraphError> {
        self.store
            .contains_named_graph(name)
            .map_err(GraphError::Store)
    }

    pub fn remove_graph(&self, name: &NamedNode) -> Result<(), GraphError> {
        self.store
            .remove_named_graph(name)
            .map_err(GraphError::Store)
    }

    pub fn clear(&self) -> Result<(), GraphError> {
        for graph in self.named_graphs()? {
            self.remove_graph(&graph)?;
        }
        for quad in self.quads()? {
            self.store
                .remove(QuadRef::from(&quad))
                .map_err(GraphError::Store)?;
        }
        Ok(())
    }

    /// why: load_from_slice commits one storage transaction, so a derivation
    /// that fails while building its quads leaves no partial graph behind.
    pub fn insert_quads(&self, quads: &[Quad]) -> Result<(), GraphError> {
        let bytes = self.serialize_quads(quads)?;
        self.store
            .load_from_slice(RdfFormat::NQuads, &bytes)
            .map_err(GraphError::Load)
    }

    fn serialize_quads(&self, quads: &[Quad]) -> Result<Vec<u8>, GraphError> {
        let mut lines: Vec<String> = Vec::with_capacity(quads.len());
        for quad in quads {
            let mut serializer =
                RdfSerializer::from_format(RdfFormat::NQuads).for_writer(Vec::new());
            serializer.serialize_quad(QuadRef::from(quad))?;
            let bytes = serializer.finish()?;
            let line = String::from_utf8(bytes).map_err(|_| GraphError::NotSolutions)?;
            lines.push(line);
        }
        lines.sort();
        Ok(lines.concat().into_bytes())
    }

    fn run_query(&self, sparql: &str) -> Result<QueryResults<'_>, GraphError> {
        let query = SparqlEvaluator::new()
            .parse_query(sparql)
            .map_err(GraphError::SparqlSyntax)?;
        query
            .on_store(&self.store)
            .execute()
            .map_err(GraphError::Query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{ontology, rdf_term};
    use oxigraph::model::GraphName;

    fn graph() -> Graph {
        Graph::in_memory().expect("graph")
    }

    fn quad(subject: &str) -> Quad {
        Quad::new(
            oxigraph::model::NamedNode::new(subject).expect("iri"),
            rdf_term("type").expect("predicate"),
            ontology("Matchday").expect("object"),
            GraphName::DefaultGraph,
        )
    }

    #[test]
    fn an_in_memory_graph_starts_empty() {
        let graph = graph();
        assert_eq!(graph.quad_count().expect("count"), 0);
        assert!(graph.named_graphs().expect("graphs").is_empty());
    }

    #[test]
    fn quads_land_atomically_and_are_queryable() {
        let graph = graph();
        graph
            .insert_quads(&[quad("https://fantatt.org/data/matchday/md_01")])
            .expect("insert");
        assert_eq!(graph.quad_count().expect("count"), 1);
        let answers = graph
            .select("SELECT ?s WHERE { ?s a <https://fantatt.org/ontology/tt#Matchday> }")
            .expect("select");
        assert_eq!(answers.len(), 1);
        assert!(answers.first().expect("row").get("s").is_some());
    }

    #[test]
    fn a_malformed_query_fails_naming_the_syntax_problem() {
        let graph = graph();
        let error = graph.select("SELECT ?s WHERE {").expect_err("bad query");
        assert!(matches!(error, GraphError::SparqlSyntax(_)));
    }

    #[test]
    fn an_ask_query_answers_a_boolean() {
        let graph = graph();
        graph
            .insert_quads(&[quad("https://fantatt.org/data/matchday/md_01")])
            .expect("insert");
        assert!(
            graph
                .ask("ASK { ?s a <https://fantatt.org/ontology/tt#Matchday> }")
                .expect("ask")
        );
        assert!(
            !graph
                .ask("ASK { ?s a <https://fantatt.org/ontology/tt#Tie> }")
                .expect("ask")
        );
    }

    #[test]
    fn clearing_the_graph_removes_every_quad() {
        let graph = graph();
        graph
            .insert_quads(&[quad("https://fantatt.org/data/matchday/md_01")])
            .expect("insert");
        graph.clear().expect("clear");
        assert_eq!(graph.quad_count().expect("count"), 0);
    }
}
