use fantatt_core::appearance::PlayerAppearance;
use fantatt_core::ids::Identifier;
use fantatt_core::matchday::MatchdayStatus;
use oxigraph::io::{RdfFormat, RdfSerializer};
use oxigraph::model::{GraphName, Literal, NamedNode, Quad, Term, Triple};
use time::Date;

use crate::commit::{Commit, CommitId};
use crate::error::GraphError;
use crate::graph::Graph;
use crate::vocabulary::{self, data, ontology, prov, rdf_term};

/// One tie of the asserted matchday, with the decision the source recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TieFacts {
    pub tie_id: Identifier,
    pub decided: bool,
}

/// Everything one asserted document contributes to the graph: the matchday
/// identity, its ties, the per-player appearances, and the commit provenance.
#[derive(Debug, Clone)]
pub struct MatchdayFacts {
    pub matchday_id: Identifier,
    pub competition_id: Identifier,
    pub number: u16,
    pub scheduled_date: Option<Date>,
    pub status: MatchdayStatus,
    pub ties: Vec<TieFacts>,
    pub appearances: Vec<PlayerAppearance>,
    pub provenance: Commit,
}

/// What one assertion committed: its content address and triple count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asserted {
    pub commit_id: CommitId,
    pub triples: usize,
}

/// Asserts one matchday into its named assertion graph.
///
/// The whole document is materialized in memory first and written in one
/// storage transaction, so a document that fails validation (an appearance of
/// another matchday, an appearance naming a tie the matchday does not
/// schedule) leaves the graph exactly as it was.
pub fn assert_matchday(graph: &Graph, facts: &MatchdayFacts) -> Result<Asserted, GraphError> {
    validate(facts)?;
    let document = document_triples(facts)?;
    let commit_id = CommitId::hash_of(&canonical_bytes(&document)?);
    let graph_name = assertion_graph(&facts.matchday_id, &commit_id);
    let mut quads = to_quads(&document, &graph_name);
    quads.extend(to_quads(
        &commit_triples(facts, &commit_id, &graph_name)?,
        &graph_name,
    ));
    let triples = quads.len();
    if graph.has_graph(&graph_name)? {
        graph.remove_graph(&graph_name)?;
    }
    graph.insert_quads(&quads)?;
    Ok(Asserted { commit_id, triples })
}

fn validate(facts: &MatchdayFacts) -> Result<(), GraphError> {
    for appearance in &facts.appearances {
        if appearance.matchday_id() != &facts.matchday_id {
            return Err(GraphError::ForeignAppearance {
                appearance: appearance.appearance_id().to_string(),
                found: appearance.matchday_id().to_string(),
                expected: facts.matchday_id.to_string(),
            });
        }
        if !facts
            .ties
            .iter()
            .any(|tie| &tie.tie_id == appearance.tie_id())
        {
            return Err(GraphError::UnknownTie {
                appearance: appearance.appearance_id().to_string(),
                tie: appearance.tie_id().to_string(),
            });
        }
    }
    Ok(())
}

fn document_triples(facts: &MatchdayFacts) -> Result<Vec<Triple>, GraphError> {
    let matchday = matchday_node(&facts.matchday_id);
    let mut triples = matchday_triples(facts, &matchday)?;
    for tie in &facts.ties {
        triples.extend(tie_triples(tie, &matchday)?);
    }
    for appearance in &facts.appearances {
        triples.extend(appearance_triples(appearance, &matchday)?);
    }
    Ok(triples)
}

fn matchday_triples(
    facts: &MatchdayFacts,
    matchday: &NamedNode,
) -> Result<Vec<Triple>, GraphError> {
    let mut triples = vec![
        type_triple(matchday, &ontology("Matchday")?)?,
        integer_triple(matchday, &ontology("number")?, facts.number as i64)?,
        object_triple(
            matchday,
            &ontology("inCompetition")?,
            &node(&format!("competition/{}", facts.competition_id))?,
        )?,
        object_triple(
            matchday,
            &ontology("status")?,
            &ontology(status_label(facts.status))?,
        )?,
    ];
    triples.extend(scheduled_date_triples(facts, matchday)?);
    Ok(triples)
}

fn scheduled_date_triples(
    facts: &MatchdayFacts,
    matchday: &NamedNode,
) -> Result<Vec<Triple>, GraphError> {
    let Some(date) = facts.scheduled_date else {
        return Ok(Vec::new());
    };
    Ok(vec![literal_triple(
        matchday,
        &ontology("scheduledDate")?,
        &vocabulary::date_literal(date)?,
    )?])
}

fn tie_triples(tie: &TieFacts, matchday: &NamedNode) -> Result<Vec<Triple>, GraphError> {
    let node = tie_node(&tie.tie_id);
    Ok(vec![
        object_triple(matchday, &ontology("hasTie")?, &node)?,
        type_triple(&node, &ontology("Tie")?)?,
        object_triple(&node, &ontology("inMatchday")?, matchday)?,
        boolean_triple(&node, &ontology("decided")?, tie.decided)?,
    ])
}

fn appearance_triples(
    appearance: &PlayerAppearance,
    matchday: &NamedNode,
) -> Result<Vec<Triple>, GraphError> {
    let node = appearance_node(appearance.appearance_id());
    let mut triples = appearance_identity_triples(appearance, matchday, &node)?;
    triples.extend(outcome_triples(&node, appearance)?);
    triples.extend(flag_triples(&node, appearance)?);
    triples.extend(relation_triples(&node, appearance)?);
    for (ordinal, set) in appearance.sets().scoreline().iter().enumerate() {
        triples.extend(set_triples(
            appearance,
            &node,
            &SetAt {
                ordinal: ordinal + 1,
                player: set.player(),
                opponent: set.opponent(),
            },
        )?);
    }
    Ok(triples)
}

fn appearance_identity_triples(
    appearance: &PlayerAppearance,
    matchday: &NamedNode,
    node: &NamedNode,
) -> Result<Vec<Triple>, GraphError> {
    let kind = kind_label(appearance.kind());
    let mut triples = vec![
        type_triple(node, &ontology("PlayerAppearance")?)?,
        player_triple(node, "player", appearance.player_id())?,
        object_triple(node, &ontology("matchday")?, matchday)?,
        object_triple(node, &ontology("tie")?, &tie_node(appearance.tie_id()))?,
        object_triple(node, &ontology("appearanceKind")?, &ontology(kind)?)?,
        boolean_triple(node, &ontology("played")?, appearance.played())?,
    ];
    triples.push(integer_triple(
        node,
        &ontology("setsWon")?,
        appearance.sets().won() as i64,
    )?);
    triples.push(integer_triple(
        node,
        &ontology("setsLost")?,
        appearance.sets().lost() as i64,
    )?);
    Ok(triples)
}

struct SetAt {
    ordinal: usize,
    player: u8,
    opponent: u8,
}

fn kind_label(kind: fantatt_core::appearance::AppearanceKind) -> &'static str {
    match kind {
        fantatt_core::appearance::AppearanceKind::Singles => "Singles",
        fantatt_core::appearance::AppearanceKind::Doubles => "Doubles",
    }
}

fn flag_triples(
    node: &NamedNode,
    appearance: &PlayerAppearance,
) -> Result<Vec<Triple>, GraphError> {
    let flags = appearance.flags();
    let flags = [
        ("isAway", flags.is_away),
        ("isDecider", flags.is_decider),
        ("isComeback", flags.is_comeback),
        ("isTeamWin", flags.is_team_win),
        ("isDoublet", flags.is_doublet),
    ];
    flags
        .iter()
        .map(|(predicate, value)| boolean_triple(node, &ontology(predicate)?, *value))
        .collect()
}

fn relation_triples(
    node: &NamedNode,
    appearance: &PlayerAppearance,
) -> Result<Vec<Triple>, GraphError> {
    let mut triples = Vec::new();
    if let Some(partner) = appearance.matchup().partner() {
        triples.push(player_triple(node, "partner", partner)?);
    }
    if let Some(opponent) = appearance.matchup().opponent() {
        triples.push(player_triple(node, "opponent", opponent)?);
    }
    if let Some(rating) = appearance.matchup().opponent_rating() {
        triples.push(integer_triple(
            node,
            &ontology("opponentRatingAdjusted")?,
            rating,
        )?);
    }
    Ok(triples)
}

fn outcome_triples(
    node: &NamedNode,
    appearance: &PlayerAppearance,
) -> Result<Vec<Triple>, GraphError> {
    let label = match appearance.outcome() {
        Some(fantatt_core::appearance::Outcome::Win) => "Win",
        Some(fantatt_core::appearance::Outcome::Loss) => "Loss",
        None => return Ok(Vec::new()),
    };
    Ok(vec![object_triple(
        node,
        &ontology("outcome")?,
        &ontology(label)?,
    )?])
}

fn set_triples(
    appearance: &PlayerAppearance,
    subject: &NamedNode,
    set: &SetAt,
) -> Result<Vec<Triple>, GraphError> {
    let set_node = node(&format!(
        "set/{}/{}",
        appearance.appearance_id(),
        set.ordinal
    ))?;
    Ok(vec![
        object_triple(subject, &ontology("hasSet")?, &set_node)?,
        type_triple(&set_node, &ontology("Set")?)?,
        integer_triple(&set_node, &ontology("setOrdinal")?, set.ordinal as i64)?,
        integer_triple(&set_node, &ontology("setPlayerPoints")?, set.player as i64)?,
        integer_triple(
            &set_node,
            &ontology("setOpponentPoints")?,
            set.opponent as i64,
        )?,
    ])
}

fn commit_triples(
    facts: &MatchdayFacts,
    commit_id: &CommitId,
    graph_name: &NamedNode,
) -> Result<Vec<Triple>, GraphError> {
    let mut triples = commit_node_triples(facts, commit_id, graph_name)?;
    let asserted = vocabulary::date_time_literal(facts.provenance.asserted_at)?;
    let commit = data(&format!("commit/{}", commit_id.as_str()))?;
    for node in document_nodes(facts)? {
        triples.push(literal_triple(
            &node,
            &ontology("bitemporalAssertedAt")?,
            &asserted,
        )?);
        triples.push(object_triple(&node, &prov("wasGeneratedBy")?, &commit)?);
        if let Some(valid_from) = facts.provenance.valid_from {
            let valid = vocabulary::date_literal(valid_from)?;
            triples.push(literal_triple(
                &node,
                &ontology("bitemporalValidFrom")?,
                &valid,
            )?);
        }
    }
    Ok(triples)
}

fn commit_node_triples(
    facts: &MatchdayFacts,
    commit_id: &CommitId,
    graph_name: &NamedNode,
) -> Result<Vec<Triple>, GraphError> {
    let commit = data(&format!("commit/{}", commit_id.as_str()))?;
    let actor = data(&format!(
        "agent/{}",
        vocabulary::segment(&facts.provenance.actor)
    ))?;
    let asserted = vocabulary::date_time_literal(facts.provenance.asserted_at)?;
    let mut triples = vec![
        type_triple(&commit, &prov("Activity")?)?,
        type_triple(&commit, &ontology("AssertionCommit")?)?,
        string_triple(&commit, &ontology("commitId")?, commit_id.as_str())?,
        object_triple(&commit, &prov("wasAttributedTo")?, &actor)?,
        literal_triple(&commit, &prov("startedAt")?, &asserted)?,
        literal_triple(&commit, &prov("endedAt")?, &asserted)?,
        string_triple(&commit, &ontology("graphName")?, graph_name.as_str())?,
    ];
    triples.extend(commit_statement_triples(&commit, &facts.provenance)?);
    Ok(triples)
}

fn commit_statement_triples(
    commit: &NamedNode,
    provenance: &Commit,
) -> Result<Vec<Triple>, GraphError> {
    Ok(vec![
        string_triple(
            commit,
            &ontology("importVersion")?,
            &provenance.import_version,
        )?,
        string_triple(commit, &ontology("sourceUrl")?, &provenance.source_url)?,
        string_triple(commit, &ontology("contentHash")?, &provenance.document_hash)?,
    ])
}

fn document_nodes(facts: &MatchdayFacts) -> Result<Vec<NamedNode>, GraphError> {
    let mut nodes = vec![matchday_node(&facts.matchday_id)];
    nodes.extend(
        facts
            .ties
            .iter()
            .map(|tie| tie_node(&tie.tie_id))
            .collect::<Vec<_>>(),
    );
    nodes.extend(
        facts
            .appearances
            .iter()
            .map(|appearance| appearance_node(appearance.appearance_id()))
            .collect::<Vec<_>>(),
    );
    Ok(nodes)
}

fn canonical_bytes(triples: &[Triple]) -> Result<Vec<u8>, GraphError> {
    let mut lines: Vec<String> = Vec::with_capacity(triples.len());
    for triple in triples {
        let mut serializer = RdfSerializer::from_format(RdfFormat::NTriples).for_writer(Vec::new());
        serializer.serialize_triple(triple)?;
        let bytes = serializer.finish()?;
        lines.push(String::from_utf8(bytes).map_err(|_| GraphError::NotSolutions)?);
    }
    lines.sort();
    Ok(lines.concat().into_bytes())
}

fn to_quads(triples: &[Triple], graph_name: &NamedNode) -> Vec<Quad> {
    triples
        .iter()
        .map(|triple| {
            Quad::new(
                triple.subject.clone(),
                triple.predicate.clone(),
                triple.object.clone(),
                GraphName::NamedNode(graph_name.clone()),
            )
        })
        .collect()
}

fn assertion_graph(matchday_id: &Identifier, commit_id: &CommitId) -> NamedNode {
    data(&format!(
        "matchday/{}/asserted/{}",
        matchday_id,
        commit_id.as_str()
    ))
    .expect("assertion graph iri")
}

fn node(path: &str) -> Result<NamedNode, GraphError> {
    data(path)
}

fn matchday_node(matchday_id: &Identifier) -> NamedNode {
    node(&format!("matchday/{}", matchday_id)).expect("matchday iri")
}

fn appearance_node(appearance_id: &Identifier) -> NamedNode {
    node(&format!("appearance/{}", appearance_id)).expect("appearance iri")
}

fn tie_node(tie_id: &Identifier) -> NamedNode {
    node(&format!("tie/{}", tie_id)).expect("tie iri")
}

fn player_node(player_id: &Identifier) -> NamedNode {
    node(&format!("player/{}", player_id)).expect("player iri")
}

fn status_label(status: MatchdayStatus) -> &'static str {
    match status {
        MatchdayStatus::Scheduled => "Scheduled",
        MatchdayStatus::Partial => "Partial",
        MatchdayStatus::Complete => "Complete",
        MatchdayStatus::Cancelled => "Cancelled",
    }
}

fn type_triple(subject: &NamedNode, class: &NamedNode) -> Result<Triple, GraphError> {
    Ok(Triple::new(
        subject.clone(),
        rdf_term("type")?,
        Term::NamedNode(class.clone()),
    ))
}

fn object_triple(
    subject: &NamedNode,
    predicate: &NamedNode,
    object: &NamedNode,
) -> Result<Triple, GraphError> {
    Ok(Triple::new(
        subject.clone(),
        predicate.clone(),
        Term::NamedNode(object.clone()),
    ))
}

fn player_triple(
    subject: &NamedNode,
    predicate: &str,
    player_id: &Identifier,
) -> Result<Triple, GraphError> {
    object_triple(subject, &ontology(predicate)?, &player_node(player_id))
}

fn integer_triple(
    subject: &NamedNode,
    predicate: &NamedNode,
    value: i64,
) -> Result<Triple, GraphError> {
    Ok(Triple::new(
        subject.clone(),
        predicate.clone(),
        Term::Literal(vocabulary::integer_literal(value)?),
    ))
}

fn boolean_triple(
    subject: &NamedNode,
    predicate: &NamedNode,
    value: bool,
) -> Result<Triple, GraphError> {
    Ok(Triple::new(
        subject.clone(),
        predicate.clone(),
        Term::Literal(vocabulary::boolean_literal(value)?),
    ))
}

fn string_triple(
    subject: &NamedNode,
    predicate: &NamedNode,
    value: &str,
) -> Result<Triple, GraphError> {
    Ok(Triple::new(
        subject.clone(),
        predicate.clone(),
        Term::Literal(vocabulary::string_literal(value)),
    ))
}

fn literal_triple(
    subject: &NamedNode,
    predicate: &NamedNode,
    value: &Literal,
) -> Result<Triple, GraphError> {
    Ok(Triple::new(
        subject.clone(),
        predicate.clone(),
        Term::Literal(value.clone()),
    ))
}
