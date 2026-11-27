mod common;

use fantatt_core::appearance::PlayerAppearance;
use fantatt_core::ids::{DomainPrefix, Identifier};
use fantatt_fixtures::FixtureSet;
use fantatt_reasoning::assert::{MatchdayFacts, assert_matchday};
use fantatt_reasoning::error::GraphError;
use fantatt_reasoning::graph::{Answers, Graph};
use fantatt_reasoning::rebuild::rebuild;
use fantatt_reasoning::snapshot::{MemorySnapshotStore, canonical_snapshot, hydrate, save};

use common::{LedgerLog, commit_for, facts, mint, normalize, parse_fixture};

const TT: &str = "https://fantatt.org/ontology/tt#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const DATA: &str = "https://fantatt.org/data/";

const EVERY_APPEARANCE: &str = "SELECT ?appearance WHERE { GRAPH ?g { ?appearance a <https://fantatt.org/ontology/tt#PlayerAppearance> } }";
const EVERY_MATCHDAY: &str = "SELECT ?matchday WHERE { GRAPH ?g { ?matchday a <https://fantatt.org/ontology/tt#Matchday> } }";
const APPEARANCE_PROVENANCE: &str = "SELECT ?fact ?assertedAt ?validFrom ?commit ?actor WHERE { GRAPH ?g { ?fact a <https://fantatt.org/ontology/tt#PlayerAppearance> ; <http://www.w3.org/ns/prov#wasGeneratedBy> ?commit ; <https://fantatt.org/ontology/tt#bitemporalAssertedAt> ?assertedAt ; <https://fantatt.org/ontology/tt#bitemporalValidFrom> ?validFrom . ?commit <http://www.w3.org/ns/prov#wasAttributedTo> ?actor . } }";
const EXPECTED_PLAYED: &str = "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>";
const ASSERTED_AT: &str = "2026-11-16T20:41:03Z";

fn recorded() -> Vec<MatchdayFacts> {
    let set = FixtureSet::discover().expect("fixture set");
    giornata_surfaces(&set)
        .iter()
        .take(3)
        .enumerate()
        .map(|(number, surface)| facts_of(&set, surface, number as u16 + 1))
        .collect()
}

fn giornata_surfaces(set: &FixtureSet) -> Vec<fantatt_fixtures::capture::Surface> {
    let mut surfaces: Vec<_> = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .filter(|surface| surface.url.contains("giornata.php"))
        .collect();
    surfaces.sort_by(|left, right| left.url.cmp(&right.url));
    surfaces
}

fn facts_of(
    set: &FixtureSet,
    surface: &fantatt_fixtures::capture::Surface,
    number: u16,
) -> MatchdayFacts {
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let parsed = parse_fixture(&bytes, &surface.url);
    facts(
        &parsed,
        number,
        commit_for(&surface.url, &bytes, parsed.giornata.date),
    )
}

fn new_graph() -> Graph {
    Graph::in_memory().expect("graph")
}

fn select(graph: &Graph, sparql: &str) -> Answers {
    graph.select(sparql).expect("select")
}

fn iri(path: &str) -> String {
    format!("{DATA}{path}")
}

fn sets_select(appearance: &str) -> String {
    format!(
        "SELECT ?points ?ordinal WHERE {{ GRAPH ?g {{ <{appearance}> <{TT}hasSet> ?set . ?set <{TT}setOrdinal> ?ordinal ; <{TT}setPlayerPoints> ?points . }} }} ORDER BY ?ordinal"
    )
}

fn played_select(appearance: &str) -> String {
    format!("SELECT ?played WHERE {{ GRAPH ?g {{ <{appearance}> <{TT}played> ?played . }} }}")
}

fn foreign(appearance: &PlayerAppearance, matchday: Identifier) -> PlayerAppearance {
    PlayerAppearance::new(foreign_facts(appearance, matchday)).expect("valid appearance")
}

fn foreign_facts(
    appearance: &PlayerAppearance,
    matchday: Identifier,
) -> fantatt_core::appearance::AppearanceFacts {
    use fantatt_core::appearance::{AppearanceContext, AppearanceKey};

    let key = AppearanceKey::new(
        appearance.appearance_id().clone(),
        AppearanceContext::new(
            appearance.player_id().clone(),
            matchday,
            appearance.tie_id().clone(),
        ),
    );
    fantatt_core::appearance::AppearanceFacts::new(key, foreign_contest(appearance))
}

fn foreign_contest(appearance: &PlayerAppearance) -> fantatt_core::appearance::AppearanceContest {
    use fantatt_core::appearance::{Matchup, SetScore, Sets};

    fantatt_core::appearance::AppearanceContest {
        kind: appearance.kind(),
        matchup: Matchup::new(
            appearance.matchup().partner().cloned(),
            appearance.matchup().opponent().cloned(),
            appearance.matchup().opponent_rating(),
        ),
        played: appearance.played(),
        outcome: appearance.outcome(),
        sets: Sets::new(
            appearance
                .sets()
                .scoreline()
                .iter()
                .map(|set| SetScore::new(set.player(), set.opponent()))
                .collect(),
        ),
        flags: appearance.flags(),
    }
}

#[test]
fn asserting_recorded_matchdays_materialises_the_documented_facts() {
    let documents = recorded();
    let graph = new_graph();
    for document in &documents {
        assert_matchday(&graph, document).expect("asserted");
    }
    let expected: usize = documents
        .iter()
        .map(|document| document.appearances.len())
        .sum();
    assert_eq!(select(&graph, EVERY_APPEARANCE).len(), expected);
    assert_eq!(select(&graph, EVERY_MATCHDAY).len(), documents.len());
    assert_eq!(graph.named_graphs().expect("graphs").len(), documents.len());
}

#[test]
fn every_asserted_fact_carries_its_provenance_and_validity() {
    let documents = recorded();
    let graph = new_graph();
    assert_matchday(&graph, &documents[0]).expect("asserted");
    let rows = select(&graph, APPEARANCE_PROVENANCE);
    assert_eq!(rows.len(), documents[0].appearances.len());
    let row = rows.first().expect("row");
    let instant = row.get("assertedAt").expect("instant").to_string();
    assert!(instant.contains(ASSERTED_AT), "{instant}");
    assert!(instant.contains(&format!("{XSD}dateTime")), "{instant}");
    let actor = row.get("actor").expect("actor").to_string();
    assert_eq!(actor, format!("<{DATA}agent/fitet-ingest>"));
    let day = row.get("validFrom").expect("day").to_string();
    assert!(day.contains(&format!("{XSD}date")), "{day}");
}

#[test]
fn a_player_appearance_queries_back_its_sets_and_flags() {
    let documents = recorded();
    let played = documents[0]
        .appearances
        .iter()
        .find(|appearance| appearance.played())
        .expect("a played appearance")
        .clone();
    let graph = new_graph();
    assert_matchday(&graph, &documents[0]).expect("asserted");
    let appearance = iri(&format!("appearance/{}", played.appearance_id()));
    let sets = select(&graph, &sets_select(&appearance));
    assert_eq!(sets.len(), played.sets().scoreline().len());
    let rows = select(&graph, &played_select(&appearance));
    let row = rows.first().expect("row");
    assert_eq!(
        row.get("played").expect("played").to_string(),
        EXPECTED_PLAYED
    );
}

#[test]
fn asserting_the_same_document_twice_keeps_one_graph() {
    let documents = recorded();
    let graph = new_graph();
    let first = assert_matchday(&graph, &documents[0]).expect("first");
    let snapshot = canonical_snapshot(&graph).expect("snapshot");
    let second = assert_matchday(&graph, &documents[0]).expect("second");
    assert_eq!(first.commit_id, second.commit_id);
    assert_eq!(canonical_snapshot(&graph).expect("snapshot"), snapshot);
    assert_eq!(graph.quad_count().expect("count"), first.triples);
}

#[test]
fn a_document_that_fails_validation_writes_no_partial_graph() {
    let documents = recorded();
    let elsewhere = mint(DomainPrefix::Matchday, "matchday:elsewhere");
    let mut foreign_matchday = documents[0].clone();
    for appearance in &mut foreign_matchday.appearances {
        *appearance = foreign(appearance, elsewhere.clone());
    }
    let graph = new_graph();
    let error = assert_matchday(&graph, &foreign_matchday).expect_err("foreign appearance");
    assert!(matches!(error, GraphError::ForeignAppearance { .. }));
    assert_eq!(graph.quad_count().expect("count"), 0);

    let mut unscheduled_tie = documents[0].clone();
    let tie = unscheduled_tie.ties.remove(0);
    unscheduled_tie
        .appearances
        .retain(|appearance| appearance.tie_id() == &tie.tie_id);
    let graph = new_graph();
    let error = assert_matchday(&graph, &unscheduled_tie).expect_err("unscheduled tie");
    assert!(matches!(error, GraphError::UnknownTie { .. }));
    assert_eq!(graph.quad_count().expect("count"), 0);
}

#[test]
fn a_failed_batch_leaves_the_committed_graph_untouched() {
    let documents = recorded();
    let graph = new_graph();
    assert_matchday(&graph, &documents[0]).expect("asserted");
    let before = canonical_snapshot(&graph).expect("snapshot");
    let elsewhere = mint(DomainPrefix::Matchday, "matchday:elsewhere");
    let mut broken = documents[0].clone();
    for appearance in &mut broken.appearances {
        *appearance = foreign(appearance, elsewhere.clone());
    }
    assert!(assert_matchday(&graph, &broken).is_err());
    assert_eq!(canonical_snapshot(&graph).expect("snapshot"), before);
    for other in &documents[1..] {
        assert_matchday(&graph, other).expect("asserted");
    }
    assert!(canonical_snapshot(&graph).expect("after").len() > before.len());
}

#[test]
fn rebuilding_from_the_ledger_reproduces_a_byte_identical_snapshot() {
    let documents = recorded();
    let graph = new_graph();
    for document in &documents {
        assert_matchday(&graph, document).expect("asserted");
    }
    let snapshot = canonical_snapshot(&graph).expect("snapshot");
    let store = MemorySnapshotStore::default();
    save(&graph, &store, "kg/v1").expect("save");
    let rebuilt = new_graph();
    let report = rebuild(&rebuilt, &LedgerLog::new(documents.clone())).expect("rebuild");
    assert_eq!(report.documents, documents.len());
    assert_eq!(report.commits, documents.len());
    assert_eq!(canonical_snapshot(&rebuilt).expect("rebuild"), snapshot);
    let hydrated = new_graph();
    hydrate(&hydrated, &store, "kg/v1").expect("hydrate");
    assert_eq!(canonical_snapshot(&hydrated).expect("hydrated"), snapshot);
}

#[test]
fn a_rebuild_over_an_unhealthy_document_names_the_problem() {
    let documents = recorded();
    let mut broken = documents[0].clone();
    broken.ties.clear();
    let graph = new_graph();
    let error = assert_matchday(&graph, &broken).expect_err("appearance names unscheduled tie");
    assert!(matches!(error, GraphError::UnknownTie { .. }));
    assert_eq!(graph.quad_count().expect("count"), 0);
    let graph = new_graph();
    let error = rebuild(&graph, &LedgerLog::new(vec![broken])).expect_err("rebuild refuses");
    assert!(matches!(error, GraphError::UnknownTie { .. }));
    assert_eq!(graph.quad_count().expect("count"), 0);
}

#[test]
fn a_rebuilt_graph_holds_one_graph_per_document() {
    let documents = recorded();
    let log = LedgerLog::new(documents.clone());
    let graph = new_graph();
    rebuild(&graph, &log).expect("rebuild");
    assert_eq!(graph.named_graphs().expect("graphs").len(), documents.len());
}

#[test]
fn minting_is_stable_and_namespaced() {
    let left = mint(DomainPrefix::Player, "player:Facenda Samuele");
    let right = mint(DomainPrefix::Player, "player:Facenda Samuele");
    assert_eq!(left, right);
    assert_eq!(normalize("FACENDA SAMUELE"), "Facenda Samuele");
    assert_eq!(left.prefix(), DomainPrefix::Player);
}
