# Brandbook

The design language is deliberately not the default SaaS look: no indigo-to-violet
gradient, no Inter at 14 px everywhere, no rounded-corner cards floating on
grey. Fantatt's identity takes its cues from the sport itself: table tennis —
a small ball, hard geometry, precise lines, a fast sound, and Italian federation
heritage.

## 1. Brand idea

**"Precision under pressure."** The aesthetic is a scoreboard crossed with a
technical drawing. Sharp corners, measurable grid, high-contrast data, one
accent colour that behaves like a live indicator.

## 2. Logo

- **Mark:** a stylised ball inside a hexagon whose outer edge is a rounded
  triangle reading as a table corner. The hexagon is a table from above; the ball
  sits at the centre of the near edge where a serve begins.
- **Wordmark:** `FANTATT` in a geometric sans, tight tracking, all caps for
  headers, sentence case in the UI.
- **Usage:** the mark is the app icon and favicon; the wordmark never appears
  smaller than 16 px. Minimum clear space equals the ball diameter on all sides.
- **Favicon:** the mark alone, at 32 px, with an SVG master and PNG exports.

## 3. Colour

Light theme is primary; dark is not an inversion but a separately tuned palette.

### Core (light)

| Token | Value (OKLCH) | Use |
| --- | --- | --- |
| `surface` | `oklch(0.985 0.004 250)` | Page background |
| `surface-raised` | `oklch(1 0 0)` | Cards, sheets |
| `surface-sunken` | `oklch(0.955 0.006 250)` | Tables, inputs |
| `ink` | `oklch(0.22 0.02 250)` | Primary text |
| `ink-muted` | `oklch(0.45 0.02 250)` | Secondary text |
| `line` | `oklch(0.88 0.008 250)` | Borders, grid |
| `line-strong` | `oklch(0.55 0.01 250)` | Emphasis borders |

### Core (dark)

| Token | Value (OKLCH) | Use |
| --- | --- | --- |
| `surface` | `oklch(0.17 0.01 250)` | Page background |
| `surface-raised` | `oklch(0.21 0.012 250)` | Cards |
| `surface-sunken` | `oklch(0.14 0.008 250)` | Sunken panels |
| `ink` | `oklch(0.95 0.005 250)` | Primary text |
| `ink-muted` | `oklch(0.72 0.01 250)` | Secondary text |
| `line` | `oklch(0.30 0.012 250)` | Borders |

### Accent — "Match Point"

| Token | Value | Use |
| --- | --- | --- |
| `accent` | `oklch(0.72 0.19 145)` | Live scores, primary actions, focus |
| `accent-strong` | `oklch(0.62 0.17 145)` | Hover, active |
| `accent-ink` | `oklch(0.18 0.03 145)` | Text on accent |

A green chosen as a table-tennis table edge, not a generic "success green".

### Semantic

| Token | Value | Use |
| --- | --- | --- |
| `positive` | `oklch(0.70 0.16 155)` | Wins, gains |
| `negative` | `oklch(0.62 0.21 25)` | Losses, reductions |
| `warning` | `oklch(0.80 0.16 80)` | Lock reminders, expiring timers |
| `critical` | `oklch(0.60 0.24 25)` | Errors, destructive |
| `neutral` | `oklch(0.62 0.02 250)` | Benched, absent |

### Tier colours (competition bands)

Distinct hues, equal weight, ordered B2 → D2:

| Tier | Hue | Value (OKLCH) |
| --- | --- | --- |
| B2 | deep blue | `oklch(0.62 0.15 255)` |
| C1 | teal | `oklch(0.66 0.12 200)` |
| C2 | green | `oklch(0.68 0.14 150)` |
| D1 | amber | `oklch(0.75 0.15 70)` |
| D2 | violet | `oklch(0.62 0.15 300)` |

Used as slot headers and player badges, always with the tier letter also present
— colour never carries meaning alone.

## 4. Typography

| Role | Family | Notes |
| --- | --- | --- |
| Display / headings | `Bricolage Grotesque` | Variable width, distinctive, not Inter |
| UI / body | `Source Sans 3` | High x-height, excellent at small sizes on mobile |
| Numerals / scores | `DM Mono` | Tabular figures, monospaced for column alignment |
| Fallback | system sans | Always set |

Both families are self-hosted, subset to `latin-ext` plus Italian diacritics,
loaded with `font-display: swap` and preloaded for the two display weights only.

Numeric rules: tabular figures everywhere numbers compare; scores to one decimal;
money and credits with the unit as a suffix at reduced weight.

## 5. Space, radius, elevation

| Token | Value |
| --- | --- |
| Space scale | 4 px base: 4, 8, 12, 16, 24, 32, 48, 64 |
| Radius-small | 2 px |
| Radius-medium | 4 px |
| Radius-large | 6 px |
| Radius-full | pills and icon buttons only |

Sharp corners are intentional. Elevation is expressed with hairline borders and
at most two discrete shadow levels; sheets and dialogs use a scrim plus a 1 px
border, never a floating card with a soft glow.

## 6. Components

| Component | Shape |
| --- | --- |
| Button | Rectangular, 44 px min height, solid fill for primary, hairline for secondary |
| Slot card | Tier-coloured left spine (3 px), player name, rating, cost, status dot |
| Live card | Sunken panel with a 2 px accent left rule and a pulsing dot while updating |
| Timer | Monospaced digits, tabular, colour escalates in the last 60 s |
| Table | Zebra-free; 1 px horizontal rules only, right-aligned numerics, sticky header |
| Sheet | Bottom sheet on mobile, side sheet on ≥ 768 px |
| Toast | Bottom-centre on mobile, top-right on desktop, 4 s auto-dismiss |

## 7. Iconography

- One stroke-based set, 1.5 px stroke, 24 px grid, 20 px in UI.
- Filled variants only for active states.
- No emoji in the interface.

## 8. Motion

| Transition | Duration | Easing |
| --- | --- | --- |
| Micro (hover, press) | 120 ms | `cubic-bezier(0.2, 0, 0, 1)` |
| State change (toast, badge) | 180 ms | same |
| Route transition (View Transitions) | 240 ms | same |
| Auction timer escalation | continuous | — |

Motion only communicates state. Decorative animation is disabled under
`prefers-reduced-motion: reduce`.

## 9. Voice

Short, direct, and specific. Errors name the rule; confirmations state the
consequence.

- "Roster saved. It locks on 30 November at 20:00."
- "Can't add Bendoni Lorenzo here: they play in C1, this slot is D1."
- "You were outbid on Steinbock Jonas."

Not: "Oops! Something went wrong."

## 10. Asset inventory

| Asset | Format |
| --- | --- |
| Logo master | SVG |
| Icon 192 / 512, maskable | PNG |
| Favicon 32 / 180 | PNG |
| Social preview | 1200×630 PNG |
| Fonts | WOFF2 subset |

## 11. Accessibility baseline

- Contrast: 4.5:1 body text, 3:1 large text and UI components.
- Focus: 2 px accent outline, 2 px offset, never removed.
- Colour blindness: tier identity always paired with a letter or label.
- Target size: 44×44 px minimum.
