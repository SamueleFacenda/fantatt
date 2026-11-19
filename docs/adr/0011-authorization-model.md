# ADR-0011: Authorization model and the declaration guard

- Status: accepted
- Date: 2026-11-18
- Goal: G0.7 (Authorization matrix harness)

## Context

G0.7 turns the authorisation principles of docs/10-security.md §4 into code:
a capability model, route declarations carrying a capability and an object
scope, an exhaustive role × route test, and a default-deny guard that rejects
undeclared routes. Three facts shaped the decisions:

- docs/07-api-spec.md §13 attributes the role definitions to `fantatt-core`,
  but docs/11-repo-structure.md §3 forbids HTTP and database code there, while
  enforcement needs the session, the pool, and the request path.
- axum 0.8.9 exposes no API to enumerate the routes of a live `Router`
  (verified against the crate source), so "adding an endpoint without declaring
  its capability fails the build" cannot be proven by inspecting a router at
  test time.
- docs/10-security.md §4 requires default deny and object-level checks on
  every id, with the database as the role source of truth and the superadmin
  list held in configuration.

## Decisions

### 1. The policy is pure in `fantatt-core`, the enforcement is middleware in `fantatt-api`

`crates/fantatt-core/src/authz.rs` holds the roles (DB-parity spellings),
capabilities (wire names), object scopes, the resolved principal, and the
default-deny `authorize` function: serde-derivable value types with no I/O,
tested table-driven and with proptest totality properties. `fantatt-api` holds
the declarations, the table lookup, the principal resolution (session id,
`fantatt.memberships`, configuration grant list), and the middleware that maps
a denial to problem+json. `fantatt-api` gains `fantatt-core` as a direct
dependency, which the layering of docs/11-repo-structure.md §3 already permits.

### 2. Registration requires a capability at compile time

`authz::get`/`post`/`put`/`delete` accept a handler and a guard — a capability
plus an object scope — and `DeclaredRouter` records one declaration per route.
There is no registration path without a guard, so an undeclared endpoint does
not compile; the five shell routes all declare `public`.

### 3. The runtime guard and a negative-control test carry the rest

Because axum 0.8.9 cannot enumerate a live router, the authorize middleware
also denies any matched route missing from the declaration table
(`ROUTE_UNDECLARED`), and the seeded integration test registers a raw axum
route beside the declared ones and asserts it answers 403 for every caller,
superadmin included. The compile-time requirement, the runtime guard, and that
test together are the guarantee; review discipline closes the remaining gap of
a route registered raw on purpose.

### 4. The superadmin grant is configuration, memberships carry league roles

The platform grant comes from `[authz] superadmins` (docs/12-configuration.md
§2), as docs/10-security.md §4 requires. League roles come from
`fantatt.memberships` with `state = 'ACTIVE'`, never from the client. A
membership row with role `SUPERADMIN` also carries the platform grant inside
its league, because the migration set's check constraint admits that spelling;
the configuration list remains the source for platform-scoped routes.

### 5. Denials log a structured event; audit rows are deferred

docs/10-security.md §4 draws audit events on the deny path. Writing an
`fantatt.audit_log` row requires the append-only hash-chain computation, which
belongs to the audit goal (G2.4); G0.7's Verify does not mandate the rows. The
middleware logs one `tracing::warn` per denial, naming the problem code.

## Consequences

- `crates/fantatt-api/openapi.json` is unchanged: every shell route is
  `public`. Per-route 403 responses join the document with the first
  capability-gated route (G1.12).
- `MembershipRole` (owner/manager) stays as it is; reconciling it with the
  five-role capability model waits for G2.4, recorded here rather than
  silently duplicated.
- The declaration table rides the outermost route layer as a request
  extension (`Arc<RouteTable>`), because axum wraps routes as layers are
  applied and the middleware must read the table after the session layer ran.
- Object scope resolution zips `MatchedPath` against the request path in a
  pure function, unit-tested apart from the HTTP surface.