use std::fmt;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::ids::Identifier;

/// Resolution state of a player identity.
///
/// Ambiguity is surfaced, never guessed: a wrong merge silently corrupts
/// standings, so an ambiguous identity waits for a human decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IdentityState {
    /// Exactly one classifica entry matches within tolerance.
    Resolved,
    /// Two or more candidates; a manager or admin decides, never auto-picked.
    Ambiguous,
    /// No candidate matched; the player is provisional.
    Unresolved,
}

impl fmt::Display for IdentityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Resolved => "RESOLVED",
            Self::Ambiguous => "AMBIGUOUS",
            Self::Unresolved => "UNRESOLVED",
        };
        f.write_str(label)
    }
}

/// One name variant a player is known by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Alias {
    name: String,
    normalised: String,
}

impl Alias {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            normalised: normalise_name(name),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn normalised(&self) -> &str {
        &self.normalised
    }
}

/// why: one shared, pure normalisation keeps ingest, reasoning, and API
/// matching the same players (docs/02-domain-model.md section 4).
pub fn normalise_name(name: &str) -> String {
    name.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One row of the published classifica a match was checked against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificaEntry {
    classifica_id: String,
    points: i64,
    region: String,
}

impl ClassificaEntry {
    pub fn new(classifica_id: &str, points: i64, region: &str) -> Self {
        Self {
            classifica_id: classifica_id.to_owned(),
            points,
            region: region.to_owned(),
        }
    }
}

/// Evidence that a classifica entry refers to this player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    entry: ClassificaEntry,
    seen_in_competition: Identifier,
    matches: u32,
}

impl Evidence {
    pub fn new(entry: ClassificaEntry, seen_in_competition: Identifier, matches: u32) -> Self {
        Self {
            entry,
            seen_in_competition,
            matches,
        }
    }
}

/// The identity reconciliation record behind a player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerIdentity {
    internal_id: Identifier,
    aliases: Vec<Alias>,
    evidence: Vec<Evidence>,
    state: IdentityState,
    resolved_at: Option<Date>,
}

impl PlayerIdentity {
    /// An identity starts unresolved: provisional until evidence says otherwise.
    pub fn unresolved(internal_id: Identifier, alias: Alias) -> Self {
        Self {
            internal_id,
            aliases: vec![alias],
            evidence: Vec::new(),
            state: IdentityState::Unresolved,
            resolved_at: None,
        }
    }

    pub fn internal_id(&self) -> &Identifier {
        &self.internal_id
    }

    pub fn state(&self) -> IdentityState {
        self.state
    }

    pub fn resolved_at(&self) -> Option<Date> {
        self.resolved_at
    }

    pub fn add_alias(&mut self, alias: Alias) {
        let known = self
            .aliases
            .iter()
            .any(|existing| existing.normalised() == alias.normalised());
        if !known {
            self.aliases.push(alias);
        }
    }

    pub fn add_evidence(&mut self, evidence: Evidence) {
        if !self.evidence.contains(&evidence) {
            self.evidence.push(evidence);
        }
    }

    /// Records a human resolution decision against a candidate.
    pub fn resolve(&mut self, as_of: Date) {
        self.state = IdentityState::Resolved;
        self.resolved_at = Some(as_of);
    }

    /// Flags the identity as ambiguous; the decision stays with a human.
    pub fn mark_ambiguous(&mut self) {
        self.state = IdentityState::Ambiguous;
        self.resolved_at = None;
    }
}

/// Who performed a merge and when.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeAudit {
    actor: Identifier,
    merged_at: Date,
}

impl MergeAudit {
    pub fn new(actor: Identifier, merged_at: Date) -> Self {
        Self { actor, merged_at }
    }
}

/// The audited record of an explicit admin merge between two identities.
///
/// Merging re-derives every affected score, so the record carries both sides
/// and the moment of the decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityMerge {
    absorbed: Identifier,
    survivor: Identifier,
    audit: MergeAudit,
}

impl IdentityMerge {
    pub fn new(absorbed: Identifier, survivor: Identifier, audit: MergeAudit) -> Self {
        Self {
            absorbed,
            survivor,
            audit,
        }
    }

    pub fn absorbed(&self) -> &Identifier {
        &self.absorbed
    }

    pub fn survivor(&self) -> &Identifier {
        &self.survivor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn id() -> Identifier {
        Identifier::new(
            crate::ids::DomainPrefix::Player,
            "01ARZ3XK7M0C4V6B8N2QWE9RTA",
        )
        .expect("valid id")
    }

    #[test]
    fn identity_starts_unresolved() {
        let identity = PlayerIdentity::unresolved(id(), Alias::new("bendoni lorenzo"));
        assert_eq!(identity.state(), IdentityState::Unresolved);
        assert_eq!(identity.resolved_at(), None);
    }

    #[test]
    fn resolution_is_explicit_and_dated() {
        let mut identity = PlayerIdentity::unresolved(id(), Alias::new("Bendoni Lorenzo"));
        let date = Date::from_ordinal_date(2026, 100).expect("valid date");
        identity.resolve(date);
        assert_eq!(identity.state(), IdentityState::Resolved);
        assert_eq!(identity.resolved_at(), Some(date));
    }

    #[test]
    fn ambiguity_is_never_auto_resolved() {
        let mut identity = PlayerIdentity::unresolved(id(), Alias::new("Bendoni Lorenzo"));
        identity.mark_ambiguous();
        assert_eq!(identity.state(), IdentityState::Ambiguous);
        assert_eq!(identity.resolved_at(), None);
    }

    #[test]
    fn aliases_deduplicate_by_normalised_form() {
        let mut identity = PlayerIdentity::unresolved(id(), Alias::new("Bendoni Lorenzo"));
        identity.add_alias(Alias::new("BENDONI   LORENZO"));
        identity.add_alias(Alias::new("Bendoni Lorenzo"));
        assert_eq!(identity.aliases.len(), 1);
    }

    proptest! {
        #[test]
        fn normalisation_is_idempotent(name in "[a-zA-Z]{1,20}( [a-zA-Z]{1,20}){0,3}") {
            let once = normalise_name(&name);
            prop_assert_eq!(normalise_name(&once), once);
        }

        #[test]
        fn normalisation_collapses_case_and_spacing(surname in "[a-zA-Z]{1,10}", first in "[a-zA-Z]{1,10}") {
            let shouty = format!("{surname}  {first}");
            let quiet = format!("{} {}", surname.to_lowercase(), first.to_lowercase());
            prop_assert_eq!(normalise_name(&shouty), normalise_name(&quiet));
        }
    }
}
