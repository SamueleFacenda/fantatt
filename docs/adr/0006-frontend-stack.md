# ADR-006: React + Vite + Tailwind v4 for the PWA

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** react 19.3.1, Vite 8.0 (2026-03-12, Rolldown), Tailwind CSS 4.3.5, shadcn/cli v4, vite-plugin-pwa 2.0.0

## Context

Requirements: mobile-first, installable PWA, app updates delivered through
notifications, a professional and original design, and shareable/saveable paths.

## Decision

React 19 + TypeScript + Vite 8, Tailwind CSS v4 with a CSS-first `@theme` token
set implementing `09-brandbook.md`, shadcn/ui over Radix primitives, and
`vite-plugin-pwa` (Workbox) for the service worker.

## Alternatives

| Option | Why rejected |
| --- | --- |
| Svelte 5 + SvelteKit | Genuinely competitive (smaller payload). React wins on ecosystem depth and hiring; recorded as the accepted fallback if payload becomes the dominant constraint. |
| Vue 3.5 + Nuxt | Strong, but 3.6 Vapor is still RC and the team is thinnest of the three for this project. |
| Solid + SolidStart | SolidStart 2.0 entered maintenance mode and Solid 2.0's async rewrite is beta. Rejected for a long-lived product. |
| Next.js | Server-centric; the API is already Rust. A Vite SPA keeps one server concern. |
| `next-pwa` | Unmaintained; Next.js now recommends Serwist. |

## Consequences

- PWA update handling is explicit: `skipWaiting` + `clients.claim` + an
  `updatefound` listener + a user-approved reload, and periodic
  `registration.update()`. Users are prompted, never silently reloaded.
- Design tokens are code, versioned, and CI-checked against the brandbook.
- Tailwind v4 gives first-party container queries and `@starting-style`, which
  the mobile-first layout and the honest state transitions use directly.
- Risk: React's baseline bundle is larger than Svelte's. Mitigated by the
  150 KB gzip budget in `08-frontend-design.md`, enforced in CI.
