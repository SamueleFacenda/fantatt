# Database Schema

Status: current | Audience: developers, operators

PostgreSQL is the transactional backbone (ADR-003). The knowledge graph is a
derived projection rebuilt from the ingest ledger; this document describes the
authoritative relational model implemented by `migrations/` (layout and
decisions in ADR-009).

## 1. Conventions

- Identifiers follow the ULID wire convention of `07-api-spec.md`: a lowercase
  prefix from `fantatt-core::ids::DomainPrefix` plus a 26-character Crockford
  base32 body. A `CHECK` constraint (`fantatt.assert_prefixed_id`) rejects
  malformed values at the database boundary. Internal rows that never reach the
  API use the same body without a prefix (`fantatt.assert_ulid`).
- Timestamps are `timestamptz`; dates are calendar dates. The wire format is
  RFC 3339 UTC.
- Money is an ISO 4217 currency code plus integer minor units, matching
  `fantatt-core::money::Money`.
- Every table carries `created_at`; mutable tables carry `updated_at`
  (application-maintained).

## 2. Migration workflow

Migrations are numbered, reversible pairs:

```
migrations/202611160001_identity_accounts.up.sql
migrations/202611160001_identity_accounts.down.sql
```

| Command | Purpose |
|---|---|
| `just db-start` / `just db-stop` | Local dev Postgres in `.dev/postgres`, loopback:5433, trust auth |
| `just db-migrate` | Apply pending migrations to the dev database |
| `just db-reset` | Rebuild the dev database and migrate |
| `just db-prepare` | Regenerate the committed offline query data (`.sqlx/`) |
| `just db-offline-check` | Fail when `.sqlx/` no longer matches the schema (part of `just check`) |

Integration tests never touch the dev database: `fantatt-test-support` boots a
private `initdb` instance per test on a unix socket in a temp directory, so DB
tests need no service, container, or network port.

## 3. Tables

| Area | Tables | Notes |
|---|---|---|
| Accounts | `users`, `credentials`, `preferences`, `invitations` | Passkeys and Google live in `credentials`; `invitations` carries a token hash, never the token |
| Sessions | `tower_sessions.session` | DDL mirrors `tower-sessions-sqlx-store` (ADR-009) |
| Identity | `players`, `player_ratings`, `identity_aliases`, `identity_ambiguity_queue` | Ratings are bitemporal; ambiguous identities are queued, never auto-merged |
| Competitions | `teams`, `competitions`, `matchdays`, `ties`, `appearances` | `appearances` is the input surface of the scoring engine (INV-13) |
| Leagues | `leagues`, `memberships`, `squads`, `slots`, `submissions` | `submissions` stores the rule trace and proof reference of each commit |
| Auctions | `lots`, `offers`, `bids` | Placeholders until G5.2 refines the aggregates |
| Tournaments | `tournaments`, `predictions` | Bracket shape lives in `tournaments.bracket` (G1.11 fills it) |
| Ingest | `ingest_ledger`, `ingest_jobs`, `quarantine` | Ledger is keyed by `(source, source_id, content_hash)`; documents that violate SHACL shapes are parked, never repaired |
| Notifications | `notifications`, `preferences` | Per-category channel matrix |
| Audit | `audit_log` | Append-only, hash-chained, queryable by actor, league, action; never pruned |

## 4. Invariants enforced by the database

| Invariant | Enforcement |
|---|---|
| INV-1: one rating per player per source per `asOf` | Unique index on `player_ratings (player_id, source, as_of)` |
| INV-2: exactly 15 slots (2 starters + 1 bench per tier) | Unique index on `slots (squad_id, tier, position)`; composition rules run in the rule bundle |
| INV-3/INV-9: tier matching and substitution | Rule bundle `roster.validate` (G1.10); the tier check exists on `slots.tier` |
| INV-4: exclusive ownership per league per month | `slots_enforce_exclusive_player` trigger refuses a second squad in an exclusive league for the same month |
| INV-6: no duplicate player within a squad | Partial unique index on `slots (squad_id, player_id)` |
| INV-12: spend never exceeds budget | `bids_enforce_budget` trigger sums non-refunded bids against `leagues.budget_minor` |
| INV-15: purge preserves league history | Soft-delete columns (`users.deleted_at`) plus tombstones; the purge job lands in G4.4 |
| Audit append-only (docs/10-security.md §11) | `audit_log_append_only` trigger refuses UPDATE/DELETE/TRUNCATE; `audit_log_chain_entry` hashes each entry onto its predecessor |

Rules that need derivation semantics — scoring, roster legality, standings —
stay in the rule bundles (G1.10) and are re-checked from proof certificates.
The database enforces what must be true of every writer, including ones that
bypass the API.

## 5. Security notes

- `pgcrypto` is installed inside the `fantatt` schema for the audit hash chain.
- `UPDATE`, `DELETE` and `TRUNCATE` are revoked from `PUBLIC` on `audit_log`.
- Session data lives server-side only; the cookie holds an opaque id
  (docs/10-security.md §1.4).