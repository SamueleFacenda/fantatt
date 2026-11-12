# Dependency Watchdog

Purpose: every direct dependency in the Rust workspace carries a row below. CI
(job introduced by G0.8) fails when a dependency's upstream release stays
unreviewed for more than 90 days, so version drift cannot accumulate silently.

## Table

| Crate | Version | Pinned on | Last upstream release | Owners | Trend | Review due |
| --- | --- | --- | --- | --- | --- | --- |
| proptest | 1.12.0 | 2026-11-12 | 2026-10-10 | 3 (2 maintainers + `proptest-rs:publish` team) | 242.9M total, ~2.99M weekly downloads; 6 releases in the last 15 months | 2027-02-10 |

## Field meaning

- **Version**: resolved version from `Cargo.lock`.
- **Pinned on**: date the workspace adopted the version.
- **Last upstream release**: release date of the newest upstream version.
- **Owners**: crates.io owners with publish rights (users plus teams).
- **Trend**: download volume and release cadence at the pinning date.
- **Review due**: pinned date plus 90 days; a newer upstream release before
  this date triggers the review obligation.
