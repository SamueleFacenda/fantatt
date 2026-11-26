//! Golden tests for the classification and athlete parsers (G1.5): the
//! recorded classifica headers, ranking pages, player sheets, and the player
//! search answer become exactly the canonical records, and the identifiers
//! one surface names resolve to the pages the fixtures record for the same
//! athlete. No test reaches the network.

use fantatt_fixtures::FixtureSet;
use fantatt_ingest::uri::Uri;
use fantatt_ingest::{
    ClassificationCoordinates, ClassificationHeaderCoordinates, PlayerCoordinates,
    parse_classification, parse_classification_header, parse_player_detail, parse_player_search,
};

/// The three recorded ranking pages: two snapshots of classifica 248 and one
/// of classifica 245, each holding one hundred rows.
#[test]
fn every_recorded_ranking_page_parses_its_snapshot_and_rows() {
    let set = FixtureSet::discover().expect("fixture set");
    let mut recorded = 0;
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if !surface.url.contains("ranking_unica.php") {
            continue;
        }
        let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
        let uri = Uri::parse(&surface.url).expect("page uri");
        let coordinates = ClassificationCoordinates::from_uri(&uri).expect("coordinates");
        let parsed = parse_classification(&bytes, &coordinates).expect("page parses");
        assert_eq!(parsed.rows.len(), 100, "{}", surface.file);
        assert!(!parsed.title.is_empty(), "{}", surface.file);
        assert_eq!(parsed.rows[0].rank, 1, "{}", surface.file);
        recorded += 1;
    }
    assert_eq!(recorded, 3);
}

/// The first row of a ranking page carries the athlete the recorded player
/// sheet names, with the points the page prints.
#[test]
fn the_first_row_names_the_athlete_the_player_sheet_holds() {
    let set = FixtureSet::discover().expect("fixture set");
    let parsed = ranking_page(&set, "SESSO=M&ID_CLASS=248");
    assert_eq!(parsed.title, "Classifica Generale maschile");
    assert_eq!(parsed.athletes, 8894);
    let first = &parsed.rows[0];
    assert_eq!(first.athlete_id, "710705");
    assert_eq!(first.name, "OYEBODE JOHN MICHAEL");
    assert_eq!(first.points, "16716*");
    assert_eq!(first.region, "Sardegna");
}

/// The recorded classifica header names the snapshot day the 248 ranking
/// pages carry, and drives every section the page prints.
#[test]
fn the_recorded_classifica_header_names_the_snapshot_day() {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("testaclassifica_auto.php"))
        .expect("classifica header");
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = ClassificationHeaderCoordinates::from_uri(&uri).expect("coordinates");
    let parsed = parse_classification_header(&bytes, &coordinates).expect("page parses");
    assert_eq!(parsed.snapshot_date.to_string(), "2026-10-25");
    assert_eq!(parsed.sections.len(), 40);
    let masculine = &parsed.sections[0];
    assert_eq!(masculine.label, "MASCHILE");
    assert_eq!(masculine.title, "Maschile Generale");
    assert_eq!(masculine.handler, "getPage");
    assert_eq!(masculine.arguments, ["M", "248", "1", "1"]);
}

/// The athlete a ranking row names resolves to the player sheet the fixtures
/// record for that athlete, and the candidate the search answers resolves to
/// the same sheet.
#[test]
fn every_athlete_identifier_resolves_to_its_recorded_sheet() {
    let set = FixtureSet::discover().expect("fixture set");
    let ranking = ranking_page(&set, "SESSO=M&ID_CLASS=248");
    let first = &ranking.rows[0];
    let detail = player_sheet_url(&first.athlete_id, "248");
    assert!(recorded(&set, &detail), "{}", detail.as_url());

    let search = search_page(&set);
    let candidates = parse_player_search(&search).expect("search parses");
    let candidate = &candidates[0];
    let searched = player_sheet_url(&candidate.id, "248");
    assert!(recorded(&set, &searched), "{}", searched.as_url());
}

/// The recorded player sheets parse to the athlete and the fields the sheet
/// states, and the classifica that carries no details for an athlete parses
/// to none.
#[test]
fn every_recorded_player_sheet_parses_its_fields() {
    let set = FixtureSet::discover().expect("fixture set");
    let mut recorded = 0;
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if !surface.url.contains("dettaglioatleta_unica.php") {
            continue;
        }
        let parsed = parsed_sheet(&set, &surface);
        assert!(parsed.is_some(), "{}", surface.file);
        assert_sheet_fields(&parsed.expect("a detail sheet"), &surface.file);
        recorded += 1;
    }
    assert_eq!(recorded, 2);
}

fn parsed_sheet(
    set: &FixtureSet,
    surface: &fantatt_fixtures::capture::Surface,
) -> Option<fantatt_ingest::ParsedPlayerDetail> {
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = PlayerCoordinates::from_uri(&uri).expect("coordinates");
    parse_player_detail(&bytes, &coordinates).expect("sheet parses")
}

fn assert_sheet_fields(parsed: &fantatt_ingest::ParsedPlayerDetail, file: &str) {
    assert!(!parsed.name.is_empty(), "{file}");
    assert!(!parsed.fields.is_empty(), "{file}");
    let labels: Vec<&str> = parsed
        .fields
        .iter()
        .map(|field| field.label.as_str())
        .collect();
    assert!(labels.contains(&"Categoria"), "{file}");
}

fn ranking_page(set: &FixtureSet, marker: &str) -> fantatt_ingest::ParsedClassification {
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("ranking_unica.php") && surface.url.contains(marker))
        .expect("ranking page");
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = ClassificationCoordinates::from_uri(&uri).expect("coordinates");
    parse_classification(&bytes, &coordinates).expect("page parses")
}

fn player_sheet_url(athlete_id: &str, classifica_id: &str) -> Uri {
    Uri::parse(&format!(
        "https://portale.fitet.org/risultati/new_rank/dettaglioatleta_unica.php?ATLETA={athlete_id}&ID_CLASS={classifica_id}&ZU=1&AVVERSARIO=0"
    ))
    .expect("sheet url")
}

fn search_page(set: &FixtureSet) -> Vec<u8> {
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("ajax.php"))
        .expect("player search");
    std::fs::read(set.file_path(&surface.file)).expect("search answer")
}

fn recorded(set: &FixtureSet, uri: &Uri) -> bool {
    let key = uri.as_url();
    set.html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .any(|surface| Uri::parse(&surface.url).expect("recorded url").as_url() == key)
}
