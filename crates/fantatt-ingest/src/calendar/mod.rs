//! The calendar parser: one `Calendario.asp` page becomes the rounds of
//! matches the championship schedules, each with the matchday links the walk
//! expands and the team score the page prints (docs/06-data-pipeline.md §2).

pub mod error;
pub mod model;
pub mod parse;

pub use error::CalendarParseError;
pub use model::{CalendarLeg, CalendarMatch, CalendarRound, ParsedCalendar};
pub use parse::{CalendarCoordinates, parse_calendar};
