use oxigraph::io::{RdfParseError, RdfSyntaxError};
use oxigraph::model::IriParseError;
use oxigraph::sparql::{QueryEvaluationError, SparqlSyntaxError, UpdateEvaluationError};
use oxigraph::store::{LoaderError, StorageError};

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("the graph store refused the operation: {0}")]
    Store(#[from] StorageError),

    #[error("the graph store could not load the serialized quads: {0}")]
    Load(#[from] LoaderError),

    #[error("the serialized quads are not valid RDF: {0}")]
    RdfParse(#[from] RdfParseError),

    #[error("the serialized quads are not valid RDF: {0}")]
    RdfSyntax(#[from] RdfSyntaxError),

    #[error("the quads could not be written: {0}")]
    Io(#[from] std::io::Error),

    #[error("the SPARQL query is malformed: {0}")]
    SparqlSyntax(#[from] SparqlSyntaxError),

    #[error("the SPARQL query failed to evaluate: {0}")]
    Query(#[from] QueryEvaluationError),

    #[error("the SPARQL update failed to evaluate: {0}")]
    Update(#[from] UpdateEvaluationError),

    #[error("the IRI {iri} is invalid: {source}")]
    Iri { iri: String, source: IriParseError },

    #[error("the {kind} value {value} cannot be rendered as an RDF literal: {source}")]
    Literal {
        kind: &'static str,
        value: String,
        source: time::error::Format,
    },

    #[error("the {kind} format description is invalid: {source}")]
    FormatDescription {
        kind: &'static str,
        source: time::error::InvalidFormatDescription,
    },

    #[error("the query must be a SELECT returning solutions")]
    NotSolutions,

    #[error("the query must be an ASK returning a boolean")]
    NotBoolean,

    #[error("the graph names a blank node, which no game fact uses")]
    BlankGraph,

    #[error("appearance {appearance} belongs to matchday {found}, not {expected}")]
    ForeignAppearance {
        appearance: String,
        found: String,
        expected: String,
    },

    #[error("appearance {appearance} names tie {tie}, which the matchday does not schedule")]
    UnknownTie { appearance: String, tie: String },

    #[error("the snapshot {key} could not be written: {message}")]
    Save { key: String, message: String },

    #[error("the snapshot {key} could not be read: {message}")]
    Read { key: String, message: String },

    #[error("the snapshot {key} is not in the store")]
    Missing { key: String },
}
