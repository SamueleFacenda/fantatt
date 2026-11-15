use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SecretParseError {
    /// why: the offending text is never echoed, so a rejected literal cannot
    /// leak into logs or an abort message.
    #[error("a secret may only be referenced as 'env:VARIABLE', never given as a literal")]
    NotAReference,

    #[error("secret reference names no variable")]
    EmptyVariableName,
}

pub(crate) const REFERENCE_PREFIX: &str = "env:";

/// A secret held by environment variable name, never by value.
///
/// Config files, environment overrides, and CLI flags may only ever hold the
/// reference. The literal value exists solely in the process environment, so
/// the reference is safe to print, log, and commit.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct SecretRef {
    variable: String,
}

impl SecretRef {
    pub fn variable_name(&self) -> &str {
        &self.variable
    }
}

impl std::fmt::Debug for SecretRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretRef({})", self)
    }
}

impl std::fmt::Display for SecretRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{REFERENCE_PREFIX}{}", self.variable)
    }
}

impl TryFrom<String> for SecretRef {
    type Error = SecretParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let variable = value
            .strip_prefix(REFERENCE_PREFIX)
            .ok_or(SecretParseError::NotAReference)?;
        if variable.is_empty() {
            return Err(SecretParseError::EmptyVariableName);
        }
        Ok(Self {
            variable: variable.to_owned(),
        })
    }
}

impl From<SecretRef> for String {
    fn from(reference: SecretRef) -> Self {
        reference.to_string()
    }
}

/// A resolved secret value.
///
/// Debug and Display are redacted so a secret never reaches logs, metrics, or
/// panic messages. Callers that need the literal opt in with `expose`.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
    value: String,
}

impl Secret {
    pub fn expose(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

impl From<String> for Secret {
    fn from(value: String) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_parses_and_names_its_variable() {
        let reference = SecretRef::try_from("env:FANTATT__DATABASE__URL".to_owned()).unwrap();
        assert_eq!(reference.variable_name(), "FANTATT__DATABASE__URL");
        assert_eq!(reference.to_string(), "env:FANTATT__DATABASE__URL");
    }

    #[test]
    fn literal_values_are_rejected_as_references() {
        assert!(SecretRef::try_from("postgres://user:pass@host/db".to_owned()).is_err());
        assert!(SecretRef::try_from("env:".to_owned()).is_err());
    }

    #[test]
    fn serde_round_trips_the_reference_form() {
        let json = serde_json::to_string(&SecretRef::try_from("env:TOKEN".to_owned()).unwrap())
            .expect("serialize");
        assert_eq!(json, "\"env:TOKEN\"");
    }

    #[test]
    fn resolved_secret_is_redacted() {
        let secret = Secret::from("hunter2".to_owned());
        assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
        assert_eq!(secret.to_string(), "<redacted>");
        assert_eq!(secret.expose(), "hunter2");
    }
}
