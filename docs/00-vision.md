# Vision and Non-Functional Requirements

## 1. What Fantatt Is

Fantatt is a fantasy table tennis platform. Managers build squads of real Italian
FITET (Federazione Italiana Tennistavolo) players and score fantasy points from
real match results scraped from the FITET results portal.

Two game modes run on the same data:

| Mode | Format | Outcome |
| --- | --- | --- |
| Weekend Championship | Monthly rosters, season-long | Monthly winner + season champion |
| Tournaments | Bracket prediction | Points per correct pick |

## 2. Goals

1. **Correct game logic.** Every scoring rule is verifiable, auditable, and
   replayable. A manager must be able to ask "why did my score change?" and get
   a machine-checkable answer.
2. **Cheap to run.** The whole stack must be hostable on a single small VPS or a
   serverless free tier, with no per-row vendor pricing.
3. **Reproducible.** `nix develop` gives a complete toolchain; `nix build`
   produces the artifacts; a NixOS module deploys the whole stack.
4. **Trustworthy.** Verified dependencies, checked credentials, documented
   authorisation, GDPR-ready from the first commit.
5. **Good on a phone.** The UI is designed mobile-first; the app is an
   installable PWA that works offline and tells the user when data changes.

## 3. Non-Functional Requirements

### 3.1 Architectural

| ID | Requirement |
| --- | --- |
| NFR-A1 | Clean architecture: dependencies point inward. Domain rules depend on no framework, no database, no HTTP. |
| NFR-A2 | Monorepo. One repository, one lockfile per ecosystem, workspace-level CI. |
| NFR-A3 | Every cross-boundary interaction has a machine-readable contract (OpenAPI 3.1, JSON Schema, protobuf-free: JSON + schemas). |
| NFR-A4 | Swappable facades for all external dependencies: email, object storage, reasoning engine, scraper source. |
| NFR-A5 | No vendor lock-in. Every hosted service has a self-hostable equivalent that is configuration-only to switch. |
| NFR-A6 | Runtime configuration everywhere: no hardcoded lifecycle numbers (auction duration, lock windows, scoring weights). |

### 3.2 Quality attributes (targets)

| Attribute | Target | Measured by |
| --- | --- | --- |
| Scoring correctness | 100% of the spec examples in `05-scoring-spec.md` §8 pass | Golden-file tests in `scoring/tests/` |
| Roster validation | Zero invalid rosters accepted | Property-based tests over rule set |
| API latency p95 | < 300 ms for reads, 1 s for scoring recompute | Load test in CI |
| Scraper freshness | Matchday results visible < 15 min from publication | Pipeline SLO dashboard |
| Availability | Single-node self-host target 99%; cloud target 99.9% | Uptime check |
| Availability | Offline PWA serves cached standings | E2E test with network cut |
| Observability | 100% of spans carry `league.id`, `matchday.id`, trace ID | OTel schema validation in CI |
| Accessibility | WCAG 2.2 AA on all primary flows | axe scan in E2E |

### 3.3 Security (see `10-security.md`)

| ID | Requirement |
| --- | --- |
| NFR-S1 | Passwords never stored; passkeys and Google OAuth2 are the credential paths. |
| NFR-S2 | Session credentials only in `HttpOnly`, `Secure`, `SameSite=Lax` cookies; no tokens in `localStorage`. |
| NFR-S3 | CSRF tokens on every mutating request (double-submit + origin check). |
| NFR-S4 | Strict CSP with nonce-based script-src; no `unsafe-inline`. |
| NFR-S5 | Authorisation enforced server-side on every request; never by UI hiding. |
| NFR-S6 | Rate limiting on auth endpoints, auction bids, and search. |
| NFR-S7 | Full audit log, append-only, tamper-evident. |

### 3.4 Privacy / GDPR

| ID | Requirement |
| --- | --- |
| NFR-P1 | Data minimisation: store player game data only, no fitness data. |
| NFR-P2 | Self-service account deletion with a 30-day grace window then hard purge. |
| NFR-P3 | Self-service data export (JSON, machine-readable). |
| NFR-P4 | Privacy policy and cookie notice are first-class configurable documents, versioned, with consent recorded per version. |
| NFR-P5 | Processing of real athletes' results is a legitimate-interest use with a documented assessment and an opt-out path. |

### 3.5 Maintainability

| ID | Requirement |
| --- | --- |
| NFR-M1 | 80% minimum line coverage, 100% rule coverage for the reasoning rule set. |
| NFR-M2 | Complexity budget: functions ≤ 20 lines, ≤ 3 parameters, nesting ≤ 4. |
| NFR-M3 | Zero `unwrap()` in production paths outside tests; no `unsafe` without a SAFETY comment. |
| NFR-M4 | Architecture Decision Records for every choice with a plausible alternative. |

### 3.6 Cost discipline

| ID | Requirement |
| --- | --- |
| NFR-C1 | Reference deployment runs on 2 vCPU / 2 GB RAM for ≤ 100 leagues / ≤ 1000 managers. |
| NFR-C2 | Serverless Cold-start SLO: < 500 ms warm, < 2 s cold on the API path. |
| NFR-C3 | No paid managed service is required for a working deployment. |

## 4. Explicitly Deferred (YAGNI for v1)

Native mobile apps, multi-language UI, federated scoring between instances,
video/streaming integration, betting or real-money features.
