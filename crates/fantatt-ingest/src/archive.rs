//! Raw-response archive, keyed by content hash.
//!
//! Every fetched body is stored so a past result can be re-parsed without
//! re-scraping (docs/06-data-pipeline.md §3.3). The filesystem is the default
//! store; an S3-compatible store arrives when the storage settings select it,
//! behind the same trait.

use std::fmt;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use sha2::{Digest, Sha256};

/// A stored body could not be kept or read back.
#[derive(Debug, thiserror::Error)]
#[error("archive rejected {key}: {reason}")]
pub struct ArchiveError {
    pub key: String,
    pub reason: String,
}

/// The content key of a body: its sha256, lowercase hex.
pub fn content_key(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut key = String::with_capacity(64);
    // why: the digest output type carries no LowerHex implementation, so the
    // hex encoding is explicit
    for byte in digest {
        write!(key, "{byte:02x}").expect("hex write into a String");
    }
    key
}

/// The seam every archive goes through.
#[async_trait]
pub trait Archive: Send + Sync {
    async fn store(&self, key: &str, bytes: &[u8]) -> Result<(), ArchiveError>;
    async fn load(&self, key: &str) -> Result<Vec<u8>, ArchiveError>;
}

/// A filesystem archive: two-hex-prefix fan-out, written atomically so an
/// interrupted store never truncates an existing object.
pub struct FilesystemArchive {
    root: PathBuf,
}

impl FilesystemArchive {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.root.join(&key[..2]).join(&key[2..])
    }

    /// Writes through a partial name and renames, so an interrupted store never
    /// leaves a truncated object under the real key.
    async fn write_atomic(&self, target: &Path, bytes: &[u8]) -> Result<(), String> {
        let parent = target.parent().map(Path::to_path_buf).unwrap_or_default();
        tokio::fs::create_dir_all(&parent)
            .await
            .map_err(|reason| reason.to_string())?;
        let partial = target.with_extension("part");
        tokio::fs::write(&partial, bytes)
            .await
            .map_err(|reason| reason.to_string())?;
        tokio::fs::rename(&partial, target)
            .await
            .map_err(|reason| reason.to_string())
    }
}

#[async_trait]
impl Archive for FilesystemArchive {
    async fn store(&self, key: &str, bytes: &[u8]) -> Result<(), ArchiveError> {
        if !is_key(key) {
            return Err(ArchiveError {
                key: key.to_owned(),
                reason: "the key must be 64 lowercase hex characters".to_owned(),
            });
        }
        self.write_atomic(&self.path_for(key), bytes)
            .await
            .map_err(|reason| ArchiveError {
                key: key.to_owned(),
                reason,
            })
    }

    /// Reads a stored body back; a missing object fails with the key named.
    async fn load(&self, key: &str) -> Result<Vec<u8>, ArchiveError> {
        let target = self.path_for(key);
        tokio::fs::read(&target)
            .await
            .map_err(|reason| ArchiveError {
                key: key.to_owned(),
                reason: reason.to_string(),
            })
    }
}

/// A key is 64 lowercase hex characters, the shape `content_key` emits.
pub fn is_key(key: &str) -> bool {
    key.len() == 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl From<ArchiveError> for crate::error::FetchError {
    fn from(error: ArchiveError) -> Self {
        crate::error::FetchError::Archive {
            key: error.key,
            reason: error.reason,
        }
    }
}

/// An in-memory archive for unit tests.
#[cfg(test)]
pub mod memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    pub struct MemoryArchive {
        objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    impl MemoryArchive {
        pub fn keys(&self) -> Vec<String> {
            self.objects
                .lock()
                .expect("objects")
                .keys()
                .cloned()
                .collect()
        }
    }

    #[async_trait]
    impl Archive for MemoryArchive {
        async fn store(&self, key: &str, bytes: &[u8]) -> Result<(), ArchiveError> {
            self.objects
                .lock()
                .expect("objects")
                .insert(key.to_owned(), bytes.to_vec());
            Ok(())
        }

        async fn load(&self, key: &str) -> Result<Vec<u8>, ArchiveError> {
            self.objects
                .lock()
                .expect("objects")
                .get(key)
                .cloned()
                .ok_or_else(|| ArchiveError {
                    key: key.to_owned(),
                    reason: "no object under this key".to_owned(),
                })
        }
    }
}

impl fmt::Debug for FilesystemArchive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FilesystemArchive")
            .field("root", &self.root)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fantatt-archive-{name}-{}", std::process::id()))
    }

    #[tokio::test]
    async fn a_body_round_trips_through_the_filesystem() {
        let root = scratch("round-trip");
        let archive = FilesystemArchive::new(&root);
        let key = content_key(b"<html>giornata</html>");
        archive
            .store(&key, b"<html>giornata</html>")
            .await
            .expect("store");
        let loaded = archive.load(&key).await.expect("load");
        assert_eq!(loaded, b"<html>giornata</html>");
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn storing_the_same_body_twice_is_idempotent() {
        let root = scratch("idempotent");
        let archive = FilesystemArchive::new(&root);
        let key = content_key(b"same bytes");
        archive.store(&key, b"same bytes").await.expect("first");
        archive.store(&key, b"same bytes").await.expect("second");
        assert_eq!(archive.load(&key).await.expect("load"), b"same bytes");
        let objects: Vec<_> = walkdir(&root);
        assert_eq!(objects.len(), 1);
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn different_bodies_get_different_keys() {
        assert_ne!(content_key(b"one"), content_key(b"two"));
        assert!(is_key(&content_key(b"one")));
        assert!(!is_key("One"));
        assert!(!is_key("short"));
    }

    #[tokio::test]
    async fn an_unknown_key_fails_with_the_key_named() {
        let root = scratch("missing");
        let archive = FilesystemArchive::new(&root);
        let error = archive
            .load(&content_key(b"never stored"))
            .await
            .expect_err("missing");
        assert!(error.to_string().contains(&content_key(b"never stored")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn a_key_that_is_not_hex_is_rejected_before_touching_disk() {
        let root = scratch("bad-key");
        let archive = FilesystemArchive::new(&root);
        let error = archive
            .store("not hex", b"body")
            .await
            .expect_err("rejected");
        assert!(error.to_string().contains("not hex"), "{error}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn the_memory_archive_serves_unit_tests() {
        let archive = memory::MemoryArchive::default();
        let key = content_key(b"in memory");
        archive.store(&key, b"in memory").await.expect("store");
        assert_eq!(archive.load(&key).await.expect("load"), b"in memory");
        assert!(archive.load(&content_key(b"absent")).await.is_err());
        assert_eq!(archive.keys(), vec![key]);
    }

    fn walkdir(root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }
}
