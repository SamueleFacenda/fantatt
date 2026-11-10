# System Architecture

## 1. C4 Context

```mermaid
C4Context
    title Fantatt - System Context

    Person(user, "Manager", "Table tennis fan playing a fantasy league")
    Person(admin, "League Admin", "Creates leagues, manages members and settings")
    Person(superadmin, "Superadmin", "Platform operator")

    System(fantatt, "Fantatt", "Fantasy table tennis platform: PWA + Rust API + reasoning engine")
    System(fitet, "FITET Results Portal", "portale.fitet.org - the only source of real results")
    System(google, "Google Identity", "OAuth2 / OIDC login, profile picture")
    System(mail, "Mail Provider", "Pluggable: SMTP or HTTP API (Postmark, Resend, SES)")
    System(rp, "Relying Party Authenticator", "Device passkeys: Face ID, fingerprint, security key")

    Rel(user, fantatt, "Uses over HTTPS", "PWA")
    Rel(fantatt, fitet, "Scrapes results, players, ratings", "HTTPS + HTML")
    Rel(fantatt, google, "Verifies ID tokens, reads profile", "HTTPS + OIDC")
    Rel(fantatt, mail, "Sends invitations, magic links, notices", "SMTP/HTTPS")
    Rel(user, rp, "Passkey ceremony", "WebAuthn")
    Rel(fantatt, rp, "Verifies assertions", "WebAuthn")
    Rel(admin, fantatt, "Manages league", "HTTPS")
    Rel(superadmin, fantatt, "Manages platform", "HTTPS")
```

## 2. C4 Container

```mermaid
C4Container
    title Fantatt - Containers

    Person(user, "Manager")

    System_Boundary(cdn, "Edge") {
        Container(sw, "Service Worker", "PWA cache, offline shells, push")
        Container(spa, "PWA Shell", "React SPA, routes, forms, installability")
    }

    System_Boundary(app, "Fantatt Platform") {
        Container(api, "API Gateway", "Rust / axum", "Authn, authz, CSRF, OpenAPI, rate limit")
        Container(domain, "Domain Services", "Rust crates", "Leagues, auctions, rosters, scoring")
        Container(reason, "Reasoning Engine", "Rust + embedded RDF store", "Game rules, derivation, verification")
        Container(ingest, "Ingest Pipeline", "Rust workers", "Scrape, normalise, reconcile, publish")
        Container(notify, "Notification Service", "Rust", "Web push, in-app, email fanout")
        Container(jobs, "Scheduler", "Rust + Postgres queue", "Cron, retries, backfill, dead-letter")
    }

    System_Boundary(data, "Data Plane") {
        ContainerDb(pg, "PostgreSQL", "Primary transactional store + queue + LISTEN/NOTIFY")
        ContainerDb(kg, "Knowledge Graph", "RDF store (Sled or Postgres-backed)", "Game state as triples")
        ContainerDb(cache, "Redis-compatible", "Sessions, rate limit, hot standings")
        ContainerDb(obj, "Object Storage", "MinIO / S3-compatible", "Avatars, exports, archives")
    }

    System_Ext(fitet, "FITET Portal", "Scraped source")
    System_Ext(otel, "OTel Collector", "Traces, metrics, logs")
    System_Ext(graf, "Grafana Stack", "Dashboards, alerts, logs, traces")

    Rel(user, sw, "HTTPS")
    Rel(sw, spa, "Cache-first")
    Rel(spa, api, "JSON + OpenAPI", "HTTPS, cookie session")
    Rel(api, domain, "In-process calls")
    Rel(domain, reason, "Rule evaluation")
    Rel(domain, kg, "SPARQL / triples")
    Rel(domain, pg, "SQL")
    Rel(domain, cache, "Session, rate limit")
    Rel(ingest, fitet, "HTML scrape")
    Rel(ingest, reason, "Assert facts")
    Rel(ingest, pg, "Watermark, raw HTML archive")
    Rel(jobs, ingest, "Triggers scrapes")
    Rel(jobs, notify, "Enqueues notifications")
    Rel(notify, user, "Web push")
    Rel(api, otel, "OTLP gRPC")
    Rel(domain, otel, "OTLP")
    Rel(ingest, otel, "OTLP")
    Rel(otel, graf, "Exports")
```

## 3. Component decomposition

### 3.1 Workspace crates

| Crate | Responsibility | Depends on |
| --- | --- | --- |
| `fantatt-core` | Domain types, invariants, port traits. No I/O. | none |
| `fantatt-reasoning` | Rule engine, RDF store binding, verification queries | `fantatt-core` |
| `fantatt-scoring` | Pure scoring functions over derived facts | `fantatt-core` |
| `fantatt-ingest` | FITET scraper, HTML parser, normaliser, reconciler | `core` |
| `fantatt-api` | axum handlers, middleware, OpenAPI, DTOs | all of the above |
| `fantatt-jobs` | Scheduler, queue workers, backfill | `ingest`, `core` |
| `fantatt-notify` | Web push (VAPID), in-app, email facade | `core` |
| `fantatt-obs` | OTel wiring, metric definitions, log redaction | none |
| `fantatt-config` | Layered config load + validation + schema | none |

### 3.2 Data plane responsibilities

| Store | Why | Alternatives |
| --- | --- | --- |
| PostgreSQL | Transactions, serialisable isolation, advisory locks, `LISTEN/NOTIFY` — the backbone of correctness for auctions and a job queue. Battle-tested, cheap, no vendor. | None recommended for the transactional role. |
| Embedded RDF store | The knowledge graph itself: game state as triples, SPARQL for derivation, closed-world checks for validation. | Oxigraph (also Rust, SPARQL 1.1) — see ADR-002. |
| Redis / Valkey compatible | Sessions, rate limits, hot standings, pub/sub for live scores. One process, optional. | Dragonfly, KeyDB, in-process `moka` for single-node. |
| Object storage | Avatars, exports, raw response archive. | S3, MinIO, R2, filesystem. |

## 4. Key interactions

### 4.1 Ingest: real-world match to verified game state

```mermaid
sequenceDiagram
    participant S as Scheduler
    participant W as Ingest Worker
    participant F as FITET Portal
    participant N as Normaliser
    participant K as Knowledge Graph
    participant R as Reasoning Engine
    participant PG as Postgres
    participant B as Notifier

    S->>W: tick (every 5 min, jittered)
    W->>PG: read watermark + registered competitions
    W->>F: fetch calendar + giornata pages
    F-->>W: HTML
    W->>N: parse to canonical MatchRecord
    N->>N: validate schema, rating sanity, set-count
    W->>PG: UPSERT raw record + archive HTML (idempotent)
    W->>K: ASSERT triples (match, players, teams, event)
    W->>R: re-derive affected facts
    R-->>K: materialise: played, absent, teamWin, doublet
    R->>R: verify constraints (SHACL-like shapes)
    W->>PG: mark matchday complete when all matches present
    W->>B: emit MatchdayFinalised
    B->>B: recompute affected league standings
```

**Idempotency:** every ingest step is keyed by `(source, source_id, content_hash)`.
Re-running a scrape with unchanged content asserts nothing new. This is what makes
"results as soon as possible" safe: we can scrape every 5 minutes.

### 4.2 Auction: strict high-concurrency correctness

```mermaid
sequenceDiagram
    participant M as Manager (browser)
    participant A as API
    participant R as Reasoning Engine
    participant PG as Postgres
    participant S as Realtime Bus

    M->>A: POST /leagues/{id}/auction/bids
    A->>PG: BEGIN TRANSACTION ISOLATION LEVEL SERIALIZABLE
    A->>PG: SELECT ... FOR UPDATE (auction window, player lot)
    A->>R: evaluate auctionRules (budget, deadline, exclusivity, ownership)
    alt Rule violation
        R-->>A: violation + rule trace
        A->>PG: ROLLBACK
        A-->>M: 422 + machine-readable rule trace
    else Accepted
        A->>PG: INSERT bid, bump lot revision
        A->>PG: COMMIT
        A->>S: publish BidAccepted (lot revision)
        A-->>M: 201 + new lot state
    end
    S-->>M: SSE push to all league subscribers
```

Two-phase anti-overbidding:

```mermaid
sequenceDiagram
    participant A as Auction State Machine
    participant M1 as Manager A
    participant M2 as Manager B
    A->>M1: OPEN (deadline T)
    M1->>A: offer 100
    Note over A: soft-close window opens, deadline → T + window
    M2->>A: offer 110 (at T - 10s)
    A->>M1: counter-offer or allow (deadline moved)
    M1->>A: allow (T + 5s)
    A->>A: SOLD at 110, window closed
```

### 4.3 Roster submission and lock

```mermaid
stateDiagram-v2
    [*] --> Open: month starts
    Open --> Locked: submitted (or deadline reached)
    Locked --> Scoring: matchday(s) played
    Scoring --> Settled: all matchday results verified
    Settled --> Open: next month
    Locked --> Open: admin unlocks (audited)
```

Roster submission is a **transactional validation**: the API calls the reasoning
engine with the whole candidate roster and the current competition facts; only a
`VALID` verdict commits. Violations return the exact rule and offending player,
so the UI can highlight the problem inline.

### 4.4 Live scores to the browser

```mermaid
sequenceDiagram
    participant F as FITET Portal
    participant W as Ingest
    participant K as Knowledge Graph
    participant PG as Postgres
    participant A as API (SSE hub)
    participant U as Browser/PWA

    W->>K: assert new matches
    W->>PG: NOTIFY matchday_updated
    PG->>A: LISTEN receives payload
    A->>A: compute delta vs client revision
    A->>U: SSE event {leagueId, revision, matchdayScores}
    U->>U: service worker updates cache + shows update banner
    U->>U: web push "3 new results in your league"
```

## 5. Deployment topologies

### 5.1 Self-host (reference)

```mermaid
flowchart TB
    subgraph host["One NixOS host"]
        direction TB
        subgraph net["nginx (TLS, HTTP/2)"]
            proxy["reverse proxy + static assets"]
        end
        subgraph svc["systemd units"]
            api["fantatt-api"]
            jobs["fantatt-jobs"]
            ingest["fantatt-ingest"]
            notify["fantatt-notify"]
        end
        subgraph stores
            pg["postgresql.service"]
            rs["valkey.service"]
            minio["minio.service"]
            otel["otelcol.service"]
        end
    end
```

### 5.2 Cloud / serverless

```mermaid
flowchart LR
    cdn["CDN + WASM edge cache"] --> lb["HTTPS LB"]
    lb --> fn["Lambda / container (API + worker, autoscaled)"]
    fn --> pg["Managed Postgres"]
    fn --> kg["Embedded KG on shared volume / S3 snapshots"]
    fn --> obj["Object storage"]
```

The API binary is identical in both. Only the config file and the process
supervisor differ. See `14-deployment.md`.

## 6. Trust boundaries

| Boundary | Threat controlled |
| --- | --- |
| Browser ↔ API | XSS, CSRF, clickjacking, session theft |
| API ↔ Reasoning | Rule tampering; rules are versioned and content-addressed |
| Ingest ↔ FITET | HTML injection, malformed data, rate-limit bans, source drift |
| Workers ↔ DB | Job poisoning, unbounded retries |
| Admin ↔ platform | privilege escalation, unaudited state changes |
