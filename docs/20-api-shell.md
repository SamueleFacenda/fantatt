# API Shell

Status: active | Refines: [07-api-spec.md](07-api-spec.md), [10-security.md](10-security.md) | Audience: development agents, client

This document describes the implemented HTTP surface of G0.6: the routes the
shell answers, the middleware between the socket and the handlers, and the
configuration it reads. It covers current state only.

## 1. Routes

| Path | Method | Behavior |
| --- | --- | --- |
| `/healthz` | GET | 200 `{"status":"ok"}`; mints no session |
| `/auth/csrf` | GET | 200 `{"token":"<64 hex>"}` plus `Set-Cookie: __Host-session`; mints a session when the request carries none |
| `/auth/session` | DELETE | 204 when the `X-CSRF-Token` matches the session, 403 problem+json otherwise; destroys the session |
| `/api/v1/csp-report` | POST | 204 for a well-formed `{"csp-report": {...}}` body, 400 problem+json for a malformed one; CSRF-exempt because browsers report without a token |
| `/openapi.json` | GET | The derived OpenAPI 3.1 document |

Every other path answers 404 problem+json, and a known path with the wrong
method answers 405 problem+json. The router is built in
`crates/fantatt-api/src/routes/mod.rs`.

## 2. Middleware stack

Applied outermost first; route-level middleware sits inside the session layer
so it can read the session:

1. `attach_request_id` — echoes a well-formed client `X-Request-Id`,
   generates 32 hex characters otherwise, logs one line per request, emits
   the response header.
2. `enforce_problem_envelope` — any error status that is not already
   problem+json is converted, carrying a `Retry-After` through.
3. `attach_security_headers` — per-response CSP with a fresh nonce (the
   directives of docs/10-security.md §3 byte-exact, nonce placeholders
   filled), `Strict-Transport-Security: max-age=63072000; includeSubDomains`,
   `X-Content-Type-Options: nosniff`. Handlers read the nonce from the
   `ResponseNonce` request extension.
4. The session layer — `SessionManagerLayer` over the Postgres store with
   the configured cookie (`__Host-session`, HttpOnly, Secure, SameSite=Lax,
   Path=/), `OnInactivity` at `session.idle_seconds`, `always_save`.
5. Route-level, inside the session layer:
   - `enforce_absolute_lifetime` — destroys a session older than
     `session.absolute_seconds` (see ADR-0010 decision 3);
   - `enforce_rate_limits` — governor keyed limiters per parsed budget;
   - `reject_oversized_bodies` — a declared `Content-Length` above
     `server.request_body_limit_bytes` is refused before the body is read;
   - `DefaultBodyLimit` — the hard cap for bodies that stream without a
     declared length;
   - `normalize_idempotency_key` — refuses a malformed key, attaches a
     well-formed one to request extensions;
   - the CSRF guard on `DELETE /auth/session` only.

## 3. Error envelope

Every error response is `application/problem+json` with `type`, `title`, and
`status`; `code`, `rule`, and `details` join only when a rule verdict needs
them (docs/07-api-spec.md §6). The named kinds live in
`crates/fantatt-api/src/error.rs`; the conformance test enumerates them.

## 4. Sessions

Server-side records in `tower_sessions.session`, cookie holding the opaque
session id. Thirty days sliding (each request renews), twelve hours absolute.
CSRF tokens are HMAC-SHA256 over the session id with `crypto.session_secret`,
recomputed per request, so rotation invalidates outstanding tokens. See
ADR-0010 for the store and expiry decisions.

## 5. Rate limits

Budgets come from the `rate_limit.rules` configuration map, parsed as
`<count>/<unit> per <key>`; keys are `ip` or session-scoped
(`session`/`user`/`account`/`manager`). Route mapping: `/auth/*` gets the
`auth` and `auth-account` budgets, `/players?search=` gets `search`,
everything else gets `reads`. An exhausted budget answers 429 problem+json
with `Retry-After`. A session-scoped budget applies only once a session
exists. Limiters are in-process per node.

## 6. OpenAPI

`ApiDoc` in `crates/fantatt-api/src/openapi.rs` derives the document from the
annotated handlers. The committed snapshot `crates/fantatt-api/openapi.json`
is compared byte-for-value by the snapshot test, and regenerated with:

```sh
nix develop -c cargo run -p fantatt-api --example openapi_snapshot > crates/fantatt-api/openapi.json
```

An undocumented response-shape change fails the test.

## 7. Running and demoing

```sh
just db-migrate          # migrate the dev database
just api-serve           # serve the shell against it (needs FANTATT__DATABASE__URL,
                         # which the recipe sets; a CSRF secret falls back to a
                         # dev-only value when unset)
just api-demo            # curl the health check, a CSRF-rejected logout, a
                         # session with its token, and the OpenAPI document
```

The binary loads configuration through `fantatt-config` and aborts fast on a
missing secret or a malformed rate rule.

## 8. Tests

`crates/fantatt-api/tests/` boots an ephemeral Postgres per test (never the
dev database): `api_shell.rs` (contract, fallbacks, error battery, OpenAPI
snapshot), `security_headers.rs` (headers, nonce rotation, cookie flags),
`csrf.rs` (accept and reject matrix, logout destroys the session),
`rate_limit.rs` (budget exhaustion to 429), `session_flow.rs` (token stays
bound to the session through the real store), `sessions.rs` (store round
trip, ADR-0009 decision 3).