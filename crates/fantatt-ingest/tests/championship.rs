//! Golden tests for the championship catalog parsers (G1.5): the recorded
//! regional menus and championship lists become exactly the championship
//! identifiers the walk expands, every link resolves to a surface the walk
//! classifies, and a mutated page fails with a selector-locating error
//! instead of a panic.

use fantatt_fixtures::FixtureSet;
use fantatt_ingest::championship::ChampionshipLink;
use fantatt_ingest::discover::Surface;
use fantatt_ingest::uri::Uri;
use fantatt_ingest::{
    ChampionshipParseError, RegionCoordinates, parse_championship_list, parse_region_menu,
};

/// The two recorded regional menus: region one states no regional
/// championship, region two names forty-one.
#[test]
fn every_recorded_regional_menu_parses_to_its_championships() {
    let set = FixtureSet::discover().expect("fixture set");
    let mut recorded = 0;
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if !surface.url.contains("menu_reg.php") {
            continue;
        }
        let parsed = parsed_menu(&set, &surface);
        assert_eq!(parsed.region_id, region_of(&surface.url));
        let expected = if parsed.region_id == "1" { 0 } else { 41 };
        assert_eq!(parsed.championships.len(), expected, "{}", surface.file);
        recorded += 1;
    }
    assert_eq!(recorded, 2);
}

/// The one regional menu that names championships opens with the first
/// feminine championship of the region.
#[test]
fn a_regional_menu_opens_with_its_first_championship() {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = region_two_menu(&set);
    let parsed = parsed_menu(&set, &surface);
    let first = &parsed.championships[0];
    assert_eq!(first.championship_id, "240");
    assert_eq!(first.contest_id.as_deref(), Some("1"));
    assert_eq!(first.label, "C femm gir. A");
}

/// The recorded championship list states the season it covers and the
/// championships of both sections.
#[test]
fn the_recorded_championship_list_parses_its_season_and_championships() {
    let parsed = parse_championship_list(&recorded_championship_list()).expect("page parses");
    assert_eq!(parsed.season, "2026/27");
    assert_eq!(parsed.championships.len(), 8);
    let masculine = &parsed.championships[0];
    assert_eq!(masculine.championship_id, "1");
    assert_eq!(masculine.contest_id.as_deref(), Some("0"));
    assert_eq!(masculine.label, "A/1 Gir.U");
    let feminine = &parsed.championships[3];
    assert_eq!(feminine.championship_id, "2");
    assert_eq!(feminine.label, "A/1 Gir.U");
}

/// Every championship a catalog names resolves against its page's directory
/// to a championship page the walk classifies, so the catalog and the walk
/// agree on what the portal offers.
#[test]
fn every_catalog_championship_resolves_to_a_walked_surface() {
    let set = FixtureSet::discover().expect("fixture set");
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if let Some(links) = catalog_links(&set, &surface) {
            assert_championship_pages(&surface.url, &links);
        }
    }
}

/// A mutated championship list fails naming the selector that broke, the way
/// the giornata drift test does.
#[test]
fn a_mutated_championship_list_fails_naming_the_selector_that_broke() {
    let text = String::from_utf8(recorded_championship_list())
        .expect("utf8 page")
        .replace("class=\"stile\"", "class=\"rotto\"");
    let error = parse_championship_list(text.as_bytes()).expect_err("mutated page");
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

fn parsed_menu(
    set: &FixtureSet,
    surface: &fantatt_fixtures::capture::Surface,
) -> fantatt_ingest::ParsedRegionMenu {
    let url = Uri::parse(&surface.url).expect("page uri");
    let coordinates = RegionCoordinates::from_uri(&url).expect("coordinates");
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    parse_region_menu(&bytes, &coordinates).expect("page parses")
}

fn region_two_menu(set: &FixtureSet) -> fantatt_fixtures::capture::Surface {
    set.html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("menu_reg.php?REG=2"))
        .expect("region two menu")
}

fn recorded_championship_list() -> Vec<u8> {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("elenco_campionati.php"))
        .expect("championship list");
    std::fs::read(set.file_path(&surface.file)).expect("page bytes")
}

/// The championships one recorded catalog page names, or none when the page
/// is not a catalog.
fn catalog_links(
    set: &FixtureSet,
    surface: &fantatt_fixtures::capture::Surface,
) -> Option<Vec<ChampionshipLink>> {
    let links = if surface.url.contains("menu_reg.php") {
        parsed_menu(set, surface).championships
    } else if surface.url.contains("elenco_campionati.php") {
        parse_championship_list(&std::fs::read(set.file_path(&surface.file)).ok()?)
            .expect("page parses")
            .championships
    } else {
        return None;
    };
    Some(links)
}

fn assert_championship_pages(page_url: &str, links: &[ChampionshipLink]) {
    let (directory, _) = page_url.rsplit_once('/').expect("a page path");
    let base = Uri::parse(&format!("{directory}/catalog.php")).expect("catalog directory");
    for link in links {
        let target = base
            .resolve(&format!(
                "campionati.php?CAM={}&CONC={}",
                link.championship_id,
                link.contest_id.clone().unwrap_or_default()
            ))
            .expect("link resolves");
        assert_eq!(
            Surface::classify(&target),
            Some(Surface::ChampionshipPage),
            "{}",
            link.label
        );
    }
}

fn region_of(page_url: &str) -> String {
    let uri = Uri::parse(page_url).expect("page uri");
    uri.param("REG").expect("region").to_string()
}
