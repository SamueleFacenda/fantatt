use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::Date;

/// FITET ranking points assigned to a player with no resolved identity.
pub const PROVISIONAL_RATING: i64 = 2000;

/// Confidence of the evidence behind a rating, mirroring identity resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RatingConfidence {
    Low,
    Medium,
    High,
}

/// Error produced when a rating cannot be parsed from its wire form.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RatingParseError {
    #[error("rating value must be a non-negative integer")]
    NegativeValue,
    #[error("rating date must be an ISO 8601 date")]
    InvalidDate,
}

/// Where a rating came from and how much the evidence can be trusted.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RatingProvenance {
    source: String,
    confidence: RatingConfidence,
}

impl RatingProvenance {
    pub fn new(source: &str, confidence: RatingConfidence) -> Self {
        Self {
            source: source.to_owned(),
            confidence,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn confidence(&self) -> RatingConfidence {
        self.confidence
    }
}

/// A published FITET ranking: integer points, effective date, provenance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Rating {
    value: i64,
    as_of: Date,
    provenance: RatingProvenance,
    provisional: bool,
}

impl Rating {
    /// Confirmed rating backed by a classifica publication.
    pub fn confirmed(
        value: i64,
        as_of: Date,
        provenance: RatingProvenance,
    ) -> Result<Self, RatingParseError> {
        if value < 0 {
            return Err(RatingParseError::NegativeValue);
        }
        Ok(Self {
            value,
            as_of,
            provenance,
            provisional: false,
        })
    }

    /// Rating for an unresolved identity: 2000 points, flagged as provisional.
    pub fn provisional(as_of: Date, source: &str) -> Self {
        Self {
            value: PROVISIONAL_RATING,
            as_of,
            provenance: RatingProvenance::new(source, RatingConfidence::Low),
            provisional: true,
        }
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn as_of(&self) -> Date {
        self.as_of
    }

    pub fn source(&self) -> &str {
        self.provenance.source()
    }

    pub fn confidence(&self) -> RatingConfidence {
        self.provenance.confidence()
    }

    pub fn is_provisional(&self) -> bool {
        self.provisional
    }
}

impl fmt::Display for Rating {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {} ({})", self.value, self.as_of, self.source())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn date() -> Date {
        Date::from_ordinal_date(2026, 100).expect("valid date")
    }

    #[test]
    fn confirmed_rating_keeps_points_date_and_source() {
        let provenance = RatingProvenance::new("classifica-2026-04", RatingConfidence::High);
        let rating = Rating::confirmed(2450, date(), provenance).expect("valid rating");
        assert_eq!(rating.value(), 2450);
        assert_eq!(rating.as_of(), date());
        assert_eq!(rating.source(), "classifica-2026-04");
        assert!(!rating.is_provisional());
    }

    #[test]
    fn negative_points_are_rejected() {
        let provenance = RatingProvenance::new("classifica", RatingConfidence::Low);
        assert_eq!(
            Rating::confirmed(-1, date(), provenance),
            Err(RatingParseError::NegativeValue)
        );
    }

    #[test]
    fn unresolved_identity_gets_the_provisional_default() {
        let rating = Rating::provisional(date(), "unresolved");
        assert_eq!(rating.value(), PROVISIONAL_RATING);
        assert_eq!(rating.value(), 2000);
        assert!(rating.is_provisional());
        assert_eq!(rating.confidence(), RatingConfidence::Low);
    }

    #[test]
    fn ratings_order_by_points() {
        let provenance = RatingProvenance::new("c", RatingConfidence::High);
        let strong = Rating::confirmed(3000, date(), provenance.clone()).expect("valid");
        let weak = Rating::confirmed(1500, date(), provenance).expect("valid");
        assert!(strong > weak);
    }

    proptest! {
        #[test]
        fn any_non_negative_integer_is_a_valid_rating(points in 0i64..i64::MAX) {
            let provenance = RatingProvenance::new("classifica", RatingConfidence::Medium);
            let rating = Rating::confirmed(points, date(), provenance)?;
            prop_assert_eq!(rating.value(), points);
            prop_assert!(!rating.is_provisional());
        }

        #[test]
        fn negative_integers_are_never_valid(points in i64::MIN..-1i64) {
            let provenance = RatingProvenance::new("classifica", RatingConfidence::Medium);
            prop_assert!(Rating::confirmed(points, date(), provenance).is_err());
        }
    }
}
