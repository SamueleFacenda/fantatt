//! The championship catalog parsers: a regional menu or a championship list
//! page becomes the championship identifiers the walk expands. Every
//! extraction names the portal structure it reads, and every failure is a
//! typed error that locates the selector, row, or link at fault instead of a
//! panic (docs/06-data-pipeline.md §2, §5).

use scraper::{ElementRef, Html, Selector};

use super::error::ChampionshipParseError;
use super::model::{ChampionshipLink, ParsedChampionshipList, ParsedRegionMenu};
use crate::html::{parse_page, texts};
use crate::names::collapse_whitespace;
use crate::uri::Uri;

/// The selector naming the championship table of a menu or list page.
const TABLE: &str = "table";

/// The selector naming one championship link on a menu or list page.
const LINK: &str = "p.testa a";

/// The selector naming the season line of a championship list.
const SEASON: &str = "p.stile";

/// The coordinates a regional menu page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCoordinates {
    /// The `REG` query value naming the region.
    pub region_id: String,
}

impl RegionCoordinates {
    /// Read the coordinates off a regional menu URI; none when the query
    /// names no region.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        uri.param("REG").map(|region| Self {
            region_id: region.to_string(),
        })
    }
}

/// Parse one regional menu into the championships of its region. A region
/// whose championship table stays empty yields an empty catalog: no regional
/// championship is a fact the page states, not a failure.
pub fn parse_region_menu(
    page: &[u8],
    coordinates: &RegionCoordinates,
) -> Result<ParsedRegionMenu, ChampionshipParseError> {
    let document = parse_page(page);
    guard_table(&document)?;
    Ok(ParsedRegionMenu {
        region_id: coordinates.region_id.clone(),
        championships: links(&document)?,
    })
}

/// Parse one championship list into the season it covers and the
/// championships it names.
pub fn parse_championship_list(
    page: &[u8],
) -> Result<ParsedChampionshipList, ChampionshipParseError> {
    let document = parse_page(page);
    guard_table(&document)?;
    Ok(ParsedChampionshipList {
        season: season(&document)?,
        championships: links(&document)?,
    })
}

// why: every catalog page prints its championships inside one table, so a
// page without the table is a structure change rather than a short catalog
fn guard_table(document: &Html) -> Result<(), ChampionshipParseError> {
    let selector = Selector::parse(TABLE).expect("static selector");
    match document.select(&selector).next() {
        Some(_) => Ok(()),
        None => Err(ChampionshipParseError::MissingSelector {
            selector: TABLE,
            purpose: "the championship table",
        }),
    }
}

/// The season the list states, e.g. `2026/27`, from the title line it prints
/// above the table.
fn season(document: &Html) -> Result<String, ChampionshipParseError> {
    let selector = Selector::parse(SEASON).expect("static selector");
    let title = texts(document, &selector)
        .first()
        .cloned()
        .unwrap_or_default();
    let Some((_, season)) = title.split_once("STAGIONE") else {
        return Err(ChampionshipParseError::MissingSelector {
            selector: SEASON,
            purpose: "the season the championships belong to",
        });
    };
    Ok(collapse_whitespace(season.trim()))
}

fn links(document: &Html) -> Result<Vec<ChampionshipLink>, ChampionshipParseError> {
    let selector = Selector::parse(LINK).expect("static selector");
    document
        .select(&selector)
        .enumerate()
        .map(|(row, anchor)| link(row, anchor))
        .collect()
}

fn link(row: usize, anchor: ElementRef<'_>) -> Result<ChampionshipLink, ChampionshipParseError> {
    let href = anchor.value().attr("href").unwrap_or_default();
    Ok(ChampionshipLink {
        championship_id: required_param(row, "CAM", href)?,
        contest_id: optional_param(href, "CONC"),
        label: collapse_whitespace(&anchor.text().collect::<String>()),
    })
}

fn required_param(
    row: usize,
    name: &'static str,
    href: &str,
) -> Result<String, ChampionshipParseError> {
    optional_param(href, name).ok_or_else(|| ChampionshipParseError::MissingParam {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    fn coordinates() -> RegionCoordinates {
        RegionCoordinates {
            region_id: "2".to_string(),
        }
    }

    /// A regional menu as the portal prints it: one championship per row.
    const MENU: &[u8] = br#"<html><body><TABLE width="120">
        <tr><td width=110><p class=testa><a href="../campionati/campionati.php?CAM=240&CONC=1">C femm gir. A</a></p></td></tr>
        <tr><td width=110><p class=testa><a href="../campionati/campionati.php?CAM=233&CONC=0">C.I. C2D1 gir. A</a></p></td></tr>
    </TABLE></body></html>"#;

    /// A region whose championship table stays empty.
    const EMPTY_MENU: &[u8] = br#"<html><body><TABLE width="120"></TABLE></body></html>"#;

    /// A championship list as the portal prints it: a season line, then a
    /// table whose first cell names the section.
    const LIST: &[u8] = br#"<html><body>
        <p class="stile"><font>CAMPIONATI NAZIONALI A SQUADRE - STAGIONE 2026/27</font></p>
        <TABLE border=2><tr><td width=66>Maschile</td>
        <td><p class=testa><a href="campionati.php?CAM=1&CONC=0">A/1 Gir.U</a></p></td>
        <td><p class=testa><a href="campionati.php?CAM=3&CONC=0">A/2 Gir.A</a></p></td></tr>
        <tr><td width=66>Femminile</td>
        <td><p class=testa><a href="campionati.php?CAM=2&CONC=0">A/1 Gir.U</a></p></td>
        <td><p class=testa><a href="campionati.php?CAM=18&CONC=-1">A/2 Gir.A</a></p></td></tr></TABLE>
    </body></html>"#;

    #[test]
    fn a_regional_menu_parses_its_championships() {
        let parsed = parse_region_menu(MENU, &coordinates()).expect("menu parses");
        assert_eq!(parsed.region_id, "2");
        assert_eq!(parsed.championships.len(), 2);
        let first = &parsed.championships[0];
        assert_eq!(first.championship_id, "240");
        assert_eq!(first.contest_id.as_deref(), Some("1"));
        assert_eq!(first.label, "C femm gir. A");
    }

    #[test]
    fn an_empty_regional_menu_is_a_catalog_without_championships() {
        let parsed = parse_region_menu(EMPTY_MENU, &coordinates()).expect("menu parses");
        assert!(parsed.championships.is_empty());
    }

    #[test]
    fn a_championship_list_parses_its_season_and_championships() {
        let parsed = parse_championship_list(LIST).expect("list parses");
        assert_eq!(parsed.season, "2026/27");
        assert_eq!(parsed.championships.len(), 4);
        let feminine = &parsed.championships[3];
        assert_eq!(feminine.championship_id, "18");
        assert_eq!(feminine.contest_id.as_deref(), Some("-1"));
        assert_eq!(feminine.label, "A/2 Gir.A");
    }

    #[test]
    fn a_page_without_the_championship_table_fails_naming_the_selector() {
        let error = parse_region_menu(b"<html><body>no table</body></html>", &coordinates())
            .expect_err("no table");
        assert!(
            matches!(
                error,
                ChampionshipParseError::MissingSelector {
                    selector: "table",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_list_without_the_season_line_fails_naming_the_selector() {
        let page = br#"<html><body><TABLE><tr><td><p class=testa>
            <a href="campionati.php?CAM=1">A/1</a></p></td></tr></TABLE></body></html>"#;
        let error = parse_championship_list(page).expect_err("no season");
        assert!(
            matches!(
                error,
                ChampionshipParseError::MissingSelector {
                    selector: "p.stile",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_link_without_a_championship_fails_naming_the_row() {
        let page = br#"<html><body><TABLE><tr><td><p class=testa>
            <a href="campionati.php?CONC=0">A/1</a></p></td></tr></TABLE></body></html>"#;
        let error = parse_region_menu(page, &coordinates()).expect_err("no CAM");
        assert!(
            matches!(
                error,
                ChampionshipParseError::MissingParam {
                    row: 0,
                    param: "CAM",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn the_coordinates_read_the_region_off_the_uri() {
        let parsed = RegionCoordinates::from_uri(&uri(
            "https://portale.fitet.org/regioni/menu_reg.php?REG=2",
        ));
        assert_eq!(parsed, Some(coordinates()));
        let without =
            RegionCoordinates::from_uri(&uri("https://portale.fitet.org/regioni/menu_reg.php"));
        assert_eq!(without, None);
    }
}
