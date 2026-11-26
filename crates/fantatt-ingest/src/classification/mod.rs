//! The classification parsers: the classifica header that names the snapshot
//! day and the ranking sections, and the ranking pages the header drives
//! (docs/06-data-pipeline.md §2).

pub mod error;
pub mod model;
pub mod parse;

pub use error::ClassificationParseError;
pub use model::{
    ClassificationRow, ParsedClassification, ParsedClassificationHeader, RankingSection,
};
pub use parse::{
    ClassificationCoordinates, ClassificationHeaderCoordinates, parse_classification,
    parse_classification_header,
};
