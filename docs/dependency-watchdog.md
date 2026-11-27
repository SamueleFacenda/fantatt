# Dependency Watchdog

Purpose: every direct dependency in the Rust workspace carries a row below.
`scripts/watchdog-check` (run by `just watchdog` and the CI docs job) fails when
a direct dependency lacks a row, when a row disagrees with `Cargo.lock`, or when
a newer upstream release has stayed unreviewed for more than 90 days, so version
drift cannot accumulate silently.

Rows were collected from the crates.io API (crate metadata, versions, owners,
downloads) on 2026-11-19. `scripts/watchdog-check` re-derives the direct
dependency list from `crates/*/Cargo.toml` and the resolved versions from
`Cargo.lock`, so a missing or stale row fails the build rather than going
unnoticed.

## Table

| Crate | Version | Pinned on | Last upstream release | Owners | Trend | Review due |
| --- | --- | --- | --- | --- | --- | --- |
| proptest | 1.12.0 | 2026-11-12 | 2026-10-10 | 3 (2 maintainers + `proptest-rs:publish` team) | 242.9M total, ~2.99M weekly downloads; 6 releases in the last 15 months | 2027-02-10 |
| rand | 0.9.5 | 2026-11-17 | 2026-11-15 (0.10.4) | 2 (1 user + `rust-random:maintainers` team) | 2206.5M total downloads; 0.10.4 is a newer major line deliberately not adopted — only the 0.9 series already resolved in the lockfile is used | 2027-02-15 |
| hmac | 0.13.0 | 2026-11-17 | 2026-03-29 (0.13.0) | 2 (1 user + `rustcrypto:MACs` team) | 766.3M total downloads; last release 2026-03-29 | 2027-02-15 |
| sha2 | 0.11.1 | 2026-11-17 | 2026-10-12 (0.11.1) | 3 (2 users + `rustcrypto:hashes` team) | 1246.6M total downloads; last release 2026-10-12 | 2027-02-15 |
| async-trait | 0.1.92 | 2026-11-17 | 2026-08-08 (0.1.92, ~3 months before pinning) | 1 (dtolnay) | 831.7M total, 217.6M recent downloads; single owner, no team; 1.3 KiB pinned crate | 2027-02-15 |
| lettre | 0.11.24 | 2026-11-19 | 2026-11-17 (0.11.24) | 2 (1 user + `github:lettre:release` team) | 24.9M total downloads; 0.11.24 is a security patch closing six SMTP advisories, previous release 0.11.23 on 2026-08-03 | 2027-02-17 |
| scraper | 0.29.0 | 2026-11-23 | 2026-10-21 (0.29.0) | 3 (causal-agent, adamreichold, cfvescovo) | 37.8M total, 11.2M recent downloads (~4.0M monthly); used in 2,188 crates; 39 releases | 2027-02-21 |
| axum | 0.8.9 | 2026-11-11 | 2026-04-14 (0.8.9) | 4 (2 users + `tokio-rs:core`, `tokio-rs:axum-release` teams) | 605.9M total, 171.0M recent downloads; HTTP surface of the whole API | 2027-02-09 |
| config | 0.14.1 | 2026-11-11 | 2026-09-30 (0.15.27) | 5 (3 users + `rust-cli:maintainers` team) | 131.5M total, 23.7M recent downloads; the 0.15 line is newer and not adopted | 2027-02-09 |
| governor | 0.10.4 | 2026-11-11 | 2025-12-16 (0.10.4) | 1 (antifuchs) | 96.2M total, 24.3M recent downloads; last release 2025-12-16 | 2027-02-09 |
| oauth2 | 5.0.0 | 2026-11-11 | 2025-01-21 (5.0.0) | 1 (ramosbugs) | 66.1M total, 20.0M recent downloads; last release 2025-01-21 | 2027-02-09 |
| openidconnect | 4.1.0 | 2026-11-11 | 2026-10-31 (4.1.0) | 1 (ramosbugs) | 18.1M total, 6.1M recent downloads | 2027-02-09 |
| oxigraph | 0.5.13 | 2026-11-11 | 2026-10-22 (0.5.13) | 1 (Tpt) | 1.0M total, 334.2K recent downloads; single owner (docs/adr/0002) | 2027-02-09 |
| reqwest | 0.13.0 | 2026-11-11 | 2026-09-08 (0.13.5) | 1 (seanmonstar) | 933.6M total, 274.9M recent downloads; a newer 0.13 patch line sits upstream | 2027-02-09 |
| serde | 1.0.229 | 2026-11-11 | 2026-07-18 (1.0.229) | 2 (1 user + `serde-rs:publish` team) | 1741.6M total, 445.4M recent downloads | 2027-02-09 |
| serde_json | 1.0.154 | 2026-11-12 | 2026-10-11 (1.0.154) | 2 (1 user + `serde-rs:publish` team) | 1648.6M total, 448.2M recent downloads | 2027-02-10 |
| sqlx | 0.9.0 | 2026-11-11 | 2026-11-18 (0.9.1) | 2 (abonander, mehcode) | 194.1M total, 60.2M recent downloads; the 0.9.1 patch was released the day before this review | 2027-02-09 |
| thiserror | 1.0.69 | 2026-11-11 | 2026-09-23 (2.0.21) | 1 (dtolnay) | 1894.8M total, 548.4M recent downloads; the 2.0 major line is newer and not adopted | 2027-02-09 |
| time | 0.3.55 | 2026-11-11 | 2026-08-01 (0.3.55) | 1 (jhpratt) | 1097.2M total, 255.4M recent downloads; last release 2026-08-01 | 2027-02-09 |
| tokio | 1.54.1 | 2026-11-11 | 2026-10-21 (1.54.1) | 3 (2 users + `tokio-rs:core` team) | 1216.5M total, 318.0M recent downloads | 2027-02-09 |
| tokio-cron-scheduler | 0.15.1 | 2026-11-11 | 2025-10-28 (0.15.1) | 1 (mvniekerk) | 7.6M total, 2.1M recent downloads; last release 2025-10-28 | 2027-02-09 |
| toml | 0.8.23 | 2026-11-11 | 2026-11-06 (1.2.1+spec-1.1.0) | 2 (1 user + `toml-rs:maintainers` team) | 1164.9M total, 328.5M recent downloads; the 1.x line reached spec 1.1.0 and is not adopted | 2027-02-09 |
| tower-sessions | 0.15.0 | 2026-11-11 | 2026-02-01 (0.15.0) | 1 (maxcountryman) | 4.9M total, 1.6M recent downloads; last release 2026-02-01 | 2027-02-09 |
| tracing | 0.1.44 | 2026-11-11 | 2025-12-18 (0.1.44) | 3 (2 users + `tokio-rs:publish-tracing` team) | 1071.1M total, 283.6M recent downloads; last release 2025-12-18 | 2027-02-09 |
| tracing-subscriber | 0.3.23 | 2026-11-11 | 2026-03-13 (0.3.23) | 3 (2 users + `tokio-rs:publish-tracing` team) | 763.4M total, 201.8M recent downloads; last release 2026-03-13 | 2027-02-09 |
| utoipa | 6.0.0 | 2026-11-11 | 2026-09-22 (6.0.0) | 1 (juhaku) | 65.5M total, 23.8M recent downloads; last release 2026-09-22 | 2027-02-09 |
| validator | 0.21.0 | 2026-11-11 | 2026-07-27 (0.21.0) | 1 (Keats) | 78.6M total, 17.6M recent downloads; last release 2026-07-27 | 2027-02-09 |
| webauthn-rs | 0.6.2 | 2026-11-11 | 2026-10-31 (0.6.2) | 4 (Firstyear, agrinman, micolous, yaleman) | 9.6M total, 3.6M recent downloads | 2027-02-09 |

## Field meaning

- **Version**: resolved version from `Cargo.lock`.
- **Pinned on**: date the workspace adopted the version.
- **Last upstream release**: release date of the newest upstream version, with
  the version in parentheses. A date newer than **Pinned on** signals a release
  the workspace has not reviewed.
- **Owners**: crates.io owners with publish rights (users plus teams).
- **Trend**: download volume and release cadence at the pinning date.
- **Review due**: pinned date plus 90 days; the checker recomputes it from
  **Pinned on**, and the printed column stays informational for readers.

## What the checker enforces

1. Every direct dependency of the workspace crates has a row, and every row
   names a direct dependency — extra rows for dropped crates fail.
2. The row version matches the version `Cargo.lock` resolves for the
   workspace requirement of that crate.
3. Dates parse as `YYYY-MM-DD`; the upstream column carries the version in
   parentheses.
4. A newer upstream release (published after the pin) that stays unreviewed
   past the review-due date fails.

Entries of `[workspace.dependencies]` that no crate declares (`tower`) are not
direct dependencies and carry no row.