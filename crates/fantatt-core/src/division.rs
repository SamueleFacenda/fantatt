use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// FITET competition band, ordered from strongest (`B2`) to weakest (`D2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DivisionTier {
    D2,
    D1,
    C2,
    C1,
    B2,
}

/// Error produced when a division tier label cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown division tier '{0}', expected one of B2, C1, C2, D1, D2")]
pub struct DivisionTierParseError(pub String);

impl DivisionTier {
    /// Wire label, e.g. `B2`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::D2 => "D2",
            Self::D1 => "D1",
            Self::C2 => "C2",
            Self::C1 => "C1",
            Self::B2 => "B2",
        }
    }

    /// All tiers from strongest to weakest.
    pub const ALL: [Self; 5] = [Self::B2, Self::C1, Self::C2, Self::D1, Self::D2];
}

impl fmt::Display for DivisionTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DivisionTier {
    type Err = DivisionTierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "B2" => Ok(Self::B2),
            "C1" => Ok(Self::C1),
            "C2" => Ok(Self::C2),
            "D1" => Ok(Self::D1),
            "D2" => Ok(Self::D2),
            other => Err(DivisionTierParseError(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip_through_from_str() {
        for tier in DivisionTier::ALL {
            assert_eq!(tier.as_str().parse::<DivisionTier>(), Ok(tier));
            assert_eq!(tier.to_string(), tier.as_str());
        }
    }

    #[test]
    fn unknown_label_is_rejected() {
        assert_eq!(
            "C3".parse::<DivisionTier>(),
            Err(DivisionTierParseError("C3".to_owned()))
        );
        assert!("b2".parse::<DivisionTier>().is_err());
    }

    #[test]
    fn tiers_are_ordered_strongest_to_weakest() {
        for (index, tier) in DivisionTier::ALL.iter().enumerate() {
            for weaker in &DivisionTier::ALL[index + 1..] {
                assert!(tier > weaker, "{tier} must rank above {weaker}");
            }
        }
    }

    #[test]
    fn serde_uses_the_wire_label() {
        assert_eq!(
            serde_json::to_string(&DivisionTier::C2).expect("serialize"),
            "\"C2\""
        );
        assert_eq!(
            serde_json::from_str::<DivisionTier>("\"D1\"").expect("deserialize"),
            DivisionTier::D1
        );
    }
}
