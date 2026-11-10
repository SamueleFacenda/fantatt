# ADR-008: Rewrite the FITET scraper in Rust

- **Status:** Accepted
- **Date:** 2026-11-10

## Context

The existing prototype (`fitet/`, Python + BeautifulSoup, deleted from the
working tree at commit `f2e922d` and recoverable from git history) works and is
fast. It has real limitations: `eval()` on stored scores, a raw SQLite file
shared with another process under a Python `Lock`, hardcoded selectors by table
index, and no observability or typed errors.

## Decision

Rewrite in Rust, preserving the scraping strategy and the domain understanding
from the prototype. The prototype is retained in git history and its recorded
fixtures (notably `assets/matches.json`, 1085 matches across both `partita` and
`torneo` sources) are promoted to golden test fixtures.

## Alternatives

| Option | Why rejected |
| --- | --- |
| Keep Python as a subprocess | Two languages, no shared types, `eval()` remains, and the JVM-era DB handoff pattern is the very thing being removed. |
| Node scraper | Weaker typing and worse runtime characteristics for a fan-out HTTP workload. |
| Third-party sports API | None provides Italian grassroots (B2–D2) results with the required granularity. |

## Consequences

- Crawl politeness is a first-class concern: default concurrency 4, a per-host
  token bucket, and a circuit breaker, because the prototype showed that >20
  concurrent requests take the source down.
- Every parse becomes a pure function tested against recorded HTML.
- Identity reconciliation replaces name-and-hope matching: ambiguity is
  surfaced, never guessed (`02-domain-model.md` §4).
- Provenance on every triple means a wrong character can be corrected without
  re-scraping the world.
- Risk: the FITET portal changes without notice. Detection: a schema-version
  fingerprint per page type, alerting when it drifts.
