//! The loader's contract: a bundle is addressed by its content, an edited file
//! reads as a new version, and an incomplete bundle refuses to load instead of
//! shipping an unaddressed ruleset.

use std::path::{Path, PathBuf};

use fantatt_rules::{Bundle, BundleError, BundleSet};

const ENTRYPOINTS: &[&str] = &[
    "roster.validate",
    "scoring.matchday",
    "auction.bid",
    "settle.standings",
];
const MANIFEST: &str = r#"name = "fantatt-core"
version = "1.0.0"
entrypoints = ["roster.validate", "scoring.matchday", "auction.bid", "settle.standings"]
requires = ["tt-ontology@1.2.0"]

[evidence]
source = "docs/04-reasoning-rules.md"
commit = "internal"
spec = "G1.8"
"#;

fn manifest_for(name: &str, version: &str) -> String {
    MANIFEST
        .replace("fantatt-core", name)
        .replace("\"1.0.0\"", &format!("\"{version}\""))
}

fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("fantatt-rules-{name}-{}", std::process::id()))
}

fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, bytes).expect("file");
}

fn full_bundle(root: &Path) -> PathBuf {
    seed_bundle(root, "fantatt-core", "1.0.0")
}

/// Write a complete bundle under `<root>/<name>/<version>/`.
fn seed_bundle(root: &Path, name: &str, version: &str) -> PathBuf {
    let bundle = root.join(name).join(version);
    write(
        &bundle,
        "manifest.toml",
        manifest_for(name, version).as_bytes(),
    );
    write(
        &bundle,
        "ontology/tt.ttl",
        b"@prefix tt: <https://fantatt.org/ontology/tt#> .\n",
    );
    for entrypoint in ENTRYPOINTS {
        write(
            &bundle,
            &format!("rules/{entrypoint}.n3"),
            b"{ ?s ?p ?o }\n",
        );
        for name in ["input.ttl", "expected.ttl", "expected.proof"] {
            write(
                &bundle,
                &format!("tests/golden/{entrypoint}/{name}"),
                b"placeholder\n",
            );
        }
    }
    bundle
}

#[test]
fn a_bundle_addresses_by_its_content() {
    let root = scratch("stable");
    let bundle = full_bundle(&root);
    let first = Bundle::load(&bundle).expect("first load");
    let second = Bundle::load(&bundle).expect("second load");
    assert_eq!(first.ruleset_version(), second.ruleset_version());
    assert_eq!(first.ruleset_version().as_str().len(), 64);
    assert_eq!(first.name(), "fantatt-core");
    assert_eq!(first.version(), "1.0.0");
    assert_eq!(first.manifest().entrypoints, ENTRYPOINTS.to_vec());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_edited_rule_file_addresses_a_new_version() {
    let root = scratch("tamper");
    let bundle = full_bundle(&root);
    let before = Bundle::load(&bundle).expect("load");
    std::fs::write(bundle.join("rules/roster.validate.n3"), b"{ ?s ?p ?o2 }\n").expect("tamper");
    let after = Bundle::load(&bundle).expect("reload");
    assert_ne!(before.ruleset_version(), after.ruleset_version());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_bundle_without_an_entrypoint_file_refuses_to_load() {
    let root = scratch("entrypoint");
    let bundle = full_bundle(&root);
    std::fs::remove_file(bundle.join("rules/scoring.matchday.n3")).expect("remove");
    let error = Bundle::load(&bundle).expect_err("missing entry point");
    assert!(
        matches!(error, BundleError::MissingEntrypoint { .. }),
        "{error}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_bundle_without_a_golden_case_refuses_to_load() {
    let root = scratch("golden");
    let bundle = full_bundle(&root);
    std::fs::remove_file(bundle.join("tests/golden/auction.bid/expected.proof")).expect("remove");
    let error = Bundle::load(&bundle).expect_err("missing golden case");
    assert!(
        matches!(error, BundleError::MissingGolden { .. }),
        "{error}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_file_the_layout_does_not_declare_refuses_to_load() {
    let root = scratch("stray");
    let bundle = full_bundle(&root);
    write(&bundle, "notes.txt", b"not declared\n");
    let error = Bundle::load(&bundle).expect_err("stray file");
    assert!(matches!(error, BundleError::Unlisted { .. }), "{error}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_manifest_that_disagrees_with_its_directory_refuses_to_load() {
    let root = scratch("identity");
    let bundle = full_bundle(&root);
    let manifest = MANIFEST.replace("version = \"1.0.0\"", "version = \"1.1.0\"");
    write(&bundle, "manifest.toml", manifest.as_bytes());
    let error = Bundle::load(&bundle).expect_err("disagreeing manifest");
    assert!(
        matches!(error, BundleError::InvalidManifest { .. }),
        "{error}"
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_bundle_set_verifies_every_bundle_it_discovers() {
    let root = scratch("set");
    full_bundle(&root);
    seed_bundle(&root, "fantatt-legacy", "2.3.4");
    let report = BundleSet::new(&root).verify().expect("verify");
    assert_eq!(report.bundles, 2);
    assert_eq!(report.entrypoints, 8);
    assert_eq!(report.versions.len(), 2);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn the_in_repo_rules_directory_discovers() {
    let set = BundleSet::discover().expect("rules directory");
    assert!(set.root().is_dir(), "{}", set.root().display());
    let bundles = set.bundles().expect("bundles");
    for bundle in &bundles {
        assert!(!bundle.files().is_empty(), "{}", bundle.name());
    }
}

/// The committed `fantatt-core` bundle, recorded so an edit to a rule file, an
/// ontology file, or a golden expectation fails the suite instead of shipping a
/// ruleset version nobody reviewed.
const COMMITTED_RULESET: &str = "e561028ae5f9496c668bb279be4248281a1137d2223048d85d4f1efcfac1ce57";

#[test]
fn the_committed_bundle_addresses_the_version_it_records() {
    let set = BundleSet::discover().expect("rules directory");
    let bundle = set
        .bundles()
        .expect("bundles")
        .into_iter()
        .next()
        .expect("a committed bundle");
    assert_eq!(bundle.ruleset_version().as_str(), COMMITTED_RULESET);
}

#[test]
fn every_graph_in_the_committed_bundles_parses_as_turtle() {
    use oxigraph::io::{RdfFormat, RdfParser};

    let set = BundleSet::discover().expect("rules directory");
    for bundle in set.bundles().expect("bundles") {
        for file in bundle.files() {
            if file.relative.ends_with(".ttl") {
                let text = String::from_utf8_lossy(&file.bytes);
                let parsed: Result<Vec<_>, _> = RdfParser::from_format(RdfFormat::Turtle)
                    .for_reader(text.as_bytes())
                    .collect();
                let quads = parsed.expect("graph parses as turtle");
                assert!(!quads.is_empty(), "{} holds no triples", file.relative);
            }
        }
    }
}
