use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::calendar::MonthYear;
use crate::division::DivisionTier;
use crate::error::DomainError;
use crate::ids::Identifier;

/// Where a slot sits in the lineup: active starters or bench.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Position {
    Starter,
    Bench,
}

/// Error produced when a slot cannot be parsed from its wire form.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SlotParseError {
    #[error("slot position must be 'starter' or 'bench'")]
    InvalidPosition,
    #[error("slot player must be a prefixed identifier")]
    InvalidPlayer,
}

/// One of the fifteen squad slots: a tier, a position, and its occupant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Slot {
    tier: DivisionTier,
    position: Position,
    player: Option<Identifier>,
}

impl Slot {
    pub fn new(tier: DivisionTier, position: Position, player: Option<Identifier>) -> Self {
        Self {
            tier,
            position,
            player,
        }
    }

    pub fn tier(&self) -> DivisionTier {
        self.tier
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn player(&self) -> Option<&Identifier> {
        self.player.as_ref()
    }

    pub fn is_filled(&self) -> bool {
        self.player.is_some()
    }

    fn from_wire(text: &str) -> Result<Self, SlotParseError> {
        let mut fields = text.split('|');
        let tier = fields
            .next()
            .and_then(|raw| raw.parse::<DivisionTier>().ok())
            .ok_or(SlotParseError::InvalidPosition)?;
        let position = match fields.next() {
            Some("starter") => Position::Starter,
            Some("bench") => Position::Bench,
            _ => return Err(SlotParseError::InvalidPosition),
        };
        let player = match fields.next() {
            Some("") | None => None,
            Some(raw) => Some(
                raw.parse::<Identifier>()
                    .map_err(|_| SlotParseError::InvalidPlayer)?,
            ),
        };
        Ok(Self::new(tier, position, player))
    }
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let position = match self.position {
            Position::Starter => "starter",
            Position::Bench => "bench",
        };
        match &self.player {
            Some(player) => write!(f, "{}|{position}|{player}", self.tier),
            None => write!(f, "{}|{position}|", self.tier),
        }
    }
}

impl std::str::FromStr for Slot {
    type Err = SlotParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_wire(s)
    }
}

impl TryFrom<String> for Slot {
    type Error = SlotParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_wire(&value)
    }
}

impl From<Slot> for String {
    fn from(slot: Slot) -> Self {
        slot.to_string()
    }
}

/// How many starters and bench players each division tier contributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquadComposition {
    starters_per_tier: u8,
    bench_per_tier: u8,
}

impl SquadComposition {
    /// The regulation squad: 2 starters + 1 bench per tier across 5 tiers.
    pub const REGULATION: Self = Self {
        starters_per_tier: 2,
        bench_per_tier: 1,
    };

    pub fn new(starters_per_tier: u8, bench_per_tier: u8) -> Self {
        Self {
            starters_per_tier,
            bench_per_tier,
        }
    }

    pub fn starters_per_tier(&self) -> u8 {
        self.starters_per_tier
    }

    pub fn bench_per_tier(&self) -> u8 {
        self.bench_per_tier
    }

    pub fn slots_per_tier(&self) -> u8 {
        self.starters_per_tier + self.bench_per_tier
    }

    pub fn total_slots(&self) -> u16 {
        u16::from(self.slots_per_tier()) * DivisionTier::ALL.len() as u16
    }
}

/// Lifecycle of a squad: mutable until lock, then immutable until unlock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SquadStatus {
    Draft,
    Submitted,
    Locked,
    Scored,
}

/// Identity of a squad across its aggregate boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SquadIdentity {
    squad_id: Identifier,
    league_id: Identifier,
    manager_id: Identifier,
}

impl SquadIdentity {
    pub fn new(squad_id: Identifier, league_id: Identifier, manager_id: Identifier) -> Self {
        Self {
            squad_id,
            league_id,
            manager_id,
        }
    }
}

/// A player selected for a slot together with the tier they actively play in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotAssignment {
    player: Identifier,
    tier: DivisionTier,
}

impl SlotAssignment {
    pub fn new(player: Identifier, tier: DivisionTier) -> Self {
        Self { player, tier }
    }
}

/// Everything a squad needs besides its slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SquadSpec {
    identity: SquadIdentity,
    month: MonthYear,
    composition: SquadComposition,
}

impl SquadSpec {
    pub fn new(identity: SquadIdentity, month: MonthYear, composition: SquadComposition) -> Self {
        Self {
            identity,
            month,
            composition,
        }
    }
}

/// A persisted squad awaiting reconstruction, slots included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSquad {
    spec: SquadSpec,
    slots: Vec<Slot>,
}

/// The audited, versioned admin action that unlocks a locked squad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminUnlock {
    actor: Identifier,
    reason: String,
    revision: u32,
}

impl AdminUnlock {
    pub fn new(actor: Identifier, reason: &str, revision: u32) -> Self {
        Self {
            actor,
            reason: reason.to_owned(),
            revision,
        }
    }
}

/// A manager's fifteen picks for one month, guarded by INV-2, INV-3, INV-6,
/// INV-7, and INV-9.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Squad {
    squad_id: Identifier,
    league_id: Identifier,
    manager_id: Identifier,
    month: MonthYear,
    composition: SquadComposition,
    slots: Vec<Slot>,
    status: SquadStatus,
    revision: u32,
}

impl Squad {
    /// Builds an empty squad whose slots follow the composition profile.
    pub fn empty(spec: SquadSpec) -> Self {
        let composition = spec.composition;
        let mut slots = Vec::with_capacity(usize::from(composition.total_slots()));
        for tier in DivisionTier::ALL {
            for _ in 0..composition.starters_per_tier {
                slots.push(Slot::new(tier, Position::Starter, None));
            }
            for _ in 0..composition.bench_per_tier {
                slots.push(Slot::new(tier, Position::Bench, None));
            }
        }
        Self {
            squad_id: spec.identity.squad_id,
            league_id: spec.identity.league_id,
            manager_id: spec.identity.manager_id,
            month: spec.month,
            composition,
            slots,
            status: SquadStatus::Draft,
            revision: 0,
        }
    }

    /// Rebuilds a squad from stored slots, re-checking composition and duplicates.
    pub fn restore(stored: StoredSquad) -> Result<Self, DomainError> {
        let expected = stored.spec.composition.total_slots();
        if stored.slots.len() != usize::from(expected) {
            return Err(DomainError::InvalidSquadComposition {
                expected,
                actual: stored.slots.len() as u16,
            });
        }
        let squad = Self {
            squad_id: stored.spec.identity.squad_id,
            league_id: stored.spec.identity.league_id,
            manager_id: stored.spec.identity.manager_id,
            month: stored.spec.month,
            composition: stored.spec.composition,
            slots: stored.slots,
            status: SquadStatus::Draft,
            revision: 0,
        };
        squad.check_no_duplicate_players()?;
        squad.check_tier_shape()?;
        Ok(squad)
    }

    pub fn squad_id(&self) -> &Identifier {
        &self.squad_id
    }

    pub fn league_id(&self) -> &Identifier {
        &self.league_id
    }

    pub fn manager_id(&self) -> &Identifier {
        &self.manager_id
    }

    pub fn month(&self) -> MonthYear {
        self.month
    }

    pub fn composition(&self) -> SquadComposition {
        self.composition
    }

    pub fn status(&self) -> SquadStatus {
        self.status
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    pub fn slot(&self, index: usize) -> Option<&Slot> {
        self.slots.get(index)
    }

    pub fn is_full(&self) -> bool {
        self.slots.iter().all(Slot::is_filled)
    }

    /// Fills an empty slot with a player whose active tier equals the slot tier.
    pub fn assign(&mut self, index: usize, assignment: SlotAssignment) -> Result<(), DomainError> {
        self.check_mutable()?;
        let total = self.composition.total_slots();
        let slot = self
            .slots
            .get(index)
            .ok_or(DomainError::UnknownSlot { index, total })?;
        check_slot_accepts(slot, &assignment)?;
        ensure_unique(&self.slots, &assignment.player)?;
        self.slots[index].player = Some(assignment.player);
        Ok(())
    }

    /// Swaps a bench player into a starter slot of the same tier.
    pub fn substitute(
        &mut self,
        bench_index: usize,
        starter_index: usize,
    ) -> Result<(), DomainError> {
        self.check_mutable()?;
        let bench_tier = self.bench_tier(bench_index)?;
        let starter_tier = self.starter_tier(starter_index)?;
        if bench_tier != starter_tier {
            return Err(DomainError::BenchSubstitutionOutsideTier {
                player: self.slots[bench_index]
                    .player()
                    .expect("bench slot checked above")
                    .clone(),
                from_tier: bench_tier,
                to_tier: starter_tier,
            });
        }
        // why: the two slots exchange occupants; slot positions stay put
        let bench_player = self.slots[bench_index]
            .player
            .take()
            .expect("checked above");
        let starter_player = self.slots[starter_index].player.replace(bench_player);
        self.slots[bench_index].player = starter_player;
        Ok(())
    }

    /// Moves a full draft squad to submitted.
    pub fn submit(&mut self) -> Result<(), DomainError> {
        if self.status != SquadStatus::Draft {
            return Err(DomainError::SquadLocked {
                squad: self.squad_id.clone(),
            });
        }
        if !self.is_full() {
            return Err(DomainError::InvalidSquadComposition {
                expected: self.composition.total_slots(),
                actual: self.filled_count() as u16,
            });
        }
        self.status = SquadStatus::Submitted;
        self.revision += 1;
        Ok(())
    }

    /// Locks a submitted squad, making it immutable until an admin unlock.
    pub fn lock(&mut self) -> Result<(), DomainError> {
        if self.status != SquadStatus::Submitted {
            return Err(DomainError::SquadLocked {
                squad: self.squad_id.clone(),
            });
        }
        self.status = SquadStatus::Locked;
        self.revision += 1;
        Ok(())
    }

    /// Audited, versioned unlock that returns a locked squad to draft.
    pub fn unlock(&mut self, _audit: AdminUnlock) -> Result<(), DomainError> {
        if self.status != SquadStatus::Locked {
            return Err(DomainError::SquadLocked {
                squad: self.squad_id.clone(),
            });
        }
        self.status = SquadStatus::Draft;
        self.revision += 1;
        Ok(())
    }

    fn check_mutable(&self) -> Result<(), DomainError> {
        if self.status != SquadStatus::Draft {
            return Err(DomainError::SquadLocked {
                squad: self.squad_id.clone(),
            });
        }
        Ok(())
    }

    fn bench_tier(&self, index: usize) -> Result<DivisionTier, DomainError> {
        self.slots
            .get(index)
            .filter(|slot| slot.position() == Position::Bench && slot.is_filled())
            .map(|slot| slot.tier())
            .ok_or(DomainError::UnknownSlot {
                index,
                total: self.composition.total_slots(),
            })
    }

    fn starter_tier(&self, index: usize) -> Result<DivisionTier, DomainError> {
        self.slots
            .get(index)
            .filter(|slot| slot.position() == Position::Starter && slot.is_filled())
            .map(|slot| slot.tier())
            .ok_or(DomainError::UnknownSlot {
                index,
                total: self.composition.total_slots(),
            })
    }

    fn filled_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_filled()).count()
    }

    fn check_no_duplicate_players(&self) -> Result<(), DomainError> {
        let mut seen: Vec<&Identifier> = Vec::with_capacity(self.slots.len());
        for slot in &self.slots {
            if let Some(player) = slot.player() {
                if seen.contains(&player) {
                    return Err(DomainError::DuplicatePlayerInSquad {
                        player: player.clone(),
                    });
                }
                seen.push(player);
            }
        }
        Ok(())
    }

    /// why: persisted squads can be truncated or reshaped, so the tier shape is
    /// re-derived rather than trusted.
    fn check_tier_shape(&self) -> Result<(), DomainError> {
        for tier in DivisionTier::ALL {
            if self.tier_counts(tier) != self.expected_counts() {
                return Err(DomainError::InvalidSquadComposition {
                    expected: self.composition.total_slots(),
                    actual: self.slots.len() as u16,
                });
            }
        }
        Ok(())
    }

    fn tier_counts(&self, tier: DivisionTier) -> SlotCounts {
        let starters = self
            .slots
            .iter()
            .filter(|slot| slot.tier() == tier && slot.position() == Position::Starter)
            .count();
        let bench = self
            .slots
            .iter()
            .filter(|slot| slot.tier() == tier && slot.position() == Position::Bench)
            .count();
        SlotCounts { starters, bench }
    }

    fn expected_counts(&self) -> SlotCounts {
        SlotCounts {
            starters: usize::from(self.composition.starters_per_tier),
            bench: usize::from(self.composition.bench_per_tier),
        }
    }
}

/// Counts of the starters and bench players a tier currently holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SlotCounts {
    starters: usize,
    bench: usize,
}

/// why: a slot accepts exactly one player, and only one whose active division
/// tier equals the slot tier (INV-2, INV-3).
fn check_slot_accepts(slot: &Slot, assignment: &SlotAssignment) -> Result<(), DomainError> {
    if slot.is_filled() {
        return Err(DomainError::DuplicatePlayerInSquad {
            player: assignment.player.clone(),
        });
    }
    if slot.tier() != assignment.tier {
        return Err(DomainError::IneligiblePlayer {
            player: assignment.player.clone(),
            slot_tier: slot.tier(),
            player_tier: assignment.tier,
        });
    }
    Ok(())
}

fn ensure_unique(slots: &[Slot], player: &Identifier) -> Result<(), DomainError> {
    let taken = slots
        .iter()
        .any(|slot| slot.player().is_some_and(|other| other == player));
    if taken {
        return Err(DomainError::DuplicatePlayerInSquad {
            player: player.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use proptest::prelude::*;

    fn identifier(prefix: DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    /// why: 'I', 'L', 'O', 'U' are excluded from Crockford base32, so distinct
    /// suffixes cycle through a safe alphabet instead of sequential letters.
    const SAFE_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    fn fill_squad_player(index: usize) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{}", SAFE_ALPHABET[index] as char);
        Identifier::new(DomainPrefix::Player, &suffix).expect("valid id")
    }

    fn squad() -> Squad {
        Squad::empty(spec())
    }

    fn spec() -> SquadSpec {
        SquadSpec::new(
            SquadIdentity::new(
                identifier(DomainPrefix::Squad, 'A'),
                identifier(DomainPrefix::League, 'B'),
                identifier(DomainPrefix::User, 'C'),
            ),
            MonthYear::new(2026, 11).expect("valid month"),
            SquadComposition::REGULATION,
        )
    }

    fn player(last: char) -> Identifier {
        identifier(DomainPrefix::Player, last)
    }

    fn fill_squad(squad: &mut Squad) {
        for index in 0..squad.slots().len() {
            let tier = squad.slot(index).expect("slot").tier();
            let assignment = SlotAssignment::new(fill_squad_player(index), tier);
            squad.assign(index, assignment).expect("fill");
        }
    }

    #[test]
    fn regulation_composition_has_fifteen_slots() {
        assert_eq!(SquadComposition::REGULATION.total_slots(), 15);
        assert_eq!(SquadComposition::REGULATION.slots_per_tier(), 3);
    }

    #[test]
    fn empty_squad_holds_two_starters_and_one_bench_per_tier() {
        let squad = squad();
        assert_eq!(squad.slots().len(), 15);
        for tier in DivisionTier::ALL {
            let starters = squad
                .slots()
                .iter()
                .filter(|slot| slot.tier() == tier && slot.position() == Position::Starter)
                .count();
            let bench = squad
                .slots()
                .iter()
                .filter(|slot| slot.tier() == tier && slot.position() == Position::Bench)
                .count();
            assert_eq!((starters, bench), (2, 1), "tier {tier}");
        }
    }

    #[test]
    fn assign_fills_matching_tier_slot() {
        let mut squad = squad();
        let assignment = SlotAssignment::new(player('A'), DivisionTier::B2);
        squad
            .assign(0, assignment)
            .expect("B2 starter accepts a B2 player");
        assert_eq!(squad.slot(0).expect("slot").player(), Some(&player('A')));
    }

    #[test]
    fn assign_rejects_tier_mismatch_as_ineligible_player() {
        let mut squad = squad();
        let assignment = SlotAssignment::new(player('A'), DivisionTier::D2);
        let error = squad
            .assign(0, assignment)
            .expect_err("D2 player cannot fill a B2 slot");
        assert_eq!(error.code(), "INELIGIBLE_PLAYER");
    }

    #[test]
    fn assign_rejects_duplicate_player_within_the_squad() {
        let mut squad = squad();
        squad
            .assign(0, SlotAssignment::new(player('A'), DivisionTier::B2))
            .expect("first");
        assert_eq!(
            squad.assign(1, SlotAssignment::new(player('A'), DivisionTier::B2)),
            Err(DomainError::DuplicatePlayerInSquad {
                player: player('A')
            })
        );
    }

    #[test]
    fn restore_rejects_wrong_slot_count() {
        let slots: Vec<Slot> = squad().slots().iter().take(14).cloned().collect();
        let stored = StoredSquad {
            spec: spec(),
            slots,
        };
        assert!(Squad::restore(stored).is_err());
    }

    #[test]
    fn restore_rejects_duplicate_players() {
        let mut slots = squad().slots().to_vec();
        slots[0].player = Some(player('A'));
        slots[1].player = Some(player('A'));
        let stored = StoredSquad {
            spec: spec(),
            slots,
        };
        assert!(Squad::restore(stored).is_err());
    }

    #[test]
    fn restore_accepts_a_partially_filled_squad() {
        let mut stored_source = squad();
        fill_squad(&mut stored_source);
        let mut slots = stored_source.slots().to_vec();
        slots[14].player = None;
        let stored = StoredSquad {
            spec: spec(),
            slots,
        };
        let restored = Squad::restore(stored).expect("empty slots keep the shape");
        assert!(!restored.is_full());
        assert!(restored.slot(14).expect("slot").player().is_none());
    }

    #[test]
    fn restore_rejects_a_slot_that_belongs_to_another_tier() {
        let mut slots = squad().slots().to_vec();
        slots[0] = Slot::new(DivisionTier::D1, Position::Starter, None);
        let stored = StoredSquad {
            spec: spec(),
            slots,
        };
        assert_eq!(
            Squad::restore(stored),
            Err(DomainError::InvalidSquadComposition {
                expected: 15,
                actual: 15,
            })
        );
    }

    #[test]
    fn assigning_over_a_filled_slot_is_rejected() {
        let mut squad = squad();
        fill_squad(&mut squad);
        let tier = squad.slot(0).expect("slot").tier();
        let assignment = SlotAssignment::new(player('Z'), tier);
        assert_eq!(
            squad.assign(0, assignment),
            Err(DomainError::DuplicatePlayerInSquad {
                player: player('Z')
            })
        );
    }

    #[test]
    fn locked_squad_rejects_assignments() {
        let mut squad = squad();
        fill_squad(&mut squad);
        squad.submit().expect("submit");
        squad.lock().expect("lock");
        let assignment = SlotAssignment::new(player('Z'), DivisionTier::B2);
        assert_eq!(
            squad.assign(0, assignment),
            Err(DomainError::SquadLocked {
                squad: squad.squad_id().clone()
            })
        );
    }

    #[test]
    fn unlock_bumps_revision_and_reopens_the_squad() {
        let mut squad = squad();
        fill_squad(&mut squad);
        squad.submit().expect("submit");
        squad.lock().expect("lock");
        let locked_revision = squad.revision();
        let audit = AdminUnlock::new(identifier(DomainPrefix::User, 'C'), "protest upheld", 7);
        squad.unlock(audit).expect("unlock");
        assert_eq!(squad.status(), SquadStatus::Draft);
        assert!(squad.revision() > locked_revision);
    }

    #[test]
    fn bench_substitution_moves_a_same_tier_player_in() {
        let mut squad = squad();
        squad
            .assign(0, SlotAssignment::new(player('A'), DivisionTier::B2))
            .expect("starter");
        squad
            .assign(1, SlotAssignment::new(player('X'), DivisionTier::B2))
            .expect("starter");
        squad
            .assign(2, SlotAssignment::new(player('B'), DivisionTier::B2))
            .expect("bench");
        squad
            .substitute(2, 0)
            .expect("same-tier substitution is legal");
        assert_eq!(squad.slot(0).expect("slot").position(), Position::Starter);
        assert_eq!(squad.slot(0).expect("slot").player(), Some(&player('B')));
        assert_eq!(squad.slot(2).expect("slot").position(), Position::Bench);
        assert_eq!(squad.slot(2).expect("slot").player(), Some(&player('A')));
    }

    #[test]
    fn cross_tier_bench_substitution_is_rejected() {
        let mut squad = squad();
        squad
            .assign(1, SlotAssignment::new(player('X'), DivisionTier::B2))
            .expect("starter");
        squad
            .assign(12, SlotAssignment::new(player('C'), DivisionTier::D2))
            .expect("fill");
        squad
            .assign(14, SlotAssignment::new(player('D'), DivisionTier::D2))
            .expect("bench");
        let error = squad
            .substitute(14, 1)
            .expect_err("cross-tier substitution is illegal");
        assert_eq!(error.code(), "BENCH_SUBSTITUTION_OUTSIDE_TIER");
    }

    #[test]
    fn submit_requires_a_full_squad() {
        let mut squad = squad();
        assert!(squad.submit().is_err());
        fill_squad(&mut squad);
        squad.submit().expect("submit full squad");
        assert_eq!(squad.status(), SquadStatus::Submitted);
    }

    #[test]
    fn assign_outside_the_squad_is_an_unknown_slot() {
        let mut squad = squad();
        let error = squad
            .assign(99, SlotAssignment::new(player('A'), DivisionTier::B2))
            .expect_err("index 99 is outside the squad");
        assert_eq!(error.code(), "UNKNOWN_SLOT");
    }

    #[test]
    fn lock_from_draft_is_rejected() {
        let mut squad = squad();
        assert_eq!(
            squad.lock(),
            Err(DomainError::SquadLocked {
                squad: squad.squad_id().clone()
            })
        );
    }

    #[test]
    fn unlock_from_draft_is_rejected() {
        let mut squad = squad();
        assert!(
            squad
                .unlock(AdminUnlock::new(
                    identifier(DomainPrefix::User, 'C'),
                    "nothing to unlock",
                    0
                ))
                .is_err()
        );
    }

    #[test]
    fn submitting_twice_is_rejected() {
        let mut squad = squad();
        fill_squad(&mut squad);
        squad.submit().expect("submit");
        assert_eq!(
            squad.submit(),
            Err(DomainError::SquadLocked {
                squad: squad.squad_id().clone()
            })
        );
    }

    #[test]
    fn restore_accepts_a_well_formed_squad() {
        let mut original = squad();
        fill_squad(&mut original);
        let stored = StoredSquad {
            spec: spec(),
            slots: original.slots().to_vec(),
        };
        let restored = Squad::restore(stored).expect("well-formed slots restore");
        assert_eq!(restored.squad_id(), original.squad_id());
        assert_eq!(restored.league_id(), original.league_id());
        assert_eq!(restored.manager_id(), original.manager_id());
        assert_eq!(restored.month(), original.month());
        assert_eq!(restored.composition(), SquadComposition::REGULATION);
        assert_eq!(restored.status(), SquadStatus::Draft);
        assert!(restored.is_full());
        assert_eq!(restored.slot(0), original.slot(0));
    }

    #[test]
    fn filled_slot_wire_round_trips() {
        let filled = Slot::new(DivisionTier::B2, Position::Starter, Some(player('A')));
        let text = filled.to_string();
        assert_eq!(text, format!("B2|starter|{}", player('A')));
        assert_eq!(text.parse::<Slot>(), Ok(filled.clone()));
        assert_eq!(Slot::try_from(text.clone()), Ok(filled.clone()));
        assert_eq!(String::from(filled), text);
    }

    #[test]
    fn empty_slot_wires_without_a_player() {
        let slot = Slot::new(DivisionTier::D2, Position::Bench, None);
        assert_eq!(slot.to_string(), "D2|bench|");
        assert!(!slot.is_filled());
        assert_eq!("D2|bench|".parse::<Slot>(), Ok(slot.clone()));
        assert_eq!(slot.tier(), DivisionTier::D2);
        assert_eq!(slot.position(), Position::Bench);
    }

    #[test]
    fn malformed_slot_wires_are_rejected() {
        assert_eq!(
            "C3|starter|".parse::<Slot>(),
            Err(SlotParseError::InvalidPosition)
        );
        assert_eq!(
            "C2|coach|".parse::<Slot>(),
            Err(SlotParseError::InvalidPosition)
        );
        assert_eq!(
            "C2|starter|not-an-id".parse::<Slot>(),
            Err(SlotParseError::InvalidPlayer)
        );
    }

    #[test]
    fn custom_compositions_expose_their_shape() {
        let composition = SquadComposition::new(3, 2);
        assert_eq!(composition.starters_per_tier(), 3);
        assert_eq!(composition.bench_per_tier(), 2);
        assert_eq!(composition.slots_per_tier(), 5);
        assert_eq!(composition.total_slots(), 25);
    }

    proptest! {
        #[test]
        fn random_fill_respects_tier_matching_and_uniqueness(
            players in prop::collection::hash_set("[0-9A-HJKMNP-TV-Z]{26}", 15..=45),
        ) {
            let chosen: Vec<Identifier> = players
                .into_iter()
                .take(15)
                .map(|suffix| Identifier::new(DomainPrefix::Player, &suffix))
                .collect::<Result<Vec<_>, _>>()?;
            let mut squad = squad();
            for (index, player) in chosen.iter().enumerate() {
                let tier = squad.slot(index).expect("slot").tier();
                squad.assign(index, SlotAssignment::new(player.clone(), tier))?;
            }
            prop_assert_eq!(squad.slots().len(), 15);
            prop_assert!(squad.is_full());
            let filled: Vec<&Identifier> = squad
                .slots()
                .iter()
                .filter_map(|slot| slot.player())
                .collect();
            prop_assert_eq!(filled.len(), 15);
        }
    }
}
