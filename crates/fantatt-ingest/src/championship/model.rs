//! The championship catalog: what the regional menus and the championship
//! lists state about the championships of one season
//! (docs/06-data-pipeline.md §2).

use serde::{Deserialize, Serialize};

/// One championship a menu or list page names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChampionshipLink {
    /// The `CAM` query value naming the championship.
    pub championship_id: String,
    /// The `CONC` query value of the section, when the link carries one.
    pub contest_id: Option<String>,
    /// The label the page prints for the championship.
    pub label: String,
}

/// What one regional menu states: the championships of one region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedRegionMenu {
    /// The `REG` query value naming the region.
    pub region_id: String,
    pub championships: Vec<ChampionshipLink>,
}

/// What one championship list states: the season it covers and every
/// championship it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedChampionshipList {
    /// The season the page states, e.g. `2026/27`.
    pub season: String,
    pub championships: Vec<ChampionshipLink>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link() -> ChampionshipLink {
        ChampionshipLink {
            championship_id: "240".to_string(),
            contest_id: Some("1".to_string()),
            label: "C femm gir. A".to_string(),
        }
    }

    #[test]
    fn a_regional_menu_round_trips_through_json() {
        let menu = ParsedRegionMenu {
            region_id: "2".to_string(),
            championships: vec![link()],
        };
        let text = serde_json::to_string(&menu).expect("serialize");
        let parsed: ParsedRegionMenu = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, menu);
    }

    #[test]
    fn a_championship_list_round_trips_through_json() {
        let list = ParsedChampionshipList {
            season: "2026/27".to_string(),
            championships: vec![link()],
        };
        let text = serde_json::to_string(&list).expect("serialize");
        let parsed: ParsedChampionshipList = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, list);
    }

    #[test]
    fn a_championship_without_a_section_round_trips_as_none() {
        let mut sectionless = link();
        sectionless.contest_id = None;
        let text = serde_json::to_string(&sectionless).expect("serialize");
        let parsed: ChampionshipLink = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed.contest_id, None);
    }
}
