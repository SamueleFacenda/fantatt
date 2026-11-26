//! The tournament records: the type header of the tournament menus, the
//! tournaments a committee list names, and the sections a bracket home lists
//! (docs/06-data-pipeline.md §2).

use serde::{Deserialize, Serialize};
use time::Date;

/// What one tournament type header states: the family of tournaments and
/// the season it covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedTournamentHeader {
    /// The family the header names, e.g. `TORNEI OPEN`.
    pub family: String,
    /// The season the header names, e.g. `2026/27`.
    pub season: String,
}

/// What one committee list states: every tournament the committee ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedTournamentList {
    pub tournaments: Vec<TournamentLink>,
}

/// One tournament a committee list names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TournamentLink {
    /// The `IDT` query value naming the tournament.
    pub tournament_id: String,
    /// The `ANNO` query value naming the season year.
    pub season_year: String,
    /// The `COM` query value of the committee that ran it.
    pub committee: String,
    /// The day the tournament was played on.
    pub date: Date,
    /// The place the tournament was played in, e.g. `Cortemaggiore (PC)`.
    pub place: String,
}

/// What one bracket home states: the tournament, when and where it was
/// played, and every section it ran.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedBracketHome {
    /// The tournament the page names, e.g. `Torneo Open Trentino`.
    pub tournament: String,
    /// The place the tournament was played in.
    pub place: String,
    /// The day the tournament was played on.
    pub date: Date,
    /// The sections the tournament ran, in the order the page lists them.
    pub sections: Vec<BracketSection>,
    /// The bracket documents the page links, e.g. the final tabelloni.
    pub documents: Vec<BracketDocument>,
}

/// One section of a tournament, named by the `GARA` query value of its
/// bracket page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BracketSection {
    /// The `GARA` query value naming the section.
    pub gara_id: String,
    /// The label the page prints, e.g. `Singolo M/F Open2 121-1500M`.
    pub label: String,
}

/// One document the bracket home links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BracketDocument {
    /// The href the page prints, e.g. `tabelloni/1_6_home.html`.
    pub href: String,
    /// The label the page prints, e.g. `Visualizza tabelloni finali`.
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The day the recorded tournaments were played on.
    fn day() -> Date {
        time::Date::from_calendar_date(2026, time::Month::September, 19).expect("a calendar day")
    }

    fn tournament() -> TournamentLink {
        TournamentLink {
            tournament_id: "1".to_string(),
            season_year: "41".to_string(),
            committee: "6".to_string(),
            date: day(),
            place: "Trento".to_string(),
        }
    }

    #[test]
    fn a_tournament_list_round_trips_through_json() {
        let list = ParsedTournamentList {
            tournaments: vec![tournament()],
        };
        let text = serde_json::to_string(&list).expect("serialize");
        let parsed: ParsedTournamentList = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, list);
    }

    #[test]
    fn a_tournament_header_round_trips_through_json() {
        let header = ParsedTournamentHeader {
            family: "TORNEI OPEN".to_string(),
            season: "2026/27".to_string(),
        };
        let text = serde_json::to_string(&header).expect("serialize");
        let parsed: ParsedTournamentHeader = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, header);
    }

    #[test]
    fn a_bracket_home_round_trips_through_json() {
        let home = ParsedBracketHome {
            tournament: "Torneo Open Trentino".to_string(),
            place: "Trento".to_string(),
            date: day(),
            sections: vec![BracketSection {
                gara_id: "1021".to_string(),
                label: "Singolo M/F Open2 121-1500M | 9-100F".to_string(),
            }],
            documents: vec![BracketDocument {
                href: "tabelloni/1_6_home.html".to_string(),
                label: "Visualizza tabelloni finali".to_string(),
            }],
        };
        let text = serde_json::to_string(&home).expect("serialize");
        let parsed: ParsedBracketHome = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, home);
    }
}
