use serde::{Deserialize, Serialize};
use time::Date;

use crate::error::DomainError;
use crate::ids::Identifier;

/// Status of one round of a real championship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MatchdayStatus {
    Scheduled,
    Partial,
    Complete,
    Cancelled,
}

/// The matchday plus the competition it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchdayIdentity {
    matchday_id: Identifier,
    competition_id: Identifier,
}

impl MatchdayIdentity {
    pub fn new(matchday_id: Identifier, competition_id: Identifier) -> Self {
        Self {
            matchday_id,
            competition_id,
        }
    }
}

/// One scheduled tie, tracked until its result arrives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTie {
    tie_id: Identifier,
    decided: bool,
}

impl ScheduledTie {
    pub fn new(tie_id: Identifier) -> Self {
        Self {
            tie_id,
            decided: false,
        }
    }

    pub fn tie_id(&self) -> &Identifier {
        &self.tie_id
    }

    pub fn is_decided(&self) -> bool {
        self.decided
    }
}

/// One round of a real championship: the atomic unit of results.
///
/// A matchday is complete only when every scheduled tie has a result (INV-10);
/// a matchday with some results is partial and flagged, never silently
/// treated as complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Matchday {
    matchday_id: Identifier,
    competition_id: Identifier,
    number: u16,
    scheduled_date: Date,
    status: MatchdayStatus,
    ties: Vec<ScheduledTie>,
}

impl Matchday {
    pub fn new(identity: MatchdayIdentity, number: u16, scheduled_date: Date) -> Self {
        Self {
            matchday_id: identity.matchday_id,
            competition_id: identity.competition_id,
            number,
            scheduled_date,
            status: MatchdayStatus::Scheduled,
            ties: Vec::new(),
        }
    }

    pub fn matchday_id(&self) -> &Identifier {
        &self.matchday_id
    }

    pub fn competition_id(&self) -> &Identifier {
        &self.competition_id
    }

    pub fn number(&self) -> u16 {
        self.number
    }

    pub fn status(&self) -> MatchdayStatus {
        self.status
    }

    pub fn scheduled_ties(&self) -> &[ScheduledTie] {
        &self.ties
    }

    pub fn add_tie(&mut self, tie_id: Identifier) {
        if !self.ties.iter().any(|tie| tie.tie_id() == &tie_id) {
            self.ties.push(ScheduledTie::new(tie_id));
        }
    }

    /// Records a tie result and re-derives the matchday status.
    pub fn record_tie_result(&mut self, tie_id: &Identifier) {
        for tie in &mut self.ties {
            if tie.tie_id() == tie_id {
                tie.decided = true;
            }
        }
        self.status = self.derived_status();
    }

    /// Completes the matchday, refusing while ties are undecided (INV-10).
    pub fn complete(&mut self) -> Result<(), DomainError> {
        if self.status == MatchdayStatus::Cancelled {
            return Err(DomainError::MatchdayIncomplete {
                matchday: self.matchday_id.clone(),
                scheduled: self.ties.len() as u16,
                decided: self.decided_count(),
            });
        }
        if self.decided_count() != self.ties.len() as u16 {
            return Err(DomainError::MatchdayIncomplete {
                matchday: self.matchday_id.clone(),
                scheduled: self.ties.len() as u16,
                decided: self.decided_count(),
            });
        }
        self.status = MatchdayStatus::Complete;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.status = MatchdayStatus::Cancelled;
    }

    fn derived_status(&self) -> MatchdayStatus {
        if self.status == MatchdayStatus::Cancelled {
            return MatchdayStatus::Cancelled;
        }
        if self.ties.is_empty() {
            MatchdayStatus::Scheduled
        } else if self.decided_count() == self.ties.len() as u16 {
            MatchdayStatus::Complete
        } else if self.decided_count() > 0 {
            MatchdayStatus::Partial
        } else {
            MatchdayStatus::Scheduled
        }
    }

    fn decided_count(&self) -> u16 {
        self.ties.iter().filter(|tie| tie.is_decided()).count() as u16
    }

    /// why: a partial matchday must report itself as partial, never as complete.
    pub fn is_partial(&self) -> bool {
        self.status == MatchdayStatus::Partial
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

    fn matchday(tie_count: usize) -> Matchday {
        let mut matchday = Matchday::new(
            MatchdayIdentity::new(
                id(DomainPrefix::Matchday, 'A'),
                id(DomainPrefix::Competition, 'B'),
            ),
            7,
            Date::from_ordinal_date(2026, 100).expect("valid date"),
        );
        for index in 0..tie_count {
            matchday.add_tie(id(DomainPrefix::Tie, index_alphabet(index)));
        }
        matchday
    }

    fn index_alphabet(index: usize) -> char {
        b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"[index] as char
    }

    #[test]
    fn scheduled_matchday_has_no_results_yet() {
        let matchday = matchday(8);
        assert_eq!(matchday.status(), MatchdayStatus::Scheduled);
        assert_eq!(matchday.scheduled_ties().len(), 8);
    }

    #[test]
    fn partial_results_flag_the_matchday_as_partial() {
        let mut matchday = matchday(8);
        let first = matchday.scheduled_ties()[0].tie_id().clone();
        matchday.record_tie_result(&first);
        assert!(matchday.is_partial());
        assert!(matchday.complete().is_err());
    }

    #[test]
    fn completing_requires_every_tie_decided() {
        let mut matchday = matchday(3);
        let ties: Vec<Identifier> = matchday
            .scheduled_ties()
            .iter()
            .map(|tie| tie.tie_id().clone())
            .collect();
        for tie in &ties[..2] {
            matchday.record_tie_result(tie);
        }
        assert_eq!(
            matchday.complete(),
            Err(DomainError::MatchdayIncomplete {
                matchday: id(DomainPrefix::Matchday, 'A'),
                scheduled: 3,
                decided: 2,
            })
        );
        matchday.record_tie_result(&ties[2]);
        matchday.complete().expect("all ties decided");
        assert_eq!(matchday.status(), MatchdayStatus::Complete);
    }

    #[test]
    fn cancelled_matchdays_cannot_complete() {
        let mut matchday = matchday(1);
        matchday.cancel();
        assert!(matchday.complete().is_err());
        let tie = matchday.scheduled_ties()[0].tie_id().clone();
        matchday.record_tie_result(&tie);
        assert_eq!(matchday.status(), MatchdayStatus::Cancelled);
    }

    #[test]
    fn duplicate_ties_are_ignored() {
        let mut matchday = matchday(2);
        let first = matchday.scheduled_ties()[0].tie_id().clone();
        matchday.add_tie(first);
        assert_eq!(matchday.scheduled_ties().len(), 2);
    }

    #[test]
    fn matchday_exposes_its_identity() {
        let mut matchday = matchday(3);
        assert_eq!(matchday.matchday_id(), &id(DomainPrefix::Matchday, 'A'));
        assert_eq!(matchday.competition_id(), &id(DomainPrefix::Competition, 'B'));
        assert_eq!(matchday.number(), 7);
        assert_eq!(matchday.status(), MatchdayStatus::Scheduled);
        let tie = matchday.scheduled_ties()[0].tie_id().clone();
        assert!(!matchday.is_partial());
        matchday.record_tie_result(&id(DomainPrefix::Tie, 'Z'));
        assert_eq!(matchday.status(), MatchdayStatus::Scheduled);
        matchday.record_tie_result(&tie);
        assert!(matchday.is_partial());
    }

    #[test]
    fn a_result_without_scheduled_ties_stays_scheduled() {
        let mut matchday = matchday(0);
        assert_eq!(matchday.scheduled_ties().len(), 0);
        matchday.record_tie_result(&id(DomainPrefix::Tie, 'A'));
        assert_eq!(matchday.status(), MatchdayStatus::Scheduled);
    }

    #[test]
    fn added_ties_report_their_state() {
        let mut matchday = matchday(1);
        let tie = matchday.scheduled_ties()[0].clone();
        assert!(!tie.is_decided());
        assert_eq!(tie.tie_id(), &id(DomainPrefix::Tie, '0'));
        matchday.record_tie_result(tie.tie_id());
        assert!(matchday.scheduled_ties()[0].is_decided());
    }

    proptest! {
        #[test]
        fn completion_holds_only_for_full_result_sets(
            decided in prop::collection::hash_set(0usize..8, 0..=8),
        ) {
            let mut matchday = matchday(8);
            let ties: Vec<Identifier> = matchday
                .scheduled_ties()
                .iter()
                .map(|tie| tie.tie_id().clone())
                .collect();
            for index in decided.iter().copied().collect::<std::collections::BTreeSet<_>>() {
                matchday.record_tie_result(&ties[index]);
            }
            let full = decided.len() == 8;
            prop_assert_eq!(matchday.complete().is_ok(), full);
            if !full && !decided.is_empty() {
                prop_assert!(matchday.is_partial());
            }
        }
    }
}
