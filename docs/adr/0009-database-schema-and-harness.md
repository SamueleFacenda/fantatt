# ADR-009: Database schema layout and the ephemeral Postgres harness

- **Status:** Accepted
- **Date:** 2026-11-16
- **Verified:** sqlx 0.9.0, sqlx-cli 0.9.0 (both in the pinned nixpkgs rev `4975466`), `postgresql` 18.6 (nixpkgs default at that revision) from the same input, `tower-sessions-sqlx-store` 0.15.1

## Context

G0.5 introduces the full relational model, the migration toolchain, and the
test harness every DB-backed goal from G1.3 onward depends on. Three questions
had no answer in the existing docs: where migrations live, how tests get a
fresh Postgres, and who owns the session table — especially because
`tower-sessions-sqlx-store` 0.15.1 resolves against sqlx **0.8.6** while the
workspace uses sqlx **0.9.0**, so the two `PgPool` types are distinct and the
store runs its own migration at startup.

## Decision

1. **Migrations live in a top-level `migrations/` directory** as numbered,
   **reversible** pairs `<version>_<name>.up.sql` / `.down.sql`. The layered
   dependency rule (`api → jobs → ingest → core`) has two crates that know the
   database, so a per-crate migrations folder would duplicate the schema.
   Crates embed the set through the compile-time `sqlx::migrate!` macro with a
   relative path.
2. **A dev-only harness crate, `fantatt-test-support`,** gives every
   database-backed test its own Postgres: it shells out to `initdb` and
   `postgres` from the dev shell, listens on a unix socket in a temp directory,
   applies the migrations, and tears the instance down when the test ends. No
   container runtime, no CI service container, no shared port.
3. **`tower_sessions.session` is mirrored in the migration set** with the exact
   DDL the store creates (`id text primary key`, `data bytea`,
   `expiry_date timestamptz`). The store's `create table if not exists` then
   no-ops, so one reversible migration set owns the whole schema. G0.6 adds a
   round-trip test against the store itself, which is where the 0.8/0.9 split
   becomes visible.
4. **sqlx-cli 0.9.0 and PostgreSQL 18.6 (nixpkgs default) enter the dev
   shell.** The offline query
   data in `.sqlx/` is committed and checked by `just db-offline-check` (part
   of `just check`), so builds stay hermetic: `cargo check` never needs a live
   database.

## Alternatives

| Option | Why rejected |
|---|---|
| Per-crate migrations directories | `fantatt-api` and `fantatt-jobs` both need the schema; two owners drift apart |
| Plain (non-reversible) migrations | G0.5 Verify demands migrate → rollback → migrate on a scratch DB; sqlx-cli's revert needs `.down.sql` |
| testcontainers / pg-embed for tests | Docker daemon in dev and CI, or another external dependency needing a watchdog row |
| Let the session store own its table | Half the schema would exist only after first boot; rollback coverage breaks |

## Consequences

- `nix develop` gains `postgresql` and `sqlx-cli`; the dev database lives in
  `.dev/postgres` on loopback:5433 with trust auth and is git-ignored.
- `pgcrypto` is enabled inside the `fantatt` schema for the audit-log hash
  chain; production provisioning (G4.6) must bootstrap it as superuser.
- The audit log is append-only at the database level (UPDATE/DELETE/TRUNCATE
  refused, chain computed by a BEFORE INSERT trigger), matching
  docs/10-security.md §11.
- The role check constraint on `memberships.role` uses the capability model of
  docs/10-security.md (superadmin, league_admin, collaborator, manager,
  member); `fantatt-core`'s narrower `MembershipRole` enum is extended in G0.7
  to the same set.
- The sqlx 0.8/0.9 split is recorded here as an accepted risk with a concrete
  re-check: G0.6 must pass a session save/load round-trip through the real
  store before the split can be called harmless.

## Verification status

- Verified 2026-11-16: `migrations/` applies and reverts cleanly through the
  harness; constraint tests for INV-1, INV-4, INV-6, INV-12 and the audit
  append-only rule run in `crates/fantatt-api/tests/db_schema.rs`; `.sqlx`
  offline data committed and checked by `just db-offline-check`.