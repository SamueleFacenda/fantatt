use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error produced when money is constructed from an invalid ISO 4217 code.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MoneyParseError {
    #[error("currency code must be exactly 3 uppercase ASCII letters, got '{0}'")]
    InvalidCurrency(String),
}

/// A non-negative monetary amount: ISO 4217 currency code + integer minor units.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Money {
    currency: String,
    minor_units: u64,
}

impl Money {
    pub fn new(currency: &str, minor_units: u64) -> Result<Self, MoneyParseError> {
        if !is_iso_4217(currency) {
            return Err(MoneyParseError::InvalidCurrency(currency.to_owned()));
        }
        Ok(Self {
            currency: currency.to_owned(),
            minor_units,
        })
    }

    pub fn currency(&self) -> &str {
        &self.currency
    }

    pub fn minor_units(&self) -> u64 {
        self.minor_units
    }

    /// Adds two amounts of the same currency, failing on overflow.
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_add(other.minor_units)
            .map(|minor_units| Self {
                currency: self.currency.clone(),
                minor_units,
            })
    }

    /// Subtracts a same-currency amount, failing on underflow.
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        if self.currency != other.currency {
            return None;
        }
        self.minor_units
            .checked_sub(other.minor_units)
            .map(|minor_units| Self {
                currency: self.currency.clone(),
                minor_units,
            })
    }

    fn from_wire(text: &str) -> Result<Self, MoneyParseError> {
        let (currency, minor_units) = text
            .split_once(' ')
            .ok_or_else(|| MoneyParseError::InvalidCurrency(text.to_owned()))?;
        let minor_units = minor_units
            .parse::<u64>()
            .map_err(|_| MoneyParseError::InvalidCurrency(text.to_owned()))?;
        Self::new(currency, minor_units)
    }
}

/// why: ISO 4217 codes are exactly three uppercase ASCII letters.
fn is_iso_4217(code: &str) -> bool {
    code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase())
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.currency, self.minor_units)
    }
}

impl std::str::FromStr for Money {
    type Err = MoneyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_wire(s)
    }
}

impl TryFrom<String> for Money {
    type Error = MoneyParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_wire(&value)
    }
}

impl From<Money> for String {
    fn from(money: Money) -> Self {
        money.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn new_accepts_iso_4217_code_with_minor_units() {
        let money = Money::new("EUR", 1500).expect("valid money");
        assert_eq!(money.currency(), "EUR");
        assert_eq!(money.minor_units(), 1500);
        assert_eq!(money.to_string(), "EUR 1500");
    }

    #[test]
    fn new_rejects_lowercase_or_wrong_length_codes() {
        assert!(Money::new("eur", 0).is_err());
        assert!(Money::new("EURO", 0).is_err());
        assert!(Money::new("EU", 0).is_err());
        assert!(Money::new("E1R", 0).is_err());
    }

    #[test]
    fn checked_add_requires_same_currency() {
        let euros = Money::new("EUR", 100).expect("valid");
        assert_eq!(
            euros.checked_add(&Money::new("EUR", 50).expect("valid")),
            Some(Money::new("EUR", 150).expect("valid"))
        );
        assert_eq!(
            euros.checked_add(&Money::new("USD", 50).expect("valid")),
            None
        );
    }

    #[test]
    fn checked_add_detects_overflow() {
        let max = Money::new("EUR", u64::MAX).expect("valid");
        assert!(
            max.checked_add(&Money::new("EUR", 1).expect("valid"))
                .is_none()
        );
    }

    #[test]
    fn checked_sub_detects_underflow_and_currency_mismatch() {
        let money = Money::new("EUR", 100).expect("valid");
        assert_eq!(
            money.checked_sub(&Money::new("EUR", 40).expect("valid")),
            Some(Money::new("EUR", 60).expect("valid"))
        );
        assert!(
            money
                .checked_sub(&Money::new("EUR", 101).expect("valid"))
                .is_none()
        );
        assert!(
            money
                .checked_sub(&Money::new("USD", 40).expect("valid"))
                .is_none()
        );
    }

    #[test]
    fn wire_format_round_trips() {
        let money = Money::new("USD", 12_34).expect("valid");
        let parsed = money.to_string().parse::<Money>().expect("parse");
        assert_eq!(parsed, money);
        let json = serde_json::to_string(&money).expect("serialize");
        let back: Money = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, money);
    }

    #[test]
    fn malformed_wire_forms_are_rejected() {
        assert!("EURno-space".parse::<Money>().is_err());
        assert!("EUR not-a-number".parse::<Money>().is_err());
        assert!("eur 100".parse::<Money>().is_err());
    }

    proptest! {
        #[test]
        fn only_three_uppercase_letters_form_currencies(candidate in "[A-Za-z0-9]{1,5}") {
            let result = Money::new(&candidate, 0);
            prop_assert_eq!(result.is_ok(), is_iso_4217(&candidate));
        }

        #[test]
        fn minor_units_add_either_sums_or_reports_overflow(a in 0u64..=u64::MAX, b in 0u64..=u64::MAX) {
            let left = Money::new("EUR", a)?;
            let right = Money::new("EUR", b)?;
            match left.checked_add(&right) {
                Some(sum) => prop_assert_eq!(sum.minor_units(), a + b),
                None => prop_assert!(a.checked_add(b).is_none()),
            }
        }
    }
}
