use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error produced when parsing a prefixed domain identifier fails.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdParseError {
    #[error("identifier is empty")]
    Empty,
    #[error("identifier must start with '{expected}_', got '{prefix}_'")]
    WrongPrefix {
        expected: &'static str,
        prefix: String,
    },
    #[error("identifier suffix must be a 26-character ULID, got '{suffix}'")]
    InvalidUlid { suffix: String },
}

/// Prefix namespace for every domain identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DomainPrefix {
    #[serde(rename = "lg")]
    League,
    #[serde(rename = "usr")]
    User,
    #[serde(rename = "plr")]
    Player,
    #[serde(rename = "cmp")]
    Competition,
    #[serde(rename = "md")]
    Matchday,
    #[serde(rename = "sq")]
    Squad,
    #[serde(rename = "tie")]
    Tie,
    #[serde(rename = "app")]
    Appearance,
    #[serde(rename = "team")]
    Team,
    #[serde(rename = "mbr")]
    Membership,
    #[serde(rename = "lot")]
    Lot,
    #[serde(rename = "off")]
    Offer,
}

impl DomainPrefix {
    /// Wire prefix including the underscore, e.g. `lg_`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::League => "lg",
            Self::User => "usr",
            Self::Player => "plr",
            Self::Competition => "cmp",
            Self::Matchday => "md",
            Self::Squad => "sq",
            Self::Tie => "tie",
            Self::Appearance => "app",
            Self::Team => "team",
            Self::Membership => "mbr",
            Self::Lot => "lot",
            Self::Offer => "off",
        }
    }
}

impl fmt::Display for DomainPrefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Length of a canonical ULID suffix (Crockford base32, 26 chars).
const ULID_LENGTH: usize = 26;

/// A domain identifier: namespace prefix + canonical 26-character ULID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Identifier {
    prefix: DomainPrefix,
    suffix: String,
}

impl Identifier {
    pub fn new(prefix: DomainPrefix, ulid: &str) -> Result<Self, IdParseError> {
        if ulid.is_empty() {
            return Err(IdParseError::Empty);
        }
        if !is_canonical_ulid(ulid) {
            return Err(IdParseError::InvalidUlid {
                suffix: ulid.to_owned(),
            });
        }
        Ok(Self {
            prefix,
            suffix: ulid.to_owned(),
        })
    }

    pub fn prefix(&self) -> DomainPrefix {
        self.prefix
    }

    pub fn ulid(&self) -> &str {
        &self.suffix
    }

    fn parse(text: &str) -> Result<Self, IdParseError> {
        let (head, suffix) = text
            .split_once('_')
            .ok_or_else(|| IdParseError::WrongPrefix {
                expected: "lg",
                prefix: text.to_owned(),
            })?;
        let prefix = prefix_from_head(head)?;
        Self::new(prefix, suffix)
    }
}

/// why: Crockford base32 alphabet excludes I, L, O, U to avoid ambiguity.
fn is_canonical_ulid(candidate: &str) -> bool {
    candidate.len() == ULID_LENGTH
        && candidate
            .chars()
            .all(|c: char| c.is_ascii_digit() || c.is_ascii_alphabetic())
        && !candidate
            .chars()
            .any(|c| matches!(c, 'I' | 'L' | 'O' | 'U' | 'i' | 'l' | 'o' | 'u'))
}

fn prefix_from_head(head: &str) -> Result<DomainPrefix, IdParseError> {
    match head {
        "lg" => Ok(DomainPrefix::League),
        "usr" => Ok(DomainPrefix::User),
        "plr" => Ok(DomainPrefix::Player),
        "cmp" => Ok(DomainPrefix::Competition),
        "md" => Ok(DomainPrefix::Matchday),
        "sq" => Ok(DomainPrefix::Squad),
        "tie" => Ok(DomainPrefix::Tie),
        "app" => Ok(DomainPrefix::Appearance),
        "team" => Ok(DomainPrefix::Team),
        "mbr" => Ok(DomainPrefix::Membership),
        "lot" => Ok(DomainPrefix::Lot),
        "off" => Ok(DomainPrefix::Offer),
        other => Err(IdParseError::WrongPrefix {
            expected: "lg",
            prefix: other.to_owned(),
        }),
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", self.prefix, self.suffix)
    }
}

impl FromStr for Identifier {
    type Err = IdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<String> for Identifier {
    type Error = IdParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<Identifier> for String {
    fn from(id: Identifier) -> Self {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const ULID: &str = "01ARZ3XK7M0C4V6B8N2QWE9RTA";

    #[test]
    fn new_accepts_canonical_ulid_with_prefix() {
        let id = Identifier::new(DomainPrefix::League, ULID).expect("valid ulid");
        assert_eq!(id.prefix(), DomainPrefix::League);
        assert_eq!(id.ulid(), ULID);
    }

    #[test]
    fn parse_round_trips_every_documented_prefix() {
        for prefix in [
            DomainPrefix::League,
            DomainPrefix::User,
            DomainPrefix::Player,
            DomainPrefix::Competition,
            DomainPrefix::Matchday,
            DomainPrefix::Squad,
            DomainPrefix::Tie,
            DomainPrefix::Appearance,
            DomainPrefix::Team,
            DomainPrefix::Membership,
            DomainPrefix::Lot,
            DomainPrefix::Offer,
        ] {
            let text = format!("{prefix}_{ULID}");
            let parsed: Identifier = text.parse().expect("parse valid identifier");
            assert_eq!(parsed.prefix(), prefix);
            assert_eq!(parsed.to_string(), text);
        }
    }

    #[test]
    fn serde_round_trips_prefix_and_ulid() {
        let id = Identifier::new(DomainPrefix::Player, ULID).expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, format!("\"plr_{ULID}\""));
        let back: Identifier = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn new_rejects_empty_ulid() {
        assert_eq!(
            Identifier::new(DomainPrefix::User, ""),
            Err(IdParseError::Empty)
        );
    }

    #[test]
    fn new_rejects_short_ulid() {
        assert_eq!(
            Identifier::new(DomainPrefix::User, "01ARZ3"),
            Err(IdParseError::InvalidUlid {
                suffix: "01ARZ3".to_owned()
            })
        );
    }

    #[test]
    fn new_rejects_ulid_with_ambiguous_crockford_letters() {
        for bad in ["I", "L", "O", "U"] {
            let mut candidate = ULID.to_owned();
            candidate.replace_range(0..1, bad);
            assert_eq!(
                Identifier::new(DomainPrefix::User, &candidate),
                Err(IdParseError::InvalidUlid {
                    suffix: candidate.clone()
                }),
                "letter {bad} is excluded from Crockford base32"
            );
        }
    }

    #[test]
    fn parse_rejects_unknown_prefix() {
        assert_eq!(
            "xx_01ARZ3XK7M0C4V6B8N2QWE9RTA".parse::<Identifier>(),
            Err(IdParseError::WrongPrefix {
                expected: "lg",
                prefix: "xx".to_owned()
            })
        );
    }

    #[test]
    fn parse_rejects_missing_separator() {
        assert_eq!(
            "lgsomething".parse::<Identifier>(),
            Err(IdParseError::WrongPrefix {
                expected: "lg",
                prefix: "lgsomething".to_owned()
            })
        );
    }

    proptest! {
        #[test]
        fn display_parses_back_to_the_same_identifier(
            prefix in prop_oneof![
                Just(DomainPrefix::League),
                Just(DomainPrefix::User),
                Just(DomainPrefix::Player),
                Just(DomainPrefix::Competition),
                Just(DomainPrefix::Matchday),
                Just(DomainPrefix::Squad),
                Just(DomainPrefix::Tie),
                Just(DomainPrefix::Appearance),
                Just(DomainPrefix::Team),
                Just(DomainPrefix::Membership),
                Just(DomainPrefix::Lot),
                Just(DomainPrefix::Offer),
            ],
            ulid in "[0-9A-HJKMNP-TV-Z]{26}",
        ) {
            let id = Identifier::new(prefix, &ulid)?;
            let text = id.to_string();
            prop_assert_eq!(text.parse::<Identifier>(), Ok(id));
        }

        #[test]
        fn wrong_length_or_alphabet_are_rejected(candidate in "\\PC{0,40}") {
            let result = Identifier::new(DomainPrefix::League, &candidate);
            if candidate.len() == 26
                && candidate
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() && !matches!(c, 'I' | 'L' | 'O' | 'U' | 'i' | 'l' | 'o' | 'u'))
            {
                prop_assert!(result.is_ok());
            } else {
                prop_assert!(result.is_err());
            }
        }
    }
}
