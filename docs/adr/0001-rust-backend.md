# ADR-001: Rust for the backend

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** axum 0.8.9 (crates.io, 2026-04-14), tokio 1.54.1, tower 0.5.3

## Context

The previous prototype used a Python scraper plus a Spring Boot (Kotlin)
backend. Three languages, three build systems, two ORMs, no shared types, and a
dependency on a JVM runtime for the API.

The requirement is a backend that is fast, safe, and cheap to host, with the
game logic expressible as rules and verifiable proofs.

## Decision

The whole backend is Rust: one workspace, one binary for the API, one for the
workers, one for the scraper. axum 0.8.9 on tokio.

## Alternatives

| Option | Why rejected |
| --- | --- |
| Kotlin + Spring Boot (status quo) | Two languages and two build systems; JVM memory cost; no native rule-engine ecosystem |
| Python backend | Runtime cost, GIL, no memory safety, weaker concurrency for auctions |
| Go | Good, but weaker type-level guarantees and a thinner reasoning/SPARQL ecosystem |
| Node/Deno backend | Single-threaded concurrency model awkward for CPU-bound scoring |

## Consequences

- `MemoryDenyWriteExecute` is safe in the hardened systemd unit, because Rust
  compiles ahead of time. The same is not true for a Node or JVM service.
- One toolchain, one lockfile, one static binary per role.
- The reasoning crates (Oxigraph, rudof, eyeron) are all Rust, so the rule
  engine embeds in the process rather than needing a sidecar.
- Risk: a smaller hiring pool than Node or JVM. Accepted.
- Risk: axum 0.9 is unreleased with no committed timeline (maintainer statement
  in tokio-rs/axum#3813, 2026-06-30). We pin 0.8.9 and plan the migration as a
  discrete track.
