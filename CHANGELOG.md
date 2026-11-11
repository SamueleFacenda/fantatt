# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Nix workspace (flake-parts + crane + rust-overlay) pinning the Rust 1.99.1 toolchain.
- Nine-crate backend skeleton (`fantatt-core`, `-scoring`, `-reasoning`, `-ingest`, `-api`, `-jobs`, `-notify`, `-obs`, `-config`) with their end-state dependencies pinned.
- Monorepo directory trees for `web/`, `nix/`, `deploy/`, `tests/{e2e,nixos,conformance}`, and `rules/`.
- Cargo-based `justfile` and a CI workflow gating on `nix flake check` plus a dependency audit.
### Changed
- Documented workspace pins: `rudof` corrected to 0.1.12 (0.3.39 does not exist) and the `reqwest` 0.13 feature list to `rustls`/`h2`, matching the pinned `Cargo.toml`.
### Deprecated
### Removed
- `rudof` 0.1.12 from `fantatt-reasoning`: the release does not compile against the current releases of its `shacl_ast`/`shacl_validation`/`srdf` family (imported APIs were removed upstream), and pinning the whole family to its 2024 releases would be dependency drift. It returns with G1.9, pinned family plus a SHACL conformance test.
### Fixed
### Security
- `cargo audit` gate now runs from `.cargo/audit.toml`. Four transitive findings are upstream-blocked at the pinned direct dependencies and are recorded there with their blocking crate, accepted risk, and reopen trigger: `quick-xml` 0.37.5 via `oxigraph` 0.5.13, `rsa` 0.9.10 via `openidconnect` 4.1.0 and `rsa` 0.10.0-rc.19 via `crypto-glue` 0.2.2 (`webauthn-rs` 0.6.2), and `libsqlite3-sys` 0.30.1, which only enters `Cargo.lock` through sqlx's platform-gated sqlite driver and is absent from the x86_64-linux build graph. Excluding the deferred `rudof` family, the dependency tree carries no unresolved advisories.