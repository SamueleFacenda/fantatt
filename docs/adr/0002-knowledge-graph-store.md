# ADR-002: Oxigraph as the knowledge-graph store

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** oxigraph 0.5.13 (2026-10-22, MIT/Apache-2.0), rudof 0.3.39, eyeron 0.9.2, reasonable 0.5.0-a1

## Context

Game state must be represented as a knowledge graph, queried with SPARQL, and
derived by reasoning rules. The requirement adds: verifiable game logic, strict
verification for high-concurrency operations, and cost-effective cloud
operation.

## Decision

Use **Oxigraph 0.5.13** as the RDF store, a Rust library embedded in the API
process, with the RocksDB on-disk backend for production and the in-memory
backend for auctions. Compose it with:

- **rudof 0.3.39** for SHACL/ShEx validation on ingest,
- **eyeron 0.9.2** for N3 game rules with checkable proof certificates,
- **reasonable** for RDFS/OWL-2-RL closure of the taxonomy.

## Alternatives

| Option | Why rejected |
| --- | --- |
| GraphDB | Free tier now requires a manually installed license, limited to 2 concurrent queries, 1 core, 5 repositories. Enterprise is per-core. |
| Apache Jena / Fuseki | Mature but a JVM dependency; we have no other JVM. |
| RDF4J | Same reason. |
| Neo4j | GPLv3 copyleft and not a reasoner; Cypher instead of SPARQL. |
| TerminusDB 12 | Maintained by Gettier, but no SPARQL, Prolog core, and no Rust embedding. |
| TypeDB 3.x | Replaced inference with explicit functions in 3.x, so no automatic reasoning. |
| Datomic / XTDB | JVM; XTDB 2.x dropped Datalog. |
| CozoDB | Dormant: no commits since 2024-12-04. |
| differential-datalog | Dormant: no commits since 2023-07-07. |
| `crepe` (Rust Datalog) | Rules are fixed at compile time; the product requirement is runtime-configurable rules. |
| `oxirs-rule` | Pre-1.0 with 3 breaking releases in 7 months. |
| Minigraf 2.0.11 | Best bitemporal support, but solo-maintained and no SPARQL/RDF. Retained as fallback if real bitemporal history proves essential. |

## Consequences

- The reasoning stack is in-process: no sidecar, no JVM, ~$0 hosting.
- Oxigraph has **no built-in reasoner**; the materialisation pipeline
  (SPARQL `CONSTRUCT`/`INSERT` into named graphs) is ours to build. That is
  ~200 lines and is the honest cost of the decision.
- Oxigraph's own README says SPARQL query evaluation is not yet optimised. We
  benchmark actual queries before committing to complex joins, and keep
  aggregations in SQL for hot paths.
- Rolls bright: no native revisioning. We model it with per-transaction named
  graphs and validity intervals.
- Detail: eyeron's proofs are the mechanism that satisfies "strict verification".
  A bid decision can be re-checked offline by a third party.
- Detail: eyeron has no storage; `reasonable` does not read an Oxigraph `Store`.
  Both are bridged by moving triples in and back out, which we test explicitly.
