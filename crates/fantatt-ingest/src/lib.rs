//! FITET ingestion: the polite fetch facade over the results portal.
//!
//! Discovery, matchday/calendar/classification parsing, and identity
//! reconciliation all live here; pure of framework logic where possible. The
//! fetch facade bounds every outbound request with a concurrency semaphore, a
//! per-host token bucket, jitter, bounded retries with exponential backoff, a
//! circuit breaker, conditional requests, and a host allow-list, so a slow or
//! broken portal can never take the pipeline down (docs/06-data-pipeline.md
//! §3.3).
#![forbid(unsafe_code)]

pub mod archive;
pub mod breaker;
pub mod calendar;
pub mod championship;
pub mod classification;
pub mod client;
pub mod conditional;
pub mod discover;
pub mod error;
pub mod giornata;
pub mod html;
pub mod names;
pub mod player;
pub mod policy;
pub mod retry;
pub mod settings;
pub mod throttle;
pub mod tournament;
pub mod transport;
pub mod uri;

pub use calendar::{
    CalendarCoordinates, CalendarLeg, CalendarMatch, CalendarParseError, CalendarRound,
    ParsedCalendar, parse_calendar,
};
pub use championship::{
    ChampionshipLink, ChampionshipParseError, ParsedChampionshipList, ParsedRegionMenu,
    RegionCoordinates, parse_championship_list, parse_region_menu,
};
pub use classification::{
    ClassificationCoordinates, ClassificationHeaderCoordinates, ClassificationParseError,
    ClassificationRow, ParsedClassification, ParsedClassificationHeader, RankingSection,
    parse_classification, parse_classification_header,
};
pub use client::{FetchClient, Fetched, FetchedPage};
pub use error::FetchError;
pub use giornata::{
    GiornataCoordinates, GiornataParseError, GiornataRow, ParsedGiornata, RawGiornata,
    parse_giornata,
};
pub use player::{
    ParsedPlayerDetail, PlayerCandidate, PlayerCoordinates, PlayerField, PlayerParseError,
    PlayerSearchParseError, parse_player_detail, parse_player_search,
};
pub use settings::FetchSettings;
pub use tournament::{
    BracketDocument, BracketHomeCoordinates, BracketSection, ParsedBracketHome,
    ParsedTournamentHeader, ParsedTournamentList, TournamentHeaderCoordinates, TournamentLink,
    TournamentListCoordinates, TournamentParseError, parse_bracket_home, parse_tournament_header,
    parse_tournament_list,
};
pub use transport::Transport;
