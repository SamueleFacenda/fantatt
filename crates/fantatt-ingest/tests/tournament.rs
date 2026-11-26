//! Golden tests for the tournament parsers (G1.5): the recorded committee
//! lists, the type header, and the bracket homes become exactly the
//! canonical records, every tournament a list names resolves to the results
//! page and the bracket home the fixtures record, and a committee that ran
//! no tournament parses to an empty catalog.

use fantatt_fixtures::{FixtureSet, capture::Surface};
use fantatt_ingest::uri::Uri;
use fantatt_ingest::{
    BracketHomeCoordinates, ParsedBracketHome, ParsedTournamentList, TournamentHeaderCoordinates,
    TournamentLink, TournamentListCoordinates, parse_bracket_home, parse_tournament_header,
    parse_tournament_list,
};

/// The four recorded committee lists: the empty one, and the committees that
/// ran one, two, and six tournaments.
#[test]
fn every_recorded_tournament_list_parses_its_tournaments() {
    let set = FixtureSet::discover().expect("fixture set");
    let mut recorded = 0;
    let mut empty = 0;
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if !surface.url.contains("elenco_tornei.php") {
            continue;
        }
        let parsed = parsed_list(&set, &surface);
        let expected = committees(&surface.url);
        assert_eq!(parsed.tournaments.len(), expected, "{}", surface.file);
        empty += usize::from(parsed.tournaments.is_empty());
        recorded += 1;
    }
    assert_eq!(recorded, 4);
    assert_eq!(empty, 1);
}

/// A committee list that ran one tournament names it with its day and its
/// place.
#[test]
fn a_committee_list_names_the_day_and_the_place() {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = list_of(&set, "100_6");
    let parsed = parsed_list(&set, &surface);
    assert_eq!(parsed.tournaments.len(), 1);
    let found = &parsed.tournaments[0];
    assert_eq!(found.tournament_id, "1");
    assert_eq!(found.season_year, "41");
    assert_eq!(found.committee, "6");
    assert_eq!(found.place, "Trento");
}

/// The recorded type header names the family of tournaments and the season.
#[test]
fn the_recorded_type_header_names_its_family_and_season() {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("testa_tornei.php"))
        .expect("tournament header");
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = TournamentHeaderCoordinates::from_uri(&uri).expect("coordinates");
    let parsed = parse_tournament_header(&bytes, &coordinates).expect("page parses");
    assert_eq!(parsed.family, "TORNEI OPEN");
    assert_eq!(parsed.season, "2026/27");
}

/// The recorded bracket home names the tournament, its place and day, the
/// sections it ran, and the bracket documents it links.
#[test]
fn the_recorded_bracket_home_names_its_tournament_and_sections() {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = bracket_home_of(&set, "1_6");
    let parsed = parsed_bracket_home(&set, &surface);
    assert_eq!(parsed.tournament, "Torneo Open Trentino");
    assert_eq!(parsed.place, "Trento");
    assert_eq!(parsed.sections.len(), 4);
    let first = &parsed.sections[0];
    assert_eq!(first.gara_id, "1021");
    assert_eq!(first.label, "Singolo M/F Open2 121-1500M | 9-100F");
    let documents: Vec<&str> = parsed
        .documents
        .iter()
        .map(|document| document.href.as_str())
        .collect();
    assert_eq!(
        documents,
        ["tabelloni/6_1_129.pdf", "tabelloni/1_6_home.html"]
    );
}

/// Every tournament a committee list names resolves to the results page the
/// fixtures record, and that page frames the bracket home the fixtures
/// record for the same tournament.
#[test]
fn every_recorded_tournament_resolves_to_its_results_and_bracket_home() {
    let set = FixtureSet::discover().expect("fixture set");
    let mut checked = 0;
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if !surface.url.contains("elenco_tornei.php") {
            continue;
        }
        let base = Uri::parse(&surface.url).expect("page uri");
        let parsed = parsed_list(&set, &surface);
        for link in &parsed.tournaments {
            checked += usize::from(check_tournament(&set, &base, link));
        }
    }
    // why: the wider sweep recorded two of the nine tournaments the lists
    // name, so the other committee lists link pages the fixture tree does not
    // hold
    assert_eq!(checked, 2);
}

/// True when the fixtures record both the results page and the bracket home
/// the tournament link names, and the results page frames the bracket home.
fn check_tournament(set: &FixtureSet, base: &Uri, link: &TournamentLink) -> bool {
    let results = base.resolve(&results_href(link)).expect("results resolves");
    if !recorded(set, &results) {
        return false;
    }
    let bracket = base.resolve(&bracket_href(link)).expect("bracket resolves");
    assert!(recorded(set, &bracket), "{}", bracket.as_url());
    assert!(
        frames(set, &results, &bracket),
        "the results page frames its bracket home"
    );
    true
}

/// The results page href the tournament link names.
fn results_href(link: &TournamentLink) -> String {
    format!(
        "visualizza_risultati.php?IDT={}&ANNO={}&COM={}",
        link.tournament_id, link.season_year, link.committee
    )
}

/// The bracket home href the tournament link names.
fn bracket_href(link: &TournamentLink) -> String {
    format!(
        "elenco_gare.php?IDT={}&COM={}",
        link.tournament_id, link.committee
    )
}

fn parsed_list(set: &FixtureSet, surface: &Surface) -> ParsedTournamentList {
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = TournamentListCoordinates::from_uri(&uri).expect("coordinates");
    parse_tournament_list(&bytes, &coordinates).expect("page parses")
}

fn parsed_bracket_home(set: &FixtureSet, surface: &Surface) -> ParsedBracketHome {
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = BracketHomeCoordinates::from_uri(&uri).expect("coordinates");
    parse_bracket_home(&bytes, &coordinates).expect("page parses")
}

/// How many tournaments the committee of one list URL ran, read off the
/// `COMIT` query value the wider sweep established.
fn committees(url: &str) -> usize {
    let uri = Uri::parse(url).expect("page uri");
    match uri.param("COMIT").unwrap_or_default() {
        "1" => 0,
        "6" => 1,
        "7" => 2,
        "9" => 6,
        other => panic!("an unrecorded committee: {other}"),
    }
}

fn list_of(set: &FixtureSet, marker: &str) -> Surface {
    set.html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.file.contains(marker) && surface.url.contains("elenco_tornei.php"))
        .expect("committee list")
}

fn bracket_home_of(set: &FixtureSet, marker: &str) -> Surface {
    set.html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.file.contains(marker) && surface.url.contains("elenco_gare.php"))
        .expect("bracket home")
}

fn recorded(set: &FixtureSet, uri: &Uri) -> bool {
    let key = uri.as_url();
    set.html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .any(|surface| Uri::parse(&surface.url).expect("recorded url").as_url() == key)
}

/// True when the recorded results page frames the bracket home page. why:
/// the frameset spells its frames relative to itself, so the page file and
/// the query are what the bytes carry.
fn frames(set: &FixtureSet, results: &Uri, bracket: &Uri) -> bool {
    let Some(surface) = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| Uri::parse(&surface.url).expect("recorded url") == *results)
    else {
        return false;
    };
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let url = bracket.as_url();
    let (path, query) = url.split_once('?').unwrap_or_default();
    let file = path.rsplit('/').next().unwrap_or_default();
    text.contains(file) && query.split('&').all(|pair| text.contains(pair))
}
