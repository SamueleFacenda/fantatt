# Contributing

## Purpose

This document is the entry point for anyone changing the Fantatt codebase: how to set up, the gates every change must pass, and the conventions those gates enforce.

## Development setup

1. Install Nix with flakes enabled.
2. Enter the development shell:

   ```sh
   nix develop
   ```

3. Install the git hooks (once per clone, inside the dev shell):

   ```sh
   just install-hooks
   ```

The dev shell provides the whole toolchain from the pinned nixpkgs input: Rust 1.99.1, just, cargo-nextest, cargo-audit, clang/bindgen for native dependencies, lefthook, gitleaks, git, and ripgrep.

## Gates

`just` with no arguments runs the full local gate suite. CI runs the same suite on every pull request.

| Gate | Recipe | What it enforces |
| --- | --- | --- |
| Format | `just fmt-check` | rustfmt is a no-op |
| Lint | `just lint` | clippy with zero warnings |
| Tests | `just test` | the whole workspace test suite |
| Comment lint | `just comment-lint` | no `what`/`how`/`obvious`/`redundant`/`mumbling`/`noise` comment leads |
| Secret scan | `just secrets` | gitleaks over staged changes |
| Audit | `just security` | `cargo audit` (needs network) |
| Commit format | `just commit-lint` | Conventional Commits headers |

Git hooks (lefthook) run the cheap deterministic checks — comment lint, staged secret scan, commit-format validation — on every commit. Never bypass them with `--no-verify`; if a hook fails, fix the cause.

## Commit format

[Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): subject`, where type is one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`, and the subject is at most 100 characters. Goal IDs appear in the scope or subject (for example `feat(G2.5): ...`), as `docs/18-execution-roadmap.md` requires.

## Code conventions

- Clean-code metrics are enforced by clippy: functions take at most 3 arguments and span at most 20 lines (`clippy.toml`), functions nested at most 4 levels, and no `unsafe` anywhere (`[workspace.lints]`).
- Comments explain why, never what. Allowed tags: `why`, `algorithm`, `todo`, `fixme`, `hack`, `note`, `legal`, `license`.
- `fantatt-core` stays pure: no HTTP, database, clock, or async runtime imports.
- Every behavior change ships its tests, its docs (current state only), and a CHANGELOG.md entry under `Unreleased`.

## Branch and merge protocol

Feature branches per goal (`feat/g0.2-...`), conventional commits, atomic commits. A branch merges when the full gate suite is green and the Definition of Done for its goal card is satisfied. `main` never takes direct pushes.

## Testing

The strategy — unit, integration, contract, E2E, VM, performance, and security levels — lives in `docs/15-testing.md`. Rule conformance runs from `rules/` once the rule bundles exist (`just rules`, from G1.8).
