# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Local gate suite in the `justfile`: rustfmt check, clippy with complexity limits (functions ≤20 lines and ≤3 arguments via `clippy.toml`, `unsafe_code` denied workspace-wide), workspace tests, a comment-lead lint, and a gitleaks secret scan over staged changes. Bare `just` runs the whole suite.
- Git hooks managed by lefthook (from the pinned nixpkgs input): comment lint and secret scan on pre-commit, Conventional Commits validation on commit-msg, installed with `just install-hooks`.
- CI split into three jobs: PR gates (gate suite, full-history secret scan, commit-range validation), dependency audit, and artifact build on pushes to `main`.
- `CONTRIBUTING.md`: development setup, gate reference, commit format, and the branch and merge protocol.
- Nix workspace (flake-parts + crane + rust-overlay) pinning the Rust 1.99.1 toolchain.
- Nine-crate backend skeleton (`fantatt-core`, `-scoring`, `-reasoning`, `-ingest`, `-api`, `-jobs`, `-notify`, `-obs`, `-config`) with their end-state dependencies pinned. `rudof` defers to G1.9: its 0.1.12 release does not compile against the current releases of its `shacl_ast`/`srdf` family.
- Monorepo directory trees for `web/`, `nix/`, `deploy/`, `tests/{e2e,nixos,conformance}`, and `rules/`.
- Cargo-based `justfile` and a CI workflow gating on `nix flake check` plus a dependency audit.
- Domain core in `fantatt-core`: prefixed ULID identifiers, division tiers (`B2`…`D2`), ISO 4217 money, ratings with the provisional 2000-point default, and the roster calendar with the week-one zero-points rule (INV-8).
- Typed domain error taxonomy covering INV-1…INV-15: every violation carries a stable SCREAMING_SNAKE code the API layer mirrors into RFC 9457 problem+json types.
- Aggregates with their invariants enforced by constructors and mutations: `Squad` (fifteen-slot composition, tier matching, uniqueness, lock/unlock, same-tier bench substitution), `League` (lifecycle plus exclusive and shared ownership claims), `Membership` (budget spending), `Player`/`PlayerIdentity` (one rating per effective date, resolution states, audited merges), `Matchday` (completion only when every tie is decided), `PlayerAppearance` (singles/doubles shape, per-set scores), `Lot`/`Offer` (highest-and-open-window bidding), `DerivedScore` (rule version and derivation record required), `PurgeRequest` (thirty-day GDPR window with tombstoned managers), and `IdempotencyKey`.
- Property tests for roster generation, budget sequences, exclusive claims, matchday completion sets, bid ladders, idempotency keys, rating uniqueness, and the purge window.
- `proptest` 1.12.0 as a workspace dev-dependency (with the row in `docs/dependency-watchdog.md`), `serde_json` for wire-format tests, and the `serde` feature of the `time` workspace dependency.
- `cargo-llvm-cov` in the dev shell and a `just coverage` recipe for the line-coverage gate.
### Changed
- Documented workspace pins: `rudof` corrected to 0.1.12 (0.3.39 does not exist) and the `reqwest` 0.13 feature list to `rustls`/`h2`, matching the pinned `Cargo.toml`.
### Deprecated
### Removed
- `rudof` 0.1.12 from `fantatt-reasoning`: the release does not compile against the current releases of its `shacl_ast`/`shacl_validation`/`srdf` family (imported APIs were removed upstream), and pinning the whole family to its 2024 releases would be dependency drift. It returns with G1.9, pinned family plus a SHACL conformance test.
### Fixed
### Security
- `cargo audit` gate now runs from `.cargo/audit.toml`. Four transitive findings are upstream-blocked at the pinned direct dependencies and are recorded there with their blocking crate, accepted risk, and reopen trigger: `quick-xml` 0.37.5 via `oxigraph` 0.5.13, `rsa` 0.9.10 via `openidconnect` 4.1.0 and `rsa` 0.10.0-rc.19 via `crypto-glue` 0.2.2 (`webauthn-rs` 0.6.2), and `libsqlite3-sys` 0.30.1, which only enters `Cargo.lock` through sqlx's platform-gated sqlite driver and is absent from the x86_64-linux build graph. Excluding the deferred `rudof` family, the dependency tree carries no unresolved advisories.