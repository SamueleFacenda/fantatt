//! The championship catalog parsers: the regional menus and the national
//! championship lists become the championship identifiers the walk expands
//! (docs/06-data-pipeline.md §2).

pub mod error;
pub mod model;
pub mod parse;

pub use error::ChampionshipParseError;
pub use model::{ChampionshipLink, ParsedChampionshipList, ParsedRegionMenu};
pub use parse::{RegionCoordinates, parse_championship_list, parse_region_menu};
