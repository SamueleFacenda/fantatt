use std::fs::DirEntry;
use std::path::{Path, PathBuf};

use crate::error::BundleError;
use crate::manifest::Manifest;
use crate::ruleset::RulesetVersion;

const RULES_DIR: &str = "rules";
const MANIFEST_FILE: &str = "manifest.toml";
const README_FILE: &str = "README.md";
const ONTOLOGY_DIR: &str = "ontology";
const QUERIES_DIR: &str = "queries";
const TESTS_DIR: &str = "tests";
const INPUT_GRAPH: &str = "input.ttl";
const EXPECTED_FACTS: &str = "expected.ttl";
const EXPECTED_PROOF: &str = "expected.proof";
const ENV_OVERRIDE: &str = "FANTATT_RULES_DIR";
const MAX_ANCESTORS: usize = 8;

/// What a bundle root may hold beside its manifest and its four sections.
const ROOT_ENTRIES: &[&str] = &[
    MANIFEST_FILE,
    README_FILE,
    ONTOLOGY_DIR,
    RULES_DIR,
    QUERIES_DIR,
    TESTS_DIR,
];

/// The sections the layout declares, in the order the walk reads them.
const SECTIONS: &[&str] = &[ONTOLOGY_DIR, RULES_DIR, QUERIES_DIR, TESTS_DIR];

/// One file a bundle carries: its path relative to the bundle root and its
/// bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFile {
    pub relative: String,
    pub bytes: Vec<u8>,
}

/// One loaded bundle: the manifest, every file it carries, and the content
/// address they hash to.
#[derive(Debug, Clone)]
pub struct Bundle {
    name: String,
    version: String,
    manifest: Manifest,
    files: Vec<BundleFile>,
    ruleset_version: RulesetVersion,
}

impl Bundle {
    /// Load the bundle rooted at one directory.
    ///
    /// The load fails when the manifest disagrees with its directory, an entry
    /// point has no rule file or no golden case, or a file the layout does not
    /// declare rides along, so an unintended edit cannot reach a ruleset version
    /// silently.
    pub fn load(root: impl Into<PathBuf>) -> Result<Self, BundleError> {
        let root = root.into();
        Ok(Loader::new(&root)?.finish())
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The content address of the manifest and every file the bundle carries.
    pub fn ruleset_version(&self) -> &RulesetVersion {
        &self.ruleset_version
    }

    /// Every file the digest covers, in sorted relative-path order.
    pub fn files(&self) -> &[BundleFile] {
        &self.files
    }
}

/// Loads one bundle: the manifest states the identity, the path must agree with
/// it, the walk collects the files, and the entry points must each name a rule
/// file and a golden case before the content is addressed.
struct Loader {
    manifest: Manifest,
    manifest_bytes: Vec<u8>,
    files: Vec<BundleFile>,
}

impl Loader {
    fn new(root: &Path) -> Result<Self, BundleError> {
        let (manifest, manifest_bytes) = read_manifest(root)?;
        let identity = bundle_identity(root)?;
        agree(root, &identity, &manifest)?;
        let files = walk(root)?;
        let loader = Self {
            manifest,
            manifest_bytes,
            files,
        };
        loader.check_rule_files()?;
        loader.check_golden_cases()?;
        Ok(loader)
    }

    fn check_rule_files(&self) -> Result<(), BundleError> {
        for entrypoint in &self.manifest.entrypoints {
            let file = Manifest::entrypoint_file(entrypoint);
            if !self.carries(&file) {
                return Err(BundleError::MissingEntrypoint {
                    bundle: self.name().to_string(),
                    entrypoint: entrypoint.clone(),
                    file,
                });
            }
        }
        Ok(())
    }

    fn check_golden_cases(&self) -> Result<(), BundleError> {
        for entrypoint in &self.manifest.entrypoints {
            for name in [INPUT_GRAPH, EXPECTED_FACTS, EXPECTED_PROOF] {
                let file = format!("{}/{name}", Manifest::golden_dir(entrypoint));
                if !self.carries(&file) {
                    return Err(BundleError::MissingGolden {
                        bundle: self.name().to_string(),
                        entrypoint: entrypoint.clone(),
                        file,
                    });
                }
            }
        }
        Ok(())
    }

    fn carries(&self, relative: &str) -> bool {
        self.files.iter().any(|file| file.relative == relative)
    }

    fn name(&self) -> String {
        self.manifest.name.clone()
    }

    fn finish(self) -> Bundle {
        let hashed: Vec<(&str, &[u8])> = self
            .files
            .iter()
            .map(|file| (file.relative.as_str(), file.bytes.as_slice()))
            .collect();
        Bundle {
            name: self.manifest.name.clone(),
            version: self.manifest.version.clone(),
            ruleset_version: RulesetVersion::of(&self.manifest_bytes, &hashed),
            manifest: self.manifest,
            files: self.files,
        }
    }
}

/// The identity the path states: a bundle lives at `rules/<name>/<semver>/`.
fn bundle_identity(root: &Path) -> Result<(String, String), BundleError> {
    let version = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let name = root
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    match (name, version) {
        (Some(name), Some(version)) => Ok((name, version)),
        _ => Err(BundleError::Layout {
            path: display(root, ""),
        }),
    }
}

/// A citation names the path, so the manifest must claim the identity its
/// directory holds: a manifest that says another version would address content
/// the path does not carry.
fn agree(root: &Path, identity: &(String, String), manifest: &Manifest) -> Result<(), BundleError> {
    let (name, version) = identity;
    if manifest.name != *name || manifest.version != *version {
        return Err(BundleError::InvalidManifest {
            path: display(root, MANIFEST_FILE),
            reason: format!(
                "the directory is {name}/{version} and the manifest says {}/{}",
                manifest.name, manifest.version
            ),
        });
    }
    Ok(())
}

fn read_manifest(root: &Path) -> Result<(Manifest, Vec<u8>), BundleError> {
    let path = root.join(MANIFEST_FILE);
    if !path.is_file() {
        return Err(BundleError::MissingManifest {
            path: display(root, MANIFEST_FILE),
        });
    }
    let bytes = read_bytes(&path)?;
    let text = String::from_utf8_lossy(&bytes);
    let manifest = Manifest::parse(&display(root, MANIFEST_FILE), &text)?;
    Ok((manifest, bytes))
}

/// Every file the bundle carries except its manifest, sorted by relative path,
/// with every root entry the layout does not declare rejected.
fn walk(root: &Path) -> Result<Vec<BundleFile>, BundleError> {
    for entry in read_dir(root)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !ROOT_ENTRIES.contains(&name.as_str()) {
            return Err(BundleError::Unlisted {
                bundle: display(root, ""),
                path: name,
            });
        }
    }
    let mut files = Vec::new();
    for section in SECTIONS {
        collect(&root.join(section), root, &mut files)?;
    }
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(files)
}

fn collect(dir: &Path, root: &Path, files: &mut Vec<BundleFile>) -> Result<(), BundleError> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in read_dir(dir)? {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, root, files)?;
        } else {
            files.push(read_file(&path, root)?);
        }
    }
    Ok(())
}

fn read_file(path: &Path, root: &Path) -> Result<BundleFile, BundleError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|reason| BundleError::Io {
            path: display(root, ""),
            reason: reason.to_string(),
        })?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(BundleFile {
        bytes: read_bytes(path)?,
        relative,
    })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, BundleError> {
    std::fs::read(path).map_err(|reason| io(path, reason))
}

fn read_dir(dir: &Path) -> Result<Vec<DirEntry>, BundleError> {
    let read = std::fs::read_dir(dir).map_err(|reason| io(dir, reason))?;
    read.collect::<Result<Vec<DirEntry>, _>>()
        .map_err(|reason| io(dir, reason))
}

fn io(path: &Path, reason: std::io::Error) -> BundleError {
    BundleError::Io {
        path: display(path, ""),
        reason: reason.to_string(),
    }
}

/// Render a path for an error message; an empty name means the path itself.
fn display(path: &Path, relative: &str) -> String {
    if relative.is_empty() {
        path.display().to_string()
    } else {
        path.join(relative).display().to_string()
    }
}

/// Every bundle under one rules directory, discovered by walking
/// `<name>/<semver>/` pairs.
#[derive(Debug, Clone)]
pub struct BundleSet {
    root: PathBuf,
}

impl BundleSet {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Locate the in-repo rules directory, or the tree `FANTATT_RULES_DIR`
    /// names.
    pub fn discover() -> Result<Self, BundleError> {
        if let Ok(dir) = std::env::var(ENV_OVERRIDE) {
            return Ok(Self::new(dir));
        }
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for _ in 0..MAX_ANCESTORS {
            let candidate = dir.join(RULES_DIR);
            if candidate.is_dir() {
                return Ok(Self::new(candidate));
            }
            if !dir.pop() {
                break;
            }
        }
        Err(BundleError::Io {
            path: RULES_DIR.to_string(),
            reason: "no rules directory above the crate".to_string(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every bundle the directory holds, ordered by name then version.
    pub fn bundles(&self) -> Result<Vec<Bundle>, BundleError> {
        let mut bundles = Vec::new();
        for name in child_dirs(&self.root)? {
            for version in child_dirs(&self.root.join(&name))? {
                bundles.push(Bundle::load(self.root.join(name.clone()).join(version))?);
            }
        }
        bundles.sort_by(|left, right| {
            (left.name(), left.version()).cmp(&(right.name(), right.version()))
        });
        Ok(bundles)
    }

    /// Load every bundle, so a malformed one fails the runner instead of
    /// shipping an unaddressed ruleset.
    pub fn verify(&self) -> Result<Verification, BundleError> {
        let bundles = self.bundles()?;
        Ok(Verification {
            bundles: bundles.len(),
            entrypoints: bundles
                .iter()
                .map(|bundle| bundle.manifest().entrypoints.len())
                .sum(),
            versions: bundles
                .iter()
                .map(|bundle| bundle.ruleset_version().as_str().to_string())
                .collect(),
        })
    }
}

fn child_dirs(dir: &Path) -> Result<Vec<String>, BundleError> {
    let mut names = Vec::new();
    for entry in read_dir(dir)? {
        let path = entry.path();
        if path.is_dir() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Ok(names)
}

/// What `BundleSet::verify` counted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    pub bundles: usize,
    pub entrypoints: usize,
    pub versions: Vec<String>,
}

impl std::fmt::Display for Verification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} bundle(s), {} entry point(s), ruleset version(s) {}",
            self.bundles,
            self.entrypoints,
            self.versions.join(", ")
        )
    }
}
