# ADR-004: Passkeys as primary, Google OAuth as convenience

- **Status:** Accepted
- **Date:** 2026-11-10
- **Verified:** webauthn-rs 0.6.2 (2026-10-31, kanidm org), oauth2 5.0.0, openidconnect 4.1.0 (both now under the `ramosbugs` org)

## Context

The product requires login with Google OAuth (providing name, email, and
profile picture) and passkey login. Security is a non-negotiable requirement.

## Decision

- **Passkeys are the primary credential path.** `webauthn-rs` 0.6.2, with user
  verification required and multiple authenticators per account.
- **Google OAuth2 is a convenience path** for onboarding, providing `name`,
  `email`, and `picture`.
- **No passwords are stored, ever.** There is no password reset flow, no
  credential database, and no password hashing path.

## Alternatives

| Option | Why rejected |
| --- | --- |
| Email + password | The single largest breach category; no benefit given passkeys exist. |
| Magic links only | Weak for repeated use, and email compromise becomes account compromise. |
| Social-only (Google only) | Single point of failure; a Google account compromise becomes a full compromise. |

## Consequences

- WebAuthn nuances become our problem: origin and RP-ID validation, challenge
  single-use, counter verification with clone detection.
- A Google-only account cannot be a league admin until it registers a passkey.
- The old prototype's `UserEntity.password` column is gone. Account creation
  takes a passkey in one flow.
- Note: `axum-login` has slowed (last release 2025-07-20); we treat it as
  optional glue and implement the small amount of auth middleware ourselves.
- Positive: a huge reduction in attack surface, which is the point.
