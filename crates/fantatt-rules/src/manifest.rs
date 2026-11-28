use serde::Deserialize;

use crate::error::BundleError;

/// The manifest of one bundle: its identity in the `<name>/<semver>` layout, the
/// entry points an engine may call, the artifacts it needs beside it, and the
/// evidence tying the bundle to the document that justifies its rules.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub entrypoints: Vec<String>,
    pub requires: Vec<String>,
    pub evidence: Evidence,
}

/// Where the bundle's content comes from: the specification it implements and,
/// when its rules were transcribed from an external source, that source and the
/// revision read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Evidence {
    pub source: String,
    pub commit: String,
    pub spec: String,
}

impl Manifest {
    /// Parse one manifest from its TOML text. The path names the file so a
    /// failure points at it.
    pub fn parse(path: &str, text: &str) -> Result<Self, BundleError> {
        let manifest: Self = toml::from_str(text).map_err(|reason| BundleError::Manifest {
            path: path.to_string(),
            reason: reason.to_string(),
        })?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    /// The file that carries one entry point: the engine calls
    /// `roster.validate` and the bundle answers with
    /// `rules/roster.validate.n3`.
    pub fn entrypoint_file(entrypoint: &str) -> String {
        format!("rules/{entrypoint}.n3")
    }

    /// The directory holding one entry point's golden case: its input graph, the
    /// facts it must derive, and the proof it must emit.
    pub fn golden_dir(entrypoint: &str) -> String {
        format!("tests/golden/{entrypoint}")
    }

    fn validate(&self, path: &str) -> Result<(), BundleError> {
        self.validate_identity(path)?;
        self.validate_entrypoints(path)?;
        self.validate_requirements(path)?;
        self.validate_evidence(path)
    }

    fn validate_identity(&self, path: &str) -> Result<(), BundleError> {
        if self.name.trim().is_empty() {
            return Err(self.incoherent(path, "the name is empty"));
        }
        if !plain_version(&self.version) {
            return Err(self.incoherent(
                path,
                &format!(
                    "the version {} is not three numeric components",
                    self.version
                ),
            ));
        }
        Ok(())
    }

    fn validate_entrypoints(&self, path: &str) -> Result<(), BundleError> {
        if self.entrypoints.is_empty() {
            return Err(self.incoherent(path, "no entry point is declared"));
        }
        for entrypoint in &self.entrypoints {
            if !named_pair(entrypoint) {
                return Err(self.incoherent(
                    path,
                    &format!("the entry point {entrypoint} is not <group>.<name>"),
                ));
            }
        }
        Ok(())
    }

    fn validate_requirements(&self, path: &str) -> Result<(), BundleError> {
        for requirement in &self.requires {
            let Some((artifact, version)) = requirement.split_once('@') else {
                return Err(self.incoherent(
                    path,
                    &format!("the requirement {requirement} is not <artifact>@<version>"),
                ));
            };
            if artifact.trim().is_empty() || !plain_version(version) {
                return Err(self.incoherent(
                    path,
                    &format!("the requirement {requirement} is not <artifact>@<version>"),
                ));
            }
        }
        Ok(())
    }

    fn validate_evidence(&self, path: &str) -> Result<(), BundleError> {
        let evidence = &self.evidence;
        for (field, value) in [
            ("source", evidence.source.as_str()),
            ("commit", evidence.commit.as_str()),
            ("spec", evidence.spec.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(self.incoherent(path, &format!("the evidence names no {field}")));
            }
        }
        Ok(())
    }

    fn incoherent(&self, path: &str, reason: &str) -> BundleError {
        BundleError::InvalidManifest {
            path: path.to_string(),
            reason: reason.to_string(),
        }
    }
}

/// A version the `<name>/<semver>` layout accepts: three dot-separated numeric
/// components, without a pre-release or build suffix, because the directory
/// name doubles as the version and must stay a directory name.
fn plain_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

/// A two-segment dotted name, the shape of an entry point such as
/// `roster.validate`.
fn named_pair(value: &str) -> bool {
    let segments: Vec<&str> = value.split('.').collect();
    segments.len() == 2
        && segments
            .iter()
            .all(|segment| !segment.is_empty() && !segment.contains(char::is_whitespace))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> &'static str {
        r#"name = "fantatt-core"
version = "1.0.0"
entrypoints = ["roster.validate", "scoring.matchday"]
requires = ["tt-ontology@1.2.0"]

[evidence]
source = "docs/04-reasoning-rules.md"
commit = "internal"
spec = "G1.8"
"#
    }

    #[test]
    fn a_manifest_parses_into_its_declared_shape() {
        let manifest = Manifest::parse("manifest.toml", manifest()).expect("manifest");
        assert_eq!(manifest.name, "fantatt-core");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.entrypoints.len(), 2);
        assert_eq!(manifest.requires, ["tt-ontology@1.2.0"]);
        assert_eq!(manifest.evidence.spec, "G1.8");
    }

    #[test]
    fn a_bundle_without_entrypoints_is_refused() {
        let text = manifest().replace(
            "entrypoints = [\"roster.validate\", \"scoring.matchday\"]",
            "entrypoints = []",
        );
        let error = Manifest::parse("manifest.toml", &text).expect_err("no entry points");
        assert!(
            matches!(error, BundleError::InvalidManifest { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_version_with_a_pre_release_suffix_is_refused() {
        let text = manifest().replace("version = \"1.0.0\"", "version = \"1.0.0-rc1\"");
        let error = Manifest::parse("manifest.toml", &text).expect_err("pre-release version");
        assert!(
            matches!(error, BundleError::InvalidManifest { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_requirement_without_a_version_is_refused() {
        let text = manifest().replace(
            "requires = [\"tt-ontology@1.2.0\"]",
            "requires = [\"tt-ontology\"]",
        );
        let error = Manifest::parse("manifest.toml", &text).expect_err("unversioned requirement");
        assert!(
            matches!(error, BundleError::InvalidManifest { .. }),
            "{error}"
        );
    }

    #[test]
    fn blank_evidence_is_refused() {
        let text = manifest().replace("commit = \"internal\"", "commit = \" \"");
        let error = Manifest::parse("manifest.toml", &text).expect_err("blank evidence");
        assert!(
            matches!(error, BundleError::InvalidManifest { .. }),
            "{error}"
        );
    }

    #[test]
    fn an_entry_point_names_its_rule_file_and_its_golden_case() {
        assert_eq!(
            Manifest::entrypoint_file("roster.validate"),
            "rules/roster.validate.n3"
        );
        assert_eq!(
            Manifest::golden_dir("roster.validate"),
            "tests/golden/roster.validate"
        );
    }
}
