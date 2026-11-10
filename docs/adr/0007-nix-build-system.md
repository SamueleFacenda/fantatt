# ADR-007: flake-parts + crane for reproducible builds

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** crane 0.24.0 (2026-08-21), flake-parts (2026-11-01), nix-unit (2026-10-23), rust-overlay (2026-11-10), Nix 2.34.9

## Context

Requirement: fully reproducible builds, `nix develop` gives a working
environment, and NixOS modules for deployment. The previous prototype already
used Nix but with `flake-utils`, which the NixOS wiki no longer recommends, and
with per-language `nix/*.nix` packages that had to be hand-maintained.

## Decision

- `flake-parts` as the flake framework (typed, with `flake.modules` for
  shareable NixOS modules).
- `crane` for Rust builds (incremental, caches cargo dependencies).
- `rust-overlay` for toolchains (replaces the unmaintained nixpkgs-mozilla).
- `nix-unit` for pure-Nix tests, wired into `nix flake check`.
- `buildNpmPackage` with `importNpmLock` for the frontend (no separate hash to
  maintain).
- `direnv` + `nix-direnv` for shell activation UX.

## Alternatives

| Option | Why rejected |
| --- | --- |
| `flake-utils` | NixOS wiki: "using flake-utils is not recommended; use Flake Parts instead". |
| `naersk` | Alive, but crane is the documented standard in 2026. |
| `dream2nix` | The v1 API is still a draft; v0.x works but the moving target is unjustified here. |
| `yarn2nix` | Last release 2021. |
| `nixos-generators` | Retired: upstreamed as `nixos-rebuild build-image` / `system.build.images`. |
| `devenv` | Feature-rich but an extra tool dependency. Optional; only adopted if managed dev-services are wanted. |

## Consequences

- `nix develop` provides the complete toolchain including browsers, OTel
  tooling, and test binaries.
- `nix flake check --all-systems` is the master gate: build, clippy, audit,
  tests, NixOS VM integration, and nix-unit.
- Builds are hash-verified from the lockfiles, which is the strongest practical
  form of supply-chain control available in this stack.
- Requires Nix ≥ 2.31.2 for crane 0.24.0. Pinned.
