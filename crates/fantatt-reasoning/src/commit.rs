use sha2::{Digest, Sha256};
use time::{Date, OffsetDateTime};

/// The provenance of one assertion: which URL and document hash it read, which
/// importer and ontology revision carried it, when it was asserted, and from
/// which day the facts apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub source_url: String,
    pub document_hash: String,
    pub actor: String,
    pub import_version: String,
    pub asserted_at: OffsetDateTime,
    pub valid_from: Option<Date>,
}

/// The content address of one assertion commit: the graph name and every
/// downstream citation name this commit, so re-asserting identical content
/// lands in the same graph instead of forking history.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitId(String);

impl CommitId {
    pub fn hash_of(bytes: &[u8]) -> Self {
        Self(hex(Sha256::digest(bytes)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn hash(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

fn hex(digest: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    digest
        .as_ref()
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[(byte >> 4) as usize] as char,
                DIGITS[(byte & 0x0f) as usize] as char,
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Month;

    fn day(day: u8) -> Date {
        Date::from_calendar_date(2026, Month::November, day).expect("valid date")
    }

    fn commit() -> Commit {
        Commit {
            source_url:
                "https://portale.fitet.org/risultati/campionati/giornata.php?INCONTRO=370793&CAM=1"
                    .to_owned(),
            document_hash: hash(b"<html>referto</html>"),
            actor: "fitet-ingest".to_owned(),
            import_version: "0.1.0".to_owned(),
            asserted_at: day(8).with_hms(20, 41, 3).expect("instant").assume_utc(),
            valid_from: Some(day(26)),
        }
    }

    #[test]
    fn identical_content_hashes_to_the_same_commit_id() {
        assert_eq!(CommitId::hash_of(b"same"), CommitId::hash_of(b"same"));
        assert_ne!(CommitId::hash_of(b"same"), CommitId::hash_of(b"other"));
    }

    #[test]
    fn a_commit_carries_its_provenance() {
        let commit = commit();
        assert_eq!(commit.actor, "fitet-ingest");
        assert_eq!(commit.import_version, "0.1.0");
        assert_eq!(commit.valid_from, Some(day(26)));
        assert_eq!(commit.document_hash.len(), 64);
    }
}
