use std::path::PathBuf;

use thiserror::Error;

use crate::environment::EnvironmentError;
use crate::validation::ValidationError;

/// Every way startup can fail before the service accepts traffic.
///
/// Each variant names the offending config key, so the abort message doubles as
/// the fix instruction.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file '{path}' is unreadable: {source}")]
    FileUnreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("env file '{path}' is unreadable: {source}")]
    EnvFileUnreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("env file '{path}' line {line_number} is not a KEY=VALUE pair: '{line}'")]
    EnvFileMalformed {
        path: PathBuf,
        line_number: usize,
        line: String,
    },

    #[error("a configuration layer could not be read: {source}")]
    Layer {
        #[source]
        source: config::ConfigError,
    },

    #[error("'{key}' names no configuration key")]
    UnknownKey { key: String },

    #[error(
        "'{key}' is a secret: a config file references it as 'env:VARIABLE' and the value is read from the environment, never set here"
    )]
    SecretField { key: String },

    #[error(
        "'{key}' holds a literal secret; reference it as 'env:VARIABLE' so the value stays out of files"
    )]
    SecretLiteral { key: String },

    #[error("'{key}' holds a table or an array and cannot be overridden with '{value}'")]
    TabularKey { key: String, value: String },

    #[error("'{key}' expects an integer, got '{value}'")]
    IntegerExpected { key: String, value: String },

    #[error("'{key}' expects a number, got '{value}'")]
    NumberExpected { key: String, value: String },

    #[error("'{key}' expects a boolean, got '{value}'")]
    BooleanExpected { key: String, value: String },

    #[error("'{key}' expects a comma-separated array, got '{value}'")]
    ArrayExpected { key: String, value: String },

    #[error("startup aborted, {} invalid value combinations:\n{}", violations.len(), bullets(violations))]
    Violations { violations: Vec<ValidationError> },

    #[error(
        "startup aborted, {} required secrets hold no value in the environment:\n{}",
        keys.len(),
        bullets(keys)
    )]
    MissingSecrets { keys: Vec<String> },
}

impl ConfigError {
    pub fn violations(violations: Vec<ValidationError>) -> Self {
        Self::Violations { violations }
    }

    pub fn missing_secrets(keys: Vec<String>) -> Self {
        Self::MissingSecrets { keys }
    }
}

impl From<EnvironmentError> for ConfigError {
    fn from(source: EnvironmentError) -> Self {
        match source {
            EnvironmentError::Unreadable { path, source } => {
                Self::EnvFileUnreadable { path, source }
            }
            EnvironmentError::Malformed {
                path,
                line_number,
                line,
            } => Self::EnvFileMalformed {
                path,
                line_number,
                line,
            },
        }
    }
}

fn bullets<T, I>(items: I) -> String
where
    T: std::fmt::Display,
    I: IntoIterator<Item = T>,
{
    items
        .into_iter()
        .map(|item| format!("  - {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

impl From<config::ConfigError> for ConfigError {
    fn from(source: config::ConfigError) -> Self {
        Self::Layer { source }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_abort_lists_every_violation_on_its_own_line() {
        let error = ConfigError::Violations {
            violations: vec![
                ValidationError::GraceWindowTooShort { days: 7 },
                ValidationError::BindPortReserved { port: 0 },
            ],
        };
        let message = error.to_string();
        assert!(
            message.contains("2 invalid value combinations"),
            "{message}"
        );
        assert!(message.contains("  - privacy.deletion_grace_days must be at least 30, got 7"));
        assert!(message.contains("  - server.port must be a usable port, got 0"));
    }

    #[test]
    fn the_abort_names_every_missing_secret() {
        let error = ConfigError::MissingSecrets {
            keys: vec![
                "database.url".to_owned(),
                "crypto.session_secret".to_owned(),
            ],
        };
        let message = error.to_string();
        assert!(message.contains("database.url"), "{message}");
        assert!(message.contains("crypto.session_secret"), "{message}");
    }

    #[test]
    fn single_item_variants_stay_on_one_line() {
        let error = ConfigError::UnknownKey {
            key: "server.porrt".to_owned(),
        };
        assert_eq!(
            error.to_string(),
            "'server.porrt' names no configuration key"
        );
    }

    #[test]
    fn layer_errors_carry_the_underlying_source() {
        let error = ConfigError::Layer {
            source: config::ConfigError::NotFound("fantatt.toml".to_owned()),
        };
        assert!(error.to_string().contains("layer"));
        let rendered = format!("{error:?}");
        assert!(rendered.contains("fantatt.toml"), "{rendered}");
        assert!(rendered.contains("not found"), "{rendered}");
    }
}
