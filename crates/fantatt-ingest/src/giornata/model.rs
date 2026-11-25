//! The canonical matchday record. A `RawGiornata` is what one referto page
//! states: the two societies, the day it was played on, where, and every tie
//! between them. `ParsedGiornata` adds the ties the page carried but the
//! matchday cannot count, each with its reason, so a partial page is
//! observable instead of silent (docs/06-data-pipeline.md §3.4).

use serde::{Deserialize, Serialize};
use time::Date;

/// One team matchday as the referto page states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawGiornata {
    /// The `CAM` query value of the championship the matchday belongs to.
    pub championship_id: String,
    /// The `INCONTRO` query value naming this matchday.
    pub contest_id: String,
    /// The day the matchday was played on or is scheduled for; a page
    /// without a confirmed date carries none.
    pub date: Option<Date>,
    pub home_team: String,
    pub away_team: String,
    /// The playing venue, when the referto names one.
    pub venue: Option<String>,
    pub rows: Vec<GiornataRow>,
}

/// One tie of the matchday: two players, or doubles pairs, and their set
/// scores in play order. A kept tie always carries a complete result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GiornataRow {
    /// The ordinal the referto prints in the row's first cell.
    pub match_number: u32,
    /// The `ID` query value of the tie's detail page, when the row links one.
    pub detail_id: Option<String>,
    pub player_a: String,
    pub player_b: String,
    /// `(home, away)` points per set; an odd count of three or five.
    pub sets: Vec<(u8, u8)>,
}

/// One parsed page: the canonical record plus everything the page carried
/// but the record did not keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedGiornata {
    pub giornata: RawGiornata,
    pub skipped: Vec<SkippedTie>,
}

/// One tie the parser read but did not keep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedTie {
    /// The zero-based index of the row inside the referto's tie table.
    pub row: usize,
    pub reason: SkipReason,
}

/// Why a tie did not enter the canonical record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    /// The row is not a tie: a header, a spacer, or prose.
    NotATie,
    /// The tie carries no result yet: every set pair is empty.
    Unplayed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> ParsedGiornata {
        ParsedGiornata {
            giornata: RawGiornata {
                championship_id: "1".to_string(),
                contest_id: "370793".to_string(),
                date: time::Date::from_calendar_date(2026, time::Month::September, 26).ok(),
                ..giornata()
            },
            skipped: vec![SkippedTie {
                row: 7,
                reason: SkipReason::Unplayed,
            }],
        }
    }

    fn giornata() -> RawGiornata {
        RawGiornata {
            championship_id: "1".to_string(),
            contest_id: "370793".to_string(),
            date: time::Date::from_calendar_date(2026, time::Month::September, 26).ok(),
            home_team: "TENNISTAVOLO SASSARI".to_string(),
            away_team: "ASD MURAVERA TT".to_string(),
            venue: Some("PALESTRA SCUOLA MEDIA N 2, CORSO COSSIGA 6".to_string()),
            rows: vec![GiornataRow {
                match_number: 1,
                detail_id: Some("1594362".to_string()),
                player_a: "Puppo Andrea".to_string(),
                player_b: "Kozul Deni".to_string(),
                sets: vec![(6, 11), (5, 11), (9, 11)],
            }],
        }
    }

    #[test]
    fn a_record_round_trips_through_json() {
        let text = serde_json::to_string(&record()).expect("serialize");
        let parsed: ParsedGiornata = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, record());
    }

    #[test]
    fn an_undated_matchday_round_trips_as_a_missing_date() {
        let mut undated = record();
        undated.giornata.date = None;
        let text = serde_json::to_string(&undated).expect("serialize");
        let parsed: ParsedGiornata = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed.giornata.date, None);
    }
}
