//! The ledger snapshot discovery diffs against: what the last sweep saw of
//! every URL it walked. The rows come from `fantatt.ingest_ledger`; the
//! scheduler owns the writes, this module owns the read model.

use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use crate::conditional::CacheValidators;
use crate::uri::Uri;

/// What the last fetch of one URL left behind.
#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub content_hash: String,
    pub validators: CacheValidators,
    pub state: LedgerState,
    pub fetched_at: SystemTime,
}

impl LedgerEntry {
    /// True when the cached entry is older than the TTL and the sweep must
    /// re-fetch the page to re-read its links.
    pub fn is_stale(&self, now: SystemTime, ttl: Duration) -> bool {
        now.duration_since(self.fetched_at)
            .map_or(true, |age| age > ttl)
    }
}

/// What the last fetch decided about the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerState {
    Fetched,
    Skipped,
    Incomplete,
}

/// The ingest ledger as a lookup keyed by normalized URL.
#[derive(Debug, Clone, Default)]
pub struct Ledger {
    entries: BTreeMap<String, LedgerEntry>,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one entry under its normalized URL key.
    pub fn insert(&mut self, url: &Uri, entry: LedgerEntry) {
        self.entries.insert(url.as_url(), entry);
    }

    /// The entry the last sweep left for one URL.
    pub fn get(&self, url: &Uri) -> Option<&LedgerEntry> {
        self.entries.get(&url.as_url())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(age: Duration) -> LedgerEntry {
        LedgerEntry {
            content_hash: "hash".to_owned(),
            validators: CacheValidators::default(),
            state: LedgerState::Fetched,
            fetched_at: SystemTime::now() - age,
        }
    }

    #[test]
    fn a_fresh_entry_is_not_stale() {
        let now = SystemTime::now();
        assert!(!entry(Duration::from_secs(60)).is_stale(now, Duration::from_secs(3600)));
    }

    #[test]
    fn an_old_entry_is_stale() {
        let now = SystemTime::now();
        assert!(entry(Duration::from_secs(7200)).is_stale(now, Duration::from_secs(3600)));
    }

    #[test]
    fn the_key_is_the_normalized_url() {
        let mut ledger = Ledger::new();
        let one = Uri::parse("https://portale.fitet.org/campionati.php?CAM=1&CONC=0").expect("url");
        let other =
            Uri::parse("https://portale.fitet.org/campionati.php?CONC=0&CAM=1").expect("url");
        ledger.insert(&one, entry(Duration::ZERO));
        assert!(ledger.get(&other).is_some());
    }
}
