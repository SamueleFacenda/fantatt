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
### Deprecated
### Removed
### Fixed
### Security