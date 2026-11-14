use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeGraphMode {
    EmbeddedRocksdb,
    EmbeddedMemory,
    Service,
}

impl std::fmt::Display for KnowledgeGraphMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::EmbeddedRocksdb => "embedded-rocksdb",
            Self::EmbeddedMemory => "embedded-memory",
            Self::Service => "service",
        };
        f.write_str(name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub mode: KnowledgeGraphMode,
    pub path: String,
    pub snapshot_interval_seconds: u32,
    pub snapshot_target: String,
}

impl KnowledgeGraph {
    pub fn stores_on_disk(&self) -> bool {
        self.mode == KnowledgeGraphMode::EmbeddedRocksdb
    }

    pub fn needs_local_path(&self) -> bool {
        self.mode != KnowledgeGraphMode::Service
    }

    pub fn snapshots_remotely(&self) -> bool {
        !self.snapshot_target.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(mode: KnowledgeGraphMode, snapshot_target: &str) -> KnowledgeGraph {
        KnowledgeGraph {
            mode,
            path: "/var/lib/fantatt/kg".to_owned(),
            snapshot_interval_seconds: 300,
            snapshot_target: snapshot_target.to_owned(),
        }
    }

    #[test]
    fn mode_variants_use_kebab_case() {
        assert_eq!(
            serde_json::to_string(&KnowledgeGraphMode::EmbeddedRocksdb).unwrap(),
            "\"embedded-rocksdb\""
        );
        assert_eq!(
            serde_json::to_string(&KnowledgeGraphMode::Service).unwrap(),
            "\"service\""
        );
    }

    #[test]
    fn only_rocksdb_holds_a_local_path() {
        assert!(graph(KnowledgeGraphMode::EmbeddedRocksdb, "").stores_on_disk());
        assert!(!graph(KnowledgeGraphMode::EmbeddedMemory, "").stores_on_disk());
        assert!(graph(KnowledgeGraphMode::EmbeddedMemory, "").needs_local_path());
        assert!(!graph(KnowledgeGraphMode::Service, "").needs_local_path());
    }

    #[test]
    fn a_blank_snapshot_target_means_local_snapshots_only() {
        assert!(!graph(KnowledgeGraphMode::EmbeddedRocksdb, "").snapshots_remotely());
        assert!(graph(KnowledgeGraphMode::EmbeddedRocksdb, "s3://bucket/kg").snapshots_remotely());
    }
}
