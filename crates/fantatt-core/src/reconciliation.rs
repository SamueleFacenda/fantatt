//! Identity reconciliation: one alias becomes one identity, a queued
//! ambiguity, or a provisional player.
//!
//! The pipeline funnels an alias through candidate classifica entries
//! (docs/06-data-pipeline.md section 3.6). Exactly one surviving identity
//! links; two or more stay ambiguous for a human; none leaves the player
//! provisional on rating 2000. Nothing here reads a clock or a database, so
//! the same decision is reproducible offline and in tests.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::identity::{Alias, Evidence, IdentityState, PlayerIdentity, normalise_name};
use crate::ids::Identifier;
use crate::rating::Rating;

/// One classifica sighting an identity could be behind an alias.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    identity: Identifier,
    normalised_name: String,
    evidence: Evidence,
}

impl Candidate {
    pub fn new(identity: Identifier, name: &str, evidence: Evidence) -> Self {
        Self {
            identity,
            normalised_name: normalise_name(name),
            evidence,
        }
    }

    pub fn identity(&self) -> &Identifier {
        &self.identity
    }

    pub fn normalised_name(&self) -> &str {
        &self.normalised_name
    }

    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
}

/// The evidence a match is narrowed by before a decision is taken.
///
/// Every field is optional: a wider scope admits more homonyms, a narrower
/// one resolves them by where and how often the name was seen.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchScope {
    competition: Option<Identifier>,
    region: Option<String>,
    minimum_matches: u32,
}

impl MatchScope {
    pub fn competition(mut self, competition: Identifier) -> Self {
        self.competition = Some(competition);
        self
    }

    pub fn region(mut self, region: &str) -> Self {
        self.region = Some(region.to_owned());
        self
    }

    /// The tolerance: a candidate needs at least this many evidence
    /// sightings to count as plausible.
    pub fn minimum_matches(mut self, minimum: u32) -> Self {
        self.minimum_matches = minimum;
        self
    }
}

/// What one reconciliation decided.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    /// Exactly one identity survived; the player links to it and inherits
    /// its classifica rating.
    Resolved {
        identity: Identifier,
        candidate: Candidate,
    },
    /// Two or more identities survived; a human decides, never auto-picked.
    Ambiguous { candidates: Vec<Candidate> },
    /// No identity survived; the player is provisional on rating 2000.
    Unresolved { provisional_rating: Rating },
}

impl Resolution {
    pub fn state(&self) -> IdentityState {
        match self {
            Self::Resolved { .. } => IdentityState::Resolved,
            Self::Ambiguous { .. } => IdentityState::Ambiguous,
            Self::Unresolved { .. } => IdentityState::Unresolved,
        }
    }

    /// The identities an ambiguous outcome waits on, ordered as the queue
    /// shows them.
    pub fn ambiguous_candidates(&self) -> &[Candidate] {
        match self {
            Self::Ambiguous { candidates } => candidates,
            _ => &[],
        }
    }
}

/// Everything one reconciliation decision needs.
#[derive(Debug, Clone)]
pub struct ReconciliationRequest {
    pub alias: Alias,
    pub candidates: Vec<Candidate>,
    pub scope: MatchScope,
    /// The rating an unresolved outcome carries; provisional by construction.
    pub provisional_rating: Rating,
}

/// Reconciles one alias: the outcome depends on how many identities survive
/// the scope, never on the order candidates arrive in.
pub fn reconcile(request: ReconciliationRequest) -> Resolution {
    let shortlist = shortlist(&request);
    match shortlist.as_slice() {
        [] => Resolution::Unresolved {
            provisional_rating: request.provisional_rating,
        },
        [single] => Resolution::Resolved {
            identity: single.identity().clone(),
            candidate: single.clone(),
        },
        multiple => Resolution::Ambiguous {
            candidates: multiple.to_vec(),
        },
    }
}

/// Candidates each identity's evidence supports, so a portal snapshot can be
/// reconciled without hand-building the candidate list.
pub fn candidates_from(identities: &[PlayerIdentity]) -> Vec<Candidate> {
    identities.iter().flat_map(candidates_of).collect()
}

fn candidates_of(identity: &PlayerIdentity) -> Vec<Candidate> {
    let Some(alias) = identity.aliases().first() else {
        return Vec::new();
    };
    identity
        .evidence()
        .iter()
        .map(|evidence| {
            Candidate::new(
                identity.internal_id().clone(),
                alias.name(),
                evidence.clone(),
            )
        })
        .collect()
}

/// why: one identity seen in several classifica entries is one identity, so
/// the shortlist collapses to one candidate per identity.
fn shortlist(request: &ReconciliationRequest) -> Vec<Candidate> {
    let wanted = normalise_name(request.alias.name());
    let mut ordered: Vec<&Candidate> = request
        .candidates
        .iter()
        .filter(|candidate| within_scope(candidate, &wanted, &request.scope))
        .collect();
    ordered.sort_by(candidate_ordering);
    ordered.dedup_by(|left, right| left.identity() == right.identity());
    ordered.into_iter().cloned().collect()
}

fn within_scope(candidate: &Candidate, wanted: &str, scope: &MatchScope) -> bool {
    candidate.normalised_name() == wanted
        && scope
            .competition
            .as_ref()
            .is_none_or(|competition| candidate.evidence().seen_in_competition() == competition)
        && scope
            .region
            .as_ref()
            .is_none_or(|region| candidate.evidence().entry().region() == region)
        && candidate.evidence().matches() >= scope.minimum_matches
}

fn candidate_ordering(left: &&Candidate, right: &&Candidate) -> Ordering {
    left.identity().cmp(right.identity()).then_with(|| {
        left.evidence()
            .entry()
            .classifica_id()
            .cmp(right.evidence().entry().classifica_id())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::ClassificaEntry;
    use crate::ids::DomainPrefix;
    use crate::rating::PROVISIONAL_RATING;
    use proptest::prelude::*;
    use time::Date;

    fn id(suffix: char) -> Identifier {
        Identifier::new(
            DomainPrefix::Player,
            &format!("01ARZ3XK7M0C4V6B8N2QWE9RT{suffix}"),
        )
        .expect("valid id")
    }

    fn competition() -> Identifier {
        Identifier::new(DomainPrefix::Competition, "01ARZ3XK7M0C4V6B8N2QWE9RTC").expect("id")
    }

    fn entry(classifica_id: &str, matches: u32, region: &str) -> Evidence {
        Evidence::new(
            ClassificaEntry::new(classifica_id, 2400, region),
            competition(),
            matches,
        )
    }

    fn candidate(suffix: char, name: &str, matches: u32) -> Candidate {
        Candidate::new(id(suffix), name, entry("2026-C1", matches, "TRENTINO"))
    }

    fn elsewhere(suffix: char, name: &str, region: &str) -> Candidate {
        Candidate::new(id(suffix), name, entry("2026-C1", 12, region))
    }

    fn request(candidates: Vec<Candidate>) -> ReconciliationRequest {
        ReconciliationRequest {
            alias: Alias::new("BENDONI LORENZO"),
            candidates,
            scope: MatchScope::default(),
            provisional_rating: Rating::provisional(
                Date::from_ordinal_date(2026, 100).expect("date"),
                "portal",
            ),
        }
    }

    #[test]
    fn a_single_surviving_identity_resolves() {
        let resolution = reconcile(request(vec![candidate('A', "BENDONI LORENZO", 12)]));
        let Resolution::Resolved {
            identity,
            candidate,
        } = resolution
        else {
            panic!("expected resolution");
        };
        assert_eq!(identity, id('A'));
        assert_eq!(candidate.evidence().entry().points(), 2400);
    }

    #[test]
    fn two_plausible_matches_stay_ambiguous_and_are_ordered() {
        let shuffled = vec![
            elsewhere('C', "Bendoni Lorenzo", "VENETO"),
            candidate('A', "BENDONI LORENZO", 12),
        ];
        let resolution = reconcile(request(shuffled));
        let Resolution::Ambiguous { candidates } = resolution else {
            panic!("expected ambiguity: {resolution:?}");
        };
        assert_eq!(
            candidates
                .iter()
                .map(|c| c.identity().clone())
                .collect::<Vec<_>>(),
            vec![id('A'), id('C')]
        );
    }

    #[test]
    fn no_surviving_candidate_leaves_the_player_provisional() {
        let resolution = reconcile(request(vec![candidate('A', "ROSSI MARCO", 12)]));
        let Resolution::Unresolved { provisional_rating } = resolution else {
            panic!("expected unresolved");
        };
        assert_eq!(provisional_rating.value(), PROVISIONAL_RATING);
        assert!(provisional_rating.is_provisional());
    }

    #[test]
    fn region_evidence_disambiguates_a_homonym() {
        let candidates = vec![
            elsewhere('C', "BENDONI LORENZO", "VENETO"),
            candidate('A', "BENDONI LORENZO", 12),
        ];
        let mut scoped = request(candidates);
        scoped.scope = MatchScope::default().region("TRENTINO");
        let Resolution::Resolved { identity, .. } = reconcile(scoped) else {
            panic!("expected resolution");
        };
        assert_eq!(identity, id('A'));
    }

    #[test]
    fn evidence_below_the_tolerance_is_not_plausible() {
        let mut scoped = request(vec![candidate('A', "BENDONI LORENZO", 1)]);
        scoped.scope = MatchScope::default().minimum_matches(6);
        assert_eq!(reconcile(scoped).state(), IdentityState::Unresolved);
    }

    #[test]
    fn one_identity_in_two_entries_is_not_ambiguous() {
        let candidates = vec![
            candidate('A', "BENDONI LORENZO", 12),
            Candidate::new(id('A'), "BENDONI LORENZO", entry("2026-C2", 9, "TRENTINO")),
        ];
        let Resolution::Resolved { identity, .. } = reconcile(request(candidates)) else {
            panic!("expected resolution");
        };
        assert_eq!(identity, id('A'));
    }

    #[test]
    fn the_outcome_state_matches_the_documented_labels() {
        assert_eq!(
            reconcile(request(vec![candidate('A', "BENDONI LORENZO", 12)])).state(),
            IdentityState::Resolved
        );
        assert_eq!(
            reconcile(request(vec![
                candidate('A', "BENDONI LORENZO", 12),
                elsewhere('B', "BENDONI LORENZO", "VENETO"),
            ]))
            .state(),
            IdentityState::Ambiguous
        );
    }

    #[test]
    fn candidates_come_from_the_identities_that_hold_them() {
        let mut identity = PlayerIdentity::unresolved(id('A'), Alias::new("BENDONI LORENZO"));
        identity.add_evidence(entry("2026-C1", 12, "TRENTINO"));
        let candidates = candidates_from(std::slice::from_ref(&identity));
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].identity(), &id('A'));
        assert_eq!(candidates[0].normalised_name(), "Bendoni Lorenzo");
    }

    proptest! {
        #[test]
        fn resolution_is_stable_under_input_ordering(
            first in candidate_strategy(),
            second in candidate_strategy(),
        ) {
            let forward = reconcile(request(vec![first.clone(), second.clone()]));
            let reversed = reconcile(request(vec![second, first]));
            prop_assert_eq!(forward, reversed);
        }
    }

    fn candidate_strategy() -> impl Strategy<Value = Candidate> {
        (
            prop::char::range('A', 'D'),
            "[A-Z ]{1,12}",
            "[A-Z]{4,8}",
            0u32..20,
        )
            .prop_map(|(suffix, name, region, matches)| {
                Candidate::new(id(suffix), &name, entry("2026-C1", matches, &region))
            })
    }
}
