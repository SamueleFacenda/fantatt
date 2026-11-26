//! The calendar: what one `Calendario.asp` page schedules. A `ParsedCalendar`
//! holds the championship it belongs to and every round of matches, each with
//! the two matchday links the walk expands and the team score the page prints
//! (docs/06-data-pipeline.md §2).

use serde::{Deserialize, Serialize};
use time::Date;

/// What one calendar page states: the championship it covers and the rounds
/// of matches it schedules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCalendar {
    /// The `CAM` query value of the championship.
    pub championship_id: String,
    /// The `ANNO` query value naming the season year.
    pub season_year: String,
    /// The label the page prints, e.g. `Serie A/1 Gir.U`.
    pub label: String,
    pub rounds: Vec<CalendarRound>,
}

/// One round of the calendar: the ordinal the page prints and the matches
/// scheduled in it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarRound {
    /// The ordinal the page prints for `N^ Giornata`.
    pub number: u32,
    pub matches: Vec<CalendarMatch>,
}

/// One scheduled match: the two societies and the two legs between them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarMatch {
    pub home_team: String,
    pub away_team: String,
    /// The leg played at the home society.
    pub home: CalendarLeg,
    /// The return leg, played at the away society.
    pub away: CalendarLeg,
}

/// One leg of a match: the matchday it is recorded on, when it is played, and
/// the team score the calendar prints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalendarLeg {
    /// The `INCONTRO` query value naming this leg's matchday.
    pub contest_id: String,
    /// The day the leg was played on or is scheduled for; a postponed leg
    /// (`Rinviata`) carries none.
    pub date: Option<Date>,
    /// The clock time the page prints, e.g. `16.00`.
    pub time: Option<String>,
    /// The `(home, away)` team set score, or None while the leg is unplayed.
    pub score: Option<(u8, u8)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leg() -> CalendarLeg {
        CalendarLeg {
            contest_id: "370793".to_string(),
            date: time::Date::from_calendar_date(2026, time::Month::September, 26).ok(),
            time: Some("16.00".to_string()),
            score: Some((0, 3)),
        }
    }

    fn calendar() -> ParsedCalendar {
        ParsedCalendar {
            championship_id: "1".to_string(),
            season_year: "41".to_string(),
            label: "Serie A/1 Gir.U".to_string(),
            rounds: vec![CalendarRound {
                number: 1,
                matches: vec![CalendarMatch {
                    home_team: "TENNISTAVOLO SASSARI".to_string(),
                    away_team: "ASD MURAVERA TT".to_string(),
                    home: leg(),
                    away: CalendarLeg {
                        contest_id: "392017".to_string(),
                        date: None,
                        time: None,
                        score: None,
                    },
                }],
            }],
        }
    }

    #[test]
    fn a_calendar_round_trips_through_json() {
        let text = serde_json::to_string(&calendar()).expect("serialize");
        let parsed: ParsedCalendar = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, calendar());
    }

    #[test]
    fn an_unplayed_leg_round_trips_without_a_score() {
        let text = serde_json::to_string(&calendar()).expect("serialize");
        let parsed: ParsedCalendar = serde_json::from_str(&text).expect("parse");
        let away = &parsed.rounds[0].matches[0].away;
        assert_eq!(away.score, None);
        assert_eq!(away.date, None);
        assert_eq!(away.time, None);
    }
}
