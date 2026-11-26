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
pub mod championship;
pub mod client;
pub mod conditional;
pub mod discover;
pub mod error;
pub mod giornata;
pub mod html;
pub mod names;
pub mod policy;
pub mod retry;
pub mod settings;
pub mod throttle;
pub mod transport;
pub mod uri;

pub use championship::{
    ChampionshipLink, ChampionshipParseError, ParsedChampionshipList, ParsedRegionMenu,
    RegionCoordinates, parse_championship_list, parse_region_menu,
};
pub use client::{FetchClient, Fetched, FetchedPage};
pub use error::FetchError;
pub use giornata::{
    GiornataCoordinates, GiornataParseError, GiornataRow, ParsedGiornata, RawGiornata,
    parse_giornata,
};
pub use settings::FetchSettings;
pub use transport::Transport;
