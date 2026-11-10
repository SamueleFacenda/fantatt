# ADR-005: Hosting topology

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** Fly.io shared-cpu-1x 256 MB at $2.19/30 days (pricing docs modified 2026-11-10); Lambda Web Adapter 1.1.0 under the `aws` org; Workers $5/month minimum

## Context

Requirements: cloud-ready, cheap, no vendor lock-in, serverless-capable, and
self-hostable. The app has long-lived SSE connections and a scraped pipeline, so
"serverless" cannot mean "stateless only".

## Decision

Primary: **one always-on small VM** (Fly.io, reference) running the API, workers,
and Postgres. Secondary (optional): the same binary behind the AWS Lambda Web
Adapter as a burst tier. Both come from the same Nix image.

## Alternatives

| Option | Why rejected |
| --- | --- |
| Cloudflare Workers as primary | Excellent for edge SSE fan-out, but WASM constraints exclude native crates and compile-time SQL checks, and there is a $5/month floor. |
| Shuttle.rs | At risk: CLI last release 2025-09 and the pricing page returns 404. Not a foundation. |
| Railway | $5/month minimum and more expensive per unit than Fly at this scale. |
| Managed Postgres only (RDS/Cloud SQL) | Vendors cost more than a managed Postgres on the same VM at our scale; retained as an option behind the config. |
| Kubernetes | Disproportionate operational cost for a single-team project. |

## Consequences

- Cold-start risk on Lambda is measured (18–22 ms for a Rust ARM64 health
  endpoint), so the burst path is viable and tested.
- SSE keeps warm connections on the always-on VM; Workers is not on the critical
  path.
- Because the image is identical, moving between these targets is a
  configuration and Terraform change, controlled by config.
- A fully self-hosted NixOS deployment is the default and the reference
  implementation; the cloud targets are documented as equivalent.
