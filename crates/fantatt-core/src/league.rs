use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::calendar::MonthYear;
use crate::error::DomainError;
use crate::ids::Identifier;
use crate::money::Money;
use crate::squad::SquadComposition;

/// How players are owned inside a league.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OwnershipMode {
    /// Auction: at most one squad per league per month owns a player.
    Exclusive,
    /// Free pick: many squads may own the same player.
    Shared,
}

/// Lifecycle of a league.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LeagueState {
    Draft,
    Open,
    Auction,
    Rostering,
    Active,
    Archived,
}

impl fmt::Display for LeagueState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Draft => "DRAFT",
            Self::Open => "OPEN",
            Self::Auction => "AUCTION",
            Self::Rostering => "ROSTERING",
            Self::Active => "ACTIVE",
            Self::Archived => "ARCHIVED",
        };
        f.write_str(label)
    }
}

/// Error produced when a league state transition is not allowed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("league cannot move from {from} to {to}")]
pub struct LeagueTransitionError {
    pub from: LeagueState,
    pub to: LeagueState,
}

/// The league and its owner, plus the season it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueIdentity {
    league_id: Identifier,
    owner_id: Identifier,
    season_year: i32,
}

impl LeagueIdentity {
    pub fn new(league_id: Identifier, owner_id: Identifier, season_year: i32) -> Self {
        Self {
            league_id,
            owner_id,
            season_year,
        }
    }
}

/// Public naming of a league.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueNaming {
    name: String,
    slug: String,
}

impl LeagueNaming {
    pub fn new(name: &str, slug: &str) -> Self {
        Self {
            name: name.to_owned(),
            slug: slug.to_owned(),
        }
    }

    /// why: slugs are derived once and stored, so URLs never drift from names.
    pub fn slug_from(name: &str) -> String {
        name.split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|part| !part.is_empty())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>()
            .join("-")
    }
}

/// Roster rules a league enforces on its squads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RosterRules {
    composition: SquadComposition,
    resell_day_of_month: u8,
    ruleset_version: String,
}

impl RosterRules {
    pub fn new(
        composition: SquadComposition,
        resell_day_of_month: u8,
        ruleset_version: &str,
    ) -> Self {
        Self {
            composition,
            resell_day_of_month,
            ruleset_version: ruleset_version.to_owned(),
        }
    }

    pub fn composition(&self) -> SquadComposition {
        self.composition
    }

    pub fn ruleset_version(&self) -> &str {
        &self.ruleset_version
    }
}

/// Money and ownership rules of a league.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueRules {
    ownership_mode: OwnershipMode,
    roster: RosterRules,
    starting_budget: Money,
}

impl LeagueRules {
    pub fn new(ownership_mode: OwnershipMode, roster: RosterRules, starting_budget: Money) -> Self {
        Self {
            ownership_mode,
            roster,
            starting_budget,
        }
    }
}

/// A manager-created fantasy league with rules, competitions, and members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct League {
    league_id: Identifier,
    owner_id: Identifier,
    season_year: i32,
    naming: LeagueNaming,
    rules: LeagueRules,
    competitions: Vec<Identifier>,
    state: LeagueState,
    claims: HashSet<(MonthYear, Identifier)>,
}

impl League {
    pub fn new(spec: LeagueSpec) -> Self {
        Self {
            league_id: spec.identity.league_id,
            owner_id: spec.identity.owner_id,
            season_year: spec.identity.season_year,
            naming: spec.naming,
            rules: spec.rules,
            competitions: Vec::new(),
            state: LeagueState::Draft,
            claims: HashSet::new(),
        }
    }

    pub fn league_id(&self) -> &Identifier {
        &self.league_id
    }

    pub fn owner_id(&self) -> &Identifier {
        &self.owner_id
    }

    pub fn state(&self) -> LeagueState {
        self.state
    }

    pub fn ownership_mode(&self) -> OwnershipMode {
        self.rules.ownership_mode
    }

    pub fn starting_budget(&self) -> &Money {
        &self.rules.starting_budget
    }

    pub fn add_competition(&mut self, competition: Identifier) {
        if !self.competitions.contains(&competition) {
            self.competitions.push(competition);
        }
    }

    /// Claims a player for a month under the league's ownership mode.
    ///
    /// Exclusive leagues allow one claim per player per month (INV-4); shared
    /// leagues allow any number (INV-5).
    pub fn claim_player(
        &mut self,
        player: Identifier,
        month: MonthYear,
    ) -> Result<(), DomainError> {
        let taken = self.claims.contains(&(month, player.clone()));
        if taken && self.rules.ownership_mode == OwnershipMode::Exclusive {
            return Err(DomainError::PlayerAlreadyOwnedInMonth {
                player,
                league: self.league_id.clone(),
            });
        }
        self.claims.insert((month, player));
        Ok(())
    }

    pub fn release_player(&mut self, player: &Identifier, month: MonthYear) {
        if self.rules.ownership_mode == OwnershipMode::Exclusive {
            self.claims.remove(&(month, player.clone()));
        }
    }

    fn transition(&mut self, to: LeagueState) -> Result<(), LeagueTransitionError> {
        let allowed = matches!(
            (self.state, to),
            (LeagueState::Draft, LeagueState::Open)
                | (LeagueState::Open, LeagueState::Auction)
                | (LeagueState::Open, LeagueState::Rostering)
                | (LeagueState::Auction, LeagueState::Rostering)
                | (LeagueState::Rostering, LeagueState::Active)
                | (LeagueState::Active, LeagueState::Archived)
        );
        if !allowed {
            return Err(LeagueTransitionError {
                from: self.state,
                to,
            });
        }
        self.state = to;
        Ok(())
    }

    pub fn open(&mut self) -> Result<(), LeagueTransitionError> {
        self.transition(LeagueState::Open)
    }

    pub fn start_auction(&mut self) -> Result<(), LeagueTransitionError> {
        if self.rules.ownership_mode == OwnershipMode::Shared {
            return Err(LeagueTransitionError {
                from: self.state,
                to: LeagueState::Auction,
            });
        }
        self.transition(LeagueState::Auction)
    }

    pub fn start_rostering(&mut self) -> Result<(), LeagueTransitionError> {
        if self.rules.ownership_mode == OwnershipMode::Exclusive && self.state == LeagueState::Open
        {
            return Err(LeagueTransitionError {
                from: self.state,
                to: LeagueState::Rostering,
            });
        }
        self.transition(LeagueState::Rostering)
    }

    pub fn activate(&mut self) -> Result<(), LeagueTransitionError> {
        self.transition(LeagueState::Active)
    }

    pub fn archive(&mut self) -> Result<(), LeagueTransitionError> {
        self.transition(LeagueState::Archived)
    }
}

/// Everything a league needs at creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeagueSpec {
    identity: LeagueIdentity,
    naming: LeagueNaming,
    rules: LeagueRules,
}

impl LeagueSpec {
    pub fn new(identity: LeagueIdentity, naming: LeagueNaming, rules: LeagueRules) -> Self {
        Self {
            identity,
            naming,
            rules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use crate::squad::SquadComposition;
    use proptest::prelude::*;

    fn id(prefix: DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    fn budget(minor_units: u64) -> Money {
        Money::new("EUR", minor_units).expect("valid money")
    }

    fn rules(mode: OwnershipMode) -> LeagueRules {
        LeagueRules::new(
            mode,
            RosterRules::new(SquadComposition::REGULATION, 15, "1.0.0"),
            budget(1000),
        )
    }

    fn league(mode: OwnershipMode) -> League {
        let spec = LeagueSpec::new(
            LeagueIdentity::new(
                id(DomainPrefix::League, 'A'),
                id(DomainPrefix::User, 'B'),
                2026,
            ),
            LeagueNaming::new("Fantabendoni", "fantabendoni"),
            rules(mode),
        );
        League::new(spec)
    }

    fn month() -> MonthYear {
        MonthYear::new(2026, 11).expect("valid month")
    }

    #[test]
    fn league_starts_in_draft() {
        let league = league(OwnershipMode::Exclusive);
        assert_eq!(league.state(), LeagueState::Draft);
        assert_eq!(league.starting_budget().minor_units(), 1000);
    }

    #[test]
    fn exclusive_mode_allows_one_claim_per_month() {
        let mut league = league(OwnershipMode::Exclusive);
        let player = id(DomainPrefix::Player, 'A');
        league
            .claim_player(player.clone(), month())
            .expect("first claim");
        assert_eq!(
            league.claim_player(player, month()),
            Err(DomainError::PlayerAlreadyOwnedInMonth {
                player: id(DomainPrefix::Player, 'A'),
                league: id(DomainPrefix::League, 'A'),
            })
        );
    }

    #[test]
    fn shared_mode_allows_many_claims() {
        let mut league = league(OwnershipMode::Shared);
        let player = id(DomainPrefix::Player, 'A');
        league.claim_player(player.clone(), month()).expect("first");
        league
            .claim_player(player, month())
            .expect("second is fine in shared mode");
    }

    #[test]
    fn releasing_frees_an_exclusive_claim() {
        let mut league = league(OwnershipMode::Exclusive);
        let player = id(DomainPrefix::Player, 'A');
        league.claim_player(player.clone(), month()).expect("first");
        league.release_player(&player, month());
        league
            .claim_player(player, month())
            .expect("claim again after release");
    }

    #[test]
    fn lifecycle_moves_forward_only() {
        let mut exclusive = league(OwnershipMode::Exclusive);
        exclusive.open().expect("open");
        exclusive.start_auction().expect("auction");
        exclusive.start_rostering().expect("roster");
        exclusive.activate().expect("activate");
        exclusive.archive().expect("archive");
        assert!(exclusive.archive().is_err());
        let mut shared = league(OwnershipMode::Shared);
        shared.open().expect("open");
        assert!(
            shared.start_auction().is_err(),
            "shared leagues skip the auction"
        );
        shared.start_rostering().expect("roster");
    }

    #[test]
    fn slugs_are_kebab_case() {
        assert_eq!(
            LeagueNaming::slug_from("Fantabendoni 2026!"),
            "fantabendoni-2026"
        );
        assert_eq!(
            LeagueNaming::slug_from("Campionato C2 Trentino"),
            "campionato-c2-trentino"
        );
    }

    #[test]
    fn league_state_displays_documented_labels() {
        assert_eq!(LeagueState::Draft.to_string(), "DRAFT");
        assert_eq!(LeagueState::Open.to_string(), "OPEN");
        assert_eq!(LeagueState::Auction.to_string(), "AUCTION");
        assert_eq!(LeagueState::Rostering.to_string(), "ROSTERING");
        assert_eq!(LeagueState::Active.to_string(), "ACTIVE");
        assert_eq!(LeagueState::Archived.to_string(), "ARCHIVED");
    }

    #[test]
    fn league_exposes_its_identity_and_rules() {
        let league = league(OwnershipMode::Shared);
        assert_eq!(league.league_id(), &id(DomainPrefix::League, 'A'));
        assert_eq!(league.owner_id(), &id(DomainPrefix::User, 'B'));
        assert_eq!(league.ownership_mode(), OwnershipMode::Shared);
        let rules = RosterRules::new(SquadComposition::new(2, 1), 15, "3.1.0");
        assert_eq!(rules.composition().total_slots(), 15);
        assert_eq!(rules.ruleset_version(), "3.1.0");
    }

    #[test]
    fn competitions_are_registered_once() {
        let mut league = league(OwnershipMode::Exclusive);
        let competition = id(DomainPrefix::Competition, 'C');
        league.add_competition(competition.clone());
        league.add_competition(competition);
        assert_eq!(league.competitions.len(), 1);
    }

    #[test]
    fn exclusive_leagues_cannot_roster_before_the_auction() {
        let mut league = league(OwnershipMode::Exclusive);
        league.open().expect("open");
        assert_eq!(
            league.start_rostering(),
            Err(LeagueTransitionError {
                from: LeagueState::Open,
                to: LeagueState::Rostering,
            })
        );
    }

    proptest! {
        #[test]
        fn exclusive_claims_are_unique_per_month(
            players in prop::collection::hash_set("[0-9A-HJKMNP-TV-Z]{26}", 1..=8),
        ) {
            let mut league = league(OwnershipMode::Exclusive);
            let month = month();
            for player in &players {
                let identifier = Identifier::new(DomainPrefix::Player, player)?;
                league.claim_player(identifier.clone(), month)?;
                prop_assert!(league.claim_player(identifier, month).is_err());
            }
            prop_assert_eq!(league.claims.len(), players.len());
        }
    }
}
