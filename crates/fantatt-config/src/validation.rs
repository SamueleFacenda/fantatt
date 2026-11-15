use thiserror::Error;

use crate::settings::Settings;

/// Cross-field and cross-section rule violation, each naming the config key it
/// belongs to so an operator can fix the offending value directly.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("app.base_url must not be empty")]
    EmptyBaseUrl,

    #[error("{key} must use https:// outside development, got '{value}'")]
    InsecureEndpoint { key: &'static str, value: String },

    #[error("server.port must be a usable port, got {port}")]
    BindPortReserved { port: u16 },

    #[error("server.public_port must be set when the service is reachable from the internet")]
    PublicPortMissing,

    #[error(
        "database.min_connections must be at least 1 and at most database.max_connections, got {min} of {max}"
    )]
    PoolShapeInvalid { min: u32, max: u32 },

    #[error(
        "session.cookie_name must start with __Host- when server.public_port is 443, got '{name}'"
    )]
    HostCookieRequired { name: String },

    #[error("session.cookie_name must not be empty")]
    CookieNameEmpty,

    #[error(
        "session.idle_seconds and session.absolute_seconds must both be positive, got {idle} and {absolute}"
    )]
    SessionLifetimesInvalid { idle: u32, absolute: u32 },

    #[error("knowledge_graph.path must be set for the {mode} mode")]
    KnowledgeGraphPathMissing { mode: String },

    #[error("ingest.concurrency must be at least 1, got {concurrency}")]
    IngestConcurrencyZero { concurrency: u32 },

    #[error("ingest.regions must name at least one region when ingest.enabled is true")]
    IngestRegionsEmpty,

    #[error("ingest.min_request_interval_ms must throttle the source, got {interval_ms}")]
    IngestUnthrottled { interval_ms: u64 },

    #[error("ingest.source_base_url must not be empty when ingest.enabled is true")]
    IngestSourceEmpty,

    #[error("csrf.header must not be empty when csrf.enabled is true")]
    CsrfHeaderEmpty,

    #[error("rate_limit.rules must not be empty")]
    RateLimitRulesEmpty,

    #[error("rules.active_bundle and rules.bundle_path must both be set")]
    RuleBundleIncomplete,

    #[error("email.from must name a sender")]
    EmailSenderEmpty,

    #[error("storage.provider 's3' needs storage.bucket, storage.endpoint, and storage.secret_key")]
    StorageIncomplete,

    #[error("observability.service_name must not be empty")]
    ServiceNameEmpty,

    #[error(
        "notifications.push_enabled needs notifications.vapid_public_key and notifications.vapid_private_key"
    )]
    PushIncomplete,

    #[error(
        "live.heartbeat_seconds must be at least 1 when live.sse_enabled is true, got {heartbeat}"
    )]
    HeartbeatZero { heartbeat: u32 },

    #[error("privacy.deletion_grace_days must be at least 30, got {days}")]
    GraceWindowTooShort { days: u32 },

    #[error(
        "auth.google.enabled needs auth.google.client_id, client_secret, scopes, and redirect_uri"
    )]
    GoogleIncomplete,

    #[error("auth.passkey.enabled needs auth.passkey.rp_id and rp_origin")]
    PasskeyIncomplete,
}

impl ValidationError {
    /// Stable machine-readable violation code, usable as an RFC 9457 `type`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyBaseUrl => "EMPTY_BASE_URL",
            Self::InsecureEndpoint { .. } => "INSECURE_ENDPOINT",
            Self::BindPortReserved { .. } => "BIND_PORT_RESERVED",
            Self::PublicPortMissing => "PUBLIC_PORT_MISSING",
            Self::PoolShapeInvalid { .. } => "POOL_SHAPE_INVALID",
            Self::HostCookieRequired { .. } => "HOST_COOKIE_REQUIRED",
            Self::CookieNameEmpty => "COOKIE_NAME_EMPTY",
            Self::SessionLifetimesInvalid { .. } => "SESSION_LIFETIMES_INVALID",
            Self::KnowledgeGraphPathMissing { .. } => "KNOWLEDGE_GRAPH_PATH_MISSING",
            violation => violation.ingest_and_facade_code(),
        }
    }

    /// why: the full table is split across two matches to keep each function
    /// inside the 20-line limit of the workspace clippy configuration.
    fn ingest_and_facade_code(&self) -> &'static str {
        match self {
            Self::IngestConcurrencyZero { .. } => "INGEST_CONCURRENCY_ZERO",
            Self::IngestRegionsEmpty => "INGEST_REGIONS_EMPTY",
            Self::IngestUnthrottled { .. } => "INGEST_UNTHROTTLED",
            Self::IngestSourceEmpty => "INGEST_SOURCE_EMPTY",
            Self::CsrfHeaderEmpty => "CSRF_HEADER_EMPTY",
            Self::RateLimitRulesEmpty => "RATE_LIMIT_RULES_EMPTY",
            Self::RuleBundleIncomplete => "RULE_BUNDLE_INCOMPLETE",
            Self::EmailSenderEmpty => "EMAIL_SENDER_EMPTY",
            Self::StorageIncomplete => "STORAGE_INCOMPLETE",
            Self::ServiceNameEmpty => "SERVICE_NAME_EMPTY",
            Self::PushIncomplete => "PUSH_INCOMPLETE",
            Self::HeartbeatZero { .. } => "HEARTBEAT_ZERO",
            Self::GraceWindowTooShort { .. } => "GRACE_WINDOW_TOO_SHORT",
            Self::GoogleIncomplete => "GOOGLE_INCOMPLETE",
            Self::PasskeyIncomplete => "PASSKEY_INCOMPLETE",
            // why: `code` handles the nine startup topology violations.
            _ => unreachable!("topology violations are coded by `code`"),
        }
    }
}

/// Collects every violation of `docs/12-configuration.md` §6, so one startup
/// abort can list all of them instead of surfacing one at a time.
pub fn collect(settings: &Settings) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    violations.extend(check_app(&settings.app));
    violations.extend(check_server(&settings.server));
    violations.extend(check_database(&settings.database));
    violations.extend(check_knowledge_graph(&settings.knowledge_graph));
    violations.extend(check_session(&settings.session, &settings.server));
    violations.extend(check_auth(&settings.auth));
    violations.extend(check_csrf(&settings.csrf));
    violations.extend(check_rate_limit(&settings.rate_limit));
    violations.extend(check_ingest(&settings.ingest));
    violations.extend(check_email(&settings.email));
    violations.extend(check_storage(&settings.storage));
    violations.extend(check_notifications(&settings.notifications));
    violations.extend(check_observability(&settings.observability, &settings.app));
    violations.extend(check_rules(&settings.rules));
    violations.extend(check_privacy(&settings.privacy));
    violations.extend(check_live(&settings.live));
    violations
}

fn check_app(app: &crate::settings::app::App) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if app.base_url.trim().is_empty() {
        violations.push(ValidationError::EmptyBaseUrl);
        return violations;
    }
    if app.is_production() && !uses_tls(&app.base_url) {
        violations.push(ValidationError::InsecureEndpoint {
            key: "app.base_url",
            value: app.base_url.clone(),
        });
    }
    violations
}

fn check_server(server: &crate::settings::server::Server) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if server.port == 0 {
        violations.push(ValidationError::BindPortReserved { port: server.port });
    }
    if server.public_port == 0 {
        violations.push(ValidationError::PublicPortMissing);
    }
    violations
}

fn check_database(database: &crate::settings::database::Database) -> Vec<ValidationError> {
    if database.pool_is_shaped_correctly() {
        Vec::new()
    } else {
        vec![ValidationError::PoolShapeInvalid {
            min: database.min_connections,
            max: database.max_connections,
        }]
    }
}

fn check_knowledge_graph(
    graph: &crate::settings::knowledge_graph::KnowledgeGraph,
) -> Vec<ValidationError> {
    if !graph.needs_local_path() || !graph.path.trim().is_empty() {
        return Vec::new();
    }
    vec![ValidationError::KnowledgeGraphPathMissing {
        mode: graph.mode.to_string(),
    }]
}

fn check_session(
    session: &crate::settings::session::Session,
    server: &crate::settings::server::Server,
) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if session.cookie_name.trim().is_empty() {
        violations.push(ValidationError::CookieNameEmpty);
    } else if !session.uses_host_prefixed_cookie() && server.serves_tls_standard_port() {
        violations.push(ValidationError::HostCookieRequired {
            name: session.cookie_name.clone(),
        });
    }
    if !session.lifetimes_are_ordered() {
        violations.push(ValidationError::SessionLifetimesInvalid {
            idle: session.idle_seconds,
            absolute: session.absolute_seconds,
        });
    }
    violations
}

fn check_auth(auth: &crate::settings::auth::Auth) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if !auth.google.is_fully_configured() {
        violations.push(ValidationError::GoogleIncomplete);
    }
    if !auth.passkey.is_fully_configured() {
        violations.push(ValidationError::PasskeyIncomplete);
    }
    violations
}

fn check_csrf(csrf: &crate::settings::csrf::Csrf) -> Vec<ValidationError> {
    if csrf.enabled && !csrf.names_a_header() {
        vec![ValidationError::CsrfHeaderEmpty]
    } else {
        Vec::new()
    }
}

fn check_rate_limit(limit: &crate::settings::rate_limit::RateLimit) -> Vec<ValidationError> {
    if limit.rules.is_empty() {
        vec![ValidationError::RateLimitRulesEmpty]
    } else {
        Vec::new()
    }
}

fn check_ingest(ingest: &crate::settings::ingest::Ingest) -> Vec<ValidationError> {
    if !ingest.enabled {
        return Vec::new();
    }
    let mut violations = throughput_violations(ingest);
    violations.extend(scrape_scope_violations(ingest));
    violations
}

/// why: both limits throttle FITET, so a slip in either is a settings error.
fn throughput_violations(ingest: &crate::settings::ingest::Ingest) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if ingest.concurrency < 1 {
        violations.push(ValidationError::IngestConcurrencyZero {
            concurrency: ingest.concurrency,
        });
    }
    if ingest.min_request_interval_ms < 1 {
        violations.push(ValidationError::IngestUnthrottled {
            interval_ms: ingest.min_request_interval_ms,
        });
    }
    violations
}

/// why: without regions and a base url there is nothing to fetch.
fn scrape_scope_violations(ingest: &crate::settings::ingest::Ingest) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if ingest.regions.is_empty() {
        violations.push(ValidationError::IngestRegionsEmpty);
    }
    if ingest.source_base_url.trim().is_empty() {
        violations.push(ValidationError::IngestSourceEmpty);
    }
    violations
}

fn check_email(email: &crate::settings::email::Email) -> Vec<ValidationError> {
    if email.names_a_sender() {
        Vec::new()
    } else {
        vec![ValidationError::EmailSenderEmpty]
    }
}

fn check_storage(storage: &crate::settings::storage::Storage) -> Vec<ValidationError> {
    if storage.is_coherent() {
        Vec::new()
    } else {
        vec![ValidationError::StorageIncomplete]
    }
}

fn check_notifications(
    notifications: &crate::settings::notifications::Notifications,
) -> Vec<ValidationError> {
    if notifications.is_coherent() {
        Vec::new()
    } else {
        vec![ValidationError::PushIncomplete]
    }
}

fn check_observability(
    observability: &crate::settings::observability::Observability,
    app: &crate::settings::app::App,
) -> Vec<ValidationError> {
    let mut violations = Vec::new();
    if observability.service_name.trim().is_empty() {
        violations.push(ValidationError::ServiceNameEmpty);
    }
    if observability.exports_telemetry()
        && app.is_production()
        && !uses_tls(&observability.otlp_endpoint)
    {
        violations.push(ValidationError::InsecureEndpoint {
            key: "observability.otlp_endpoint",
            value: observability.otlp_endpoint.clone(),
        });
    }
    violations
}

fn check_rules(rules: &crate::settings::rules::Rules) -> Vec<ValidationError> {
    if rules.names_a_bundle() {
        Vec::new()
    } else {
        vec![ValidationError::RuleBundleIncomplete]
    }
}

fn check_privacy(privacy: &crate::settings::privacy::Privacy) -> Vec<ValidationError> {
    if privacy.keeps_the_purge_promise() {
        Vec::new()
    } else {
        vec![ValidationError::GraceWindowTooShort {
            days: privacy.deletion_grace_days,
        }]
    }
}

fn check_live(live: &crate::settings::live::Live) -> Vec<ValidationError> {
    if live.has_a_live_heartbeat() {
        Vec::new()
    } else {
        vec![ValidationError::HeartbeatZero {
            heartbeat: live.heartbeat_seconds,
        }]
    }
}

fn uses_tls(endpoint: &str) -> bool {
    endpoint.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::app::DeploymentEnvironment;

    const EXPECTED_TOGGLE_CODES: [&str; 6] = [
        "CSRF_HEADER_EMPTY",
        "RATE_LIMIT_RULES_EMPTY",
        "EMAIL_SENDER_EMPTY",
        "SERVICE_NAME_EMPTY",
        "RULE_BUNDLE_INCOMPLETE",
        "HEARTBEAT_ZERO",
    ];

    fn invalid_after(edit: impl Fn(&mut Settings)) -> Vec<ValidationError> {
        let mut settings = Settings::default();
        edit(&mut settings);
        collect(&settings)
    }

    fn codes(violations: &[ValidationError]) -> Vec<&'static str> {
        violations.iter().map(ValidationError::code).collect()
    }

    fn only_code(violations: &[ValidationError]) -> &'static str {
        codes(violations).first().copied().unwrap_or("NONE")
    }

    #[test]
    fn the_compiled_in_defaults_are_valid() {
        assert!(collect(&Settings::default()).is_empty());
    }

    #[test]
    fn a_bad_bind_port_is_rejected() {
        assert_eq!(
            only_code(&invalid_after(|settings| settings.server.port = 0)),
            "BIND_PORT_RESERVED"
        );
    }

    #[test]
    fn a_missing_public_port_is_rejected() {
        assert_eq!(
            only_code(&invalid_after(|settings| settings.server.public_port = 0)),
            "PUBLIC_PORT_MISSING"
        );
    }

    #[test]
    fn a_zero_concurrency_is_rejected() {
        assert_eq!(
            invalid_after(|settings| settings.ingest.concurrency = 0),
            vec![ValidationError::IngestConcurrencyZero { concurrency: 0 }]
        );
    }

    #[test]
    fn an_unthrottled_scrape_is_rejected() {
        assert_eq!(
            invalid_after(|settings| settings.ingest.min_request_interval_ms = 0),
            vec![ValidationError::IngestUnthrottled { interval_ms: 0 }]
        );
    }

    #[test]
    fn the_cookie_rule_binds_session_and_server_together() {
        let plain_cookie_on_tls =
            invalid_after(|settings| settings.session.cookie_name = "session".to_owned());
        assert_eq!(plain_cookie_on_tls.len(), 1);
        assert!(matches!(
            plain_cookie_on_tls[0],
            ValidationError::HostCookieRequired { .. }
        ));
        assert!(invalid_after(|settings| settings.server.public_port = 8443).is_empty());
        assert!(
            invalid_after(|settings| settings.session.cookie_name = "__Host-s".to_owned())
                .is_empty()
        );
    }

    #[test]
    fn the_grace_window_must_keep_the_purge_promise() {
        assert_eq!(
            only_code(&invalid_after(|settings| settings
                .privacy
                .deletion_grace_days = 7)),
            "GRACE_WINDOW_TOO_SHORT"
        );
    }

    #[test]
    fn production_requires_tls_endpoints() {
        let violations = invalid_after(|settings| {
            settings.observability.otlp_endpoint = "http://127.0.0.1:4317".to_owned();
        });
        assert_eq!(
            violations,
            vec![ValidationError::InsecureEndpoint {
                key: "observability.otlp_endpoint",
                value: "http://127.0.0.1:4317".to_owned(),
            }]
        );
    }

    #[test]
    fn development_tolerates_a_plain_http_endpoint() {
        assert!(
            invalid_after(|settings| {
                settings.app.environment = DeploymentEnvironment::Development;
                settings.observability.otlp_endpoint = "http://127.0.0.1:4317".to_owned();
            })
            .is_empty()
        );
    }

    #[test]
    fn an_embedded_graph_needs_a_path() {
        assert_eq!(
            only_code(&invalid_after(|settings| {
                settings.knowledge_graph.path = String::new();
            })),
            "KNOWLEDGE_GRAPH_PATH_MISSING"
        );
    }

    #[test]
    fn an_inverted_pool_is_rejected() {
        assert_eq!(
            invalid_after(|settings| settings.database.min_connections = 99),
            vec![ValidationError::PoolShapeInvalid { min: 99, max: 20 }]
        );
    }

    /// why: each facade is broken by one empty field, so the table pairs that
    /// field with the violation it must raise.
    type FacadeCase = (&'static str, fn(&mut Settings));

    fn facade_cases() -> Vec<FacadeCase> {
        vec![
            ("STORAGE_INCOMPLETE", |settings: &mut Settings| {
                settings.storage.provider = crate::settings::storage::StorageProvider::S3;
                settings.storage.bucket = String::new();
            }),
            ("PUSH_INCOMPLETE", |settings: &mut Settings| {
                settings.notifications.push_enabled = true;
                settings.notifications.vapid_public_key = String::new();
            }),
            ("GOOGLE_INCOMPLETE", |settings: &mut Settings| {
                settings.auth.google.client_id = String::new();
                settings.auth.google.enabled = true;
            }),
            ("PASSKEY_INCOMPLETE", |settings: &mut Settings| {
                settings.auth.passkey.rp_id = String::new();
                settings.auth.passkey.enabled = true;
            }),
        ]
    }

    #[test]
    fn incomplete_facades_are_named() {
        for (expected, edit) in facade_cases() {
            assert_eq!(only_code(&invalid_after(edit)), expected, "{expected}");
        }
    }

    #[test]
    fn empty_toggles_are_collected_together() {
        let violations = invalid_after(|settings| {
            settings.ingest.enabled = false;
            settings.csrf.header = String::new();
            settings.rate_limit.rules.clear();
            settings.email.from = String::new();
            settings.rules.active_bundle = String::new();
            settings.rules.bundle_path = String::new();
            settings.observability.service_name = String::new();
            settings.live.heartbeat_seconds = 0;
        });

        assert_eq!(codes(&violations), EXPECTED_TOGGLE_CODES);
    }

    #[test]
    fn a_disabled_ingest_keeps_its_scrape_rules_silent() {
        assert!(
            invalid_after(|settings| {
                settings.ingest.enabled = false;
                settings.ingest.concurrency = 0;
                settings.ingest.regions.clear();
            })
            .is_empty()
        );
    }
}
