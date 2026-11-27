//! Fixture-backed scaffolding: recorded giornata pages become the asserted
//! documents the graph runtime commits and rebuilds from.

use fantatt_core::appearance::{
    AppearanceContest, AppearanceContext, AppearanceFacts, AppearanceFlags, AppearanceKey,
    AppearanceKind, Matchup, Outcome, PlayerAppearance, SetScore, Sets,
};
use fantatt_core::ids::{DomainPrefix, Identifier};
use fantatt_core::matchday::MatchdayStatus;
use fantatt_ingest::giornata::{GiornataCoordinates, GiornataRow, ParsedGiornata, parse_giornata};
use fantatt_ingest::names::normalize_name;
use fantatt_ingest::uri::Uri;
use sha2::{Digest, Sha256};
use time::{Date, OffsetDateTime};

use fantatt_reasoning::assert::{MatchdayFacts, TieFacts};
use fantatt_reasoning::commit::Commit;
use fantatt_reasoning::rebuild::Ledger;

/// Deterministic minting: identical material always yields the same identity,
/// so a rebuild of the same documents addresses the same nodes.
pub fn mint(prefix: DomainPrefix, material: &str) -> Identifier {
    let digest = Sha256::digest(material.as_bytes());
    let suffix = base32_crockford(&digest[..16]);
    Identifier::new(prefix, &suffix).expect("minted identifier")
}

fn base32_crockford(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut suffix = String::with_capacity(26);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 && suffix.len() < 26 {
            bits -= 5;
            suffix.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    while suffix.len() < 26 {
        suffix.push('0');
    }
    suffix
}

pub struct LedgerLog {
    documents: Vec<MatchdayFacts>,
}

impl LedgerLog {
    pub fn new(documents: Vec<MatchdayFacts>) -> Self {
        Self { documents }
    }
}

impl Ledger for LedgerLog {
    fn documents(&self) -> Vec<MatchdayFacts> {
        self.documents.clone()
    }
}

pub fn parse_fixture(bytes: &[u8], page_url: &str) -> ParsedGiornata {
    let uri = Uri::parse(page_url).expect("page uri");
    let coordinates = GiornataCoordinates::from_uri(&uri).expect("coordinates");
    parse_giornata(bytes, &coordinates).expect("page parses")
}

pub fn commit_for(page_url: &str, bytes: &[u8], valid_from: Option<Date>) -> Commit {
    Commit {
        source_url: page_url.to_owned(),
        document_hash: hex(Sha256::digest(bytes)),
        actor: "fitet-ingest".to_owned(),
        import_version: "0.1.0".to_owned(),
        asserted_at: asserted_now(),
        valid_from,
    }
}

pub fn asserted_now() -> OffsetDateTime {
    Date::from_calendar_date(2026, time::Month::November, 16)
        .expect("day")
        .with_hms(20, 41, 3)
        .expect("instant")
        .assume_utc()
}

/// Maps one parsed giornata into the asserted document the graph commits:
/// every tie row becomes two appearances, one from each player's side.
pub fn facts(parsed: &ParsedGiornata, number: u16, provenance: Commit) -> MatchdayFacts {
    let giornata = &parsed.giornata;
    let board = MatchBoard::new(&giornata.rows);
    let walk = Walk {
        contest_id: &giornata.contest_id,
        rows: &giornata.rows,
        matchday: &matchday_of(giornata),
        board: &board,
    };
    let (ties, appearances) = walk.facts();
    MatchdayFacts {
        matchday_id: matchday_of(giornata),
        competition_id: competition_of(giornata),
        number,
        scheduled_date: giornata.date,
        status: status(&giornata.rows),
        ties,
        appearances,
        provenance,
    }
}

fn competition_of(giornata: &fantatt_ingest::giornata::RawGiornata) -> Identifier {
    mint(
        DomainPrefix::Competition,
        &format!("competition:{}", giornata.championship_id),
    )
}

fn matchday_of(giornata: &fantatt_ingest::giornata::RawGiornata) -> Identifier {
    mint(
        DomainPrefix::Matchday,
        &format!(
            "matchday:{}:{}",
            giornata.championship_id, giornata.contest_id
        ),
    )
}

struct MatchBoard {
    doubles: bool,
    team_win: bool,
}

impl Clone for MatchBoard {
    fn clone(&self) -> Self {
        Self {
            doubles: self.doubles,
            team_win: self.team_win,
        }
    }
}

impl MatchBoard {
    fn new(rows: &[GiornataRow]) -> Self {
        let home = rows
            .iter()
            .flat_map(|row| row.sets.iter().map(|(a, b)| u32::from(a > b)))
            .sum::<u32>();
        let away = rows
            .iter()
            .flat_map(|row| row.sets.iter().map(|(a, b)| u32::from(b > a)))
            .sum::<u32>();
        Self {
            doubles: rows
                .iter()
                .any(|row| row.player_a.contains(" - ") || row.player_b.contains(" - ")),
            team_win: home > away,
        }
    }
}

struct TieContext {
    matchday: Identifier,
    tie: Identifier,
    board: MatchBoard,
    decider: bool,
}

struct Walk<'a> {
    contest_id: &'a str,
    rows: &'a [GiornataRow],
    matchday: &'a Identifier,
    board: &'a MatchBoard,
}

impl Walk<'_> {
    fn facts(&self) -> (Vec<TieFacts>, Vec<PlayerAppearance>) {
        let mut ties = Vec::new();
        let mut appearances = Vec::new();
        for row in self.rows {
            let tie = self.tie_of(row);
            ties.push(TieFacts {
                tie_id: tie.clone(),
                decided: !row.sets.is_empty(),
            });
            let context = TieContext {
                matchday: self.matchday.clone(),
                tie,
                board: self.board.clone(),
                decider: is_decider(self.rows, row),
            };
            for side in [Side::Home, Side::Away] {
                appearances.push(appearance(row, side, &context));
            }
        }
        (ties, appearances)
    }

    fn tie_of(&self, row: &GiornataRow) -> Identifier {
        mint(
            DomainPrefix::Tie,
            &format!("tie:{}:{}", self.contest_id, row.match_number),
        )
    }
}

fn is_decider(rows: &[GiornataRow], row: &GiornataRow) -> bool {
    let last = rows.iter().map(|row| row.match_number).max();
    if last != Some(row.match_number) {
        return false;
    }
    let before: u32 = rows
        .iter()
        .filter(|other| other.match_number < row.match_number)
        .flat_map(|other| other.sets.iter().map(|(a, b)| u32::from(a > b)))
        .sum();
    let others: u32 = rows
        .iter()
        .filter(|other| other.match_number < row.match_number)
        .flat_map(|other| other.sets.iter().map(|(a, b)| u32::from(b > a)))
        .sum();
    before == others
}

enum Side {
    Home,
    Away,
}

fn appearance(row: &GiornataRow, side: Side, context: &TieContext) -> PlayerAppearance {
    let home_side = matches!(side, Side::Home);
    let (player, opponent) = players(row, home_side);
    let played = !row.sets.is_empty();
    let sets = Sets::new(scoreline(&row.sets, home_side));
    let won = won(&sets);
    let key = appearance_key(&player, context, home_side);
    let flags = flags(row, context, &SideFlags { home_side, won });
    let contest = contest(ContestFacts {
        kind: kind_of(context.board.doubles),
        opponent,
        played,
        won,
        sets,
        flags,
    });
    PlayerAppearance::new(AppearanceFacts::new(key, contest)).expect("valid appearance")
}

struct SideFlags {
    home_side: bool,
    won: bool,
}

struct ContestFacts {
    kind: AppearanceKind,
    opponent: Identifier,
    played: bool,
    won: bool,
    sets: Sets,
    flags: AppearanceFlags,
}

fn contest(facts: ContestFacts) -> AppearanceContest {
    AppearanceContest {
        kind: facts.kind,
        matchup: Matchup::new(None, Some(facts.opponent), None),
        played: facts.played,
        outcome: facts.played.then_some(if facts.won {
            Outcome::Win
        } else {
            Outcome::Loss
        }),
        sets: facts.sets,
        flags: facts.flags,
    }
}

fn flags(row: &GiornataRow, context: &TieContext, side: &SideFlags) -> AppearanceFlags {
    let (home, away) = row.sets.first().copied().unwrap_or((0, 0));
    let lost_first = if side.home_side {
        home < away
    } else {
        away < home
    };
    AppearanceFlags {
        is_decider: context.decider,
        is_comeback: side.won && lost_first,
        is_away: !side.home_side,
        is_team_win: context.board.team_win,
        is_doublet: context.board.doubles && side.won,
    }
}

fn players(row: &GiornataRow, home_side: bool) -> (Identifier, Identifier) {
    let (home, away) = (
        mint(DomainPrefix::Player, &format!("player:{}", row.player_a)),
        mint(DomainPrefix::Player, &format!("player:{}", row.player_b)),
    );
    if home_side {
        (home, away)
    } else {
        (away, home)
    }
}

fn appearance_key(player: &Identifier, context: &TieContext, home_side: bool) -> AppearanceKey {
    let appearance = mint(
        DomainPrefix::Appearance,
        &format!(
            "appearance:{}:{}:{}",
            context.tie,
            player,
            if home_side { "home" } else { "away" }
        ),
    );
    AppearanceKey::new(
        appearance,
        AppearanceContext::new(
            player.clone(),
            context.matchday.clone(),
            context.tie.clone(),
        ),
    )
}

fn scoreline(sets: &[(u8, u8)], home_side: bool) -> Vec<SetScore> {
    sets.iter()
        .map(|(home, away)| {
            if home_side {
                SetScore::new(*home, *away)
            } else {
                SetScore::new(*away, *home)
            }
        })
        .collect()
}

fn won(sets: &Sets) -> bool {
    sets.won() as u32 * 2 > sets.scoreline().len() as u32
}

fn kind_of(doubles: bool) -> AppearanceKind {
    if doubles {
        AppearanceKind::Doubles
    } else {
        AppearanceKind::Singles
    }
}

fn status(rows: &[GiornataRow]) -> MatchdayStatus {
    if rows.is_empty() || rows.iter().all(|row| row.sets.is_empty()) {
        MatchdayStatus::Scheduled
    } else if rows.iter().all(|row| !row.sets.is_empty()) {
        MatchdayStatus::Complete
    } else {
        MatchdayStatus::Partial
    }
}

pub fn normalize(name: &str) -> String {
    normalize_name(name)
}

fn hex(digest: impl AsRef<[u8]>) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    digest
        .as_ref()
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[(byte >> 4) as usize] as char,
                DIGITS[(byte & 0x0f) as usize] as char,
            ]
        })
        .collect()
}
