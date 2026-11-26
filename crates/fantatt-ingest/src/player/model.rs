//! The athlete records: what a player detail sheet states about one athlete,
//! and what the player search answers for one term
//! (docs/06-data-pipeline.md §2).

use serde::{Deserialize, Serialize};

use crate::uri::Uri;

/// The coordinates a player detail page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerCoordinates {
    /// The `ATLETA` query value naming the athlete.
    pub athlete_id: String,
    /// The `ID_CLASS` query value naming the classifica the sheet reads.
    pub classifica_id: String,
}

impl PlayerCoordinates {
    /// Read the coordinates off a player detail URI; none when the query
    /// names no athlete or no classifica.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        Some(Self {
            athlete_id: uri.param("ATLETA")?.to_string(),
            classifica_id: uri.param("ID_CLASS")?.to_string(),
        })
    }
}

/// What one player detail sheet states. why: a classifica that carries no
/// details for an athlete parses to none, because the sheet prints the notice
/// instead of the fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedPlayerDetail {
    /// The athlete the sheet names.
    pub name: String,
    /// The nationality the flag image names, when the sheet carries one.
    pub nationality: Option<String>,
    /// The fields the sheet states, in the order it prints them.
    pub fields: Vec<PlayerField>,
}

/// One label and the value the sheet prints for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerField {
    /// The label the sheet prints, e.g. `Categoria`.
    pub label: String,
    /// The value the sheet prints, e.g. `Straniero punti 14.247,00`: kept
    /// verbatim, locale decimal comma included.
    pub value: String,
}

/// One candidate the player search answers for a term.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCandidate {
    /// The `ATLETA` id the candidate carries.
    pub id: String,
    /// The name the portal holds for the candidate.
    pub value: String,
    /// The label the search prints, with the marked match in its HTML.
    pub label: String,
}

/// The notice a sheet prints when the classifica carries no details for the
/// athlete.
pub const NO_ATHLETE_NOTICE: &str = "Non sono presenti dettagli per questo atleta";

#[cfg(test)]
mod tests {
    use super::*;

    use crate::uri::Uri;

    #[test]
    fn a_player_detail_round_trips_through_json() {
        let detail = ParsedPlayerDetail {
            name: "KOZUL DENI".to_string(),
            nationality: Some("Slovenia".to_string()),
            fields: vec![PlayerField {
                label: "Categoria".to_string(),
                value: "Straniero punti 14.247,00".to_string(),
            }],
        };
        let text = serde_json::to_string(&detail).expect("serialize");
        let parsed: ParsedPlayerDetail = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, detail);
    }

    #[test]
    fn a_candidate_round_trips_through_json() {
        let candidate = PlayerCandidate {
            id: "737436".to_string(),
            value: "KOZUL DENI".to_string(),
            label: "<span class=\"hl_results\">KOZUL</span> DENI (18/03/1997) [947035]".to_string(),
        };
        let text = serde_json::to_string(&candidate).expect("serialize");
        let parsed: PlayerCandidate = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, candidate);
    }

    #[test]
    fn the_coordinates_read_the_athlete_and_the_classifica_off_the_uri() {
        let uri = crate::uri::Uri::parse(
            "https://portale.fitet.org/risultati/new_rank/dettaglioatleta_unica.php?ATLETA=737436&ID_CLASS=248&ZU=1",
        )
        .expect("url");
        let found = PlayerCoordinates::from_uri(&uri).expect("coordinates");
        assert_eq!(found.athlete_id, "737436");
        assert_eq!(found.classifica_id, "248");
        let without = Uri::parse(
            "https://portale.fitet.org/risultati/new_rank/dettaglioatleta_unica.php?ATLETA=737436",
        )
        .expect("url");
        assert_eq!(PlayerCoordinates::from_uri(&without), None);
    }
}
