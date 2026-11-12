use serde::{Deserialize, Serialize};
use time::Date;

use crate::error::DomainError;
use crate::ids::Identifier;

/// Which score a derivation belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreKey {
    player: Identifier,
    matchday: Identifier,
}

impl ScoreKey {
    pub fn new(player: Identifier, matchday: Identifier) -> Self {
        Self { player, matchday }
    }
}

/// The provenance of a derived fact: which rules produced it, from which
/// input revision, at what time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivationRecord {
    ruleset_version: String,
    input_revision: u32,
    computed_at: Date,
}

impl DerivationRecord {
    pub fn new(ruleset_version: &str, input_revision: u32, computed_at: Date) -> Self {
        Self {
            ruleset_version: ruleset_version.to_owned(),
            input_revision,
            computed_at,
        }
    }

    pub fn ruleset_version(&self) -> &str {
        &self.ruleset_version
    }

    pub fn input_revision(&self) -> u32 {
        self.input_revision
    }

    pub fn computed_at(&self) -> Date {
        self.computed_at
    }
}

/// Fantasy points derived for one player in one matchday.
///
/// A stored score never exists without a derivation record (INV-13), and the
/// record always names the rule bundle version that produced it (INV-14), so
/// a ruleset update can invalidate every affected score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedScore {
    key: ScoreKey,
    points: i32,
    derivation: DerivationRecord,
}

impl DerivedScore {
    pub fn new(
        key: ScoreKey,
        points: i32,
        derivation: DerivationRecord,
    ) -> Result<Self, DomainError> {
        if derivation.ruleset_version().trim().is_empty() {
            return Err(DomainError::RuleVersionMissing {
                player: key.player.clone(),
            });
        }
        Ok(Self {
            key,
            points,
            derivation,
        })
    }

    pub fn player(&self) -> &Identifier {
        &self.key.player
    }

    pub fn matchday(&self) -> &Identifier {
        &self.key.matchday
    }

    pub fn points(&self) -> i32 {
        self.points
    }

    pub fn ruleset_version(&self) -> &str {
        self.derivation.ruleset_version()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use proptest::prelude::*;

    fn id(prefix: DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    fn key() -> ScoreKey {
        ScoreKey::new(
            id(DomainPrefix::Player, 'A'),
            id(DomainPrefix::Matchday, 'B'),
        )
    }

    fn record(version: &str) -> DerivationRecord {
        DerivationRecord::new(
            version,
            7,
            Date::from_ordinal_date(2026, 100).expect("valid date"),
        )
    }

    #[test]
    fn a_score_always_carries_its_derivation_record() {
        let score = DerivedScore::new(key(), 42, record("1.2.0")).expect("valid derivation");
        assert_eq!(score.points(), 42);
        assert_eq!(score.ruleset_version(), "1.2.0");
        assert_eq!(score.player(), &id(DomainPrefix::Player, 'A'));
        assert_eq!(score.matchday(), &id(DomainPrefix::Matchday, 'B'));
    }

    #[test]
    fn a_score_without_a_rule_version_is_rejected() {
        let error = DerivedScore::new(key(), 42, record("")).expect_err("no rule version");
        assert_eq!(error.code(), "RULE_VERSION_MISSING");
    }

    #[test]
    fn derivation_records_carry_their_input_revision() {
        let record = record("2.0.0");
        assert_eq!(record.input_revision(), 7);
        assert_eq!(record.ruleset_version(), "2.0.0");
        assert_eq!(
            record.computed_at(),
            Date::from_ordinal_date(2026, 100).expect("valid date")
        );
    }

    proptest! {
        #[test]
        fn every_non_empty_rule_version_yields_a_score(
            version in "1\\.[0-9]{1,2}\\.[0-9]{1,2}",
            points in i32::MIN..i32::MAX,
        ) {
            let score = DerivedScore::new(key(), points, record(&version))?;
            prop_assert_eq!(score.ruleset_version(), version);
            prop_assert_eq!(score.points(), points);
        }
    }

    #[test]
    fn whitespace_only_rule_versions_are_rejected() {
        assert!(DerivedScore::new(key(), 0, record("   ")).is_err());
    }
}
