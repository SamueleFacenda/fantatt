# Dependency Watchdog

Purpose: every direct dependency in the Rust workspace carries a row below. CI
(job introduced by G0.8) fails when a dependency's upstream release stays
unreviewed for more than 90 days, so version drift cannot accumulate silently.

## Table

| Crate | Version | Pinned on | Last upstream release | Owners | Trend | Review due |
| --- | --- | --- | --- | --- | --- | --- |
| proptest | 1.12.0 | 2026-11-12 | 2026-10-10 | 3 (2 maintainers + `proptest-rs:publish` team) | 242.9M total, ~2.99M weekly downloads; 6 releases in the last 15 months | 2027-02-10 |
| rand | 0.9.5 | 2026-11-17 | 2026-11-15 (0.10.4) | 2 (1 user + `rust-random:maintainers` team) | 2206.5M total downloads; 0.10.4 is a newer major line deliberately not adopted — only the 0.9 series already resolved in the lockfile is used | 2027-02-15 |
| hmac | 0.13.0 | 2026-11-17 | 2026-03-29 (0.13.0) | 2 (1 user + `rustcrypto:MACs` team) | 766.3M total downloads; last release 2026-03-29 | 2027-02-15 |
| sha2 | 0.11.1 | 2026-11-17 | 2026-10-12 (0.11.1) | 3 (2 users + `rustcrypto:hashes` team) | 1246.6M total downloads; last release 2026-10-12 | 2027-02-15 |
| async-trait | 0.1.92 | 2026-11-17 | 2026-08 (0.1.92, ~3 months before pinning) | 1 (dtolnay) | 855 SLoC, 1.3 KiB pinned crate; single owner, no team; download volume not captured at pinning | 2027-02-15 |
| lettre | 0.11.24 | 2026-11-19 | 2026-11-17 (0.11.24) | 2 (1 user + `github:lettre:release` team) | 24.9M total downloads; 0.11.24 is a security patch closing six SMTP advisories, previous release 0.11.23 on 2026-08-03 | 2027-02-17 |

## Field meaning

- **Version**: resolved version from `Cargo.lock`.
- **Pinned on**: date the workspace adopted the version.
- **Last upstream release**: release date of the newest upstream version.
- **Owners**: crates.io owners with publish rights (users plus teams).
- **Trend**: download volume and release cadence at the pinning date.
- **Review due**: pinned date plus 90 days; a newer upstream release before
  this date triggers the review obligation.
