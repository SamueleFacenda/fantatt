# Domain Model

## 1. Ubiquitous language

| Term | Meaning |
| --- | --- |
| **Player** | A real FITET athlete. Has a canonical identity, a rating, and a gender. |
| **Rating** | FITET ranking points as published by the classifica. Integer, provisional players default to 2000. |
| **Division tier** | Competition band: `B2`, `C1`, `C2`, `D1`, `D2`. Ordered high to low. |
| **Competition** | A real championship (e.g. "C2 Trentino girone A") or tournament. A league selects which competitions it draws from. |
| **Matchday (giornata)** | One round of a real championship. The atomic unit of results. |
| **League** | A manager-created fantasy league with rules, competitions, budget, and members. |
| **Manager** | A user participating in a league, owning zero or one squad. |
| **Squad** | A manager's 15 selected players for a month (10 starters, 5 bench). |
| **Ownership mode** | `exclusive` (auction, one owner per player per league) or `shared` (free pick, many owners). |
| **Lot** | One player under auction in an exclusive-mode league. |
| **Offer** | A bid or counter-offer on a lot. |
| **Resell phase** | Monthly window in exclusive mode where managers trade players among themselves. |
| **Matchday score** | Fantasy points earned by a squad in one matchday. |
| **Standings** | Aggregation of matchday scores into month and season. |
| **Lineup / starters / bench** | The 10 active and 5 reserve players of a squad. |
| **Ineligible player** | A player whose division does not match the slot tier. |
| **First week of the year** | Week 1 is excluded: league assignment is unknown before the first matchday. |

## 2. Entities and value objects

```
Player (aggregate root)
  playerId           UUID (internal identity)
  fitetPersonId      external key, nullable until reconciled
  fullName           "Surname Name" (FITET normalisation)
  gender             MALE | FEMALE
  baseRating         integer (FITET classifica points)
  ratingAsOf         date
  activeCompetitions set<CompetitionId>
  affiliations       set<(CompetitionId, DivisionTier, TeamId)>  // current season

Rating (value object)
  value, asOf, source, confidence

Competition (aggregate root)
  competitionId
  kind              CHAMPIONSHIP | TOURNAMENT
  region            "Trentino" ...
  seasonYear
  divisionTier
  name
  genderBand        MALE | FEMALE | MIXED
  matchdays         set<MatchdayId>

Matchday (aggregate root)
  matchdayId
  competitionId
  number            1..n
  scheduledDate
  status            SCHEDULED | PARTIAL | COMPLETE | CANCELLED
  ties              set<TieId>

Tie (aggregate root)                     // one team-vs-team encounter
  tieId
  matchdayId
  homeTeamId, awayTeamId
  homeScore, awayScore                   // team match result
  winnerId
  format            SWAYTHLING | MINI_SWAYTHLING | NEW_SWAYTHLING | CUSTOM
 PlayerAppearance   list

PlayerAppearance (value object)          // one real match involving one player
  appearanceId
  playerId
  matchdayId
  tieId
  kind             SINGLES | DOUBLES
  partnerId        UUID?                  // doubles only
  opponentId       UUID?                  // singles only
  opponentRating   integer, gender-adjusted
  played           boolean
  outcome          WIN | LOSS
  setsWon, setsLost
  scoreline        [(11,8),(11,8),(8,11),(7,11),(6,11)]
  isDecider        boolean                // match decided the tie (3-3)
  isComeback       boolean                // won after 0-2 down
  isAway           boolean
  isTeamWin        boolean
  isDoublet        boolean                // 2 singles wins same matchday

League (aggregate root)
  leagueId
  name, slug
  ownerId
  seasonYear
  ownershipMode     EXCLUSIVE | SHARED
  competitions      set<CompetitionId>
  startingBudget    money
  squadSize         15 (fixed by ruleset, configurable)
  startersPerTier   2, benchPerTier 1 (configurable weights)
  resellDayOfMonth  integer
  auctionConfig     AuctionConfig
  rulesetVersion    semver (points to reasoning rule bundle)
  state             DRAFT | OPEN | AUCTION | ROSTERING | ACTIVE | ARCHIVED
  settings          LeagueSettings (money, caps, privacy)

Membership (aggregate root)
  leagueId, managerId, role, joinedAt, budget, state

Squad (aggregate root)
  squadId, leagueId, managerId, monthYear
  slots             Slot[15] (tier, position, playerId | null)
  status            DRAFT | SUBMITTED | LOCKED | SCORED
  revision           integer (optimistic concurrency)

Offer / Lot (exclusive mode)
  lotId, leagueId, playerId, status OPEN | SOLD | UNSOLD
  offers            list<Offer>
  currentBest       offer ref
  deadline          timestamp
  softCloseWindow   duration
```

## 3. Invariants

| ID | Invariant |
| --- | --- |
| INV-1 | A player has exactly one rating value per `asOf` date per classifica. |
| INV-2 | A squad contains exactly 15 slots: per tier, 2 starters + 1 bench. |
| INV-3 | A slot player's active division tier equals the slot tier. Violation = INELIGIBLE_PLAYER. |
| INV-4 | In exclusive mode, a player appears in at most one squad per league per month. |
| INV-5 | In shared mode, a player may appear in multiple squads. |
| INV-6 | No duplicate player within the same squad. |
| INV-7 | A squad is immutable after lock until admin unlock (which is audited and versioned). |
| INV-8 | Week 1 of the calendar year contributes 0 points for every player. |
| INV-9 | A bench player substitutes only within their own tier. |
| INV-10 | A matchday is complete only when all scheduled ties have results. Partial matchdays are flagged, not silently treated as complete. |
| INV-11 | An offer is accepted only if it is the highest and the window is open. Overbidding is impossible under serialized isolation. |
| INV-12 | Total auction spend per manager ≤ starting budget. |
| INV-13 | A player's fantasy points derive only from asserted appearances and rule evaluation; no stored score without a derivation record. |
| INV-14 | Every stored score carries the rule bundle version that produced it. |
| INV-15 | Deleted accounts purge personal data within 30 days; league history anonymises managers rather than deleting their matchday scores (league integrity). |

## 4. Identity reconciliation (players)

The FITET portal has no stable player ID in the scraped surfaces the prototype used:
players are matched by name and validated against a classifica.

Model:

```
PlayerIdentity (aggregate)
  internalId        UUID
  aliases           set<Alias>  // "Bendoni Lorenzo", "BENDONI Lorenzo", nickname variants
  evidence          set<Evidence>
                   // (classificaId, points, region, seenInCompetitionId, matches count)
  resolvedAt
  confidence        HIGH | MEDIUM | LOW | AMBIGUOUS

resolution states:
  RESOLVED   → exactly one classifica entry matches within tolerance
  AMBIGUOUS  → ≥2 candidates; manager/admin resolves; never auto-picked
  UNRESOLVED → provisional player, rating 2000, flagged in UI
```

Rules:

- A name normalisation function is pure and shared by ingest, reasoning, and API.
- Merging two identities is an explicit admin action, recorded in the audit log, and re-derives every affected score.
- Ambiguity is surfaced, never guessed. A wrong merge silently corrupts standings.

## 5. Aggregate boundaries and consistency

| Aggregate | Transaction boundary | Cross-aggregate rule enforcement |
| --- | --- | --- |
| League | Its own settings and lifecycle | Eventual via reasoning verification over the whole KG snapshot |
| Squad | Submission validation + slot replacement | Immediate: the reasoning verdict is part of the commit |
| Lot / Offer | Bid commit | Immediate, serializable, plus a rule trace |
| Competition / Matchday | Ingest of one matchday | Eventual: recompute on completion |
| PlayerRating | One import batch | Eventual: recompute affected matchdays, record old value for audit |

Rule violations that cross aggregates are **rejected at the point of mutation**
using the reasoning engine over the current KG snapshot, so a squad is never
persisted invalid (same for offers). Aggregates that are expensive to check
(competitions) verify asynchronously and publish a reconciliation report.

## 6. Derived vs stored facts

| Stored (source of truth) | Derived (recomputable, cached) |
| --- | --- |
| Players, aliases, ratings, affiliations | Cost index |
| Competitions, matchdays, ties, appearances | Player matchday fantasy points |
| Leagues, memberships, ruleset version | Squad matchday score |
| Squads, slots, submissions | Month / season standings |
| Lots, offers, auction events | Live standings deltas |
| Rule bundles (content-addressed) | Verification reports |

Derived facts are never written without a `derivedFrom` pointer to the rule
version and input revision, so they can be invalidated wholesale on a ruleset
update (`13-observability.md` covers the recompute job).

## 7. Bounded contexts

| Context | Owner | Notes |
| --- | --- | --- |
| Ingest | Scraping, normalisation, reconciliation | Owns FITET source knowledge |
| Game | Leagues, squads, auctions, scoring | Owns rules, rulesets |
| Identity | Users, authn, credentials, GDPR | Owns PII boundary |
| Social/Notify | Push, email, in-app | Stateless fanout |
| Ops | Config, telemetry, audit | Cross-cutting |

Contexts communicate via domain events, not by reading each other's tables.
