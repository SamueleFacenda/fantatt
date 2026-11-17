//! Shared state: the loaded settings, the database pool, and the security
//! material derived from them (rate limiters and the CSRF key).

use std::sync::Arc;

use fantatt_config::loader::Loaded;
use fantatt_config::settings::Settings;
use sqlx::PgPool;

use crate::rate_limit::{RateLimits, RateRuleError};

const SESSION_SECRET_KEY: &str = "crypto.session_secret";

#[derive(Clone)]
pub struct AppState {
    settings: Arc<Settings>,
    pool: PgPool,
    rate_limits: Arc<RateLimits>,
    csrf_key: Arc<[u8]>,
}

impl AppState {
    /// Builds the state from loaded configuration, failing fast when a rate
    /// rule is malformed or the CSRF secret is absent.
    pub fn build(loaded: &Loaded, pool: PgPool) -> Result<Self, StateBuildError> {
        let settings = loaded.settings();
        let rate_limits = RateLimits::build(&settings.rate_limit.rules)?;
        let csrf_key = csrf_key(loaded, settings)?;
        Ok(Self {
            settings: Arc::new(settings.clone()),
            pool,
            rate_limits: Arc::new(rate_limits),
            csrf_key,
        })
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn rate_limits(&self) -> &RateLimits {
        &self.rate_limits
    }

    pub fn csrf_key(&self) -> &[u8] {
        &self.csrf_key
    }
}

fn csrf_key(loaded: &Loaded, settings: &Settings) -> Result<Arc<[u8]>, StateBuildError> {
    let variable = settings.crypto.session_secret.variable_name();
    let secret =
        loaded
            .secret(SESSION_SECRET_KEY)
            .ok_or_else(|| StateBuildError::MissingCsrfSecret {
                variable: variable.to_owned(),
            })?;
    if secret.expose().trim().is_empty() {
        return Err(StateBuildError::MissingCsrfSecret {
            variable: variable.to_owned(),
        });
    }
    Ok(Arc::from(secret.expose().as_bytes()))
}

#[derive(Debug, thiserror::Error)]
pub enum StateBuildError {
    #[error("a rate limit rule is invalid: {0}")]
    InvalidRateRule(#[from] RateRuleError),
    #[error("the CSRF secret is absent; declare {variable}")]
    MissingCsrfSecret { variable: String },
}
