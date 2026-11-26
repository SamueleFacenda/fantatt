//! The athlete parsers: a player detail sheet becomes the athlete it names
//! and the fields it states, and a player search answer becomes the
//! candidates it holds. Every extraction names the portal structure it
//! reads, and every failure is a typed error that locates the selector at
//! fault instead of a panic (docs/06-data-pipeline.md §2, §5).

use scraper::{Html, Selector};

use super::error::{PlayerParseError, PlayerSearchParseError};
use super::model::{NO_ATHLETE_NOTICE, ParsedPlayerDetail, PlayerCandidate, PlayerField};
use crate::html::parse_page;
use crate::names::collapse_whitespace;

/// The selector naming the athlete the sheet names.
const NAME: &str = "p.titolo";

/// The selector naming one field label of the sheet.
const LABEL: &str = "p.style_scheda";

/// The selector naming one field value of the sheet.
const VALUE: &str = "p.style_scheda2";

/// The selector naming the flag image of the athlete.
const FLAG: &str = "img.immagine";

/// The raw candidate one search answer entry carries.
#[derive(serde::Deserialize)]
struct RawCandidate {
    id: String,
    value: String,
    label: String,
}

/// Parse one player detail sheet. why: a classifica that carries no details
/// for the athlete prints the notice instead of the fields, which parses to
/// none rather than to an empty sheet.
pub fn parse_player_detail(
    page: &[u8],
    _coordinates: &super::model::PlayerCoordinates,
) -> Result<Option<ParsedPlayerDetail>, PlayerParseError> {
    let document = parse_page(page);
    if sheet_text(&document).contains(NO_ATHLETE_NOTICE) {
        return Ok(None);
    }
    Ok(Some(ParsedPlayerDetail {
        name: athlete_name(&document)?,
        nationality: nationality(&document),
        fields: fields(&document),
    }))
}

/// The athlete the sheet names in its title line.
fn athlete_name(document: &Html) -> Result<String, PlayerParseError> {
    let selector = Selector::parse(NAME).expect("static selector");
    let name = collapse_whitespace(
        &document
            .select(&selector)
            .next()
            .map(|element| element.text().collect::<String>())
            .unwrap_or_default(),
    );
    if name.is_empty() {
        return Err(PlayerParseError::MissingSelector {
            selector: NAME,
            purpose: "the athlete the sheet names",
        });
    }
    Ok(name)
}

/// The nationality the flag image names, when the sheet carries one.
fn nationality(document: &Html) -> Option<String> {
    let selector = Selector::parse(FLAG).expect("static selector");
    document
        .select(&selector)
        .find_map(|image| image.value().attr("title"))
        .map(str::to_string)
}

/// Every field the sheet states, in the order it prints them.
fn fields(document: &Html) -> Vec<PlayerField> {
    let label_selector = Selector::parse(LABEL).expect("static selector");
    let value_selector = Selector::parse(VALUE).expect("static selector");
    let labels: Vec<String> = document
        .select(&label_selector)
        .map(|label| collapse_whitespace(&label.text().collect::<String>()))
        .collect();
    let mut values = document.select(&value_selector);
    labels
        .into_iter()
        .map(|label| PlayerField {
            label,
            value: values
                .next()
                .map(|value| collapse_whitespace(&value.text().collect::<String>()))
                .unwrap_or_default(),
        })
        .collect()
}

/// The whole text of the sheet, so the notice the portal prints for an
/// unknown athlete is found whatever element carries it.
fn sheet_text(document: &Html) -> String {
    document.root_element().text().collect::<String>()
}

/// Parse one player search answer into the candidates it holds.
pub fn parse_player_search(page: &[u8]) -> Result<Vec<PlayerCandidate>, PlayerSearchParseError> {
    let candidates: Vec<RawCandidate> =
        serde_json::from_slice(page).map_err(|_| PlayerSearchParseError::MalformedJson {
            text: String::from_utf8_lossy(page).into_owned(),
        })?;
    Ok(candidates
        .into_iter()
        .map(|candidate| PlayerCandidate {
            id: candidate.id,
            value: candidate.value,
            label: candidate.label,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinates() -> super::super::model::PlayerCoordinates {
        super::super::model::PlayerCoordinates {
            athlete_id: "737436".to_string(),
            classifica_id: "248".to_string(),
        }
    }

    /// A player detail sheet as the portal prints it: the athlete, the flag,
    /// and the label and value of every field.
    const SHEET: &[u8] = br##"<html><body><div align=center><table>
        <tr><td class="style2"><p class="titolo">&nbsp;&nbsp;KOZUL DENI&nbsp;&nbsp;<img class=immagine src=../../images/bandiere/SI.png title=Slovenia></p></td></tr>
        <tr><td><p class="style_scheda">Dati di Nascita</p></td>
        <td><p class="style_scheda2">18/03/1997 a </p></td></tr>
        <tr><td><p class="style_scheda">Altezza - Peso</p></td>
        <td><p class="style_scheda2"> - </p></td></tr>
        <tr><td><p class="style_scheda">Categoria</p></td>
        <td><p class=style_scheda2>Straniero&nbsp;&nbsp&nbsp;&nbsp;punti 14.247,00</p></td></tr>
        <tr><td><p class="style_scheda">Prima Societa</p></td>
        <td><p class="style_scheda2"></p></td></tr>
    </table></div></body></html>"##;

    /// The notice a classifica prints when it carries no details for the
    /// athlete.
    const ABSENT_SHEET: &[u8] = br#"<html><body>
        <p class="style_scheda">Non sono presenti dettagli per questo atleta in questa classifica!</p>
    </body></html>"#;

    const SEARCH: &[u8] = br#"[{"id":"737436","value":"KOZUL DENI","label":"<span class=\"hl_results\">KOZUL</span> DENI (18/03/1997) [947035]"}]"#;

    /// The fields the sheet states, in the order it prints them.
    const FIELDS: &[&str] = &[
        "Dati di Nascita",
        "Altezza - Peso",
        "Categoria",
        "Prima Societa",
    ];

    #[test]
    fn a_player_sheet_parses_the_athlete_and_its_fields() {
        let parsed = parse_player_detail(SHEET, &coordinates())
            .expect("sheet parses")
            .expect("a detail sheet");
        assert_eq!(parsed.name, "KOZUL DENI");
        assert_eq!(parsed.nationality.as_deref(), Some("Slovenia"));
        let labels: Vec<&str> = parsed
            .fields
            .iter()
            .map(|field| field.label.as_str())
            .collect();
        assert_eq!(labels, FIELDS);
        assert_eq!(parsed.fields[2].value, "Straniero punti 14.247,00");
        assert_eq!(parsed.fields[3].value, "");
    }

    #[test]
    fn a_sheet_without_details_parses_to_none() {
        let parsed = parse_player_detail(ABSENT_SHEET, &coordinates()).expect("sheet parses");
        assert_eq!(parsed, None);
    }

    #[test]
    fn a_sheet_without_the_athlete_fails_naming_the_selector() {
        let page = br#"<html><body><table><tr><td><p class="style_scheda">Dati di Nascita</p></td>
            <td><p class="style_scheda2">18/03/1997</p></td></tr></table></body></html>"#;
        let error = parse_player_detail(page, &coordinates()).expect_err("no athlete");
        assert!(
            matches!(
                error,
                PlayerParseError::MissingSelector {
                    selector: "p.titolo",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_search_answer_parses_its_candidates() {
        let parsed = parse_player_search(SEARCH).expect("search parses");
        assert_eq!(parsed.len(), 1);
        let candidate = &parsed[0];
        assert_eq!(candidate.id, "737436");
        assert_eq!(candidate.value, "KOZUL DENI");
        assert!(
            candidate.label.contains("hl_results"),
            "{}",
            candidate.label
        );
    }

    #[test]
    fn an_answer_that_is_not_json_fails_naming_the_text() {
        let error = parse_player_search(b"<html>not json</html>").expect_err("not json");
        assert!(
            matches!(error, PlayerSearchParseError::MalformedJson { .. }),
            "{error}"
        );
    }
}
