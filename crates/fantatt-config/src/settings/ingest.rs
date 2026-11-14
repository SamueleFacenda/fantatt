use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ingest {
    pub enabled: bool,
    pub concurrency: u32,
    pub min_request_interval_ms: u64,
    pub source_base_url: String,
    pub regions: Vec<String>,
    pub discovery_interval_minutes: u32,
    pub matchday_interval_minutes: u32,
    pub classifica_interval_hours: u32,
    pub respect_robots: bool,
}

impl Ingest {
    /// A disabled pipeline may hold stale placeholders; an enabled one needs at
    /// least one worker and one region, and must throttle the source.
    pub fn is_coherent(&self) -> bool {
        if self.enabled {
            self.concurrency >= 1
                && !self.regions.is_empty()
                && self.min_request_interval_ms >= 1
                && !self.source_base_url.is_empty()
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest(enabled: bool) -> Ingest {
        Ingest {
            enabled,
            concurrency: 4,
            min_request_interval_ms: 750,
            source_base_url: "https://portale.fitet.org/".to_owned(),
            regions: vec!["Trentino".to_owned()],
            discovery_interval_minutes: 30,
            matchday_interval_minutes: 5,
            classifica_interval_hours: 24,
            respect_robots: true,
        }
    }

    #[test]
    fn an_enabled_pipeline_needs_workers_regions_and_throttling() {
        assert!(ingest(true).is_coherent());
        let mut no_workers = ingest(true);
        no_workers.concurrency = 0;
        assert!(!no_workers.is_coherent());
        let mut no_regions = ingest(true);
        no_regions.regions.clear();
        assert!(!no_regions.is_coherent());
        let mut unthrottled = ingest(true);
        unthrottled.min_request_interval_ms = 0;
        assert!(!unthrottled.is_coherent());
    }

    #[test]
    fn a_disabled_pipeline_is_always_coherent() {
        let mut off = ingest(false);
        off.concurrency = 0;
        off.regions.clear();
        assert!(off.is_coherent());
    }
}
