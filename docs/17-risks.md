# Risks and Mitigations

## 1. Technical risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| R1 | FITET portal changes structure, breaking parsing | High | Medium | Page-type schema fingerprints with drift alerts; a parser layer isolated behind a facade; recorded fixtures make the break visible in CI quickly |
| R2 | FITET portal blocks or rate-limits us | Medium | High | Concurrency 4 + token bucket + jitter + circuit breaker; archived raw responses mean we survive an outage; cache everything possible |
| R3 | Player identity ambiguity corrupts standings | Medium | High | Never auto-merge; ambiguity is a first-class state with an admin queue; ratings are bitemporal so a merge triggers a re-derivation that is visible |
| R4 | Oxigraph query performance under real load | Medium | Medium | Benchmark actual queries before committing; keep hot aggregations in SQL; snapshot to object storage for serverless |
| R5 | Reasoning supply chain (crates are young) | Medium | High | Versioned rule bundles, content-addressed; proofs re-checkable; fallback store identified (Minigraf) in ADR-002 |
| R6 | axum 0.9 is unreleased and may break us | Medium | Low | Pin 0.8.9; migration treated as a discrete, planned track |
| R7 | Scraping volume at scale (many regions) | Low | Medium | Configurable region set; per-competition priority; ingest is horizontally scalable and deduplicated |

## 2. Product risks

| # | Risk | Impact | Mitigation |
| --- | --- | --- | --- |
| P1 | Scoring disputes | High | Full breakdown exposed per player, per rule, per ruleset version; the "why this score" trace view in the admin console |
| P2 | Auctions feel unfair or buggy | High | Serialisable isolation plus proof certificates; soft-close windows; event log anyone can read |
| P3 | First-week exclusion confuses managers | Medium | Stated in the rules page and the UI; per-league configurable |
| P4 | Managers abandon rosters mid-month | Medium | Deadline reminders, auto-carry of last month's roster as a starting point |

## 3. Operational risks

| # | Risk | Impact | Mitigation |
| --- | --- | --- | --- |
| O1 | Self-host is too hard | High | One NixOS option produces a working deployment; a compose file exists for non-Nix hosts |
| O2 | Telemetry cost blows up | Medium | 14-day trace retention with tail sampling, 30-day metrics, no per-request high-cardinality labels |
| O3 | GDPR deletion vs league integrity | Medium | Tombstoned managers keep standings coherent while personal data is purged; documented in the privacy policy |
| O4 | Single maintainer | High | Documentation, ADRs, reproducible builds, and a monorepo so everything needed is in one place |

## 4. Cost estimate (reference configuration)

| Component | Self-hosted (VPS) | Cloud (Fly.io) |
| --- | --- | --- |
| Compute (2 vCPU / 2 GB) | €5/mo | $3.69/mo (512 MB) |
| Postgres | included | included |
| Backups (object storage) | €1/mo | < $1/mo |
| Email | provider-dependent | provider-dependent |
| Observability | included | included |
| **Total** | **~€6–10/mo** | **~$5–15/mo** |

Against the SLO: 100 leagues, 1000 managers, full scraping of selected regions.

## 5. Dependency watchdog

A tracking table in the repo lists every dependency with: crate, version,
pinned date, last upstream release, maintainer count, download trend, and a
review status. CI fails if a pinned version's upstream has a new release
unreviewed for more than 90 days. This operationalises the "verify dependencies,
do not trust memory" requirement.
