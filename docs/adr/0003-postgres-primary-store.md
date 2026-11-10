# ADR-003: PostgreSQL as the transactional backbone

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** sqlx 0.9.0 (2026-05-21, now under the `transact-rs` org), `tower-sessions` 0.15.0, `tower-sessions-sqlx-store` 0.15.1

## Context

The platform needs transactions with serialisable isolation for auctions, a
durable queue, and a place for records that are not graph-shaped (accounts,
invitations, audit). The knowledge graph handles semantics; it is a derived,
replayable view, not the system of record.

## Decision

PostgreSQL is the primary transactional store, with `sqlx` 0.9.0 for
compile-time-checked SQL. The graph is a derived projection that can always be
rebuilt from the Postgres ingest ledger plus the rule bundles.

## Alternatives

| Option | Why rejected |
| --- | --- |
| Store everything in the KG | Correctness for auctions depends on serializable transactions and advisory locks; that is Postgres territory. |
| A separate queue broker (RabbitMQ, NATS) as the baseline | One more process to run for a benefit only needed at larger scale. Postgres `LISTEN/NOTIFY` covers our volume; NATS JetStream remains an option behind the queue facade. |
| sea-orm / diesel | Both healthy. sea-orm adds an ORM abstraction layer we do not need; diesel is sync-first. sqlx gives checked raw SQL plus `PgListener`, which is exactly our shape. |

## Consequences

- Single-node deployment needs exactly one database process: cheap.
- Session storage is in Postgres via `tower-sessions-sqlx-store`, so sessions
  work across a horizontally scaled API without another service.
- Advisories and `SELECT ... FOR UPDATE` give us auction correctness
  (`02-domain-model.md` INV-11).
- Note: `axum-sessions` is deprecated (last release 2023-10-13, README points to
  tower-sessions). We do not use it.
- Note: sqlx moved out of the launchbadge org into `transact-rs`. Governance
  changed but the crate is actively maintained; we pin the version and monitor.
- Risk: the ingest ledger grows. Mitigation: monthly partition pruning and
  archiving of raw match records.
