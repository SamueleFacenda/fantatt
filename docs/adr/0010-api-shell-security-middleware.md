# ADR-0010: API shell security middleware

- Status: accepted
- Date: 2026-11-17
- Goal: G0.6 (API shell and security middleware)

## Context

G0.6 builds the hardened, empty HTTP surface: an axum app answering health
checks and rejecting everything else, with session, CSRF, header,
error-envelope, rate-limit, and OpenAPI machinery that later goals reuse.
Four decisions had to be made because the obvious choices conflict with the
workspace pins or with facts verified against the upstream crates.

## Decisions

### 1. The session store is hand-rolled on the workspace sqlx

`tower-sessions-sqlx-store` 0.15.1 resolves against sqlx 0.8.6 while the
workspace uses sqlx 0.9.0. Adopting the crate would compile two sqlx majors
into one binary and would need a second, alias-named pool just for sessions.
The store is instead implemented in `crates/fantatt-api/src/sessions.rs` on
sqlx 0.9, against the `tower_sessions.session` mirror table of the migration
set (ADR-0009), using compile-time-checked queries and the committed `.sqlx`
offline data. `tower-sessions-sqlx-store` is dropped from `fantatt-api`; the
migrations keep the table. The round-trip obligation of ADR-0009 decision 3
is met by `crates/fantatt-api/tests/sessions.rs` and
`crates/fantatt-api/tests/session_flow.rs`.

Consequence: the session cookie holds tower-sessions' 128-bit `Id`
(base64url, 22 characters), not the 256-bit id that docs/10-security.md §1.4
names. 128 bits of cryptographic randomness still exceeds the OWASP minimum;
the wording divergence is recorded here rather than papered over.

### 2. CSRF is HMAC-SHA256 in-house, not `axum_csrf`

docs/10-security.md §2 names `axum_csrf` 0.12.0. That crate does not exist in
the workspace dependency graph, and pulling an unreviewed crate to reimplement
40 lines would be supply-chain theatre. The token is
`hex(HMAC-SHA256(crypto.session_secret, session_id))`, recomputed per request,
so every session rotation invalidates outstanding tokens with no cleanup.
The header name comes from configuration (`csrf.header`).

### 3. The twelve-hour absolute cap is middleware, not configuration

tower-sessions 0.15 offers `OnInactivity`, `OnSessionEnd`, and `AtDateTime`
only: there is no absolute-lifetime variant. The creation timestamp rides in
the session data under `started_at`, and the session-lifetime middleware
destroys any session older than `session.absolute_seconds` (12h by default).
The thirty-day sliding window is the layer's `OnInactivity` expiry with
`always_save`, so every request renews it.

### 4. Rate limits run in-process per node

governor 0.10.4 keyed limiters back the budgets of docs/07-api-spec.md §14,
parsed from the `rate_limit.rules` configuration strings. The Valkey/Redis
multi-node store of docs/10-security.md §6 waits for its own dependency
decision; the in-process limiter is the documented single-node fallback and
is what ships. A session-scoped budget applies only once a session exists,
so anonymous visitors never share one bucket.

## Consequences

- CSP nonces, request ids, and the CSRF key use `rand`, `hmac`, `sha2`, and
  `async-trait` promoted to direct dependencies in a separate PR; all four
  already resolved in `Cargo.lock`.
- The CSRF issuance route (`GET /auth/csrf`) is the only route that mints a
  session; anonymous visitors of other routes mint none.
- `POST /api/v1/csp-report` is exempt from the CSRF guard: browsers report
  violations without a token and the route changes no state.
- Deferred with this ADR: session rotation on login and role change (G3.2,
  when authentication exists), Idempotency-Key replay persistence (G2.4,
  the first creating endpoint), response compression (br/gzip of
  docs/07-api-spec.md §1), and the `report-to` CSP directive (needs served
  HTML, G3.x).