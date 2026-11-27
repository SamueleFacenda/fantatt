use sha2::{Digest, Sha256};

/// The content address of one bundle: the SHA-256 of the manifest bytes followed
/// by every file the bundle carries, in sorted relative-path order.
///
/// Each contribution is prefixed with its name length and its byte length, so no
/// two different file sets feed the same byte stream: an edited rule, a renamed
/// file, and a file that joined the bundle each read as a new version rather
/// than colliding with the old one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RulesetVersion(String);

impl RulesetVersion {
    /// Hash one manifest and the files that travel with it, in the sorted order
    /// the bundle walk produces.
    pub fn of(manifest: &[u8], files: &[(&str, &[u8])]) -> Self {
        let mut hasher = Sha256::new();
        feed(&mut hasher, MANIFEST, manifest);
        for (relative, bytes) in files {
            feed(&mut hasher, relative.as_bytes(), bytes);
        }
        Self(hex(hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const MANIFEST: &[u8] = b"manifest.toml";

fn feed(hasher: &mut Sha256, name: &[u8], bytes: &[u8]) {
    hasher.update((name.len() as u64).to_be_bytes());
    hasher.update(name);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn hex(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_content_addresses_the_same_version() {
        let first = RulesetVersion::of(b"manifest", &[("rules/a.n3", b"{ ?s ?p ?o }")]);
        let second = RulesetVersion::of(b"manifest", &[("rules/a.n3", b"{ ?s ?p ?o }")]);
        assert_eq!(first, second);
        assert_eq!(first.as_str().len(), 64);
    }

    #[test]
    fn one_changed_byte_addresses_a_new_version() {
        let original = RulesetVersion::of(b"manifest", &[("rules/a.n3", b"15")]);
        let tampered = RulesetVersion::of(b"manifest", &[("rules/a.n3", b"16")]);
        assert_ne!(original, tampered);
    }

    #[test]
    fn a_renamed_file_addresses_a_new_version() {
        let original = RulesetVersion::of(b"manifest", &[("rules/a.n3", b"15")]);
        let renamed = RulesetVersion::of(b"manifest", &[("rules/b.n3", b"15")]);
        assert_ne!(original, renamed);
    }

    #[test]
    fn a_concatenation_cannot_forge_the_same_stream() {
        let split =
            RulesetVersion::of(b"manifest", &[("rules/a.n3", b"15"), ("rules/b.n3", b"16")]);
        let joined = RulesetVersion::of(b"manifest", &[("rules/a.n3", b"1516")]);
        assert_ne!(split, joined);
    }
}
