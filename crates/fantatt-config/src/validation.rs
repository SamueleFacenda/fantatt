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
    violations.extend(check_observability(&settings.observability));
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
) -> Vec<ValidationError> {
    if !observability.service_name.trim().is_empty() {
        Vec::new()
    } else {
        vec![ValidationError::ServiceNameEmpty]
    }
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
