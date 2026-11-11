# Execution Roadmap — Atomic Goals

Status: active | Refines: [16-roadmap.md](16-roadmap.md) | Audience: development agents, client

This roadmap decomposes the phase plan in `16-roadmap.md` into atomic goals. Each goal is sized for one coding agent session and is independently verifiable through automated tests and, at checkpoints, a live demo the client can inspect.

## 1. How to read this roadmap

Goal card format:

- `ID` — stable identifier; referenced in commits (`feat(G2.5): ...`), PR titles, and the changelog
- `Outcome` — one sentence describing the observable result
- `Build` — what the agent implements
- `Verify` — the exact checks that must pass before the goal is done
- `Demo` — how the client checks the goal at a checkpoint (empty for internal-only goals)
- `Depends` — prerequisite goal IDs

Rules governing every goal:

1. **Definition of Done**: feature branch → PR → all CI gates green (fmt, clippy, tests, coverage ≥80% lines / 100% rules and invariants, cargo+npm audit, semgrep, trivy, secret scan), docs updated for behavior changes, CHANGELOG entry under `Unreleased`, demo recorded where specified. Merged with conventional commits; no direct push to main.
2. **Testable incrementally**: every goal ships its own tests. No goal relies on a future goal for verification. DB-backed goals use a fresh ephemeral Postgres; parser goals use fixtures with no network.
3. **Atomic size**: one goal = one agent session. If a goal exceeds one session, the agent stops, pushes the branch, and reports the split — it does not expand scope silently.
4. **No dependency drift**: versions come from the verified table in [16-roadmap.md](16-roadmap.md) and the dependency watchdog. Any new dependency is a separate PR with justification, last-release check, maintainer count, and download count (see 10-security.md).
5. **Rules carry proofs**: anything enforced by a rule (roster validity, scoring, auction bids) verifies through the rule bundle conformance suite plus, where mandated, an eyeron proof certificate re-checked offline by a separate process.

## 2. Client interaction protocol

Agile checkpoints work as follows:

- **Per-milestone checkpoint** (end of each phase below): the client receives a demo script (commands and URLs to run locally against a Nix-built environment) plus the goal status table. The client accepts, or files change requests that become new goals inserted into the roadmap.
- **Roadmap updates**: this document is versioned. Every accepted change adds a line to [§9 Changelog](#9-roadmap-changelog) with date and reason. Removed or deferred goals are never deleted — they are marked `deferred` or `superseded` with a pointer.
- **Soft ordering**: `Depends` defines hard ordering. Goals inside a phase without mutual dependency may be built in parallel by multiple agents; the phase exit criteria still gate the checkpoint.
- **Definition of "demoable"**: the goal is reachable from the running local stack (`just dev` or `nix run .#dev`) without manual data surgery, or the demo script seeds that state deterministically.

## 3. Phases at a glance

| Phase | Title | Goals | Exit criteria |
|---|---|---|---|
| 0 | Foundation | G0.1–G0.8 | Empty but complete skeleton: Nix workspace, schema, hardened API shell, CI gates green |
| 1 | Data & Knowledge Graph | G1.1–G1.12 | Real FITET matchday scraped → asserted → queried; every rule has conformance tests |
| 2 | Core Game (server) | G2.1–G2.8 | Full month end-to-end through the API; scores reproducible from the ledger |
| 3 | Web App & Live | G3.1–G3.9 | Mobile-first PWA plays a full month; live matchday, offline, push all verified |
| 4 | Tournaments, Admin, Compliance, Ops | G4.1–G4.9 | Self-host from one NixOS option; GDPR, observability, serverless, 30-day soak ready |
| 5 | Exclusive Ownership & Auctions (deferred epic) | G5.1–G5.8 | Auction mode verifiable offline: serializable bids, proof certificates, soft-close |
| A | Audits (interleaved) | SA-1…SA-3, QA-1…QA-3 | Audit findings closed or filed as new goals with owner |

Audits are first-class goals with the same Definition of Done. They are scheduled in the phase tables below.

## 4. Phase 0 — Foundation

### G0.1 — Repository skeleton

- **Outcome**: the `ai-project` template is removed and a Nix-managed Rust workspace with all planned crates exists, building from a clean checkout.
- **Build**: flake-parts flake with dev shell, crane-based Rust builds, rust-overlay; crates `fantatt-core`, `fantatt-reasoning`, `fantatt-scoring`, `fantatt-ingest`, `fantatt-api`, `fantatt-jobs`, `fantatt-notify`, `fantatt-obs`, `fantatt-config` as compiling library stubs; `web/`, `nix/`, `deploy/`, `tests/{e2e,nixos,conformance}`, `rules/` directory trees per 11-repo-structure.md; delete template Cargo.toml/package.json/pyproject.toml and template sources.
- **Verify**: `nix flake check --all-systems`, `nix build`, `cargo workspace` build offline from the Nix store; CI green on a sample PR.
- **Demo**: `nix develop` + `cargo check` output on the client machine.
- **Depends**: —

### G0.2 — Engineering gates

- **Outcome**: every future PR is forced through format, lint, test, audit, and secret gates.
- **Build**: `justfile` (default recipe runs all gates), rustfmt + clippy with strict lints (including complexity limits: function length ≤20 lines, ≤3 args, nesting ≤4), `commitlint` conventional commits, `gitleaks`/`git-secrets` pre-commit, GitHub Actions pipeline (PR: gates; main: build artifacts), CHANGELOG.md in keepachangelog format, initial CONTRIBUTING.md.
- **Verify**: a seed PR fails when a `what`-comment, an unsafe block in `fantatt-core`, or a bad commit message is introduced; passes when clean.
- **Depends**: G0.1

### G0.3 — Domain core

- **Outcome**: the entire domain model of 02-domain-model.md exists as pure Rust with no I/O, and every invariant INV-1…INV-15 is machine-enforced.
- **Build**: `fantatt-core` value objects and aggregates (ULID-prefixed IDs per 07-api-spec.md, `DivisionTier`, `Rating`, `Squad`, `Slot`, `League`, `Matchday`, `Appearance`), invariant checks as typed constructors returning `Result<_, DomainError>`, `thiserror` error taxonomy naming rules (e.g. `INELIGIBLE_PLAYER`), serde derives; dependencies limited to serde/thiserror/time.
- **Verify**: `cargo test -p fantatt-core` with one test per invariant plus boundary cases; `proptest` generators for rosters (INV-2/3/4/6), budgets (INV-12), and idempotency keys; coverage 100% of invariant code.
- **Depends**: G0.1

### G0.4 — Configuration system

- **Outcome**: the service validates its whole configuration at startup and fails fast on missing secrets.
- **Build**: `fantatt-config` layered loader (defaults TOML → file → env → CLI), typed settings for every module, validation rejecting invalid combinations, zero secret values in files, startup abort listing missing secrets.
- **Verify**: unit tests per invalid combination (missing secret, bad port, negative concurrency); integration test boots with an env file and fails with a named error when a secret is absent.
- **Depends**: G0.1

### G0.5 — Database schema and migrations

- **Outcome**: the full relational model exists with compile-time-checked queries and a repeatable migrate/bootstrap path.
- **Build**: Postgres schema for users, credentials, sessions, leagues, memberships, invitations, squads, slots, submissions, competitions, matchdays, ties, appearances, offers, lots, bids, tournaments, predictions, notifications, preferences, audit_log, ingest ledger and jobs, identity aliases and the ambiguity queue; sqlx migrations with `sqlx::query!` compile-time checking; `just db-migrate`, `just db-reset`; test harness spinning an ephemeral Postgres for integration tests.
- **Verify**: migrate → rollback → migrate on a scratch DB; every `query!` compiles (offline `sqlx prepare` data committed); DB-level constraint tests (unique squad per exclusive league+month, budget check, append-only audit trigger).
- **Depends**: G0.1

### G0.6 — API shell and security middleware

- **Outcome**: a hardened, empty HTTP surface answers health checks and rejects everything else.
- **Build**: axum app with tower-sessions (Postgres store, `__Host-session` HttpOnly+Secure+SameSite=Lax, 30d idle / 12h absolute, rotation on login and role change), CSRF double-submit (`X-CSRF-Token`), nonce-based CSP plus HSTS/XCTO/frame-ancestors/frame-ancestors, RFC 9457 problem+json error envelope, request-ID and tracing middleware, `Idempotency-Key` handling for POST, 1MB body limit, rate limiting via governor (auth 10/min IP + 5/min account, search 60/min, reads 300/min), utoipa OpenAPI document emitted at build time.
- **Verify**: `/healthz` contract test; header-flag tests read from a real browser session (Cookie flags, CSP report endpoint); CSRF rejection test on mutating route without token; error-shape conformance (every error response is problem+json with `type`/`title`/`status`); OpenAPI snapshot test fails on undocumented response-shape change.
- **Demo**: `curl` script hitting healthz, a CSRF-rejected POST, and the OpenAPI JSON.
- **Depends**: G0.4, G0.5

### G0.7 — Authorization matrix harness

- **Outcome**: adding an endpoint without declaring its capability fails the build.
- **Build**: capability model in `fantatt-core` (roles: superadmin, league_admin, collaborator, manager, member; object-level checks), route declarations carrying capability + object scope, and an exhaustive role × route test generated from the route table; default-deny guard rejecting undeclared routes.
- **Verify**: seeded test walks every route × every role and asserts allow/deny per the matrix in 10-security.md; a stubbed new route without capability fails CI.
- **Depends**: G0.6

### G0.8 — Documentation baseline and dependency watchdog

- **Outcome**: new contributors and the watchdog have a machine-checked home.
- **Build**: README (one-command dev setup), CONTRIBUTING, ADR copies of `docs/adr/` carried forward unchanged, dependency watchdog table (name, pinned date, last release, maintainers, trend) in `docs/`, markdown link check in CI, CI job failing when a dependency's upstream release is unreviewed for >90 days.
- **Verify**: docs CI job; watchdog job fails on a deliberately stale row.
- **Depends**: G0.1

**Checkpoint 0** — client runs the demo scripts above against a clean Nix build and approves the skeleton.

## 5. Phase 1 — Data & Knowledge Graph

### G1.1 — Fixture promotion

- **Outcome**: all recorded FITET fixtures are versioned in-repo and loadable by tests with no network.
- **Build**: recover `fitet/assets/matches.json` (1085 matches) from commit `444d843` plus recorded HTML surfaces into `tests/fixtures/fitet/`; fixture index crate/module with typed accessors; `just fixtures-verify` confirming count and hashes.
- **Verify**: fixture loader round-trip test; hash manifest test.
- **Depends**: G0.1

### G1.2 — Fetch facade and politeness

- **Outcome**: every outbound fetch is polite, bounded, and observable, and dies gracefully when FITET blocks.
- **Build**: reqwest+rustls client with global concurrency semaphore ≤4, token bucket, jitter, bounded retries with exponential backoff, conditional requests (ETag/Last-Modified), circuit breaker with half-open probing, per-request timeouts, `User-Agent` policy, archive/storage facade (local filesystem default, S3-compatible optional); all I/O behind a trait so unit tests use a mock server.
- **Verify**: integration tests with a local mock HTTP server asserting concurrency cap, retry behavior, and circuit-open after N failures; zero network in `cargo test`.
- **Depends**: G0.1

### G1.3 — Discover and diff

- **Outcome**: a deterministic work list is produced by comparing the portal against the ingest ledger.
- **Build**: region → championship → matchday discovery (and tournament discovery), URI normalization, content-hash diff against the ledger, job table writes with dedupe, `fantatt-jobs` scheduler skeleton with Postgres queue.
- **Verify**: unit tests over recorded menus produce the expected job list; re-running discover on unchanged fixtures yields zero jobs (idempotence); ledger schema migration test.
- **Depends**: G1.1, G0.5

### G1.4 — Matchday (giornata) parser

- **Outcome**: real HTML matchdays become canonical records with precise, selector-level error reporting.
- **Build**: `RawGiornata` / `GiornataRow` canonical model; parser extracting date, venue, home/away, players, sets, walkovers (`assente`/`ritirato` → 0-3/3-0), unplayed ties skipped with reason; entity name normalization shared with identity; validation returning typed errors naming the failing selector; guard against the `(incontri disputati 63/72, completezza: 87.5%)` partial-bracket marker.
- **Verify**: golden tests over every recorded fixture (1085 matches) comparing parsed output to expected canonical JSON; drift test that a mutated HTML sample fails with a selector-locating error rather than panicking.
- **Depends**: G1.1

### G1.5 — Calendar, classification, and player surface parsers

- **Outcome**: all 13 FITET URL surfaces in 06-data-pipeline.md are parseable.
- **Build**: parsers for championship list, regional menus, tournament list, `Calendario.asp`, `giornata.php`, `new_rank` classification, `ajax.php?term=` search, `dettaglioatleta_unica.php` player detail, `tabelloni` bracket home.
- **Verify**: golden tests per surface from fixtures; canonical cross-surface consistency test (same matchday id and same set scores in calendar, giornata, and classification).
- **Depends**: G1.4

### G1.6 — Identity reconciliation

- **Outcome**: players from independent pages resolve to stable identities, with ambiguity queued instead of guessed.
- **Build**: alias normalization (pure, shared with parser), candidate matching producing `RESOLVED` / `AMBIGUOUS` / `UNRESOLVED`, provisional rating 2000 for unresolved, bitemporal ratings (`assertedAt`/`validFrom`), ambiguity queue table + resolution API for admins (queue surfacing itself lands in G4.3).
- **Verify**: unit tests incl. homonym cases where two plausible matches exist (must be `AMBIGUOUS`, never auto-picked); proptest that resolution is stable under input ordering; DB constraint test on rating uniqueness per identity per `asOf` (INV-1).
- **Depends**: G1.4

### G1.7 — Knowledge graph runtime

- **Outcome**: facts land in Oxigraph with mandatory provenance and can be snapshotted and rebuilt.
- **Build**: `fantatt-reasoning` Oxigraph wrapper (named graphs per derivation commit, no blank nodes), assert pipeline with bitemporal + provenance triples, snapshot save/load via storage facade, rebuild-from-ledger function (KG is a projection; Postgres ledger authoritative).
- **Verify**: integration test asserts a fixture matchday and queries it back with provenance; kill-mid-derivation test leaves no partial graph (ADR-002 fallback path documented); rebuild-from-ledger produces byte-identical snapshot.
- **Depends**: G1.6, ADR-0002, ADR-0003

### G1.8 — Rule bundle format and content addressing

- **Outcome**: rules are immutable, versioned artifacts identified by content hash.
- **Build**: `rules/<name>/<semver>/` layout with `manifest.toml` (entrypoints `roster.validate`, `scoring.matchday`, `auction.bid`, `settle.standings`; `requires tt-ontology`), TTL ontology, N3 rules, SPARQL queries, `tests/` golden folder (input graph, expected facts, expected proof); loader computing `rulesetVersion = SHA-256(manifest + files)`; `just rules` runner.
- **Verify**: hash stability test; tamper detection changes the hash; loader rejects a bundle missing an entrypoint.
- **Depends**: G0.1

### G1.9 — Ontology and SHACL shapes

- **Outcome**: malformed data never enters the graph; it is quarantined with a report.
- **Build**: table-tennis ontology TTLs, SHACL shapes via rudof, quarantine flow (failed document parked with shape-violation report, never silently repaired).
- **Verify**: shape conformance tests over good and broken fixtures; integration test proves a violating ingest document is quarantined and the graph stays unchanged.
- **Depends**: G1.7

### G1.10 — Reasoning engine integration and first rule bundle

- **Outcome**: roster validity and scoring are decided by N3 rules that emit offline-verifiable proofs.
- **Build**: eyeron integration; rule bundle `fantatt-core 1.0.0` implementing roster validation and matchday scoring entrypoints; materialisation of derived facts into named graphs; proof certificates emitted for verdicts.
- **Verify**: conformance suite inside the bundle (expected facts + expected proof diff) run by `just rules`; `eyeron --check-proof` re-verifies a stored proof from a separate process with no services running; CI fails if a rule changes without updating expected files.
- **Depends**: G1.8, G1.9

### G1.11 — Tournament bracket ingestion

- **Outcome**: `tabelloni` brackets (gironi + eliminatorie) are modeled completely enough for the predictor mode.
- **Build**: bracket parser capturing `bit_count`, byes, consolation bracket, third-place, `(R)` retired → 0; canonical gironi/eliminatorie model; skip partial brackets using the completeness marker.
- **Verify**: golden tests from recorded brackets incl. a deliberately incomplete bracket that must be skipped with a logged reason.
- **Depends**: G1.5

### G1.12 — Discovery read APIs

- **Outcome**: clients can browse competitions, matchdays, and players with full breakdowns.
- **Build**: `/api/v1` endpoints for competitions, matchdays, players search/detail/appearances with the scoring breakdown view; OpenAPI annotations; generated TypeScript client; contract test suite.
- **Verify**: conformance tests against recorded fixture data; pagination proptests (limit+cursor, no duplicates/loss); response-shape change without version bump fails the build.
- **Demo**: documented API walkthrough against a seeded instance.
- **Depends**: G1.6, G1.11

**Checkpoint 1** — client runs one script that scrapes a real matchday from a recorded replay, asserts it, and queries a player's appearances through the API.

### SA-1 — Security audit 1

- **Outcome**: the ingest and API surfaces are reviewed adversarially before any real user data exists.
- **Scope**: SSRF policy for outbound fetches (allow-list of fitet.it hosts, redirect policy), secrets handling, authz matrix completeness, CSRF/CSP suite, input-size limits, `cargo audit` + `trivy` + `semgrep` + secret scan on the new dependency set, review of quarantine and drift-alert paths against R1/R2 in 17-risks.md.
- **Verify**: findings triaged and fixed, or filed as goals with owners and deadlines; audit report appended to `docs/audits/SA-1.md`; re-run of every scan clean.
- **Depends**: G0.8, G1.12

## 6. Phase 2 — Core Game (server)

### G2.1 — Scoring engine

- **Outcome**: scoring is bit-exact against the specification, with no rule interpretation left to the implementer.
- **Build**: `fantatt-scoring` pure engine: tier base points (B2 15/3, C1 12/3, C2 10/2, D1 8/2, D2 6/1), gender-adjusted opponent (male vs female ×0.75), rating ratio (singles only, only when >1.0), away ×1.2, decider ×1.3, comeback ×1.3 all multiplicative, doubles = base×away×comeback ÷ 2 with no rating/decider multiplier, bonuses +3 team win and +2 individual doublet, week-1 = 0, internal f64 with 4-decimal storage rounding and 1-decimal round-half-away-from-zero display.
- **Verify**: golden files for every worked example in 05-scoring-spec §8 plus boundary cases (0-2 comeback, 3-3 decider, doubles, absence/walkover, week-1 exclusion, rating below threshold); coverage 100% of engine code; a fuzz test asserting rounding determinism.
- **Depends**: G0.3

### G2.2 — Scoring as rules with engine parity

- **Outcome**: the reasoning bundle and the Rust engine cannot silently diverge.
- **Build**: scoring rules expressed in the `fantatt-core 1.0.0` bundle `scoring.matchday` entrypoint; materialisation of per-player, per-appearance score facts into the KG.
- **Verify**: parity suite running identical inputs (all §8 examples + boundaries + generated proptests) through Rust and N3 engines and asserting identical stored 4-decimal values; proof certificate for each materialised score; `just rules` green.
- **Depends**: G1.10, G2.1

### G2.3 — Cost index

- **Outcome**: a versioned, reproducible salary estimate exists for every player in every league.
- **Build**: cost index computation over the nine inputs (rating, tier, form, difficulty, schedule, availability, depth, league scarcity) normalized to the league budget, emitted as a versioned rule bundle entrypoint; recompute job in `fantatt-jobs`.
- **Verify**: golden tests on fixture inputs; determinism (same inputs → identical 4-decimal costs); recompute job idempotence test; rulesetVersion recorded with every published index.
- **Depends**: G2.1, G1.10

### G2.4 — Leagues, membership, invitations

- **Outcome**: organizers run leagues; managers join; every state change is audited.
- **Build**: league create/update/state machine, settings (scoring format, budget, roster rules, calendar), membership and role assignment restricted to permitted capabilities, invitations (link and email), join requests, hash-chained append-only audit log entries for every mutation.
- **Verify**: integration tests per transition incl. illegal transitions; authz matrix updated and exhaustive test passes (object-level checks, no IDOR); audit-chain tamper-evidence test.
- **Demo**: create a league, invite a second account, accept — via API or seeded UI once G3.6 exists.
- **Depends**: G0.7, G0.5

### G2.5 — Squads: draft, validate, submit, lock

- **Outcome**: roster legality is decided by the same rules the client UI mirrors, at commit time, inside the transaction.
- **Build**: squad draft CRUD (INV-2/3/4/5/6), slot recomputation from the player's active division tier, commit path invoking `roster.validate` inside the DB transaction and returning 422 with a rule trace (`[{rule, violation, players}]`) on failure, submission with lock, organizer unlock (audited), optimistic concurrency via squad revision.
- **Verify**: API tests for legal roster (accepts) and each violation class (422 naming the exact rule, never a generic message); concurrency test where two PATCHes race — one wins, no lost update; proof stored for each accepted commit; unit tests that a locked squad rejects edits (INV-7).
- **Demo**: POST a deliberately illegal roster and read the named rule in the 422 body.
- **Depends**: G1.10, G2.4

### G2.6 — Substitutions

- **Outcome**: absent players are replaced from the bench within their own tier, deterministically.
- **Build**: absence detection from asserted appearances, tier-matched bench substitution (INV-9), recorded substitution decisions with provenance, organizer override path.
- **Verify**: golden tests over recorded matchdays containing absences; API test that an illegal cross-tier substitution is rejected naming the rule.
- **Depends**: G2.5, G1.12

### G2.7 — Standings

- **Outcome**: matchday, month, and season standings follow the specified tie-breaking and survive ruleset changes.
- **Build**: aggregation with the documented ordering (compare unrounded, then fewest bench points, then alphabetical), week-1 zero-point handling, cache with invalidation on new assertions or ruleset publication, historical standings pinned to the rulesetVersion that produced them.
- **Verify**: golden tests for each documented example and tie-break case; recompute determinism (full season recompute reproduces identical standings, target <60s per 15-testing.md); historical pinning test.
- **Demo**: API call returning a month's standings next to a hand-computed table from the fixture.
- **Depends**: G2.2

### G2.8 — Matchday completion and event-driven recompute

- **Outcome**: a matchday is only complete when its ties are decided, and completion fans out to affected leagues exactly once.
- **Build**: matchday state machine, completeness evaluation (INV-10), `finalized` event via Postgres LISTEN/NOTIFY consumed by `fantatt-jobs`, recompute of standings and cost index for subscribed leagues, outbox pattern guaranteeing exactly-once downstream application.
- **Verify**: integration test replays a fixture matchday and asserts final standings; duplicate-event test (same matchday finalized twice) yields one recompute; mid-recompute kill test restarts cleanly.
- **Depends**: G2.7, G2.6

**Checkpoint 2** — client plays a full month through the API with fixture data: rosters, scoring, substitutions, standings.

### QA-1 — Quality and refactor audit 1

- **Outcome**: the server core meets clean-code gates before the UI multiplies its surface.
- **Scope**: clippy complexity lints (function length ≤20, args ≤3, nesting ≤4, cognitive complexity ≤15), dead-code and duplicate-logic sweep (scoring formula must have exactly one implementation), doc-currentity check (docs match behavior, no diff-anchored comments), ADR updates for decisions taken during G1–G2.
- **Verify**: findings fixed or filed; gates strengthened where they permitted a violation.
- **Depends**: G2.8

## 7. Phase 3 — Web App & Live Matchday

### G3.1 — Web skeleton and design system

- **Outcome**: the app shell builds fast, tests run in CI, and tokens trace back to the brandbook.
- **Build**: Vite + React 19 + TypeScript + Tailwind 4 + shadcn/ui; design tokens generated from 09-brandbook.md with a CI check that brand colors and type scale match; route shell with lazy loading; TanStack Query + URL-state + session-context architecture (no Redux); OpenAPI-generated typed client wired to a contract-testing harness.
- **Verify**: build budget enforced (<150KB gz JS initial route); unit tests for token source; client type test against the live OpenAPI document.
- **Demo**: running app with placeholder content on the phone viewport.
- **Depends**: G0.6

### G3.2 — Authentication UI

- **Outcome**: users sign in with Google or a passkey; no password path exists.
- **Build**: OAuth Google start/callback screens wired to real backend routes (mock OIDC in tests with identical claim shape incl. `picture`), passkey registration and login via WebAuthn virtual authenticator in Playwright, session bootstrap and route guards, email-verified enforcement UI.
- **Verify**: E2E login flows (Google mock + passkey virtual authenticator), logout, session-expiry behavior, and authz matrix exercised through the UI; unit tests for guard logic.
- **Demo**: sign in on a phone viewport with the virtual authenticator.
- **Depends**: G0.6, G3.1

### G3.3 — PWA shell

- **Outcome**: the app installs, works offline, and updates predictably.
- **Build**: manifest (standalone, maskable icons 192/512), service worker with CacheFirst shell / SWR navigations / NetworkFirst live scores, offline banner, update flow (`updateViaCache: none`, `skipWaiting` + `clients.claim`, user-approved reload, 30-minute update check), i18n scaffolding (fluent/ICU, `it` default).
- **Verify**: E2E first-visit install, offline navigation, forced-update flow with assert on reload prompt; Lighthouse PWA checks in CI.
- **Demo**: install to home screen on a phone, toggle airplane mode, reload.
- **Depends**: G3.1

### G3.4 — Player and competition browsing

- **Outcome**: users find players and read complete, verifiable score breakdowns.
- **Build**: search with filters, player detail page (career, division, rating history, appearances with per-match breakdown and rule references), competition and matchday views.
- **Verify**: component tests; E2E flow on 390×844; axe zero violations on every new route.
- **Depends**: G1.12, G3.1

### G3.5 — Roster builder

- **Outcome**: the client-side validation cannot drift from the server's because it is generated from the same bundle.
- **Build**: 15-slot tier grid UI, live validation compiled from the same rule artifacts as the backend (shared codegen from the `roster.validate` entrypoint), inline errors naming rules exactly as the 422 body does, draft persistence in localStorage, cost index always visible, submit/lock against G2.5 endpoints with optimistic concurrency.
- **Verify**: E2E flow builds a legal roster and rejects each violation class with the exact rule name shown; draft-survives-reload test; unit tests for the generated validator; contract test that generated validator and server 422 agree on a seeded roster set.
- **Demo**: build a roster, watch slot rules reject a wrong-tier player by name.
- **Depends**: G2.5, G3.4

### G3.6 — League home, invitations, and standings UI

- **Outcome**: managers join leagues and follow standings on mobile.
- **Build**: league list/detail/home with standings (matchday/month/season tabs), invitation accept and join-request flows, member list, settings view for organizers.
- **Verify**: E2E create → invite → join → view standings; axe checks.
- **Demo**: full league lifecycle in the browser.
- **Depends**: G2.4, G2.7, G3.5

### G3.7 — Live matchday over SSE

- **Outcome**: the matchday page updates live with server-authoritative revisions.
- **Build**: `/api/v1/stream` SSE hub with per-league channels, monotonic revision numbers, heartbeat, TanStack Query invalidation bridge, live card with delta badges and live substitution display, server-time clock from the `Date` header.
- **Verify**: integration test drives a fixture matchday to completion and asserts delivery order and revision monotonicity; E2E live test with a scripted feed; load test at 200 concurrent readers p95 <300ms.
- **Demo**: watch a scripted matchday tick over live on a phone.
- **Depends**: G2.8, G3.6

### G3.8 — Notifications

- **Outcome**: users are notified in-app, by email, and (optionally) push for the categories they chose.
- **Build**: notification inbox with read state, per-category preferences, email facade (lettre) with Mailpit capture in tests, VAPID push subscription endpoint and per-category push, unsubscribe links, outbound dedupe via the outbox.
- **Verify**: integration tests per channel with Mailpit; preferences matrix test (every category × channel); opt-out propagation test; push payload schema test.
- **Depends**: G2.4, G3.6

### G3.9 — Offline write queue

- **Outcome**: a manager with no connectivity can still save drafts and queue writes that replay safely.
- **Build**: durable write queue in localStorage/IndexedDB with idempotency keys matching server-side handling, replay with conflict resolution (409 → surfaced to user), sync status indicator.
- **Verify**: E2E offline draft → back online → replay accepted; duplicate replay (double-tap replay) results in one server-side effect (idempotency test).
- **Depends**: G3.5, G3.8

**Checkpoint 3** — client completes a month entirely on a phone, including live matchday and offline draft.

### QA-2 — Quality, accessibility, and performance audit

- **Outcome**: the frontend is WCAG 2.2 AA, under budget, and free of accumulation-driven smells.
- **Scope**: axe on every route at three viewports (390×844, 768×1024, 1440×900), keyboard-only walkthroughs, 44px touch targets, `prefers-reduced-motion`; bundle budget and LCP <1.5s on 4G emulation; React query/caching review (no redundant refetch storms); component duplication sweep.
- **Verify**: audit report in `docs/audits/QA-2.md`; findings fixed or filed; perf budgets enforced in CI.
- **Depends**: G3.9

### SA-2 — Security audit 2 (pre-exposure)

- **Outcome**: the real attack surface — sessions, passkeys, CSRF, CSP, authz, rate limits, IDOR — is verified from a browser.
- **Scope**: WebAuthn review (challenge single-use ≤5min, RP-ID/origin validation, clone detection, user-verification requirement), session rotation matrix, cookie-flag assertions read from a real browser, CSP report endpoint ingestion, XSS probe suite (including `dangerouslySetInnerHTML` absence scan), IDOR tests on every object route, rate-limit verification, avatar re-encoding origin isolation, dependency and supply-chain review, GDPR consent and privacy-policy wiring.
- **Verify**: report in `docs/audits/SA-2.md`; all findings closed or filed with owners; authz matrix suite 100%.
- **Depends**: G3.9

## 8. Phase 4 — Tournaments, Admin, Compliance, Ops

### G4.1 — Tournament predictor (server)

- **Outcome**: managers predict brackets and earn points per accuracy.
- **Build**: prediction storage keyed to bracket structure, deadline enforcement (UTC), points rules per correct pick advancing, live point totals as ties resolve.
- **Verify**: contract + conformance tests including post-deadline rejection and scoring of partially correct brackets.
- **Depends**: G1.11, G2.4

### G4.2 — Tournament predictor UI

- **Outcome**: bracket picking is usable on a phone and shows live points.
- **Build**: bracket picker (gironi + eliminatorie), deadline countdown, points view, live updates over SSE.
- **Verify**: E2E pick → deadline → points appear; axe checks.
- **Depends**: G4.1, G3.7

### G4.3 — Admin console

- **Outcome**: operators resolve identity ambiguity, quarantine, jobs, rulesets, users, and roles without DB access.
- **Build**: identity reconciliation queue and resolve flow, quarantine browser with shape-violation reports, ingest job trigger/monitor, ruleset publish/rollback, users and roles, config view, audit-log viewer (filter by actor/league/action), telemetry status page.
- **Verify**: E2E flows per section with non-superadmin denial; every action produces an audit entry; IDOR tests.
- **Demo**: resolve one queued ambiguity end-to-end.
- **Depends**: G1.6, G1.9, SA-1

### G4.4 — GDPR compliance

- **Outcome**: users can export and delete their data, and league history stays intact.
- **Build**: versioned privacy policy and consent records, self-service JSON export job, two-phase deletion (request → 30-day grace → purge), tombstones preserving standings integrity, retention jobs for events and ingest payloads.
- **Verify**: E2E export (download, shape, completeness) and deletion (grace period, cancel path, purge); purge test proving PII removal while standings still reference tombstones; consent-version wiring test.
- **Depends**: G2.4, G3.8

### G4.5 — Observability

- **Outcome**: every request, job, and derivation is traceable end-to-end, and trace attributes are validated in CI.
- **Build**: OTel wiring across api/jobs/ingest/reasoning (traces, metrics, logs), schema validation in CI requiring `league.id` / `matchday.id` attributes on relevant spans, sampling policy, RUM metrics, Prometheus alerts, Grafana dashboards and Tempo/Loki datasources provisioned as code in `deploy/observability`, trace-to-score debug path (span → derivation commit → proof certificate).
- **Verify**: CI schema check fails on a span missing required attributes; a scripted bug reproduces a trace visible in the dashboard; alert rules unit-tested syntactically.
- **Depends**: G2.8, G3.7

### G4.6 — NixOS module and VM test suite

- **Outcome**: one `services.fantatt.enable = true` self-hosts the whole stack on a hardened machine.
- **Build**: NixOS module (enable, package, domain, database, storage, observability `createLocally` options, ingest regions, secrets files) wiring api + ingest worker + scheduler + notify + ACME/TLS + firewall; hardened systemd unit (DynamicUser, SystemCallFilter `@system-service`, MemoryDenyWriteExecute, DevicePolicy closed, ProcSubset pid, empty capability set); `nixosTest` suite: full-stack boot, `systemd-analyze security` score threshold, TLS proxy, backup restore reproducing identical scores, mid-derivation kill, upgrade N→N+1, multi-node auction-serialization placeholder (activated in G5.3).
- **Verify**: VM suite green in CI; security score reported; restore test asserts byte-identical standings.
- **Depends**: G4.4, G4.5

### G4.7 — Serverless recipe

- **Outcome**: the same binary deploys to Lambda behind the web adapter.
- **Build**: `dockerTools.buildLayeredImage` (non-root 65534) for Lambda Web Adapter, ARM64 LTO profile, Terraform/OpenTofu under `deploy/serverless`, cold-start benchmark.
- **Verify**: deploy smoke test in CI (staging account or localstack); cold start <2s measured and asserted.
- **Depends**: G4.6

### G4.8 — Compose recipe, backups, and DR runbook

- **Outcome**: non-Nix hosts can run the stack, and recovery is rehearsed, not assumed.
- **Build**: `deploy/compose` recipe, Postgres WAL + basebackup schedule (RPO ≤5min), KG snapshot job, audit-log retention (never pruned), DR runbook with restore verification.
- **Verify**: drift test restores a backup and re-derives identical scores; runbook executed once as a game day with results recorded.
- **Depends**: G4.6

### G4.9 — Staging soak and release readiness

- **Outcome**: the stack is proven over 30 days and releases are boring.
- **Build**: staging VM deployment, uptime and uptime-budget checks, dependency watchdog enforced in CI (>90 days unreviewed upstream fails), release tagging `vX.Y.Z` with previous-closure rollback, backup-restore rehearsal schedule, load test suite wired to weekly CI.
- **Verify**: 30-day soak report (zero unplanned outages beyond budget, no lost ingest days, alert noise review); release/rollback dry run in staging.
- **Demo**: client inspects dashboards and the soak report.
- **Depends**: G4.6, G4.7, G4.8

**Checkpoint 4** — client self-hosts from a single NixOS option and reviews the soak report.

### SA-3 — Security audit 3 (launch gate)

- **Outcome**: a launch decision with evidence, not vibes.
- **Scope**: full threat-model pass against 10-security.md and 17-risks.md, external-style review of authn/authz, dependency and supply-chain re-check (watchdog table current), secrets and credential-store review, GDPR conformity review (lawful bases, consent versions, deletion evidence), incident-response and breach-notification runbook, admin-console abuse scenarios, rate-limit and DoS posture on the ingest path.
- **Verify**: report in `docs/audits/SA-3.md`; every critical/high closed; medium/low filed with dates; sign-off recorded in this roadmap's changelog.
- **Depends**: G4.9

## 9. Phase 5 — Exclusive Ownership & Auctions (deferred, low priority)

The exclusive-player-ownership auction mode is intentionally sequenced last. It depends on the stable core game (rules, proofs, standings) being complete, because its fairness guarantees build on the serializable-transaction and proof-certificate machinery proven in Phase 2. All hard requirements below are preserved from 02/04/07; only the sequencing changes. It activates only for leagues whose mode flag enables it; the shared Weekend Championship is unaffected.

### G5.1 — Exclusive mode data model and gating

- **Outcome**: a league can be created in exclusive mode with its stricter invariants enforced everywhere.
- **Build**: mode flag in league settings, domain rules INV-4/INV-12 (one squad per player per league per month; spend ≤ budget) wired into the roster validator, UI gating and copy.
- **Verify**: conformance tests proving exclusive invariants only bind in exclusive leagues; shared-league regression suite still green.
- **Depends**: G2.5, G3.6

### G5.2 — Auction model and rules

- **Outcome**: lots, offers, and soft-close windows are first-class, rule-checked aggregates.
- **Build**: lot lifecycle (draft → open → soft-close → closed → paid), offer/bid aggregates, counter-offers, resell window, `auction.bid` entrypoint rules in the bundle (highest offer + open window, budget check, no double ownership), rejection returning named rule traces.
- **Verify**: bundle conformance suite; invariant unit tests (INV-10/11/12 equivalents).
- **Depends**: G5.1, G1.10

### G5.3 — Serializable bid submission

- **Outcome**: concurrent bids can never double-award or overdraw a budget.
- **Build**: bid endpoint running in SERIALIZABLE isolation with `SELECT ... FOR UPDATE` on lot and manager-budget rows, rule verdict + proof certificate per accepted bid, idempotency keys, serialization-failure retry with bounded backoff, full bid event log.
- **Verify**: concurrency suite (many processes, overlapping last-second bids) asserting exactly one winner per lot and never over budget; serialization-retry liveness test; stored proof re-checked offline with `eyeron --check-proof` in a separate process; nixosTest multi-node serialization case activated.
- **Depends**: G5.2, QA-1

### G5.4 — Soft-close timers

- **Outcome**: the auction window extends deterministically under last-second pressure.
- **Build**: server-time-authoritative deadlines (served time endpoint + `Date` header offset), configurable soft-close window, extension on qualifying late bids, broadcast of revised deadlines.
- **Verify**: integration tests with a controllable clock: low bid at deadline−1s extends once and only once; timer math property tests; client clock-skew test (bogus client clock cannot win time).
- **Depends**: G5.3, G3.7

### G5.5 — Auction UI and outbid notifications

- **Outcome**: managers can bid, be outbid, counter, and accept — with every state visible.
- **Build**: lot cards with live counts, server-time timer (last 500ms of the countdown disables the button to absorb clock skew), outbid push + in-app notification, counter-offer flow, accept flow for organizers, SSE auction event stream.
- **Verify**: E2E flows: bid → outbid (push+in-app captured) → counter → accept; rejected bid shows the named rule inline; axe checks.
- **Demo**: two browsers race for one lot with a visible winner.
- **Depends**: G5.4, G3.5, G3.8

### G5.6 — Resell phase

- **Outcome**: won players can be resold between managers inside the monthly window.
- **Build**: resell window rules, manager-to-manager sales with rule checks, both squads updated atomically, event history.
- **Verify**: E2E resell accepted and rejected (rule named); atomicity test on concurrent resell + roster edit.
- **Depends**: G5.5

### G5.7 — Offline dispute re-verification

- **Outcome**: any bid can be re-audited years later with no running services.
- **Build**: archived snapshot tooling (bundle + input facts + proof certificate per decision) plus a standalone checker invoking `eyeron --check-proof`, admin UI linking a disputed bid to its certificate and re-check result.
- **Verify**: checker re-verifies a stored proof from an archived DB dump on an isolated process; tampered proof fails loudly; CLI documented in the runbook.
- **Depends**: G5.3

### G5.8 — Auction performance and fairness gate

- **Outcome**: the auction holds its guarantees under the documented load.
- **Build**: performance suite — 50 concurrent bids across 20 lots, p95 <1s, no lost updates, no overdraw; fairness report generation.
- **Verify**: perf suite green in CI; report archived per release.
- **Depends**: G5.5

**Checkpoint 5** — client runs the demo: two clients racing a lot, outbid push, offline re-verification of the winning bid.

### QA-3 — Final quality audit

- **Outcome**: the codebase and docs are professional-grade before handover.
- **Scope**: clean-code conformance across all crates and web (metrics gates as CI), architecture layering check (core has no I/O; only api/jobs/notify know HTTP/DB/clock), docs current-state check against implemented behavior, test pyramid balance review, dead-code and unused-dependency sweep (`cargo machete`/`knip`), final format/dependency tidy.
- **Verify**: report in `docs/audits/QA-3.md`; findings closed; CI gates tightened to prevent regressions.
- **Depends**: G5.8, SA-3

## 10. Cross-cutting gates (apply to every goal)

- **Before merge**: fmt, clippy (strict), unit + integration + contract tests, coverage ≥80% lines / 100% rules and invariants, `cargo audit`, `npm audit`, trivy on images, semgrep ruleset, gitleaks, authz matrix, `just rules` conformance + proof checks.
- **Every behavior change**: docs in the same PR (current state only, no diff-anchored prose), CHANGELOG entry, OpenAPI client regenerated and type-tested.
- **Every new dependency**: separate PR with justification, last-release/maintainer/download check, lockfile update, and watchdog row.
- **Rule changes**: expected-file updates in the bundle in the same PR, proof regeneration, `just rules` green.
- **Secrets**: never in code or tests; tests use env injection or ephemeral credentials.

## 11. Sequencing and parallelism notes

- Hard chains: G0.1 → nearly everything; G1.10 → G2.2/G2.5 → G3.5; G2.8 → G3.7 → G4.5.
- Safe parallelism: G1.2 ⊗ G1.3 ⊥ G1.4 (after G1.1); G2.1/G2.3 ⊥ G2.4; G3.1 ⊥ G2.x; G4.1/G4.4 ⊥ G4.5. Phase 5 cannot start before QA-1 because its fairness gate reuses the concurrency harness proven there.
- audits never block feature branches but always block the checkpoint that follows them.

## 12. Explicitly not planned (YAGNI)

Carried from 00/16: native mobile apps, real-money or betting features, multi-tenant federation of leagues, video/streaming, ML-based predictions, non-Italian federation sources. Push notifications exist only for the categories defined in G3.8 and G5.5.

## 13. Risk mitigation ownership

Each risk in 17-risks.md maps to goals that retire it, and is re-checked at each audit: R1 → G1.4/G1.5 drift tests + SA-1; R2 → G1.2 + SA-1; R3 → G1.6 + G4.3 queue; R4 → benchmark in G2.7 (hot aggregations in SQL, KG projection) + QA-1; R5 → G1.10 proofs + G5.7 offline checker; R6 → version pinning in G0.1 and watchdog in G0.8; R7 → ingest telemetry in G4.5. P1 → G3.4 breakdown view; P2 → G5.3/G5.4/G5.7; O1 → G4.6; O3 → G4.4; O4 → watchdogs + soak in G4.9.

## 14. Roadmap changelog

| Date | Change | Reason |
|---|---|---|
| 2026-11-11 | Roadmap created: docs/16-roadmap.md phases decomposed into 54 atomic goals (G0.1–G5.8) plus three security and three quality audits; auction mode moved from Phase 3 to a deferred Phase 5 epic | Client instruction: exclusive ownership mode is low priority to be built later, while its technical requirements stay unchanged |
