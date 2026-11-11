# Tests

Integration suites that live outside the Rust crates.

| Path | Purpose |
|---|---|
| `e2e/` | Playwright browser flows (built in Phase 3). |
| `nixos/` | NixOS VM integration tests (`nixosTest`). |
| `conformance/` | API contract tests and rule-bundle conformance suites. |

Unit tests live with their code inside `crates/*`.
