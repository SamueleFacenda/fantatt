# Roadmap

## Phase 0 — Foundation (weeks 1–3)

**Goal:** an empty but complete skeleton that builds, tests, and deploys.

- Nix skeleton: flake-parts, crane, dev shell, CI with `nix flake check`
- `fantatt-core`: domain types and invariants, no I/O
- Postgres schema and migration runner
- Empty axum service with health endpoint, session middleware, CSRF, CSP
- Authorisation matrix test harness
- README, contributing guide, ADR set

**Exit criteria:** `nix develop && nix flake check` green on a PR; a hardened
NixOS VM boots the service and passes `systemd-analyze security`.

## Phase 1 — Data (weeks 3–6)

**Goal:** the real world enters the system.

- `fantatt-ingest`: discover, diff, fetch, parse for calendar and giornate
- Recorded fixtures from the legacy prototype become golden tests
- Player identity reconciliation with an ambiguity queue
- Knowledge graph: ontology, assert pipeline, provenance, snapshots
- `fantatt-reasoning`: rule bundles, materialisation, SHACL validation
- Tournament brackets (gironi + eliminatorie)
- Player search and detail APIs

**Exit criteria:** a real matchday can be scraped, asserted, and queried; the
identity queue is usable; every rule in the bundle has conformance tests.

## Phase 2 — Game (weeks 6–10)

**Goal:** a season can be played.

- Scoring engine with full spec conformance (`05-scoring-spec.md` §8 examples)
- League creation, invitations, membership, roles
- Roster builder: slot validation, substitution, cost model, lock
- Standings: matchday, month, season
- Cost index computation
- Live matchday pipeline: SSE, notifications, offline PWA shell
- Web app: PWA shell, roster flow, standings, player pages

**Exit criteria:** a full month plays end-to-end with real data; E2E browser
suite passes on the phone viewport.

## Phase 3 — Auctions (weeks 10–14)

**Goal:** high-concurrency correctness under proof.

- Exclusive ownership mode, lots, bids, soft-close, resell
- Serialisable isolation + rule verdict + proof certificate
- Outbid notifications, timer escalation
- Concurrency test: many processes, many bids, exactly one winner

**Exit criteria:** the concurrency suite passes deterministically; a bid
decision can be re-verified offline by a separate process.

## Phase 4 — Tournaments and polish (weeks 14–18)

**Goal:** both modes, admin console, compliance, observability.

- Predictor mode: brackets, picks, points
- Admin console: identity queue, quarantine, jobs, rulesets, config
- GDPR: policy, notice, export, deletion, retention
- Observability: dashboards, alerts, RUM, trace-to-score debugging
- Accessibility pass across every primary flow
- NixOS module options: observability stack, storage, mail facade
- Serverless deployment recipe

**Exit criteria:** a real user can self-host the whole stack from a single
NixOS option; the deployment runs for 30 days with real data.

## Cross-cutting, from day one

Reproducibility, dependency review, linting, formatting, tests, security
scans, and the changelog. These are gates, not phases.

## Deliberately not planned (YAGNI)

Native mobile apps, real-money features, betting, multi-tenant federation,
streaming/video integration, machine-learning predictions.
