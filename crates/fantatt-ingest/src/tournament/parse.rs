//! The tournament parsers: a type header, a committee list of tournaments,
//! and a bracket home become the canonical records of the model module.
//! Every extraction names the portal structure it reads, and every failure
//! is a typed error that locates the row, cell, or selector at fault instead
//! of a panic (docs/06-data-pipeline.md §2, §5).

use scraper::{ElementRef, Html, Selector};
use time::{Date, Month};

use super::error::TournamentParseError;
use super::model::{
    BracketDocument, BracketSection, ParsedBracketHome, ParsedTournamentHeader,
    ParsedTournamentList, TournamentLink,
};
use crate::html::parse_page;
use crate::names::collapse_whitespace;
use crate::uri::Uri;

/// The selector naming the tournament table of a list page.
const TABLE: &str = "table";

/// The selector naming the season line of a type header.
const HEADING: &str = "p.stile";

/// The selector naming one tournament link of a committee list.
const LIST_LINK: &str = "p.dettagli a";

/// The selector naming the identity lines of a bracket home.
const IDENTITY: &str = "p.dettagli b";

/// The selector naming one section link of a bracket home.
const SECTION_LINK: &str = "p.dettagli a";

/// The selector naming one bracket document link of a bracket home.
const DOCUMENT_LINK: &str = "p.sint a";

/// The coordinates a tournament list or header page carries in its query
/// string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TournamentListCoordinates {
    /// The `TOR` query value naming the tournament type.
    pub tournament_type: String,
    /// The `COMIT` query value naming the committee.
    pub committee: String,
}

impl TournamentListCoordinates {
    /// Read the coordinates off a list page URI; none when the query names
    /// no tournament type or no committee.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        Some(Self {
            tournament_type: uri.param("TOR")?.to_string(),
            committee: uri.param("COMIT")?.to_string(),
        })
    }
}

/// The coordinates a type header page carries in its query string.
pub type TournamentHeaderCoordinates = TournamentListCoordinates;

/// The coordinates a bracket home page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketHomeCoordinates {
    /// The `IDT` query value naming the tournament.
    pub tournament_id: String,
    /// The `COM` query value of the committee that ran it.
    pub committee: String,
}

impl BracketHomeCoordinates {
    /// Read the coordinates off a bracket home URI; none when the query
    /// names no tournament or no committee.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        Some(Self {
            tournament_id: uri.param("IDT")?.to_string(),
            committee: uri.param("COM")?.to_string(),
        })
    }
}

/// Parse one tournament list page. A committee that ran no tournament prints
/// an empty list page, which parses to an empty catalog.
pub fn parse_tournament_list(
    page: &[u8],
    _coordinates: &TournamentListCoordinates,
) -> Result<ParsedTournamentList, TournamentParseError> {
    let document = parse_page(page);
    guard_table(&document)?;
    let selector = Selector::parse(LIST_LINK).expect("static selector");
    let tournaments = document
        .select(&selector)
        .enumerate()
        .map(|(row, link)| tournament(row, link))
        .collect::<Result<Vec<TournamentLink>, TournamentParseError>>()?;
    Ok(ParsedTournamentList { tournaments })
}

/// Parse one tournament type header page into the family it names and the
/// season it covers.
pub fn parse_tournament_header(
    page: &[u8],
    _coordinates: &TournamentHeaderCoordinates,
) -> Result<ParsedTournamentHeader, TournamentParseError> {
    let document = parse_page(page);
    let selector = Selector::parse(HEADING).expect("static selector");
    let text = collapse_whitespace(&joined_text(&document, &selector));
    let (family, season) =
        text.split_once(" - STAGIONE ")
            .ok_or(TournamentParseError::MissingSelector {
                selector: HEADING,
                purpose: "the tournament family and its season",
            })?;
    Ok(ParsedTournamentHeader {
        family: collapse_whitespace(family),
        season: collapse_whitespace(season),
    })
}

/// Parse one bracket home page into the tournament, when and where it was
/// played, and every section it ran. why: the bracket tables stay with the
/// canonical bracket model, so the home yields its links and its identity
/// only (docs/06-data-pipeline.md §3.4).
pub fn parse_bracket_home(
    page: &[u8],
    _coordinates: &BracketHomeCoordinates,
) -> Result<ParsedBracketHome, TournamentParseError> {
    let document = parse_page(page);
    let (tournament, place, date) = identity(&document)?;
    Ok(ParsedBracketHome {
        tournament,
        place,
        date,
        sections: sections(&document)?,
        documents: documents(&document),
    })
}

/// The tournament, its place, and its day, read off the three identity lines
/// a bracket home prints above its sections.
fn identity(document: &Html) -> Result<(String, String, Date), TournamentParseError> {
    let lines = identity_lines(document)?;
    let [_, tournament, place] = lines.as_slice() else {
        return Err(TournamentParseError::MissingSelector {
            selector: IDENTITY,
            purpose: "the tournament, its place, and its day",
        });
    };
    let (place, day) = place
        .split_once(" - ")
        .ok_or(TournamentParseError::MissingSelector {
            selector: IDENTITY,
            purpose: "the place and the day of the tournament",
        })?;
    let day = parse_date(day).ok_or_else(|| TournamentParseError::MalformedDate {
        row: 2,
        text: day.to_string(),
    })?;
    Ok((tournament.clone(), place.to_string(), day))
}

/// The three identity lines a bracket home prints: the page heading, the
/// tournament, and the place and day it was played.
fn identity_lines(document: &Html) -> Result<Vec<String>, TournamentParseError> {
    let selector = Selector::parse(IDENTITY).expect("static selector");
    let lines: Vec<String> = document
        .select(&selector)
        .map(|line| collapse_whitespace(&line.text().collect::<String>()))
        .collect();
    if lines.len() < 3 {
        return Err(TournamentParseError::MissingSelector {
            selector: IDENTITY,
            purpose: "the tournament, its place, and its day",
        });
    }
    Ok(lines)
}

/// The sections the bracket home lists, in the order the page prints them.
fn sections(document: &Html) -> Result<Vec<BracketSection>, TournamentParseError> {
    let selector = Selector::parse(SECTION_LINK).expect("static selector");
    document
        .select(&selector)
        .enumerate()
        .map(|(row, link)| section(row, link))
        .collect()
}

/// The bracket documents the page links, e.g. the final tabelloni.
fn documents(document: &Html) -> Vec<BracketDocument> {
    let selector = Selector::parse(DOCUMENT_LINK).expect("static selector");
    document
        .select(&selector)
        .map(|link| BracketDocument {
            href: link.value().attr("href").unwrap_or_default().to_string(),
            label: collapse_whitespace(&link.text().collect::<String>()),
        })
        .collect()
}

fn tournament(row: usize, link: ElementRef<'_>) -> Result<TournamentLink, TournamentParseError> {
    let href = link.value().attr("href").unwrap_or_default();
    let parts = link_parts(link);
    let [date, place] = parts.as_slice() else {
        return Err(TournamentParseError::MissingCell {
            row,
            purpose: "the day and the place of the tournament",
        });
    };
    Ok(TournamentLink {
        tournament_id: required_param(row, "IDT", href)?,
        season_year: required_param(row, "ANNO", href)?,
        committee: required_param(row, "COM", href)?,
        date: parse_date(date).ok_or_else(|| TournamentParseError::MalformedDate {
            row,
            text: date.clone(),
        })?,
        place: place.clone(),
    })
}

/// The day and the place one tournament link prints, in that order.
fn link_parts(link: ElementRef<'_>) -> Vec<String> {
    link.text()
        .map(collapse_whitespace)
        .filter(|part| !part.is_empty())
        .collect()
}

fn section(row: usize, link: ElementRef<'_>) -> Result<BracketSection, TournamentParseError> {
    let href = link.value().attr("href").unwrap_or_default();
    Ok(BracketSection {
        gara_id: required_param(row, "GARA", href)?,
        label: collapse_whitespace(&link.text().collect::<String>()),
    })
}

fn required_param(
    row: usize,
    name: &'static str,
    href: &str,
) -> Result<String, TournamentParseError> {
    optional_param(href, name).ok_or_else(|| TournamentParseError::MissingParam {
        row,
        param: name,
        href: href.to_string(),
    })
}

/// The value of one query parameter of a raw href, which the pages spell
/// relative to the page that holds them.
fn optional_param(href: &str, name: &str) -> Option<String> {
    let (_, query) = href.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
        .map(str::to_string)
}

/// One day as the portal prints it: `dd/mm/yyyy`.
fn parse_date(text: &str) -> Option<Date> {
    let mut fields = text.trim().split('/');
    let (Some(day), Some(month), Some(year)) = (fields.next(), fields.next(), fields.next()) else {
        return None;
    };
    let (Ok(day), Ok(month), Ok(year)) =
        (day.parse::<u8>(), month.parse::<u8>(), year.parse::<i32>())
    else {
        return None;
    };
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

// why: every catalog page prints its tournaments inside one table, so a page
// without the table is a structure change rather than an empty committee
fn guard_table(document: &Html) -> Result<(), TournamentParseError> {
    let selector = Selector::parse(TABLE).expect("static selector");
    match document.select(&selector).next() {
        Some(_) => Ok(()),
        None => Err(TournamentParseError::MissingSelector {
            selector: TABLE,
            purpose: "the tournament table",
        }),
    }
}

/// The text of every selected element, joined the way the page reads.
fn joined_text(document: &Html, selector: &Selector) -> String {
    document
        .select(selector)
        .map(|element| element.text().collect::<String>())
        .collect::<Vec<String>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The day the recorded tournaments were played on.
    fn day() -> Date {
        time::Date::from_calendar_date(2026, time::Month::September, 19).expect("a calendar day")
    }

    fn uri(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    fn list_coordinates() -> TournamentListCoordinates {
        TournamentListCoordinates {
            tournament_type: "100".to_string(),
            committee: "6".to_string(),
        }
    }

    fn bracket_coordinates() -> BracketHomeCoordinates {
        BracketHomeCoordinates {
            tournament_id: "1".to_string(),
            committee: "6".to_string(),
        }
    }

    /// A committee list that ran two tournaments, as the page prints them.
    const LIST: &[u8] = br#"<html><body><TABLE>
        <tr><td><p class="dettagli">|&nbsp;</p></td><td><p class=dettagli>
        <a href="visualizza_risultati.php?IDT=1&ANNO=41&COM=6" target="inferiore">19/09/2026<br>Trento</a></p></td></tr>
        <tr><td><p class="dettagli">|&nbsp;</p></td><td><p class=dettagli>
        <a href="visualizza_risultati.php?IDT=3&ANNO=41&COM=6" target="inferiore">12/09/2026<br>Cortemaggiore (PC)</a></p></td></tr>
    </TABLE></body></html>"#;

    /// A committee list that ran no tournament: the page holds the table and
    /// the separator the portal prints.
    const EMPTY_LIST: &[u8] = br##"<html><body><TABLE>
        <tr><td width=30><p class="dettagli"><font color="#1D63AB">|&nbsp;</font></p></td></tr>
    </table></body></html>"##;

    /// A type header as the page prints it.
    const HEADER: &[u8] = br#"<html><body>
        <p class="stile" align=center>TORNEI&nbsp;OPEN - STAGIONE&nbsp; 2026/27</p>
    </body></html>"#;

    /// A bracket home as the page prints it: three identity lines, two
    /// bracket documents, and the sections the tournament ran.
    const BRACKET_HOME: &[u8] = br##"<html><body>
        <p class="dettagli" style="text-align: center"><b>Risultati delle gare disputate</b></p>
        <p class="dettagli" style="text-align: center"><b>Torneo Open Trentino</b></p>
        <p class="dettagli" style="text-align: center"><b>Trento - 19/09/2026</b></p>
        <table cellspacing="4" width="260"><tr><td class="style1">
        <p class="sint"><a href="tabelloni/6_1_129.pdf"><font color="#800000">Dati sintetici torneo</font></a></p>
        <p class="sint"><a href="tabelloni/1_6_home.html"><font color="#009900">Visualizza tabelloni finali</font></a></p>
        <p class="sint">- - - - - - - - - -</p></td></tr>
        <tr><td><p class=dettagli><a href="visualizza.php?ANNO=41&GARA=1021&IDT=1&COM=6">Singolo M/F  Open2 121-1500M | 9-100F</a></p></td></tr>
        <tr><td><p class=dettagli><a href="visualizza.php?ANNO=41&GARA=1022&IDT=1&COM=6">Singolo M/F  Open3 451-2500M | 26-200F</a></p></td></tr>
        </table></body></html>"##;

    #[test]
    fn a_tournament_list_parses_its_tournaments() {
        let parsed = parse_tournament_list(LIST, &list_coordinates()).expect("list parses");
        assert_eq!(parsed.tournaments.len(), 2);
        let first = &parsed.tournaments[0];
        assert_eq!(first.tournament_id, "1");
        assert_eq!(first.season_year, "41");
        assert_eq!(first.committee, "6");
        assert_eq!(first.date, day());
        assert_eq!(first.place, "Trento");
        assert_eq!(parsed.tournaments[1].place, "Cortemaggiore (PC)");
    }

    #[test]
    fn an_empty_committee_list_is_a_catalog_without_tournaments() {
        let parsed = parse_tournament_list(EMPTY_LIST, &list_coordinates()).expect("list parses");
        assert!(parsed.tournaments.is_empty());
    }

    #[test]
    fn a_type_header_parses_its_family_and_season() {
        let parsed = parse_tournament_header(HEADER, &list_coordinates()).expect("header parses");
        assert_eq!(parsed.family, "TORNEI OPEN");
        assert_eq!(parsed.season, "2026/27");
    }

    #[test]
    fn a_bracket_home_parses_its_identity_and_sections() {
        let parsed =
            parse_bracket_home(BRACKET_HOME, &bracket_coordinates()).expect("bracket home parses");
        assert_eq!(parsed.tournament, "Torneo Open Trentino");
        assert_eq!(parsed.place, "Trento");
        assert_eq!(parsed.date, day());
        assert_eq!(parsed.sections.len(), 2);
        let first = &parsed.sections[0];
        assert_eq!(first.gara_id, "1021");
        assert_eq!(first.label, "Singolo M/F Open2 121-1500M | 9-100F");
        assert_eq!(parsed.documents.len(), 2);
        assert_eq!(parsed.documents[0].href, "tabelloni/6_1_129.pdf");
        assert_eq!(parsed.documents[0].label, "Dati sintetici torneo");
        assert_eq!(parsed.documents[1].href, "tabelloni/1_6_home.html");
    }

    #[test]
    fn a_page_without_the_tournament_table_fails_naming_the_selector() {
        let error =
            parse_tournament_list(b"<html><body>no table</body></html>", &list_coordinates())
                .expect_err("no table");
        assert!(
            matches!(
                error,
                TournamentParseError::MissingSelector {
                    selector: "table",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_header_without_the_season_line_fails_naming_the_selector() {
        let page = br#"<html><body><p class="stile">TORNEI OPEN</p></body></html>"#;
        let error = parse_tournament_header(page, &list_coordinates()).expect_err("no season");
        assert!(
            matches!(
                error,
                TournamentParseError::MissingSelector {
                    selector: "p.stile",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_link_without_a_tournament_fails_naming_the_row() {
        let page = br#"<html><body><TABLE><tr><td><p class=dettagli>
            <a href="visualizza_risultati.php?ANNO=41&COM=6" target="inferiore">19/09/2026<br>Trento</a>
            </p></td></tr></TABLE></body></html>"#;
        let error = parse_tournament_list(page, &list_coordinates()).expect_err("no IDT");
        assert!(
            matches!(
                error,
                TournamentParseError::MissingParam {
                    row: 0,
                    param: "IDT",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_bracket_home_without_its_identity_fails_naming_the_selector() {
        let page = br#"<html><body><TABLE><tr><td><p class=dettagli>
            <a href="visualizza.php?GARA=1">Singolo</a></p></td></tr></TABLE></body></html>"#;
        let error = parse_bracket_home(page, &bracket_coordinates()).expect_err("no identity");
        assert!(
            matches!(
                error,
                TournamentParseError::MissingSelector {
                    selector: "p.dettagli b",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn the_coordinates_read_the_tournament_and_committee_off_the_uri() {
        let list = TournamentListCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/tornei/elenco_tornei.php?TOR=100&COMIT=6&ID=0",
        ));
        assert_eq!(list, Some(list_coordinates()));
        let bracket = BracketHomeCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/tornei/elenco_gare.php?IDT=1&COM=6",
        ));
        assert_eq!(bracket, Some(bracket_coordinates()));
        let without = BracketHomeCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/tornei/elenco_gare.php?IDT=1",
        ));
        assert_eq!(without, None);
    }
}
