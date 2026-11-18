pub mod app;
pub mod auth;
pub mod authz;
pub mod crypto;
pub mod csrf;
pub mod database;
pub mod email;
pub mod ingest;
pub mod knowledge_graph;
pub mod live;
pub mod notifications;
pub mod observability;
pub mod privacy;
pub mod rate_limit;
pub mod rules;
pub mod server;
pub mod session;
pub mod storage;

use serde::{Deserialize, Serialize};

pub use secret::SecretRef;

use crate::secret;

use app::App;
use auth::Auth;
use authz::Authz;
use crypto::Crypto;
use csrf::Csrf;
use database::Database;
use email::Email;
use ingest::Ingest;
use knowledge_graph::KnowledgeGraph;
use live::Live;
use notifications::Notifications;
use observability::Observability;
use privacy::Privacy;
use rate_limit::RateLimit;
use rules::Rules;
use server::Server;
use session::Session;
use storage::Storage;

/// Config keys whose value is an `env:` reference and never a literal
/// (`docs/12-configuration.md` §3). A variable named by one of these holds the
/// secret, so it is not an override target.
pub const SECRET_FIELDS: [&str; 6] = [
    "database.url",
    "auth.google.client_secret",
    "notifications.vapid_private_key",
    "storage.secret_key",
    "observability.otlp_headers",
    "crypto.session_secret",
];

/// The complete typed configuration for every module of the service.
///
/// Field order declares the configuration file layout, so each section below
/// mirrors a table of `docs/12-configuration.md` §2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub app: App,
    pub server: Server,
    pub database: Database,
    pub knowledge_graph: KnowledgeGraph,
    pub session: Session,
    pub auth: Auth,
    pub authz: Authz,
    pub csrf: Csrf,
    pub rate_limit: RateLimit,
    pub ingest: Ingest,
    pub email: Email,
    pub storage: Storage,
    pub notifications: Notifications,
    pub observability: Observability,
    pub rules: Rules,
    pub privacy: Privacy,
    pub live: Live,
    pub crypto: Crypto,
}

impl Default for Settings {
    /// why: the defaults are compiled in from `defaults.toml`, which is part of
    /// the crate source and verified by `defaults_are_typed_and_documented`.
    fn default() -> Self {
        toml::from_str(include_str!("../defaults.toml")).expect("compiled-in defaults must parse")
    }
}

impl Settings {
    /// The schema shape of the compiled-in defaults, used to type environment
    /// and CLI overrides without duplicating the field list.
    pub fn schema_shape() -> toml::Value {
        toml::from_str(include_str!("../defaults.toml")).expect("compiled-in defaults must parse")
    }

    /// The leaf type of a dotted config key, or `None` when no such key exists.
    pub fn key_kind(key: &str) -> Option<KeyKind> {
        toml_kind(&Self::schema_shape(), key.split('.'))
    }

    /// Whether an environment variable or CLI flag may set this key.
    ///
    /// Secret references are only settable in a config file, and the variable
    /// they name is where the literal lives.
    pub fn is_settable_key(key: &str) -> bool {
        !SECRET_FIELDS.contains(&key) && Self::key_kind(key).is_some()
    }

    /// The secrets this deployment needs, as `(config key, reference)` pairs,
    /// each gated by the feature that consumes it.
    pub fn required_secrets(&self) -> Vec<(String, SecretRef)> {
        [
            ("database.url", true),
            ("crypto.session_secret", true),
            ("auth.google.client_secret", self.auth.google.enabled),
            (
                "notifications.vapid_private_key",
                self.notifications.push_enabled,
            ),
            ("storage.secret_key", self.storage.is_object_store()),
            (
                "observability.otlp_headers",
                self.observability.needs_auth_headers(),
            ),
        ]
        .into_iter()
        .filter(|(_, needed)| *needed)
        .map(|(key, _)| (key.to_owned(), self.reference_for(key)))
        .collect()
    }

    /// The `env:` reference a secret field resolves against.
    pub fn reference_for(&self, key: &str) -> SecretRef {
        match key {
            "database.url" => self.database.url.clone(),
            "auth.google.client_secret" => self.auth.google.client_secret.clone(),
            "notifications.vapid_private_key" => self.notifications.vapid_private_key.clone(),
            "storage.secret_key" => self.storage.secret_key.clone(),
            "observability.otlp_headers" => self.observability.otlp_headers.clone(),
            "crypto.session_secret" => self.crypto.session_secret.clone(),
            other => panic!("no secret reference for {other}"),
        }
    }
}

/// The leaf type a config value must be written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Integer,
    Float,
    Boolean,
    Text,
    Array,
}

fn toml_kind<'a, I>(schema: &'a toml::Value, path: I) -> Option<KeyKind>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut leaf = schema;
    for segment in path {
        match leaf {
            toml::Value::Table(map) => leaf = map.get(segment)?,
            _ => return None,
        }
    }
    kind_of(leaf)
}

fn kind_of(leaf: &toml::Value) -> Option<KeyKind> {
    match leaf {
        toml::Value::Integer(_) => Some(KeyKind::Integer),
        toml::Value::Float(_) => Some(KeyKind::Float),
        toml::Value::Boolean(_) => Some(KeyKind::Boolean),
        toml::Value::String(_) => Some(KeyKind::Text),
        toml::Value::Array(_) => Some(KeyKind::Array),
        toml::Value::Table(_) | toml::Value::Datetime(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_assign_every_documented_section() {
        let settings = Settings::default();
        assert_eq!(settings.app.name, "Fantatt");
        assert_eq!(settings.server.port, 8080);
        assert_eq!(settings.ingest.concurrency, 4);
        assert_eq!(settings.privacy.deletion_grace_days, 30);
        assert_eq!(settings.session.cookie_name, "__Host-session");
        assert!(settings.authz.superadmins.is_empty());
        assert_eq!(
            settings.crypto.session_secret.variable_name(),
            "FANTATT__CRYPTO__SESSION_SECRET"
        );
    }

    #[test]
    fn every_secret_field_is_typed_as_a_reference() {
        for key in SECRET_FIELDS {
            let kind = Settings::key_kind(key);
            assert_eq!(kind, Some(KeyKind::Text), "{key} should be text");
        }
    }

    #[test]
    fn unknown_and_tabular_keys_have_no_kind() {
        assert_eq!(Settings::key_kind("server.porrt"), None);
        assert_eq!(Settings::key_kind("server"), None);
        assert_eq!(Settings::key_kind("server.port.deep"), None);
    }

    #[test]
    fn secret_fields_are_not_settable_from_an_override() {
        assert!(!Settings::is_settable_key("database.url"));
        assert!(Settings::is_settable_key("server.port"));
        assert!(!Settings::is_settable_key("server"));
    }

    #[test]
    fn only_enabled_features_need_their_secret() {
        let settings = Settings::default();
        let keys: Vec<String> = settings
            .required_secrets()
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, vec!["database.url", "crypto.session_secret"]);
        let mut with_google = settings;
        with_google.auth.google.enabled = true;
        assert!(
            with_google
                .required_secrets()
                .iter()
                .any(|(key, _)| key == "auth.google.client_secret")
        );
    }

    #[test]
    fn the_schema_shape_matches_the_documented_leaf_types() {
        let shape = Settings::schema_shape();
        assert_eq!(shape["server"]["port"].as_integer(), Some(8080));
        assert_eq!(shape["server"]["public_port"].as_integer(), Some(443));
        assert_eq!(shape["ingest"]["respect_robots"].as_bool(), Some(true));
        assert_eq!(
            shape["knowledge_graph"]["mode"].as_str(),
            Some("embedded-rocksdb")
        );
        assert_eq!(
            shape["knowledge_graph"]["snapshot_target"].as_str(),
            Some("")
        );
        assert_eq!(shape["ingest"]["regions"][0].as_str(), Some("Trentino"));
    }
}
