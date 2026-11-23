//! Recorded FITET golden fixtures for the ingest pipeline (G1.1).
//!
//! The fixture tree lives in `tests/fixtures/fitet/`: the legacy match corpus
//! recovered from the prototype plus the HTML surfaces recorded on 2026-11-20
//! (see `tests/fixtures/fitet/README.md`). Tests read the set through this
//! crate, so no test touches the network or an absolute path.
#![forbid(unsafe_code)]

pub mod capture;
pub mod error;
pub mod records;

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use capture::CaptureIndex;
pub use error::FixtureError;
use records::RawMatch;
use sha2::{Digest, Sha256};

const FIXTURE_DIR: &str = "tests/fixtures/fitet";
const ENV_OVERRIDE: &str = "FANTATT_FIXTURES_DIR";
const MANIFEST: &str = "manifest.sha256";
const CAPTURE_INDEX: &str = "capture.json";
const README: &str = "README.md";
const MAX_ANCESTORS: usize = 8;

/// One canonical match array held by the set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSet {
    pub file: String,
    pub records: Vec<RawMatch>,
}

/// A versioned, hash-verified fixture set.
#[derive(Debug, Clone)]
pub struct FixtureSet {
    root: PathBuf,
}

impl FixtureSet {
    /// Build a set rooted at a directory; tests point this at a scratch tree.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Locate the in-repo fixture set, or the tree named by `FANTATT_FIXTURES_DIR`.
    pub fn discover() -> Result<Self, FixtureError> {
        if let Ok(dir) = std::env::var(ENV_OVERRIDE) {
            return Ok(Self::new(dir));
        }
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for _ in 0..MAX_ANCESTORS {
            let candidate = dir.join(FIXTURE_DIR);
            if candidate.is_dir() {
                return Ok(Self::new(candidate));
            }
            if !dir.pop() {
                break;
            }
        }
        Err(FixtureError::Missing {
            path: FIXTURE_DIR.to_string(),
        })
    }

    /// The set's root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a manifest-relative path inside the set.
    pub fn file_path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// The capture index describing every recorded surface.
    pub fn capture_index(&self) -> Result<CaptureIndex, FixtureError> {
        let path = self.file_path(CAPTURE_INDEX);
        let text = std::fs::read_to_string(&path).map_err(|reason| FixtureError::Io {
            path: display_path(&path),
            reason: reason.to_string(),
        })?;
        serde_json::from_str(&text).map_err(|reason| FixtureError::Parse {
            path: CAPTURE_INDEX.to_string(),
            reason: reason.to_string(),
        })
    }

    /// The canonical match arrays held by the set.
    pub fn match_sets(&self) -> Result<Vec<MatchSet>, FixtureError> {
        let index = self.capture_index()?;
        let mut sets = Vec::new();
        for surface in index.match_sets() {
            let path = self.file_path(&surface.file);
            sets.push(records::read_match_set(&path, &surface.file)?);
        }
        Ok(sets)
    }

    /// Every recorded match across the set's canonical arrays.
    pub fn legacy_matches(&self) -> Result<Vec<RawMatch>, FixtureError> {
        Ok(self
            .match_sets()?
            .into_iter()
            .flat_map(|set| set.records)
            .collect())
    }

    /// The recorded HTML and JSON pages, in capture-index order.
    pub fn html_surfaces(&self) -> Result<Vec<capture::Surface>, FixtureError> {
        Ok(self.capture_index()?.html_surfaces().cloned().collect())
    }

    /// Hash every file against the manifest and validate every match array.
    pub fn verify(&self) -> Result<Verification, FixtureError> {
        let manifest = self.read_manifest()?;
        let files = self.unlisted(&manifest)?;
        let hashed = hash_files(&files, &manifest)?;
        self.check_targets(&manifest)?;
        self.check_surface_files()?;
        Ok(Verification {
            entries: manifest.len(),
            files: hashed,
            matches: self.legacy_matches()?.len(),
            surfaces: self.html_surfaces()?.len(),
        })
    }

    fn read_manifest(&self) -> Result<BTreeMap<String, String>, FixtureError> {
        let path = self.file_path(MANIFEST);
        let text = std::fs::read_to_string(&path).map_err(|reason| FixtureError::Io {
            path: display_path(&path),
            reason: reason.to_string(),
        })?;
        let mut entries = BTreeMap::new();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let (file, hash) = parse_manifest_line(line)?;
            entries.insert(file, hash);
        }
        Ok(entries)
    }

    /// Tree files the manifest does not list; a stray edit or addition is fatal.
    fn unlisted(
        &self,
        manifest: &BTreeMap<String, String>,
    ) -> Result<Vec<(String, PathBuf)>, FixtureError> {
        let files = walk_files(&self.root, &[MANIFEST, README])?;
        for (relative, _) in &files {
            if !manifest.contains_key(relative) {
                return Err(FixtureError::Unlisted {
                    path: relative.clone(),
                });
            }
        }
        Ok(files)
    }

    /// Every manifest entry must resolve to a real file.
    fn check_targets(&self, manifest: &BTreeMap<String, String>) -> Result<(), FixtureError> {
        for relative in manifest.keys() {
            if !self.file_path(relative).is_file() {
                return Err(FixtureError::MissingFile {
                    path: relative.clone(),
                });
            }
        }
        Ok(())
    }

    /// Every surface the capture index declares must exist on disk.
    fn check_surface_files(&self) -> Result<(), FixtureError> {
        for surface in &self.capture_index()?.surfaces {
            if !self.file_path(&surface.file).is_file() {
                return Err(FixtureError::MissingFile {
                    path: surface.file.clone(),
                });
            }
        }
        Ok(())
    }
}

/// What `FixtureSet::verify` counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verification {
    pub entries: usize,
    pub files: usize,
    pub matches: usize,
    pub surfaces: usize,
}

impl fmt::Display for Verification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} files hashed against {} manifest entries, {} match records, {} html surfaces",
            self.files, self.entries, self.matches, self.surfaces
        )
    }
}

fn parse_manifest_line(line: &str) -> Result<(String, String), FixtureError> {
    let mut fields = line.split_whitespace();
    match (fields.next(), fields.next(), fields.next()) {
        (Some("sha256"), Some(hash), Some(file)) => Ok((file.to_string(), hash.to_string())),
        _ => Err(FixtureError::Manifest {
            path: MANIFEST.to_string(),
            entry: line.to_string(),
        }),
    }
}

fn walk_files(root: &Path, excluded: &[&str]) -> Result<Vec<(String, PathBuf)>, FixtureError> {
    let walker = Walker { root, excluded };
    let mut files = Vec::new();
    walker.visit(root, &mut files)?;
    files.sort();
    Ok(files)
}

struct Walker<'a> {
    root: &'a Path,
    excluded: &'a [&'a str],
}

impl Walker<'_> {
    fn visit(&self, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<(), FixtureError> {
        let entries = std::fs::read_dir(dir).map_err(|reason| FixtureError::Io {
            path: display_path(dir),
            reason: reason.to_string(),
        })?;
        for entry in entries {
            let entry = entry.map_err(|reason| FixtureError::Io {
                path: display_path(dir),
                reason: reason.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() {
                self.visit(&path, files)?;
            } else if self.listed(&path) {
                files.push(self.entry(path)?);
            }
        }
        Ok(())
    }

    fn listed(&self, path: &Path) -> bool {
        !self
            .excluded
            .iter()
            .any(|name| path.file_name().is_some_and(|file| file == *name))
    }

    fn entry(&self, path: PathBuf) -> Result<(String, PathBuf), FixtureError> {
        let relative = path
            .strip_prefix(self.root)
            .map_err(|reason| FixtureError::Io {
                path: display_path(&path),
                reason: reason.to_string(),
            })?;
        Ok((relative.to_string_lossy().replace('\\', "/"), path))
    }
}

fn hash_files(
    files: &[(String, PathBuf)],
    manifest: &BTreeMap<String, String>,
) -> Result<usize, FixtureError> {
    for (relative, path) in files {
        let bytes = std::fs::read(path).map_err(|reason| FixtureError::Io {
            path: relative.clone(),
            reason: reason.to_string(),
        })?;
        if sha256_hex(&bytes) != manifest[relative] {
            return Err(FixtureError::Hash {
                path: relative.clone(),
                reason: "digest differs from the manifest line".to_string(),
            });
        }
    }
    Ok(files.len())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

// why: the digest output type carries no LowerHex implementation, so the hex
// encoding is explicit
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut hex = String::with_capacity(64);
    for byte in hasher.finalize() {
        write!(hex, "{byte:02x}").expect("hex write into a String");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_tree_verifies() {
        let set = FixtureSet::discover().expect("fixture tree");
        let report = set.verify().expect("verification");
        assert_eq!(report.matches, 1085);
        assert_eq!(report.surfaces, 16);
        assert_eq!(report.files, report.entries);
    }

    #[test]
    fn legacy_corpus_round_trips() {
        let set = FixtureSet::discover().expect("fixture tree");
        let matches = set.legacy_matches().expect("legacy matches");
        assert_eq!(matches.len(), 1085);
        // why: the corpus keeps both surface types of the prototype
        assert!(matches.iter().any(|m| m.source.source_type == "partita"));
        assert!(matches.iter().any(|m| m.source.source_type == "torneo"));
        let text = serde_json::to_string(&matches).expect("serialize");
        let reparsed: Vec<RawMatch> = serde_json::from_str(&text).expect("parse");
        assert_eq!(matches, reparsed);
    }

    #[test]
    fn every_recorded_surface_exists_on_disk() {
        let set = FixtureSet::discover().expect("fixture tree");
        let surfaces = set.html_surfaces().expect("surfaces");
        assert!(!surfaces.is_empty());
        for surface in &surfaces {
            assert!(set.file_path(&surface.file).is_file(), "{}", surface.file);
        }
    }

    #[test]
    fn match_detail_page_cross_references_the_searched_athlete() {
        // why: the ajax search surface (G1.5) resolves the same athlete id
        // that the match detail page names
        let set = FixtureSet::discover().expect("fixture tree");
        let detail = std::fs::read_to_string(set.file_path("html/dettaglio_partita_1594362.html"))
            .expect("detail");
        let search =
            std::fs::read_to_string(set.file_path("html/ajax_kozul.json")).expect("search");
        assert!(detail.contains("737436"));
        assert!(search.contains("737436"));
        assert!(detail.contains("PUPPO"));
    }

    #[test]
    fn a_json_object_is_not_a_match_array() {
        let root = scratch("object");
        std::fs::create_dir_all(&root).expect("scratch dir");
        std::fs::write(root.join(CAPTURE_INDEX), ONE_MATCH_INDEX).expect("index");
        std::fs::write(root.join("matches.json"), "{\"one\": \"a\"}").expect("object");
        let set = FixtureSet::new(&root);
        let error = set.match_sets().expect_err("object array");
        assert!(
            matches!(error, FixtureError::UnsupportedFile { .. }),
            "{error}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_tampered_file_fails_the_manifest() {
        let root = scratch("tamper");
        let source = FixtureSet::discover().expect("fixture tree");
        seed_manifest_tree(&root, &source);
        let mut bytes = std::fs::read(root.join("matches.json")).expect("read");
        bytes.push(b' ');
        std::fs::write(root.join("matches.json"), bytes).expect("write");
        let error = FixtureSet::new(&root)
            .verify()
            .expect_err("tampered digest");
        assert!(matches!(error, FixtureError::Hash { .. }), "{error}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_stray_file_fails_the_manifest() {
        let root = scratch("stray");
        let source = FixtureSet::discover().expect("fixture tree");
        seed_manifest_tree(&root, &source);
        std::fs::write(root.join("stray.txt"), "not listed").expect("stray");
        let error = FixtureSet::new(&root).verify().expect_err("unlisted file");
        assert!(matches!(error, FixtureError::Unlisted { .. }), "{error}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_seeded_tree_verifies() {
        let root = scratch("seeded");
        let source = FixtureSet::discover().expect("fixture tree");
        seed_manifest_tree(&root, &source);
        let report = FixtureSet::new(&root).verify().expect("seeded verify");
        assert_eq!(report.files, 2);
        assert_eq!(report.matches, 1085);
        std::fs::remove_dir_all(&root).ok();
    }

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fantatt-fixtures-{name}-{}", std::process::id()))
    }

    fn seed_manifest_tree(root: &Path, set: &FixtureSet) {
        std::fs::create_dir_all(root).expect("scratch dir");
        std::fs::copy(set.file_path("matches.json"), root.join("matches.json")).expect("copy");
        std::fs::write(root.join(CAPTURE_INDEX), ONE_MATCH_INDEX).expect("index");
        let mut manifest = String::new();
        for file in [CAPTURE_INDEX, "matches.json"] {
            let bytes = std::fs::read(root.join(file)).expect("read");
            manifest.push_str(&format!("sha256 {} {file}\n", sha256_hex(&bytes)));
        }
        std::fs::write(root.join(MANIFEST), manifest).expect("manifest");
    }

    const ONE_MATCH_INDEX: &str = r#"{"version":1,"captured_on":"2026-11-20","surfaces":[{"kind":"legacy-matches","file":"matches.json","url":"legacy:444d843"}]}"#;
}
