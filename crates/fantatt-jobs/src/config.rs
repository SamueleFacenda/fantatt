//! The runtime the scheduler wires from configuration: fetch politeness, the
//! discovery seed, and the sweep cadence (docs/12-configuration.md §ingest).
//!
//! The `[ingest]` section carries the knobs an operator tunes; every other
//! politeness bound stays at the G1.2 defaults, which the validation of the
//! facade keeps coherent.

use std::time::Duration;

use fantatt_config::settings::ingest::Ingest;
use fantatt_ingest::uri::{Uri, UriError};
use fantatt_ingest::{FetchSettings, discover};

/// What one scheduler process runs with.
pub struct Runtime {
    pub settings: FetchSettings,
    pub seed: Uri,
    pub cadence: Duration,
    /// How long a walked page settles its cache window before a sweep
    /// re-reads it (docs/06-data-pipeline.md §3.1).
    pub ttl: Duration,
}

/// Why the configuration cannot drive a scheduler.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("the ingest settings are incoherent for an enabled pipeline")]
    Incoherent,

    #[error("the source base URL cannot seed discovery: {0}")]
    Seed(#[from] UriError),
}

impl Runtime {
    /// Maps the `[ingest]` section onto the fetch facade and discovery.
    pub fn from_config(ingest: &Ingest) -> Result<Self, RuntimeError> {
        if ingest.enabled && !ingest.is_coherent() {
            return Err(RuntimeError::Incoherent);
        }
        let settings = FetchSettings {
            concurrency: ingest.concurrency as usize,
            min_interval_ms: ingest.min_request_interval_ms,
            source_base_url: ingest.source_base_url.clone(),
            ..FetchSettings::default()
        };
        Ok(Self {
            settings,
            seed: discover::seed(&ingest.source_base_url)?,
            cadence: Duration::from_secs(u64::from(ingest.discovery_interval_minutes) * 60),
            ttl: discover::DEFAULT_TTL,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest() -> Ingest {
        Ingest {
            enabled: true,
            concurrency: 2,
            min_request_interval_ms: 900,
            source_base_url: "https://portale.fitet.org/".to_owned(),
            regions: vec!["Valle d'Aosta".to_owned()],
            discovery_interval_minutes: 15,
            matchday_interval_minutes: 5,
            classifica_interval_hours: 24,
            respect_robots: true,
        }
    }

    #[test]
    fn the_config_maps_onto_the_fetch_settings() {
        let runtime = Runtime::from_config(&ingest()).expect("runtime");
        assert_eq!(runtime.settings.concurrency, 2);
        assert_eq!(runtime.settings.min_interval_ms, 900);
        assert_eq!(
            runtime.settings.source_base_url,
            "https://portale.fitet.org/".to_owned()
        );
        assert_eq!(runtime.cadence, Duration::from_secs(15 * 60));
        assert_eq!(
            runtime.seed.as_url(),
            "https://portale.fitet.org/menu.php".to_owned()
        );
    }

    #[test]
    fn an_incoherent_enabled_pipeline_is_rejected() {
        let mut broken = ingest();
        broken.concurrency = 0;
        assert!(matches!(
            Runtime::from_config(&broken),
            Err(RuntimeError::Incoherent)
        ));
    }

    #[test]
    fn a_base_url_without_a_trailing_slash_still_seeds_the_menu() {
        let mut trimmed = ingest();
        trimmed.source_base_url = "https://portale.fitet.org".to_owned();
        let runtime = Runtime::from_config(&trimmed).expect("runtime");
        assert_eq!(runtime.seed.as_url(), "https://portale.fitet.org/menu.php");
    }

    #[test]
    fn a_disabled_pipeline_runs_disabled() {
        let mut off = ingest();
        off.enabled = false;
        off.concurrency = 0;
        assert!(Runtime::from_config(&off).is_ok());
    }
}
