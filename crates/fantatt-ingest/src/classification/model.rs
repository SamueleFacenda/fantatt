//! The classification records: the snapshot a classifica header names, and
//! the ranking rows a ranking page holds (docs/06-data-pipeline.md §2).

use serde::{Deserialize, Serialize};
use time::Date;

/// What one classifica header states: the day the snapshot was taken on and
/// the ranking sections the page drives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedClassificationHeader {
    /// The day the ranking snapshot was taken on.
    pub snapshot_date: Date,
    /// The ranking sections the page drives, in the order it prints them.
    pub sections: Vec<RankingSection>,
}

/// One ranking section the header drives with its page handler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankingSection {
    /// The label the page prints, e.g. `MASCHILE`.
    pub label: String,
    /// The title the page prints, e.g. `Maschile Generale`.
    pub title: String,
    /// The handler the page calls for the section, e.g. `getPageGiov`.
    pub handler: String,
    /// The arguments the handler takes: the gender, the classifica id, the
    /// zone, and the section code the page passes.
    pub arguments: Vec<String>,
}

/// What one ranking page states: the snapshot it belongs to and every
/// athlete row it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedClassification {
    /// The title the page prints, e.g. `Classifica Generale maschile`.
    pub title: String,
    /// The day the ranking snapshot was taken on.
    pub snapshot_date: Date,
    /// How many athletes the snapshot holds.
    pub athletes: u32,
    pub rows: Vec<ClassificationRow>,
}

/// One row of a ranking page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificationRow {
    /// The position the page prints.
    pub rank: u32,
    /// The `ATLETA` id the page's handler names.
    pub athlete_id: String,
    pub name: String,
    /// The day the athlete was born on.
    pub birth_date: Date,
    /// The sector the page prints, e.g. `S` for the senior sector.
    pub sector: String,
    /// The points the page prints, e.g. `16716*`: a locale string the
    /// pipeline keeps verbatim rather than parsing into a float.
    pub points: String,
    /// The position change the page prints, e.g. `0`, `1`, or `-1`.
    pub change: String,
    /// The region the athlete belongs to.
    pub region: String,
    /// The category the page prints.
    pub category: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> ClassificationRow {
        ClassificationRow {
            rank: 1,
            athlete_id: "710705".to_string(),
            name: "OYEBODE JOHN MICHAEL".to_string(),
            birth_date: time::Date::from_calendar_date(2002, time::Month::February, 25)
                .expect("a calendar day"),
            sector: "S".to_string(),
            points: "16716*".to_string(),
            change: "0".to_string(),
            region: "Sardegna".to_string(),
            category: "1".to_string(),
        }
    }

    #[test]
    fn a_ranking_page_round_trips_through_json() {
        let page = ParsedClassification {
            title: "Classifica Generale maschile".to_string(),
            snapshot_date: time::Date::from_calendar_date(2026, time::Month::October, 25)
                .expect("a calendar day"),
            athletes: 8894,
            rows: vec![row()],
        };
        let text = serde_json::to_string(&page).expect("serialize");
        let parsed: ParsedClassification = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, page);
    }

    #[test]
    fn a_classifica_header_round_trips_through_json() {
        let header = ParsedClassificationHeader {
            snapshot_date: time::Date::from_calendar_date(2026, time::Month::October, 25)
                .expect("a calendar day"),
            sections: vec![RankingSection {
                label: "MASCHILE".to_string(),
                title: "Maschile Generale".to_string(),
                handler: "getPage".to_string(),
                arguments: vec![
                    "M".to_string(),
                    "248".to_string(),
                    "1".to_string(),
                    "1".to_string(),
                ],
            }],
        };
        let text = serde_json::to_string(&header).expect("serialize");
        let parsed: ParsedClassificationHeader = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, header);
    }
}
