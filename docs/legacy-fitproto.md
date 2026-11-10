# Legacy Prototype Analysis: the `fitet` scraper

The original repository contained a working Python prototype of a FITET results
scraper. It was deleted from the working tree in commit `f2e922d` ("Clean
start!") and is recoverable from git history at commit `444d843`. This document
records what it does, what it got right, and what the Rust rewrite replaces.

## 1. What the prototype contains

| File | Purpose |
|---|---|
| `fitet/fitetparser.py` | All scraping and HTML parsing (402 lines) |
| `fitet/entities.py` | SQLAlchemy models: `player`, `match`, `event` |
| `fitet/threadutils.py` | A `WaitableThreadPool` wrapper tracking outstanding tasks |
| `fitet/caching.py` | A persistent pickle cache with a one-week TTL |
| `fitet/__main__.py` | CLI: `--update`, `--dump-path`, `--region`, player queries |
| `nix/fitet-parser.nix` | Nix packaging as a Python application |
| `assets/matches.json` | 1085 recorded matches used here as golden fixtures |
| `backend/.../SqliteFitetParsedMatchRepository.kt` | A Spring Boot reader over the SQLite dump |

## 2. Source structure it discovered

```
menu.php                                  → home menus (campionati, manifestazioni, classifiche, regioni)
risultati/regioni/menu_reg.php?REG=       → championship list for a region
risultati/regioni/menu_tor.php?REG=       → tournament types
risultati/tornei/elenco_tornei.php?TOR=&COMIT=&ID=0
risultati/tornei/tabelloni/{id}_{reg}_home.html
risultati/campionati/Calendario.asp?CAM=&ANNO=
risultati/campionati/giornata.php?CAM=&INCONTRO=&FORMULA=1
risultati/new_rank/ajax.php?term=         → XHR player search
risultati/new_rank/dettaglioatleta_unica.php?ATLETA=&ID_CLASS=&ZU=1&AVVERSARIO=0
```

## 3. Data model it produced

```sql
CREATE TABLE player (id INTEGER PRIMARY KEY, name VARCHAR(80) UNIQUE, score INTEGER);
CREATE TABLE event  (id INTEGER PRIMARY KEY, name VARCHAR(80) UNIQUE, date DATETIME);
CREATE TABLE "match" (id INTEGER PRIMARY KEY,
                      one_id REFERENCES player(id), two_id REFERENCES player(id),
                      _score VARCHAR(80), event_id REFERENCES event(id));
```

An entry in `assets/matches.json`:

```json
{
  "one": "Stracchi Leonardo",
  "two": "Bendoni Lorenzo",
  "score": [[11, 8], [11, 8], [8, 11], [7, 11], [6, 11]],
  "source": {"type": "torneo", "a": "6", "b": "6", "date": "12/11/2023"}
}
```

`source.type` is either `partita` (championship giornata) or `torneo` (tournament
bracket). `a`/`b` are numeric ids whose meaning depends on the type. `score` is a
list of `[points_one, points_two]` set pairs, 3–5 sets.

**This record shape is the input contract for the new pipeline**, expressed as
`RawMatch` in `06-data-pipeline.md` §3.4 and as `PlayerAppearance` + set scores
in the KG (`03-knowledge-graph.md`).

## 4. What it got right

1. **The scraping strategy works.** Regions → championships/campotypes →
   calendars → giornate, with brackets handled separately. Fast and reliable
   enough to run on a cron.
2. **Concurrency discipline.** The author discovered that more than about 20
   concurrent requests take the site down. The new pipeline inherits a lower
   default (4) plus a per-host token bucket.
3. **Optimistic completeness checking.** The
   `(incontri disputati 63/72, completezza: 87.5%)` marker is the only reliable
   signal that a tabellone may be trusted. Partial brackets are skipped, never
   partially ingested. This is now an explicit rule (`06-data-pipeline.md` §3.2).
4. **Event deduplication by name.** `_already_parsed_events_names` made re-runs
   cheap. Replaced by content-hash idempotency in the ingest ledger.
5. **Provisional handling.** A player absent from the classifica gets a default
   rating of 2000 (`NEW_PLAYER_SCORE`) rather than failing the whole import.
   Now the `UNRESOLVED` state with an explicit flag (`02-domain-model.md` §4).
6. **Absent/withdrawn matches.** `assente`/`ritirato` becomes a 0–3 or 3–0
   walkover. Preserved verbatim in the new parser rules.
7. **Real terminal knowledge.** Tournament brackets include byes (`< X >`),
   consolation brackets, third-place matches, and retired matches (`(R)` → 0
   points). The bit-count/structure logic in `parse_eliminatorie_table` is the
   reference for the Rust parser.

## 5. What it did wrong (and what replaces it)

| Problem | Prototype behaviour | Replacement |
|---|---|---|
| Stored scores deserialised with `eval()` | `self._score` is a Python `repr()` of a list, read with `eval()` | Typed Rust structs, stored as structured columns |
| SQLite file shared with a JVM backend | The Kotlin side copied the file under a Python lock to block readers | Postgres is the single writer and reader |
| Hardcoded table indices | `tables[1]`, `tables[3]`, `tables[5]`, `tables[7]` of the homepage | Selectors by content signature, validated with a fingerprint per page type |
| Player identity by name string | Name matching against the classifica, first hit wins | Explicit reconciliation with `RESOLVED`/`AMBIGUOUS`/`UNRESOLVED` states and an admin queue |
| No observability | `icecream` print statements | OpenTelemetry spans for every stage |
| No typed errors | `ValueError` with string messages, swallowed by a thread pool | Typed errors, quarantine queue, admin reporting |
| Ratings overwritten in place | `score` on `player` mutated freely | Bitemporal ratings; historical scoring is reproducible |
| Mixed concerns | Parsing, persistence, threading, and control flow in one class | Layered crates: `core`, `ingest`, `reasoning`, `api` |
| No tests | Zero FITET coverage | Golden fixtures from `assets/matches.json`, 100% parser coverage |

## 6. Unfinished work the prototype flagged

Its own README listed exactly the features built here:

- web UI for fantasy table tennis → `docs/08-frontend-design.md`
- player auction → `docs/01-architecture.md` §4.2, `docs/04-reasoning-rules.md` §5
- formation management (lineup with order and reserves) → `docs/08-frontend-design.md` §4.1
- score assignment → `docs/05-scoring-spec.md`
- match simulation (A–X and score comparison) → deferred (`docs/00-vision.md` §4)
- the `Top AB` tabellone type was skipped with `TODO che schifo` → noted in `docs/06-data-pipeline.md` §6

## 7. Fixture promotion

`assets/matches.json` becomes `tests/fixtures/legacy_matches.json`: 1085 match
records, 522 from championship giornate and 563 from tournaments. Every one is a
regression test for the new parser: parse the recorded HTML (captured alongside),
assert the canonical record, compare against the fixture row.

## 8. Old backend entities worth keeping conceptually

The deleted Kotlin backend had entities that map onto the new model:

| Kotlin | New concept |
|---|---|
| `PlayerEntity.points` | `Player.rating` (now bitemporal) |
| `RoundEntity` (start/end/resultTime, index) | `Matchday` (`tt:Matchday`) with `status` |
| `LeagueEntity.nDivisions` | Division tier configuration |
| `ScoreEntity(round, player, score)` | Derived `tt:SquadMatchdayScore` with `derivedFrom` |
| `LineupManager` default and carry-forward logic | Roster carry-forward plus automatic bench substitution |
| `GameFormat` (Swaythling, MiniSwaythling, NewSwaythling) | `tt:TeamFormat` values, preserved exactly |

`GameFormat` in particular is retained verbatim: the Swaythling 9-match order
(A–X, B–Y, C–Z, B–X, A–Z, C–Y, B–Z, C–X, A–Y) and its two reduced variants are
the team-format vocabulary of the new ontology.
