use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error produced when an idempotency key cannot be used.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdempotencyKeyError {
    #[error("idempotency key must be {min}..={max} characters, got {length}")]
    InvalidLength {
        length: usize,
        min: usize,
        max: usize,
    },
    #[error("idempotency key must use ASCII letters, digits, '-' or '_'")]
    InvalidCharacters,
}

/// Shortest accepted key.
pub const IDEMPOTENCY_KEY_MIN: usize = 8;
/// Longest accepted key.
pub const IDEMPOTENCY_KEY_MAX: usize = 64;

/// The opaque client token that makes a creating POST replayable.
///
/// Every creating POST carries an `Idempotency-Key` header
/// (docs/07-api-spec.md section 1), so retries never duplicate writes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct IdempotencyKey(String);

impl IdempotencyKey {
    pub fn new(raw: &str) -> Result<Self, IdempotencyKeyError> {
        if raw.len() < IDEMPOTENCY_KEY_MIN || raw.len() > IDEMPOTENCY_KEY_MAX {
            return Err(IdempotencyKeyError::InvalidLength {
                length: raw.len(),
                min: IDEMPOTENCY_KEY_MIN,
                max: IDEMPOTENCY_KEY_MAX,
            });
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err(IdempotencyKeyError::InvalidCharacters);
        }
        Ok(Self(raw.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for IdempotencyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for IdempotencyKey {
    type Err = IdempotencyKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TryFrom<String> for IdempotencyKey {
    type Error = IdempotencyKeyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl From<IdempotencyKey> for String {
    fn from(key: IdempotencyKey) -> Self {
        key.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn a_generated_token_is_a_valid_key() {
        let key = IdempotencyKey::new("01ARZ3XK7M0C4V6B8N2QWE9RTA").expect("valid key");
        assert_eq!(key.as_str().len(), 26);
        assert_eq!(key.to_string(), "01ARZ3XK7M0C4V6B8N2QWE9RTA");
    }

    #[test]
    fn short_keys_are_rejected() {
        assert!(matches!(
            IdempotencyKey::new("short"),
            Err(IdempotencyKeyError::InvalidLength { .. })
        ));
    }

    #[test]
    fn long_keys_are_rejected() {
        let long = "a".repeat(IDEMPOTENCY_KEY_MAX + 1);
        assert!(IdempotencyKey::new(&long).is_err());
    }

    #[test]
    fn forbidden_characters_are_rejected() {
        assert!(matches!(
            IdempotencyKey::new("with space inside"),
            Err(IdempotencyKeyError::InvalidCharacters)
        ));
        assert!(IdempotencyKey::new("dots.are.bad1").is_err());
    }

    #[test]
    fn dashes_and_underscores_are_allowed() {
        assert!(IdempotencyKey::new("retry-2026_11").is_ok());
    }

    #[test]
    fn wire_format_round_trips() {
        let key = IdempotencyKey::new("01ARZ3XK7M0C4V6B8N2QWE9RTA").expect("valid key");
        let json = serde_json::to_string(&key).expect("serialize");
        let back: IdempotencyKey = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, key);
    }

    #[test]
    fn keys_parse_from_str_and_string() {
        let raw = "01ARZ3XK7M0C4V6B8N2QWE9RTA";
        let parsed: IdempotencyKey = raw.parse().expect("parse");
        assert_eq!(parsed.as_str(), raw);
        let converted: IdempotencyKey = raw.to_owned().try_into().expect("try_from");
        assert_eq!(converted, parsed);
        assert_eq!(String::from(converted), raw.to_owned());
    }

    proptest! {
        #[test]
        fn keys_in_range_with_a_safe_alphabet_parse(
            key in "[0-9A-Za-z_-]{8,64}",
        ) {
            let parsed = IdempotencyKey::new(&key)?;
            prop_assert_eq!(parsed.as_str(), key);
        }

        #[test]
        fn out_of_range_lengths_are_rejected(key in "[0-9A-Za-z_-]{0,100}") {
            let result = IdempotencyKey::new(&key);
            let in_range = key.len() >= IDEMPOTENCY_KEY_MIN && key.len() <= IDEMPOTENCY_KEY_MAX;
            prop_assert_eq!(result.is_ok(), in_range);
        }
    }
}
