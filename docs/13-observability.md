# Observability

Full telemetry from raw HTTP requests to game flows. The goal: a manager's
complaint about a score can be traced to the exact facts and rules that produced
it.

## 1. Stack

| Signal | Tool | Version |
| --- | --- | --- |
| Traces | OpenTelemetry SDK (`tracing-opentelemetry` 0.34.0, `opentelemetry-otlp` 0.33.1) → OTel Collector → Tempo | — |
| Metrics | `opentelemetry` 0.33.1 → Collector → Prometheus | — |
| Logs | `tracing-subscriber` → OTLP → Loki (via Alloy) | — |
| Dashboards | Grafana | — |
| Alerts | Grafana alerting | — |

Promtail and the Grafana Agent are **dead ends** (Promtail reached EOL on
2026-03-02 and its NixOS module was removed). Alloy replaces it. Jaeger has no
NixOS module, so traces go to Tempo.

## 2. Span model

Every request produces a server span with these attributes, so traces are
queryable by game dimension, not just URL:

| Attribute | Example |
| --- | --- |
| `http.request.method`, `http.route` | `POST /api/v1/leagues/{leagueId}/squads/me/submissions` |
| `http.response.status_code` | 422 |
| `fantatt.league.id` | `lg_2kd` |
| `fantatt.matchday.id` | `md_991` |
| `fantatt.ruleset.version` | `fantatt-core/1.4.0` |
| `fantatt.user.id` | pseudonymous hash |
| `fantatt.squad.id` | `sq_7f1` |
| `fantatt.player.id` | `plr_8f2a` |

Domain events are emitted as span events, so a submission trace shows every rule
that ran, in order, with its verdict.

## 3. Golden signals and SLOs

| Metric | Target |
| --- | --- |
| API availability | 99.9% |
| API latency p95 reads | < 300 ms |
| API latency p95 scoring recompute | < 1 s |
| Ingest freshness | < 15 min from publication to visible score |
| Ingest success rate | > 99.5% per run |
| Rule evaluation errors | 0 |
| Failed push deliveries | < 1% |

Dashboard per service plus a "game health" panel: leagues in each state, live
matchdays, pending recomputes, quarantine size, ambiguous identities.

## 4. Instrumented flows

| Flow | Traced as |
| --- | --- |
| Roster submission | Request → validation → rule evaluation → commit |
| Auction bid | Request → isolation → rule evaluation → commit → broadcast |
| Ingest | Discover → diff → fetch → parse → validate → assert → derive |
| Score recompute | Load snapshot → run rules → write graph → recompute standings |
| Notification | Event → fanout → push/email |
| GDPR deletion | Request → purge → verify |

Every traced flow emits at least one metric with a `ruleset.version` label, so a
rules change can be measured in error rate and latency.

## 5. RUM and frontend telemetry

- Web Vitals (LCP, INP, CLS) reported per route.
- A span per navigation, including service-worker cache hits and misses.
- An offline-duration histogram.
- Push delivery outcomes where the browser allows reporting.

## 6. Logs

JSON, structured, one line per event, redacted by policy:

| Field | Rule |
| --- | --- |
| Email addresses | Never logged |
| Names | Logged only in operational contexts with a debug-level gate |
| Trace id | Always present |
| `err.name`, `err.message`, `err.stack` | Structured error records |

`fantatt-obs` exports a `redact()` used by every layer, and a CI test asserts
that no log line contains `@`.

## 7. Retention and cost

| Signal | Retention |
| --- | --- |
| Metrics | 30 days at full resolution, then downsampled |
| Traces | 14 days, 100% of errors, 10% sampled of successes |
| Logs | 30 days |
| Audit log | Forever (it is a compliance record, not telemetry) |

Tail-based sampling keeps error traces at 100% while keeping storage cheap.

## 8. Debugging aids

- `X-Trace-Id` response header, shown in user-facing errors so a manager can
  quote it in a bug report.
- A `GET /admin/telemetry/status` endpoint reporting pipeline health and the
  last span export failure.
- A local trace viewer in the dev shell for offline debugging.
- A "why this score" trace view in the league admin console: a manager's
  matchday score expanded into the facts and rules that produced it, which is
  both an operational and a product feature.
