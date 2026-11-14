use std::collections::BTreeMap;
use std::path::Path;

use thiserror::Error;

use crate::settings;

/// Prefix of every environment variable that participates in a layer.
const VARIABLE_PREFIX: &str = "FANTATT__";

/// Key of variable that points at the deployment config file.
const CONFIG_PATH_VARIABLE: &str = "FANTATT__CONFIG_FILE";

/// Key of variable that points at the deployment env file.
const ENV_FILE_VARIABLE: &str = "FANTATT__ENV_FILE";

/// Variables that steer the boot itself rather than carry a setting.
const CONTROL_VARIABLES: [&str; 2] = [CONFIG_PATH_VARIABLE, ENV_FILE_VARIABLE];

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error("env file '{path}' is unreadable: {source}")]
    Unreadable {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("env file '{path}' line {line_number} is not a KEY=VALUE pair: '{line}'")]
    Malformed {
        path: std::path::PathBuf,
        line_number: usize,
        line: String,
    },
}

/// An injected environment, read either from the process or from a file.
///
/// The process environment is deliberately not read implicitly: every caller
/// states where its variables come from so a boot can be exercised in tests.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Environment {
    variables: BTreeMap<String, String>,
}

impl Environment {
    pub fn from_process() -> Self {
        Self {
            variables: std::env::vars().collect(),
        }
    }

    pub fn from_pairs<'a, I>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        Self {
            variables: pairs
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
        }
    }

    pub fn from_env_file(path: &Path) -> Result<Self, EnvironmentError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| EnvironmentError::Unreadable {
                path: path.to_owned(),
                source,
            })?;
        Self::from_dotenv(&text, path)
    }

    /// Parses `KEY=VALUE` lines, ignoring blanks and `#` comments. Values may be
    /// quoted to preserve leading or trailing whitespace.
    pub fn from_dotenv(text: &str, origin: &Path) -> Result<Self, EnvironmentError> {
        let mut variables = BTreeMap::new();
        for (index, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = entry(line).ok_or_else(|| malformed(origin, index + 1, line))?;
            variables.insert(key, unquote(value));
        }
        Ok(Self { variables })
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(String::as_str)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.variables.contains_key(key)
    }

    /// The config file this environment selects, if any.
    pub fn config_path(&self) -> Option<&str> {
        self.get(CONFIG_PATH_VARIABLE)
    }

    /// The env file this deployment declares its variables in, if any.
    pub fn env_file_path(&self) -> Option<&str> {
        self.get(ENV_FILE_VARIABLE)
    }

    /// Records the variables of `other` that this environment does not hold.
    ///
    /// why: a deployment declares its variables in a file, and an operator who
    /// exports one at runtime must win over that file.
    pub fn overlay(&self, other: &Environment) -> Self {
        let mut variables = self.variables.clone();
        for (key, value) in &other.variables {
            variables
                .entry(key.clone())
                .or_insert_with(|| value.clone());
        }
        Self { variables }
    }

    /// The FANTATT-prefixed variables, lowercased and split into dotted keys.
    ///
    /// Secret fields are excluded: the variable named by an `env:` reference
    /// holds the literal, so `FANTATT__DATABASE__URL` must not be misread as an
    /// override of the reference itself. Control variables are excluded for the
    /// same reason: they steer the boot, they are not settings.
    pub fn dotted_overrides(&self) -> Vec<(String, String)> {
        self.variables
            .iter()
            .filter(|(key, _)| key.starts_with(VARIABLE_PREFIX))
            .filter(|(key, _)| !CONTROL_VARIABLES.contains(&key.as_str()))
            .map(|(key, value)| (dotted_key(key), value.clone()))
            .filter(|(key, _)| !settings::SECRET_FIELDS.contains(&key.as_str()))
            .collect()
    }

    /// Records the variables that secret references resolve against.
    pub fn secrets(&self) -> BTreeMap<String, String> {
        self.variables.clone()
    }
}

fn malformed(origin: &Path, line_number: usize, line: &str) -> EnvironmentError {
    EnvironmentError::Malformed {
        path: origin.to_owned(),
        line_number,
        line: line.to_owned(),
    }
}

/// why: a key without a value, or a value attached to no key, hides a typo in a
/// file nobody re-reads once the service is up.
fn entry(line: &str) -> Option<(String, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key.to_owned(), value.trim()))
}

fn dotted_key(variable: &str) -> String {
    variable
        .trim_start_matches(VARIABLE_PREFIX)
        .to_lowercase()
        .replace("__", ".")
}

/// why: a quoted dotenv value keeps its whitespace, an unquoted one has already
/// been trimmed.
fn unquote(value: &str) -> String {
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].replace("\\\"", "\"")
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixed_variables_become_dotted_keys() {
        let environment = Environment::from_pairs([
            ("FANTATT__SERVER__PORT", "9090"),
            ("FANTATT__APP__NAME", "Fantatt Test"),
            ("PATH", "/usr/bin"),
        ]);
        assert_eq!(
            environment.dotted_overrides(),
            vec![
                ("app.name".to_owned(), "Fantatt Test".to_owned()),
                ("server.port".to_owned(), "9090".to_owned()),
            ]
        );
    }

    #[test]
    fn secret_variables_are_not_read_as_overrides() {
        let environment = Environment::from_pairs([
            ("FANTATT__DATABASE__URL", "postgres://literal"),
            ("FANTATT__CRYPTO__SESSION_SECRET", "literal"),
            ("FANTATT__SERVER__PORT", "9090"),
        ]);
        assert_eq!(
            environment.dotted_overrides(),
            vec![("server.port".to_owned(), "9090".to_owned())]
        );
        assert!(environment.contains("FANTATT__DATABASE__URL"));
    }

    #[test]
    fn control_variables_are_not_read_as_overrides() {
        let environment = Environment::from_pairs([
            ("FANTATT__CONFIG_FILE", "deploy/fantatt.toml"),
            ("FANTATT__ENV_FILE", "deploy/fantatt.env"),
            ("FANTATT__SERVER__PORT", "9090"),
        ]);
        assert_eq!(
            environment.dotted_overrides(),
            vec![("server.port".to_owned(), "9090".to_owned())]
        );
    }

    #[test]
    fn env_file_parses_values_comments_and_quotes() {
        let text = "# comment\nFANTATT__SERVER__PORT=9090\nFANTATT__APP__NAME=\"fantatt \"\n\n";
        let environment = Environment::from_dotenv(text, Path::new(".env")).expect("valid");
        assert_eq!(environment.get("FANTATT__SERVER__PORT"), Some("9090"));
        assert_eq!(environment.get("FANTATT__APP__NAME"), Some("fantatt "));
    }

    #[test]
    fn a_line_without_an_equals_sign_is_rejected_and_named() {
        let error = Environment::from_dotenv("NONSENSE\n", Path::new(".env")).unwrap_err();
        match &error {
            EnvironmentError::Malformed {
                line_number, line, ..
            } => {
                assert_eq!(*line_number, 1);
                assert_eq!(line, "NONSENSE");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn the_config_and_env_file_are_read_from_variables() {
        let environment = Environment::from_pairs([
            ("FANTATT__CONFIG_FILE", "deploy/fantatt.toml"),
            ("FANTATT__ENV_FILE", "deploy/fantatt.env"),
        ]);
        assert_eq!(environment.config_path(), Some("deploy/fantatt.toml"));
        assert_eq!(environment.env_file_path(), Some("deploy/fantatt.env"));
    }
}
