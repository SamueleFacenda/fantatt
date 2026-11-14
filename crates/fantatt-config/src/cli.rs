use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{flag} needs a value, as in --set section.key=value")]
    FlagNeedsValue { flag: &'static str },

    #[error("--set needs a section.key=value pair, got '{entry}'")]
    MalformedEntry { entry: String },

    #[error("'{key}' names no configuration key")]
    UnknownKey { key: String },
}

/// The command-line layer: a config file path and dotted `--set` overrides.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cli {
    config_path: Option<String>,
    overrides: Vec<(String, String)>,
}

impl Cli {
    pub fn parse<'a, I>(arguments: I) -> Result<Self, CliError>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut cli = Self::default();
        let mut arguments = arguments.into_iter().peekable();
        while let Some(argument) = arguments.next() {
            match argument {
                "--config" => {
                    let path = arguments
                        .next()
                        .ok_or(CliError::FlagNeedsValue { flag: "--config" })?;
                    cli.config_path = Some(path.to_owned());
                }
                "--set" => {
                    let entry = arguments
                        .next()
                        .ok_or(CliError::FlagNeedsValue { flag: "--set" })?;
                    cli.overrides.push(parse_entry(entry)?);
                }
                _ => {}
            }
        }
        Ok(cli)
    }

    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref()
    }

    pub fn overrides(&self) -> &[(String, String)] {
        &self.overrides
    }
}

fn parse_entry(entry: &str) -> Result<(String, String), CliError> {
    let (raw_key, value) = entry
        .split_once('=')
        .ok_or_else(|| CliError::MalformedEntry {
            entry: entry.to_owned(),
        })?;
    let key = raw_key.trim();
    if !crate::settings::Settings::is_settable_key(key) {
        return Err(CliError::UnknownKey {
            key: key.to_owned(),
        });
    }
    Ok((key.to_owned(), value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_path_and_set_overrides_are_parsed() {
        let cli = Cli::parse([
            "--config",
            "deploy/fantatt.toml",
            "--set",
            "server.port=9090",
            "--set",
            "app.name=Fantatt Test",
        ])
        .expect("valid cli");
        assert_eq!(cli.config_path(), Some("deploy/fantatt.toml"));
        assert_eq!(
            cli.overrides(),
            &[
                ("server.port".to_owned(), "9090".to_owned()),
                ("app.name".to_owned(), "Fantatt Test".to_owned()),
            ]
        );
    }

    #[test]
    fn no_flags_yield_an_empty_layer() {
        let cli = Cli::parse(["serve", "--workers", "4"]).expect("valid cli");
        assert_eq!(cli, Cli::default());
    }

    #[test]
    fn a_flag_without_a_value_is_named() {
        let error = Cli::parse(["--config"]).unwrap_err();
        match &error {
            CliError::FlagNeedsValue { flag } => assert_eq!(*flag, "--config"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn an_entry_without_an_equals_sign_is_named() {
        let error = Cli::parse(["--set", "server.port"]).unwrap_err();
        match &error {
            CliError::MalformedEntry { entry } => assert_eq!(entry, "server.port"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn a_secret_may_only_be_referenced_never_set() {
        let error = Cli::parse(["--set", "database.url=postgres://literal"]).unwrap_err();
        assert!(matches!(error, CliError::UnknownKey { .. }), "{error:?}");
    }

    #[test]
    fn a_key_without_a_section_is_rejected() {
        assert!(Cli::parse(["--set", "port=9090"]).is_err());
    }
}
