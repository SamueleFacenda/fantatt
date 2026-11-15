use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageProvider {
    S3,
    Filesystem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Storage {
    pub provider: StorageProvider,
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
    pub secret_key: SecretRef,
}

impl Storage {
    /// The S3 facade needs a bucket and an endpoint; the signing key is only
    /// checked for a value at startup when the facade is selected.
    pub fn is_coherent(&self) -> bool {
        self.provider != StorageProvider::S3
            || (!self.bucket.trim().is_empty() && !self.endpoint.trim().is_empty())
    }

    pub fn is_object_store(&self) -> bool {
        self.provider == StorageProvider::S3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage(is_s3: bool, bucket: &str, endpoint: &str) -> Storage {
        Storage {
            bucket: bucket.to_owned(),
            endpoint: endpoint.to_owned(),
            region: "us-east-1".to_owned(),
            secret_key: SecretRef::try_from("env:FANTATT__STORAGE__SECRET_KEY".to_owned()).unwrap(),
            provider: if is_s3 {
                StorageProvider::S3
            } else {
                StorageProvider::Filesystem
            },
        }
    }

    #[test]
    fn provider_variants_use_kebab_case() {
        assert_eq!(
            serde_json::to_string(&StorageProvider::S3).unwrap(),
            "\"s3\""
        );
        assert_eq!(
            serde_json::to_string(&StorageProvider::Filesystem).unwrap(),
            "\"filesystem\""
        );
    }

    #[test]
    fn s3_needs_bucket_and_endpoint() {
        assert!(storage(true, "fantatt", "https://s3.example.org").is_coherent());
        assert!(!storage(true, "", "https://s3.example.org").is_coherent());
        assert!(!storage(true, "fantatt", "  ").is_coherent());
    }

    #[test]
    fn filesystem_needs_no_object_store_settings() {
        let local = storage(false, "", "");
        assert!(local.is_coherent());
        assert!(!local.is_object_store());
    }
}
