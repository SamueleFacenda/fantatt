//! Golden tests for the giornata parser (G1.4): every recorded referto
//! becomes exactly the canonical record committed under `tests/golden/`, the
//! legacy corpus is representable as canonical ties, and a mutated page fails
//! with a selector-locating error instead of a panic.

use fantatt_fixtures::FixtureSet;
use fantatt_ingest::names::normalize_name;
use fantatt_ingest::uri::Uri;
use fantatt_ingest::{GiornataCoordinates, GiornataParseError, GiornataRow, parse_giornata};

/// The walkover encoding the legacy corpus carries for a player who did not
/// contest a tie.
const WALKOVERS: [[(u8, u8); 3]; 2] = [[(0, 11), (0, 11), (0, 11)], [(11, 0), (11, 0), (11, 0)]];

#[test]
fn every_recorded_giornata_parses_to_its_expected_canonical_record() {
    let set = FixtureSet::discover().expect("fixture set");
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let mut recorded = 0;
    for surface in set.html_surfaces().expect("recorded surfaces") {
        if !surface.url.contains("giornata.php") {
            continue;
        }
        let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
        let uri = Uri::parse(&surface.url).expect("page uri");
        let coordinates = GiornataCoordinates::from_uri(&uri).expect("coordinates");
        let parsed = parse_giornata(&bytes, &coordinates).expect("page parses");
        let serialized = serde_json::to_string_pretty(&parsed).expect("serialize");
        let expected_path = golden.join(format!("giornata_{}.json", coordinates.contest_id));
        let expected = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("missing golden record: {}", expected_path.display()));
        assert_eq!(format!("{serialized}\n"), expected, "{}", surface.file);
        recorded += 1;
    }
    assert_eq!(recorded, 12);
}

#[test]
fn every_legacy_record_is_representable_as_a_canonical_tie() {
    let set = FixtureSet::discover().expect("fixture set");
    let records = set.legacy_matches().expect("legacy matches");
    assert_eq!(records.len(), 1085);
    let mut walkovers = 0;
    for record in &records {
        let row = canonical_tie(record);
        walkovers += usize::from(WALKOVERS.iter().any(|sets| row.sets == *sets));
    }
    assert_eq!(walkovers, 18);
}

fn canonical_tie(record: &fantatt_fixtures::records::RawMatch) -> GiornataRow {
    assert_eq!(normalize_name(&record.one), record.one, "{}", record.one);
    assert_eq!(normalize_name(&record.two), record.two, "{}", record.two);
    let sets = record
        .score
        .iter()
        .map(|[home, away]| (*home, *away))
        .collect();
    assert!((3..=5).contains(&record.score.len()), "{}", record.one);
    GiornataRow {
        match_number: 1,
        detail_id: None,
        player_a: record.one.clone(),
        player_b: record.two.clone(),
        sets,
    }
}

#[test]
fn a_mutated_referto_fails_naming_the_selector_that_broke() {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = first_giornata_surface(&set);
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let text = String::from_utf8(bytes)
        .expect("utf8 page")
        .replace("titolo_scheda", "titolo_rotto");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = GiornataCoordinates::from_uri(&uri).expect("coordinates");
    let error = parse_giornata(text.as_bytes(), &coordinates).expect_err("mutated page");
    assert!(
        matches!(
            error,
            GiornataParseError::MissingSelector {
                selector: "p.titolo_scheda",
                ..
            }
        ),
        "{error}"
    );
}

fn first_giornata_surface(set: &FixtureSet) -> fantatt_fixtures::capture::Surface {
    set.html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("giornata.php"))
        .expect("giornata surface")
}
