# Scoring Specification

This is the normative reference for the scoring engine. `04-reasoning-rules.md`
encodes it as rules; this document states the arithmetic and worked examples.

## 1. Division tiers and base points

| Tier | Order | Base win | Base loss |
| --- | --- | --- | --- |
| B2 | 1 (highest) | 15 | 3 |
| C1 | 2 | 12 | 3 |
| C2 | 3 | 10 | 2 |
| D1 | 4 | 8 | 2 |
| D2 | 5 (lowest) | 6 | 1 |

The tier of the **player's active competition** is used, not the tier of the tie.

## 2. Multipliers

### 2.1 Gender rating adjustment

```
Rating_adjusted(opponent) = opponent.baseRating * 0.75
                            if player.gender == MALE and opponent.gender == FEMALE
                            else opponent.baseRating
```

### 2.2 Rating ratio

```
Ratio = (Rating_adjusted(opponent) - Rating(player)) / 400
Mult_rating = Ratio if Ratio > 1.0 else 1.0
```

Applied to singles only. Doubles always use `1.0`.

### 2.3 Contextual multipliers

| Condition | Field | Multiplier |
| --- | --- | --- |
| Away match | `isAway` | 1.2 |
| Deciding match win (tie decided 3-3, player won the decider) | `isDecider` | 1.3 |
| Comeback win from 0-2 down | `isComeback` | 1.3 |

Multipliers compose multiplicatively.

## 3. Singles

```
Points_singles = BasePoints
               * Mult_away
               * Mult_rating
               * Mult_comeback
               * Mult_decider
```

## 4. Doubles

```
Points_doubles = (BasePoints * Mult_away * Mult_comeback) / 2
```

No rating multiplier. No decider multiplier is specified for doubles in the
source spec; the team-victory and doublet bonuses still apply as usual.

## 5. Additive bonuses (per matchday)

| Bonus | Value | Condition |
| --- | --- | --- |
| Team match victory | +3 | the player's real team won the tie |
| Individual doublet | +2 | player won exactly 2 singles in the same matchday |

These are added after summing individual match points.

## 6. Complete matchday score

```
Matchday_score(player) = Σ over singles matches Points_singles
                       + Σ over doubles matches Points_doubles
                       + 3 [team win]
                       + 2 [doublet]

Manager_score(player) = Σ over league player matches Points_singles
                      + Σ over league player doubles matches Points_doubles
                      + 3 [team win]
                      + 2 [doublet]          # for the player, per matchday

Squad_matchday_score = Σ over 10 active (or substituted) slots of player scores
Bench players score 0 unless substituted for an absent starter in their tier.
Absent = the starter has no appearance with `played = true` in that matchday.
```

## 7. First week of the year

Week 1 of each calendar year contributes **0** to every score, for every player
and every league. Rationale: league assignment for players is unknown before the
first matchday, so points would be arbitrary. Implemented as a rule that marks
those matchdays `tt:Excluded` and excludes them from every aggregation.

## 8. Worked example (from the spec, D1 player, rating 4070, away)

**Match 1 — vs female player, base 5970, adjusted 5970 × 0.75 = 4432, won 3-2:**

```
Ratio = (4432 - 4070) / 400 = 0.905 ≤ 1.0  →  Mult_rating = 1.0
Points = 8 (D1 base) × 1.2 (away) × 1.0 = 9.6
```

**Match 2 — vs opponent rating 4847, won 3-1:**

```
Ratio = (4847 - 4070) / 400 = 1.9425      →  Mult_rating = 1.94
Points = 8 × 1.2 × 1.94 = 18.624
```

**Bonuses:** team win +3, individual doublet +2.

```
Total = 9.6 + 18.624 + 3 + 2 = 33.224  → 33.2 (round half up to 1 decimal)
```

**Rounding rule:** keep full precision internally; round for display to 1
decimal using round-half-away-from-zero. Standings sum unrounded values and only
round at presentation, so a dispute can be recomputed exactly.

## 9. Rounding and precision policy

| Quantity | Precision |
| --- | --- |
| Internal computation | f64 exact arithmetic, no intermediate rounding |
| Stored scores | Decimal with 4 places |
| Displayed scores | 1 decimal |
| Rankings | Compare unrounded totals; ties broken by fewest bench points, then alphabetical |

## 10. Player cost index

Player cost derives from a **separate cost model**, not the FITET ranking points
directly. Inputs:

| Input | Source | Direction |
| --- | --- | --- |
| Base FITET ranking points | classifica | higher rating → more expensive |
| League tier played | active competition | higher tier → more expensive |
| Recent form | last 5 matchday win rate, points scored | improving form → more expensive |
| Match difficulty | average opponent adjusted rating faced | harder schedule → cheaper |
| Schedule placement | average tier of opponents per week | tougher weekly matchups → cheaper |
| Availability | absence rate, cancellations | unreliable → cheaper |
| Depth | quality of bench alternatives at the same tier | few alternatives → expensive |
| League scarcity | players at that tier owned by rivals (exclusive mode) | scarce → expensive |

Normalisation to the league's starting budget is configurable per league so a
budget of 1000 and a budget of 10000 both produce meaningful prices. The formula
lives in a versioned rule bundle too, so a cost-model change is explicit and
re-derivable.

## 11. Standings

```
Matchday_score →  Monthly_standings → Monthly_winner
                       ↓
                  Season_standings → Season_champion
```

| Level | Rule |
| --- | --- |
| Matchday | Sum of the 10 active (or substituted) slot scores |
| Month | Sum of all matchday scores in the calendar month |
| Season | Cumulative sum of every monthly total across the competitive season |

Both monthly and season champions are declared. A league may also enable weekly
leaderboard prices (configurable, off by default).
