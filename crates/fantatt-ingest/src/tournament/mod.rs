//! The tournament parsers: the tournament type header, the committee list of
//! tournaments, and the bracket home that lists the sections a tournament ran
//! (docs/06-data-pipeline.md §2).

pub mod error;
pub mod model;
pub mod parse;

pub use error::TournamentParseError;
pub use model::{
    BracketDocument, BracketSection, ParsedBracketHome, ParsedTournamentHeader,
    ParsedTournamentList, TournamentLink,
};
pub use parse::{
    BracketHomeCoordinates, TournamentHeaderCoordinates, TournamentListCoordinates,
    parse_bracket_home, parse_tournament_header, parse_tournament_list,
};
