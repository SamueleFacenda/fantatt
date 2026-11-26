//! The athlete parsers: the player detail sheet a classifica names, and the
//! candidate list the player search answers (docs/06-data-pipeline.md §2).

pub mod error;
pub mod model;
pub mod parse;

pub use error::{PlayerParseError, PlayerSearchParseError};
pub use model::{ParsedPlayerDetail, PlayerCandidate, PlayerCoordinates, PlayerField};
pub use parse::{parse_player_detail, parse_player_search};
