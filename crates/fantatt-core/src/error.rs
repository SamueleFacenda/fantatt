use thiserror::Error;

use crate::division::DivisionTier;
use crate::ids::Identifier;
use crate::money::Money;

/// Typed domain rule violation.
///
/// Each variant encodes exactly one invariant of `docs/02-domain-model.md` §3
/// or one value-object rule, and maps to a stable machine-readable code used
/// as the RFC 9457 problem `type` by the API layer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("player {player} already has a rating dated {as_of}")]
    DuplicateRatingForDate {
        player: Identifier,
        as_of: time::Date,
    },

    #[error("squad must hold {expected} slots, got {actual}")]
    InvalidSquadComposition { expected: u16, actual: u16 },

    #[error("slot index {index} is outside the {total}-slot squad")]
    UnknownSlot { index: usize, total: u16 },

    #[error("player {player} plays in {player_tier} and cannot fill a {slot_tier} slot")]
    IneligiblePlayer {
        player: Identifier,
        slot_tier: DivisionTier,
        player_tier: DivisionTier,
    },

    #[error("player {player} is already owned in league {league} this month")]
    PlayerAlreadyOwnedInMonth {
        player: Identifier,
        league: Identifier,
    },

    #[error("player {player} appears twice in the same squad")]
    DuplicatePlayerInSquad { player: Identifier },

    #[error("squad {squad} is locked and immutable until an audited admin unlock")]
    SquadLocked { squad: Identifier },

    #[error("week 1 of the season scores zero points, got {points} for player {player}")]
    WeekOnePointsNotZero { player: Identifier, points: i32 },

    #[error(
        "player {player} substitutes from {from_tier} into {to_tier}, bench substitution stays within the tier"
    )]
    BenchSubstitutionOutsideTier {
        player: Identifier,
        from_tier: DivisionTier,
        to_tier: DivisionTier,
    },

    #[error("matchday {matchday} has results for {decided} of {scheduled} ties")]
    MatchdayIncomplete {
        matchday: Identifier,
        scheduled: u16,
        decided: u16,
    },

    #[error("offer {offered} is below the current best {best} on lot {lot}")]
    OfferNotHighest {
        lot: Identifier,
        offered: Money,
        best: Money,
    },

    #[error("auction window for lot {lot} is closed")]
    AuctionWindowClosed { lot: Identifier },

    #[error("manager spent {spent} exceeding the starting budget {budget}")]
    BudgetExceeded { spent: Money, budget: Money },

    #[error("fantasy points for player {player} carry no derivation record")]
    ScoreWithoutDerivation { player: Identifier },

    #[error("stored score for player {player} carries no rule bundle version")]
    RuleVersionMissing { player: Identifier },

    #[error("account {manager} still holds personal data after the 30-day purge window")]
    PurgeWindowExceeded {
        manager: Identifier,
        days_retained: u16,
    },

    #[error("manager {manager} is not an active member of the league")]
    MembershipNotActive { manager: Identifier },
}

impl DomainError {
    /// Stable machine-readable violation code, mirrored into problem+json `type`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateRatingForDate { .. } => "DUPLICATE_RATING_FOR_DATE",
            Self::InvalidSquadComposition { .. } => "INVALID_SQUAD_COMPOSITION",
            Self::UnknownSlot { .. } => "UNKNOWN_SLOT",
            Self::IneligiblePlayer { .. } => "INELIGIBLE_PLAYER",
            Self::PlayerAlreadyOwnedInMonth { .. } => "PLAYER_ALREADY_OWNED_IN_MONTH",
            Self::DuplicatePlayerInSquad { .. } => "DUPLICATE_PLAYER_IN_SQUAD",
            Self::SquadLocked { .. } => "SQUAD_LOCKED",
            Self::WeekOnePointsNotZero { .. } => "WEEK_ONE_POINTS_NOT_ZERO",
            Self::BenchSubstitutionOutsideTier { .. } => "BENCH_SUBSTITUTION_OUTSIDE_TIER",
            Self::MatchdayIncomplete { .. } => "MATCHDAY_INCOMPLETE",
            Self::OfferNotHighest { .. } => "OFFER_NOT_HIGHEST",
            Self::AuctionWindowClosed { .. } => "AUCTION_WINDOW_CLOSED",
            Self::BudgetExceeded { .. } => "BUDGET_EXCEEDED",
            Self::ScoreWithoutDerivation { .. } => "SCORE_WITHOUT_DERIVATION",
            Self::RuleVersionMissing { .. } => "RULE_VERSION_MISSING",
            Self::PurgeWindowExceeded { .. } => "PURGE_WINDOW_EXCEEDED",
            Self::MembershipNotActive { .. } => "MEMBERSHIP_NOT_ACTIVE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identifier(prefix: crate::ids::DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    fn player() -> Identifier {
        identifier(crate::ids::DomainPrefix::Player, 'A')
    }

    fn money(minor_units: u64) -> Money {
        Money::new("EUR", minor_units).expect("valid money")
    }

    fn assert_code(error: DomainError, code: &str) {
        assert_eq!(error.code(), code);
    }

    #[test]
    fn ineligible_player_uses_the_documented_code() {
        let error = DomainError::IneligiblePlayer {
            player: player(),
            slot_tier: DivisionTier::B2,
            player_tier: DivisionTier::D2,
        };
        assert_code(error.clone(), "INELIGIBLE_PLAYER");
        assert!(error.to_string().contains("D2"), "message: {error}");
    }

    #[test]
    fn rating_and_composition_codes() {
        let date = time::Date::from_ordinal_date(2026, 100).expect("valid date");
        assert_code(
            DomainError::DuplicateRatingForDate {
                player: player(),
                as_of: date,
            },
            "DUPLICATE_RATING_FOR_DATE",
        );
        assert_code(
            DomainError::InvalidSquadComposition {
                expected: 15,
                actual: 14,
            },
            "INVALID_SQUAD_COMPOSITION",
        );
    }

    #[test]
    fn squad_codes() {
        assert_code(
            DomainError::PlayerAlreadyOwnedInMonth {
                player: player(),
                league: identifier(crate::ids::DomainPrefix::League, 'B'),
            },
            "PLAYER_ALREADY_OWNED_IN_MONTH",
        );
        assert_code(
            DomainError::DuplicatePlayerInSquad { player: player() },
            "DUPLICATE_PLAYER_IN_SQUAD",
        );
        assert_code(
            DomainError::SquadLocked {
                squad: identifier(crate::ids::DomainPrefix::Squad, 'C'),
            },
            "SQUAD_LOCKED",
        );
    }

    #[test]
    fn substitution_codes() {
        assert_code(
            DomainError::BenchSubstitutionOutsideTier {
                player: player(),
                from_tier: DivisionTier::C1,
                to_tier: DivisionTier::D1,
            },
            "BENCH_SUBSTITUTION_OUTSIDE_TIER",
        );
        assert_code(
            DomainError::WeekOnePointsNotZero {
                player: player(),
                points: 5,
            },
            "WEEK_ONE_POINTS_NOT_ZERO",
        );
    }

    #[test]
    fn matchday_code() {
        assert_code(
            DomainError::MatchdayIncomplete {
                matchday: identifier(crate::ids::DomainPrefix::Matchday, 'D'),
                scheduled: 8,
                decided: 6,
            },
            "MATCHDAY_INCOMPLETE",
        );
    }

    #[test]
    fn auction_codes() {
        let lot = identifier(crate::ids::DomainPrefix::Lot, 'E');
        assert_code(
            DomainError::OfferNotHighest {
                lot: lot.clone(),
                offered: money(100),
                best: money(200),
            },
            "OFFER_NOT_HIGHEST",
        );
        assert_code(
            DomainError::AuctionWindowClosed { lot },
            "AUCTION_WINDOW_CLOSED",
        );
        assert_code(
            DomainError::BudgetExceeded {
                spent: money(1200),
                budget: money(1000),
            },
            "BUDGET_EXCEEDED",
        );
    }

    #[test]
    fn derivation_and_purge_codes() {
        assert_code(
            DomainError::ScoreWithoutDerivation { player: player() },
            "SCORE_WITHOUT_DERIVATION",
        );
        assert_code(
            DomainError::RuleVersionMissing { player: player() },
            "RULE_VERSION_MISSING",
        );
        assert_code(
            DomainError::PurgeWindowExceeded {
                manager: identifier(crate::ids::DomainPrefix::User, 'G'),
                days_retained: 31,
            },
            "PURGE_WINDOW_EXCEEDED",
        );
    }
}
