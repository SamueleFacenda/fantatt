//! The giornata (matchday) parser: one referto page of the results portal
//! becomes the canonical record the pipeline asserts and scores
//! (docs/06-data-pipeline.md §3.4).

pub mod error;
pub mod model;
pub mod parse;

pub use error::GiornataParseError;
pub use model::{GiornataRow, ParsedGiornata, RawGiornata, SkipReason, SkippedTie};
pub use parse::{GiornataCoordinates, parse_giornata};
