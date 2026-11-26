//! The classification parsers: a classifica header becomes the snapshot day
//! and the ranking sections, and a ranking page becomes the snapshot and its
//! rows. Every extraction names the portal structure it reads, and every
//! failure is a typed error that locates the selector, row, or cell at fault
//! instead of a panic (docs/06-data-pipeline.md §2, §5).

use scraper::{ElementRef, Html, Selector};
use time::{Date, Month};

use super::error::ClassificationParseError;
use super::model::{
    ClassificationRow, ParsedClassification, ParsedClassificationHeader, RankingSection,
};
use crate::html::parse_page;
use crate::names::collapse_whitespace;
use crate::uri::Uri;

/// The selector naming the snapshot line of a classifica header.
const SNAPSHOT: &str = "p.stile_t";

/// The selector naming one ranking section link of the header.
const SECTION: &str = "a";

/// The selector naming the heading line of a ranking page.
const HEADING: &str = "p.titoli";

/// The selector naming one row of the ranking table.
const ROW: &str = "tr";

/// The selector naming one cell of a ranking row.
const CELL: &str = "td";

/// The cells one ranking row prints, in the order the page prints them.
const RANK_CELL: usize = 0;
const NAME_CELL: usize = 1;
const BIRTH_CELL: usize = 2;
const SECTOR_CELL: usize = 3;
const POINTS_CELL: usize = 4;
const CHANGE_CELL: usize = 5;
const REGION_CELL: usize = 6;
const CATEGORY_CELL: usize = 7;

/// How many cells one ranking row prints.
const ROW_CELLS: usize = 8;

/// The coordinates a classifica header page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationHeaderCoordinates {
    /// The `ID_CLASS` query value naming the classifica.
    pub classifica_id: String,
}

impl ClassificationHeaderCoordinates {
    /// Read the coordinates off a header URI; none when the query names no
    /// classifica.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        uri.param("ID_CLASS").map(|classifica| Self {
            classifica_id: classifica.to_string(),
        })
    }
}

/// The coordinates a ranking page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassificationCoordinates {
    /// The `ID_CLASS` query value naming the classifica.
    pub classifica_id: String,
    /// The `SESSO` query value naming the gender the page ranks.
    pub gender: String,
}

impl ClassificationCoordinates {
    /// Read the coordinates off a ranking page URI; none when the query
    /// names no classifica or no gender.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        Some(Self {
            classifica_id: uri.param("ID_CLASS")?.to_string(),
            gender: uri.param("SESSO")?.to_string(),
        })
    }
}

/// Parse one classifica header into the snapshot day and the ranking
/// sections the page drives.
pub fn parse_classification_header(
    page: &[u8],
    _coordinates: &ClassificationHeaderCoordinates,
) -> Result<ParsedClassificationHeader, ClassificationParseError> {
    let document = parse_page(page);
    Ok(ParsedClassificationHeader {
        snapshot_date: snapshot_date(&document)?,
        sections: sections(&document),
    })
}

/// Parse one ranking page into the snapshot it belongs to and its rows.
pub fn parse_classification(
    page: &[u8],
    _coordinates: &ClassificationCoordinates,
) -> Result<ParsedClassification, ClassificationParseError> {
    let document = parse_page(page);
    let (title, snapshot_date, athletes) = heading(&document)?;
    Ok(ParsedClassification {
        title,
        snapshot_date,
        athletes,
        rows: rows(&document)?,
    })
}

/// The day the header names after its `AL` marker.
fn snapshot_date(document: &Html) -> Result<Date, ClassificationParseError> {
    let selector = Selector::parse(SNAPSHOT).expect("static selector");
    let text = collapse_whitespace(&joined_text(document, &selector));
    let (_, day) = text
        .split_once(" AL ")
        .ok_or(ClassificationParseError::MissingSelector {
            selector: SNAPSHOT,
            purpose: "the day the ranking snapshot was taken on",
        })?;
    parse_date(day).ok_or(ClassificationParseError::MissingSelector {
        selector: SNAPSHOT,
        purpose: "the day the ranking snapshot was taken on",
    })
}

/// Every ranking section link the header drives, with the handler and the
/// arguments its `onclick` carries.
fn sections(document: &Html) -> Vec<RankingSection> {
    let selector = Selector::parse(SECTION).expect("static selector");
    document
        .select(&selector)
        .filter_map(|link| {
            let onclick = link.value().attr("onclick")?;
            let (handler, arguments) = handler_call(onclick)?;
            Some(RankingSection {
                label: collapse_whitespace(&link.text().collect::<String>()),
                title: link.value().attr("title").unwrap_or_default().to_string(),
                handler: handler.to_string(),
                arguments,
            })
        })
        .collect()
}

/// Split one page handler call into its name and its arguments.
fn handler_call(onclick: &str) -> Option<(&str, Vec<String>)> {
    let (handler, rest) = onclick.split_once('(')?;
    let arguments = rest
        .strip_suffix(')')?
        .split(',')
        .map(|argument| argument.trim().trim_matches(['"', '\'']).to_string())
        .filter(|argument| !argument.is_empty())
        .collect();
    Some((handler, arguments))
}

/// The title, the snapshot day, and the athlete count a ranking page prints
/// in its heading line.
fn heading(document: &Html) -> Result<(String, Date, u32), ClassificationParseError> {
    let text = heading_text(document)?;
    let (title, rest) =
        text.split_once(" al ")
            .ok_or(ClassificationParseError::MissingSelector {
                selector: HEADING,
                purpose: "the title, the snapshot day, and the athlete count",
            })?;
    let (day, count) =
        rest.split_once(" (Atleti: ")
            .ok_or(ClassificationParseError::MalformedCount {
                text: text.to_string(),
            })?;
    let athletes = athlete_count(count, &text)?;
    Ok((
        title.to_string(),
        parse_date(day).ok_or(ClassificationParseError::MalformedCount {
            text: text.to_string(),
        })?,
        athletes,
    ))
}

/// The heading line the ranking page prints above its rows.
fn heading_text(document: &Html) -> Result<String, ClassificationParseError> {
    let selector = Selector::parse(HEADING).expect("static selector");
    let text = collapse_whitespace(&joined_text(document, &selector));
    if text.is_empty() {
        return Err(ClassificationParseError::MissingSelector {
            selector: HEADING,
            purpose: "the title, the snapshot day, and the athlete count",
        });
    }
    Ok(text)
}

/// The athlete count the heading prints inside its parentheses.
fn athlete_count(count: &str, heading: &str) -> Result<u32, ClassificationParseError> {
    count
        .trim_end_matches(')')
        .parse()
        .map_err(|_| ClassificationParseError::MalformedCount {
            text: heading.to_string(),
        })
}

/// Every ranking row the page prints, in document order. why: the page
/// prints a header row of the same shape, so a row counts when its first
/// cell holds the rank the page prints.
fn rows(document: &Html) -> Result<Vec<ClassificationRow>, ClassificationParseError> {
    let selector = Selector::parse(ROW).expect("static selector");
    document
        .select(&selector)
        .filter_map(|row| ranking_rank(row).map(|rank| (rank, row)))
        .enumerate()
        .map(|(index, (rank, row))| classification_row(index, rank, row))
        .collect()
}

/// The rank a ranking row prints in its first cell, or None for the header
/// row the page prints in the same shape.
fn ranking_rank(row: ElementRef<'_>) -> Option<u32> {
    cell_count(row)
        .eq(&ROW_CELLS)
        .then(|| cell(row, RANK_CELL))
        .flatten()
        .and_then(|rank| rank.parse().ok())
}

fn classification_row(
    index: usize,
    rank: u32,
    row: ElementRef<'_>,
) -> Result<ClassificationRow, ClassificationParseError> {
    Ok(ClassificationRow {
        rank,
        athlete_id: athlete_id(index, row)?,
        name: required_cell(index, row, NAME_CELL)?,
        birth_date: required_date(index, row, BIRTH_CELL)?,
        sector: cell(row, SECTOR_CELL).unwrap_or_default(),
        points: cell(row, POINTS_CELL).unwrap_or_default(),
        change: cell(row, CHANGE_CELL).unwrap_or_default(),
        region: cell(row, REGION_CELL).unwrap_or_default(),
        category: cell(row, CATEGORY_CELL).unwrap_or_default(),
    })
}

/// The `ATLETA` id the row's handler names, read off the first argument of
/// the `getScheda` call its anchor carries.
fn athlete_id(index: usize, row: ElementRef<'_>) -> Result<String, ClassificationParseError> {
    let selector = Selector::parse("a").expect("static selector");
    row.select(&selector)
        .next()
        .and_then(|link| link.value().attr("onclick"))
        .and_then(handler_call)
        .and_then(|(_, arguments)| arguments.first().cloned())
        .ok_or(ClassificationParseError::MissingAthlete {
            row: index,
            onclick: String::new(),
        })
}

fn required_cell(
    index: usize,
    row: ElementRef<'_>,
    cell: usize,
) -> Result<String, ClassificationParseError> {
    self::cell(row, cell).ok_or(ClassificationParseError::MissingCell { row: index, cell })
}

fn required_date(
    index: usize,
    row: ElementRef<'_>,
    cell: usize,
) -> Result<Date, ClassificationParseError> {
    cell_date(row, cell).ok_or(ClassificationParseError::MissingCell { row: index, cell })
}

fn cell_count(row: ElementRef<'_>) -> usize {
    let selector = Selector::parse(CELL).expect("static selector");
    row.select(&selector).count()
}

fn cell(row: ElementRef<'_>, cell: usize) -> Option<String> {
    let selector = Selector::parse(CELL).expect("static selector");
    row.select(&selector)
        .nth(cell)
        .map(|found| collapse_whitespace(&found.text().collect::<String>()))
        .filter(|text| !text.is_empty())
}

fn cell_date(row: ElementRef<'_>, index: usize) -> Option<Date> {
    parse_date(&cell(row, index)?)
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

/// The text of the first selected element: the heading line a page prints
/// once, above the rows.
fn joined_text(document: &Html, selector: &Selector) -> String {
    document
        .select(selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day() -> Date {
        time::Date::from_calendar_date(2026, time::Month::October, 25).expect("a calendar day")
    }

    fn uri(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    fn header_coordinates() -> ClassificationHeaderCoordinates {
        ClassificationHeaderCoordinates {
            classifica_id: "248".to_string(),
        }
    }

    fn page_coordinates() -> ClassificationCoordinates {
        ClassificationCoordinates {
            classifica_id: "248".to_string(),
            gender: "M".to_string(),
        }
    }

    /// A classifica header as the page prints it: a snapshot line, the two
    /// gender sections, and the giovanili sections.
    const HEADER: &[u8] = br#"<html><body>
        <p class="stile_t" align="center">CLASSIFICHE&nbsp; NAZIONALI&nbsp; INDIVIDUALI&nbsp; AL&nbsp; 25/10/2026</p>
        <p class="stile_titoli"><b><a id="all" onclick='getPage("M", 248, 1, 1)' title="Maschile Generale">MASCHILE</a></b></p>
        <p class="stile_titoli"><a id="all" onclick='getPage("F", 248, 1, 1)' title="Femminile Generale">FEMMINILE</a></p>
        <p class="stile_menu" align="center">Giovanili</p>
        <p class="stile_menu" align="center"><a id="all" onclick='getPageGiov("M", 248, 1, 0, "G")' title="Under11">U11</a></p>
    </body></html>"#;

    /// A ranking page as the page prints it: a heading line and two rows.
    const RANKING: &[u8] = br#"<html><body><center><table>
        <tr><td height="20" colspan="6" class="style2">
            <p class="titoli">Classifica Generale maschile al 25/10/2026&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;(Atleti: 8894)</p></td></tr>
        <tr><td colspan="8"><p class="titoli">posizione</p></td></tr>
        <tr><td width=35><center><p class=dettagli2>1</p></center></td>
        <td width=230><p class=dettagli2><b> <a id="all" onclick='getScheda(710705, 248, 1, 0)' title="Scheda_dettaglio">OYEBODE JOHN MICHAEL</a> </b></p></td>
        <td width=65><center><p class=dettagli2>25/02/2002</p></td><td width=12><center><p class=dettagli2>S</p></center></td>
        <td width=35><center><p class=dettagli2>16716*</p></center></td><td width=30><center><p class=dettagli2>0</p></center></td>
        <td width=120><p class=dettagli2>Sardegna</p></td><td width=10><center><p class=dettagli2>1</p></center></td></tr>
        <tr><td width=35><center><p class=dettagli2>2</p></center></td>
        <td width=230><p class=dettagli2><b> <a id="all" onclick='getScheda(594848, 248, 1, 0)' title="Scheda_dettaglio">BOBOCICA MIHAI RAZVAN</a> </b></p></td>
        <td width=65><center><p class=dettagli2>08/09/1986</p></td><td width=12><center><p class=dettagli2>1</p></center></td>
        <td width=35><center><p class=dettagli2>15837*</p></center></td><td width=30><center><p class=dettagli2>1</p></center></td>
        <td width=120><p class=dettagli2>Lazio</p></td><td width=10><center><p class=dettagli2>1</p></center></td></tr>
    </table></center></body></html>"#;

    #[test]
    fn a_classifica_header_parses_its_snapshot_and_sections() {
        let parsed = parse_classification_header(HEADER, &header_coordinates()).expect("parses");
        assert_eq!(parsed.snapshot_date, day());
        assert_eq!(parsed.sections.len(), 3);
        let first = &parsed.sections[0];
        assert_eq!(first.label, "MASCHILE");
        assert_eq!(first.title, "Maschile Generale");
        assert_eq!(first.handler, "getPage");
        assert_eq!(first.arguments, ["M", "248", "1", "1"]);
        let giovanili = &parsed.sections[2];
        assert_eq!(giovanili.handler, "getPageGiov");
        assert_eq!(giovanili.label, "U11");
    }

    #[test]
    fn a_ranking_page_parses_its_heading_and_rows() {
        let parsed = parse_classification(RANKING, &page_coordinates()).expect("parses");
        assert_eq!(parsed.title, "Classifica Generale maschile");
        assert_eq!(parsed.snapshot_date, day());
        assert_eq!(parsed.athletes, 8894);
        assert_eq!(parsed.rows.len(), 2);
        let first = &parsed.rows[0];
        assert_eq!(first.rank, 1);
        assert_eq!(first.athlete_id, "710705");
        assert_eq!(first.name, "OYEBODE JOHN MICHAEL");
        assert_eq!(first.sector, "S");
        assert_eq!(first.points, "16716*");
        assert_eq!(first.change, "0");
        assert_eq!(first.region, "Sardegna");
        assert_eq!(first.category, "1");
    }

    #[test]
    fn a_ranking_page_keeps_the_points_as_the_page_prints_them() {
        let parsed = parse_classification(RANKING, &page_coordinates()).expect("parses");
        assert_eq!(parsed.rows[0].points, "16716*");
    }

    #[test]
    fn a_header_without_the_snapshot_line_fails_naming_the_selector() {
        let page = br#"<html><body><p class="stile_t">CLASSIFICHE NAZIONALI</p></body></html>"#;
        let error = parse_classification_header(page, &header_coordinates()).expect_err("no day");
        assert!(
            matches!(
                error,
                ClassificationParseError::MissingSelector {
                    selector: "p.stile_t",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_ranking_page_without_its_heading_fails_naming_the_selector() {
        let page = br#"<html><body><table><tr><td><p class="titoli">posizione</p></td></tr>
            </table></body></html>"#;
        let error = parse_classification(page, &page_coordinates()).expect_err("no heading");
        assert!(
            matches!(
                error,
                ClassificationParseError::MissingSelector {
                    selector: "p.titoli",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_row_without_an_athlete_fails_naming_the_row() {
        let page = br#"<html><body><table>
            <tr><td height="20" colspan="6"><p class="titoli">Classifica al 25/10/2026 (Atleti: 2)</p></td></tr>
            <tr><td>1</td><td>NO LINK</td><td>25/02/2002</td><td>S</td><td>1*</td><td>0</td>
            <td>Sardegna</td><td>1</td></tr></table></body></html>"#;
        let error = parse_classification(page, &page_coordinates()).expect_err("no athlete");
        assert!(
            matches!(
                error,
                ClassificationParseError::MissingAthlete { row: 0, .. }
            ),
            "{error}"
        );
    }

    #[test]
    fn the_coordinates_read_the_classifica_and_the_gender_off_the_uri() {
        let page = ClassificationCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/new_rank/ranking_unica.php?SESSO=M&ID_CLASS=248&ZU=1",
        ));
        assert_eq!(page, Some(page_coordinates()));
        let header = ClassificationHeaderCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/new_rank/testaclassifica_auto.php?ID_CLASS=248&ID=1&PASS=20",
        ));
        assert_eq!(header, Some(header_coordinates()));
        let without = ClassificationCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/new_rank/ranking_unica.php?SESSO=M",
        ));
        assert_eq!(without, None);
    }
}
