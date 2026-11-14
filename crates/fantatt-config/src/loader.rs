use std::path::{Path, PathBuf};

use config::{Config, File, FileFormat, Source};

use crate::cli::Cli;
use crate::environment::Environment;
use crate::error::ConfigError;
use crate::overrides::OverrideSet;
use crate::secret::{self, Secret};
use crate::settings::{self, Settings};
use crate::validation;

/// Settings plus the secrets their references resolve to.
#[derive(Debug, Clone)]
pub struct Loaded {
    settings: Settings,
    secrets: std::collections::BTreeMap<String, Secret>,
}

impl Loaded {
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn secret(&self, key: &str) -> Option<&Secret> {
        self.secrets.get(key)
    }
}

/// Loads every layer in order and validates the result.
///
/// Precedence, lowest first: compiled-in defaults, config file, environment,
/// CLI. Secrets resolve last, so a missing one aborts with a single message
/// naming every absent key.
pub fn load(environment: &Environment, cli: &Cli) -> Result<Loaded, ConfigError> {
    let environment = with_declared_env_file(environment)?;
    let settings = layered_settings(&environment, cli)?;
    let violations = validation::collect(&settings);
    if !violations.is_empty() {
        return Err(ConfigError::Violations { violations });
    }
    let secrets = resolve_secrets(&settings, &environment)?;
    Ok(Loaded { settings, secrets })
}

/// Merges the env file named by `FANTATT__ENV_FILE` so a deployment declares its
/// variables in a file while runtime exports still win.
fn with_declared_env_file(environment: &Environment) -> Result<Environment, ConfigError> {
    let Some(path) = environment.env_file_path() else {
        return Ok(environment.clone());
    };
    let declared = Environment::from_env_file(Path::new(path)).map_err(ConfigError::from)?;
    Ok(environment.overlay(&declared))
}

fn layered_settings(environment: &Environment, cli: &Cli) -> Result<Settings, ConfigError> {
    let config = Config::builder()
        .add_source(config::File::from_str(
            include_str!("defaults.toml"),
            FileFormat::Toml,
        ))
        .add_source(file_layer(environment, cli)?)
        .add_source(env_layer(environment)?)
        .add_source(cli_layer(cli)?)
        .build()
        .map_err(|source| ConfigError::Layer { source })?;
    check_secret_references(&config)?;
    config
        .try_deserialize()
        .map_err(|source| ConfigError::Layer { source })
}

/// A config file may only ever hold `env:` references: a literal would land in
/// a file, a backup, or a diff.
fn check_secret_references(layers: &Config) -> Result<(), ConfigError> {
    for key in settings::SECRET_FIELDS {
        let Ok(value) = layers.get::<String>(key) else {
            continue;
        };
        if !value.starts_with(secret::REFERENCE_PREFIX) {
            return Err(ConfigError::SecretLiteral {
                key: key.to_owned(),
            });
        }
    }
    Ok(())
}

/// An explicitly selected file must exist; the default path is optional so a
/// deployment can run on defaults, environment, and CLI alone.
fn file_layer(environment: &Environment, cli: &Cli) -> Result<ConfigFile, ConfigError> {
    let selected = cli.config_path().or_else(|| environment.config_path());
    if let Some(path) = &selected
        && !Path::new(path).exists()
    {
        return Err(ConfigError::FileUnreadable {
            path: PathBuf::from(path),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "no such config file"),
        });
    }
    Ok(ConfigFile {
        path: selected.map(PathBuf::from),
    })
}

/// A TOML config file as a configuration layer.
#[derive(Debug, Clone)]
struct ConfigFile {
    path: Option<PathBuf>,
}

impl Source for ConfigFile {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<config::Map<String, config::Value>, config::ConfigError> {
        let Some(path) = &self.path else {
            return Ok(config::Map::new());
        };
        let text = std::fs::read_to_string(path)
            .map_err(|source| config::ConfigError::Message(source.to_string()))?;
        File::from_str(&text, FileFormat::Toml).collect()
    }
}

fn env_layer(environment: &Environment) -> Result<OverrideSet, ConfigError> {
    let pairs: Vec<(String, String)> = environment.dotted_overrides();
    let borrowed: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    OverrideSet::build(borrowed)
}

fn cli_layer(cli: &Cli) -> Result<OverrideSet, ConfigError> {
    let borrowed: Vec<(&str, &str)> = cli
        .overrides()
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    OverrideSet::build(borrowed)
}

/// Resolves every `env:` reference against the environment, collecting all
/// missing ones into one abort so an operator sees the whole picture.
fn resolve_secrets(
    settings: &Settings,
    environment: &Environment,
) -> Result<std::collections::BTreeMap<String, Secret>, ConfigError> {
    let mut resolved = std::collections::BTreeMap::new();
    let mut missing = Vec::new();
    for (key, reference) in settings.required_secrets() {
        match environment.get(reference.variable_name()) {
            Some(value) if !value.trim().is_empty() => {
                resolved.insert(key.clone(), Secret::from(value.to_owned()));
            }
            _ => missing.push(format!("{key} ({reference})")),
        }
    }
    if missing.is_empty() {
        Ok(resolved)
    } else {
        Err(ConfigError::MissingSecrets { keys: missing })
    }
}
