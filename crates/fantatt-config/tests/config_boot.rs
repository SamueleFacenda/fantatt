//! Boot-level integration tests: the whole pipeline from a config file, an env
//! file, and the CLI into typed settings, plus the startup aborts described in
//! `docs/18-execution-roadmap.md` G0.4.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use fantatt_config::cli::Cli;
use fantatt_config::environment::Environment;
use fantatt_config::error::ConfigError;
use fantatt_config::loader::{self, Loaded};

const REQUIRED_SECRETS: &str = "\
FANTATT__DATABASE__URL=postgres://fantatt@/fantatt
FANTATT__CRYPTO__SESSION_SECRET=session-secret
";

const STAGING_CONFIG: &str = r#"
[app]
name = "Fantatt Staging"
environment = "staging"

[server]
port = 8081
"#;

const INCOMPATIBLE_CONFIG: &str = r#"
[server]
public_port = 443

[session]
cookie_name = "session"

[privacy]
deletion_grace_days = 7
"#;

fn workspace(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("fantatt-config-{label}-{}", process::id()));
    fs::remove_dir_all(&path).ok();
    fs::create_dir_all(&path).expect("temp workspace");
    path
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap_or_else(|source| panic!("write {}: {source}", path.display()));
}

/// A config file, an env file layered on it, and the CLI pointing at the file.
fn staging_layers(root: &Path) -> (Environment, Cli) {
    let config = root.join("fantatt.toml");
    let env = root.join("fantatt.env");
    write(&config, STAGING_CONFIG);
    write(
        &env,
        &format!("{REQUIRED_SECRETS}FANTATT__SERVER__PORT=9090\nFANTATT__APP__LOCALE=en\n"),
    );
    let environment = Environment::from_env_file(&env).expect("parsed env file");
    let cli = Cli::parse(["--config", config.to_str().expect("utf-8 path")]).expect("cli");
    (environment, cli)
}

fn abort_from_config(root: &Path, contents: &str) -> ConfigError {
    let config = root.join("fantatt.toml");
    write(&config, contents);
    let cli = Cli::parse(["--config", config.to_str().expect("utf-8 path")]).expect("cli");
    loader::load(&Environment::default(), &cli).expect_err("must abort")
}

fn assert_precedence(loaded: &Loaded) {
    let settings = loaded.settings();
    assert_eq!(settings.app.name, "Fantatt Staging", "file beats defaults");
    assert_eq!(settings.server.port, 9090, "env beats file");
    assert_eq!(settings.app.locale, "en", "env beats file");
    assert_eq!(
        settings.server.public_port, 443,
        "unset keys keep the default"
    );
    assert_eq!(
        loaded.secret("database.url").unwrap().expose(),
        "postgres://fantatt@/fantatt"
    );
}

#[test]
fn boots_from_a_config_file_and_an_env_file_with_layered_precedence() {
    let root = workspace("layers");
    let (environment, cli) = staging_layers(&root);

    let loaded = loader::load(&environment, &cli).expect("boots");

    assert_precedence(&loaded);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn cli_overrides_beat_the_environment() {
    let root = workspace("cli-wins");
    let env = root.join("fantatt.env");
    write(
        &env,
        &format!("{REQUIRED_SECRETS}FANTATT__SERVER__PORT=9090\n"),
    );
    let environment = Environment::from_env_file(&env).expect("parsed env file");
    let cli = Cli::parse(["--set", "server.port=7070"]).expect("cli");

    let loaded = loader::load(&environment, &cli).expect("boots");

    assert_eq!(loaded.settings().server.port, 7070);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_secret_aborts_startup_naming_every_absent_key() {
    let root = workspace("missing-secret");
    let env = root.join("fantatt.env");
    write(&env, "FANTATT__CRYPTO__SESSION_SECRET=session-secret\n");
    let environment = Environment::from_env_file(&env).expect("parsed env file");

    let error = loader::load(&environment, &Cli::default()).expect_err("must abort");

    let message = error.to_string();
    assert!(message.contains("database.url"), "{message}");
    assert!(message.contains("FANTATT__DATABASE__URL"), "{message}");
    assert!(!message.contains("crypto.session_secret"), "{message}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_literal_secret_in_the_config_file_is_rejected() {
    let root = workspace("literal-secret");
    let error = abort_from_config(
        &root,
        "[database]\nurl = \"postgres://user:pass@host/fantatt\"\n",
    );

    assert!(
        error.to_string().contains("database.url"),
        "the abort names the offending key: {error}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn invalid_combinations_are_listed_together_in_one_abort() {
    let root = workspace("invalid-combinations");
    let error = abort_from_config(&root, INCOMPATIBLE_CONFIG);

    let message = error.to_string();
    assert!(message.contains("session.cookie_name"), "{message}");
    assert!(message.contains("privacy.deletion_grace_days"), "{message}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_negative_concurrency_is_rejected_naming_the_key() {
    let root = workspace("negative-concurrency");
    let config = root.join("fantatt.toml");
    write(&config, "[ingest]\nconcurrency = -1\n");
    let cli = Cli::parse(["--config", config.to_str().expect("utf-8 path")]).expect("cli");

    let error = loader::load(&Environment::default(), &cli).expect_err("must abort");

    assert!(
        error.to_string().contains("ingest.concurrency"),
        "the abort names the offending key: {error}"
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_configured_config_file_that_does_not_exist_is_named() {
    let root = workspace("absent-file");
    let absent = root.join("nowhere.toml").display().to_string();
    let cli = Cli::parse(["--config", &absent]).expect("cli");

    let error = loader::load(&Environment::default(), &cli).expect_err("must abort");

    assert!(error.to_string().contains("nowhere.toml"), "{error}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_malformed_env_file_reports_the_line() {
    let root = workspace("malformed-env");
    let env = root.join("fantatt.env");
    write(&env, "FANTATT__SERVER__PORT=9090\nNONSENSE\n");

    let error = loader::load(
        &Environment::from_pairs([("FANTATT__ENV_FILE", env.to_str().expect("utf-8 path"))]),
        &Cli::default(),
    )
    .expect_err("must fail");

    let message = error.to_string();
    assert!(message.contains("line 2"), "{message}");
    assert!(message.contains("NONSENSE"), "{message}");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn the_declared_env_file_supplies_its_variables() {
    let root = workspace("declared-env-file");
    let env = root.join("fantatt.env");
    write(&env, REQUIRED_SECRETS);

    let loaded = loader::load(
        &Environment::from_pairs([("FANTATT__ENV_FILE", env.to_str().expect("utf-8 path"))]),
        &Cli::parse(["--set", "app.locale=it"]).expect("cli"),
    )
    .expect("boots from the declared env file");

    assert_eq!(loaded.settings().app.locale, "it");
    assert_eq!(loaded.settings().server.port, 8080);
    fs::remove_dir_all(&root).ok();
}
