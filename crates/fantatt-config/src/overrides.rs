use config::{Map, Source, Value};

use crate::error::ConfigError;
use crate::settings::{KeyKind, Settings};

/// A layer of raw `key=value` text from the environment or the CLI, typed
/// against the settings schema.
///
/// Typing at the layer keeps the merge free of guessing: `server.port=8080`
/// becomes an integer and `app.name=8080` stays a string, exactly as the
/// compiled-in defaults declare.
#[derive(Debug, Clone)]
pub struct OverrideSet {
    entries: Vec<(String, Value)>,
}

impl OverrideSet {
    pub fn build<'a, I>(pairs: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut entries = Vec::new();
        for (key, raw) in pairs {
            entries.push((key.to_owned(), typed_value(key, raw)?));
        }
        Ok(Self { entries })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Source for OverrideSet {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<Map<String, Value>, config::ConfigError> {
        Ok(self
            .entries
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect())
    }
}

fn typed_value(key: &str, raw: &str) -> Result<Value, ConfigError> {
    check_settable(key)?;
    match Settings::key_kind(key) {
        Some(KeyKind::Integer) => parse_integer(key, raw).map(Value::from),
        Some(KeyKind::Float) => parse_number(key, raw).map(Value::from),
        Some(KeyKind::Boolean) => parse_flag(key, raw).map(Value::from),
        Some(KeyKind::Array) => Ok(Value::from(split_items(raw))),
        Some(KeyKind::Text) | None => Ok(Value::from(raw.to_owned())),
    }
}

fn parse_integer(key: &str, raw: &str) -> Result<i64, ConfigError> {
    raw.trim()
        .parse::<i64>()
        .map_err(|_| integer_expected(key, raw))
}

fn parse_number(key: &str, raw: &str) -> Result<f64, ConfigError> {
    raw.trim()
        .parse::<f64>()
        .map_err(|_| float_expected(key, raw))
}

fn parse_flag(key: &str, raw: &str) -> Result<bool, ConfigError> {
    raw.trim()
        .parse::<bool>()
        .map_err(|_| boolean_expected(key, raw))
}

fn integer_expected(key: &str, raw: &str) -> ConfigError {
    ConfigError::IntegerExpected {
        key: key.to_owned(),
        value: raw.to_owned(),
    }
}

fn float_expected(key: &str, raw: &str) -> ConfigError {
    ConfigError::NumberExpected {
        key: key.to_owned(),
        value: raw.to_owned(),
    }
}

fn boolean_expected(key: &str, raw: &str) -> ConfigError {
    ConfigError::BooleanExpected {
        key: key.to_owned(),
        value: raw.to_owned(),
    }
}

fn split_items(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Environment and CLI layers carry references and plain values, never literals
/// for secret fields, so a secret field is settable in a file only.
fn check_settable(key: &str) -> Result<(), ConfigError> {
    if key.is_empty() {
        return Err(ConfigError::UnknownKey {
            key: key.to_owned(),
        });
    }
    if crate::settings::SECRET_FIELDS.contains(&key) {
        return Err(ConfigError::SecretField {
            key: key.to_owned(),
        });
    }
    if Settings::key_kind(key).is_none() {
        return Err(ConfigError::UnknownKey {
            key: key.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use config::Config;

    fn build_config(overrides: &OverrideSet) -> Config {
        Config::builder()
            .add_source(overrides.clone())
            .build()
            .unwrap()
    }

    #[test]
    fn integers_booleans_and_text_are_typed_from_the_schema() {
        let overrides = OverrideSet::build([
            ("server.port", "9090"),
            ("ingest.respect_robots", "false"),
            ("app.name", "8080"),
        ])
        .unwrap();
        let config = build_config(&overrides);
        assert_eq!(config.get_int("server.port").unwrap(), 9090);
        assert!(!config.get_bool("ingest.respect_robots").unwrap());
        assert_eq!(config.get_string("app.name").unwrap(), "8080");
    }

    #[test]
    fn arrays_are_split_on_commas() {
        let overrides = OverrideSet::build([("ingest.regions", "Trentino,Veneto")]).unwrap();
        let config = build_config(&overrides);
        assert_eq!(
            config.get_array("ingest.regions").unwrap(),
            vec![Value::from("Trentino"), Value::from("Veneto")]
        );
    }

    #[test]
    fn an_integer_for_an_integer_field_is_rejected_and_named() {
        let error = OverrideSet::build([("server.port", "eighty")]).unwrap_err();
        match &error {
            ConfigError::IntegerExpected { key, value } => {
                assert_eq!(key, "server.port");
                assert_eq!(value, "eighty");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn a_boolean_is_rejected_when_the_field_expects_one() {
        let error = OverrideSet::build([("ingest.respect_robots", "yes")]).unwrap_err();
        assert!(matches!(error, ConfigError::BooleanExpected { .. }));
    }

    #[test]
    fn an_unknown_key_is_rejected() {
        let error = OverrideSet::build([("server.porrt", "9090")]).unwrap_err();
        assert!(
            matches!(error, ConfigError::UnknownKey { .. }),
            "unexpected error: {error:?}"
        );
    }
}
