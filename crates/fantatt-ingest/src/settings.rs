//! Fetch-facade settings: politeness bounds, breaker thresholds, and the seed
//! of the host allow-list.
//!
//! The defaults encode the portal constraints measured by the prototype
//! (docs/06-data-pipeline.md §2): at most four concurrent requests worldwide
//! and at least 750 ms between two requests to the same host, because more
//! than ~20 concurrent requests take the site down.

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::policy::HostPolicy;

/// The client identifies itself; the browser User-Agent of the fixture capture
/// remains the documented override when the portal rejects this one.
pub const DEFAULT_USER_AGENT: &str =
    "fantatt-ingest/0.1.0 (+https://github.com/SamueleFacenda/fantatt)";

/// Ceiling for the global semaphore; the portal survives bursts but not floods.
const MAX_CONCURRENCY: usize = 16;

/// Ceiling for the attempts of one fetch.
const MAX_ATTEMPTS: u32 = 10;

/// How the facade bounds every request it sends.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FetchSettings {
    pub concurrency: usize,
    pub min_interval_ms: u64,
    pub jitter_ms: u64,
    pub max_attempts: u32,
    pub backoff_base_ms: u64,
    pub backoff_cap_ms: u64,
    pub breaker_threshold: u32,
    pub breaker_reset_ms: u64,
    pub timeout_ms: u64,
    pub max_redirects: usize,
    pub source_base_url: String,
    pub allowed_hosts: Vec<String>,
    pub user_agent: String,
    pub archive_path: Option<PathBuf>,
}

impl Default for FetchSettings {
    fn default() -> Self {
        Self {
            concurrency: 4,
            min_interval_ms: 750,
            jitter_ms: 250,
            max_attempts: 4,
            backoff_base_ms: 250,
            backoff_cap_ms: 5_000,
            breaker_threshold: 5,
            breaker_reset_ms: 30_000,
            timeout_ms: 15_000,
            max_redirects: 5,
            source_base_url: "https://portale.fitet.org/".to_owned(),
            allowed_hosts: Vec::new(),
            user_agent: DEFAULT_USER_AGENT.to_owned(),
            archive_path: None,
        }
    }
}

impl FetchSettings {
    pub fn interval(&self) -> Duration {
        Duration::from_millis(self.min_interval_ms)
    }

    pub fn jitter(&self) -> Duration {
        Duration::from_millis(self.jitter_ms)
    }

    pub fn backoff_base(&self) -> Duration {
        Duration::from_millis(self.backoff_base_ms)
    }

    pub fn backoff_cap(&self) -> Duration {
        Duration::from_millis(self.backoff_cap_ms)
    }

    pub fn breaker_reset(&self) -> Duration {
        Duration::from_millis(self.breaker_reset_ms)
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// Rejects combinations that would flood, hang, or misidentify the client.
    pub fn validate(&self) -> Result<(), SettingsViolation> {
        self.validate_bounds()?;
        self.validate_identity()
    }

    fn validate_bounds(&self) -> Result<(), SettingsViolation> {
        if !(1..=MAX_CONCURRENCY).contains(&self.concurrency) {
            return Err(SettingsViolation::Concurrency {
                value: self.concurrency,
            });
        }
        if self.min_interval_ms == 0 {
            return Err(SettingsViolation::Interval {
                value: self.min_interval_ms,
            });
        }
        if !(1..=MAX_ATTEMPTS).contains(&self.max_attempts) {
            return Err(SettingsViolation::Attempts {
                value: self.max_attempts,
            });
        }
        if self.backoff_cap_ms < self.backoff_base_ms {
            return Err(SettingsViolation::Backoff);
        }
        Ok(())
    }

    fn validate_identity(&self) -> Result<(), SettingsViolation> {
        self.validate_breaker()?;
        if self.user_agent.trim().is_empty() {
            return Err(SettingsViolation::UserAgent);
        }
        if reqwest::Url::parse(&self.source_base_url)
            .ok()
            .filter(|url| url.host_str().is_some())
            .is_none()
        {
            return Err(SettingsViolation::BaseUrl {
                value: self.source_base_url.clone(),
            });
        }
        Ok(())
    }

    fn validate_breaker(&self) -> Result<(), SettingsViolation> {
        if self.breaker_threshold == 0 {
            return Err(SettingsViolation::Threshold {
                value: self.breaker_threshold,
            });
        }
        if self.breaker_reset_ms == 0 {
            return Err(SettingsViolation::Reset {
                value: self.breaker_reset_ms,
            });
        }
        if self.timeout_ms == 0 {
            return Err(SettingsViolation::Timeout {
                value: self.timeout_ms,
            });
        }
        Ok(())
    }

    /// The allow-list seeded by the source base URL plus the extra hosts.
    pub fn host_policy(&self) -> Result<HostPolicy, SettingsViolation> {
        let base = reqwest::Url::parse(&self.source_base_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .ok_or_else(|| SettingsViolation::BaseUrl {
                value: self.source_base_url.clone(),
            })?;
        let mut hosts = vec![base];
        hosts.extend(self.allowed_hosts.iter().cloned());
        Ok(HostPolicy::new(hosts))
    }
}

/// A settings combination the facade refuses to run with.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SettingsViolation {
    #[error("concurrency must be between 1 and {MAX_CONCURRENCY}, got {value}")]
    Concurrency { value: usize },

    #[error("min_interval_ms must be at least 1, got {value}")]
    Interval { value: u64 },

    #[error("max_attempts must be between 1 and {MAX_ATTEMPTS}, got {value}")]
    Attempts { value: u32 },

    #[error("backoff_cap_ms must be at least backoff_base_ms")]
    Backoff,

    #[error("breaker_threshold must be at least 1, got {value}")]
    Threshold { value: u32 },

    #[error("breaker_reset_ms must be at least 1, got {value}")]
    Reset { value: u64 },

    #[error("timeout_ms must be at least 1, got {value}")]
    Timeout { value: u64 },

    #[error("user_agent must name the client")]
    UserAgent,

    #[error("source_base_url must be an absolute URL with a host, got {value:?}")]
    BaseUrl { value: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_coherent() {
        assert!(FetchSettings::default().validate().is_ok());
    }

    #[test]
    fn a_flooding_concurrency_is_rejected() {
        let settings = FetchSettings {
            concurrency: 0,
            ..FetchSettings::default()
        };
        assert_eq!(
            settings.validate(),
            Err(SettingsViolation::Concurrency { value: 0 })
        );
        let settings = FetchSettings {
            concurrency: MAX_CONCURRENCY + 1,
            ..FetchSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn an_unthrottled_pipeline_is_rejected() {
        let settings = FetchSettings {
            min_interval_ms: 0,
            ..FetchSettings::default()
        };
        assert_eq!(
            settings.validate(),
            Err(SettingsViolation::Interval { value: 0 })
        );
    }

    #[test]
    fn an_inverted_backoff_window_is_rejected() {
        let settings = FetchSettings {
            backoff_base_ms: 10_000,
            backoff_cap_ms: 1_000,
            ..FetchSettings::default()
        };
        assert_eq!(settings.validate(), Err(SettingsViolation::Backoff));
    }

    #[test]
    fn an_empty_user_agent_is_rejected() {
        let settings = FetchSettings {
            user_agent: "  ".to_owned(),
            ..FetchSettings::default()
        };
        assert_eq!(settings.validate(), Err(SettingsViolation::UserAgent));
    }

    #[test]
    fn a_hostless_base_url_is_rejected() {
        let settings = FetchSettings {
            source_base_url: "portale.fitet.org".to_owned(),
            ..FetchSettings::default()
        };
        assert_eq!(
            settings.validate(),
            Err(SettingsViolation::BaseUrl {
                value: "portale.fitet.org".to_owned()
            })
        );
    }

    #[test]
    fn the_host_policy_seeds_the_base_host_and_the_extras() {
        let settings = FetchSettings {
            allowed_hosts: vec!["127.0.0.1".to_owned()],
            ..FetchSettings::default()
        };
        let policy = settings.host_policy().expect("policy");
        assert!(policy.allows("portale.fitet.org"));
        assert!(policy.allows("127.0.0.1"));
        assert!(!policy.allows("example.org"));
    }

    #[test]
    fn a_partial_document_fills_from_the_defaults() {
        let settings: FetchSettings =
            serde_json::from_str(r#"{"concurrency": 2}"#).expect("settings");
        assert_eq!(settings.concurrency, 2);
        assert_eq!(settings.min_interval_ms, 750);
        assert_eq!(settings.user_agent, DEFAULT_USER_AGENT);
        assert_eq!(settings.archive_path, None);
    }
}
