use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::Identifier;

/// Whether a player played singles or doubles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AppearanceKind {
    Singles,
    Doubles,
}

/// Whether the player won or lost the match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Outcome {
    Win,
    Loss,
}

/// One set score from the player's perspective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetScore {
    player: u8,
    opponent: u8,
}

impl SetScore {
    pub fn new(player: u8, opponent: u8) -> Self {
        Self { player, opponent }
    }

    pub fn player(&self) -> u8 {
        self.player
    }

    pub fn opponent(&self) -> u8 {
        self.opponent
    }

    pub fn is_won(&self) -> bool {
        self.player > self.opponent
    }
}

/// The flags the reasoning engine derives from a match result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AppearanceFlags {
    pub is_decider: bool,
    pub is_comeback: bool,
    pub is_away: bool,
    pub is_team_win: bool,
    pub is_doublet: bool,
}

/// Who a player met, and how strong they were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Matchup {
    partner: Option<Identifier>,
    opponent: Option<Identifier>,
    opponent_rating: Option<i64>,
}

impl Matchup {
    pub fn new(
        partner: Option<Identifier>,
        opponent: Option<Identifier>,
        opponent_rating: Option<i64>,
    ) -> Self {
        Self {
            partner,
            opponent,
            opponent_rating,
        }
    }

    pub fn partner(&self) -> Option<&Identifier> {
        self.partner.as_ref()
    }

    pub fn opponent(&self) -> Option<&Identifier> {
        self.opponent.as_ref()
    }

    pub fn opponent_rating(&self) -> Option<i64> {
        self.opponent_rating
    }
}

/// How the sets fell, from the player's perspective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sets {
    scoreline: Vec<SetScore>,
}

impl Sets {
    pub fn new(scoreline: Vec<SetScore>) -> Self {
        Self { scoreline }
    }

    pub fn scoreline(&self) -> &[SetScore] {
        &self.scoreline
    }

    pub fn won(&self) -> u16 {
        self.scoreline.iter().filter(|set| set.is_won()).count() as u16
    }

    pub fn lost(&self) -> u16 {
        self.scoreline.iter().filter(|set| !set.is_won()).count() as u16
    }
}

/// One real match involving one player: the asserted input to scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAppearance {
    appearance_id: Identifier,
    player_id: Identifier,
    matchday_id: Identifier,
    tie_id: Identifier,
    kind: AppearanceKind,
    matchup: Matchup,
    played: bool,
    outcome: Option<Outcome>,
    sets: Sets,
    flags: AppearanceFlags,
}

/// Which match an appearance belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceKey {
    appearance_id: Identifier,
    player_id: Identifier,
    matchday_id: Identifier,
    tie_id: Identifier,
}

impl AppearanceKey {
    pub fn new(appearance_id: Identifier, context: AppearanceContext) -> Self {
        Self {
            appearance_id,
            player_id: context.player_id,
            matchday_id: context.matchday_id,
            tie_id: context.tie_id,
        }
    }
}

/// The match and matchday an appearance belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceContext {
    player_id: Identifier,
    matchday_id: Identifier,
    tie_id: Identifier,
}

impl AppearanceContext {
    pub fn new(player_id: Identifier, matchday_id: Identifier, tie_id: Identifier) -> Self {
        Self {
            player_id,
            matchday_id,
            tie_id,
        }
    }
}

/// The raw facts of an appearance before validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceFacts {
    key: AppearanceKey,
    contest: AppearanceContest,
}

impl AppearanceFacts {
    pub fn new(key: AppearanceKey, contest: AppearanceContest) -> Self {
        Self { key, contest }
    }
}

/// The contested part of an appearance: who met whom, and how it fell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceContest {
    pub kind: AppearanceKind,
    pub matchup: Matchup,
    pub played: bool,
    pub outcome: Option<Outcome>,
    pub sets: Sets,
    pub flags: AppearanceFlags,
}

impl PlayerAppearance {
    /// Builds an appearance, checking the match-shape rules.
    pub fn new(facts: AppearanceFacts) -> Result<Self, DomainError> {
        let appearance = Self {
            appearance_id: facts.key.appearance_id,
            player_id: facts.key.player_id,
            matchday_id: facts.key.matchday_id,
            tie_id: facts.key.tie_id,
            kind: facts.contest.kind,
            matchup: facts.contest.matchup,
            played: facts.contest.played,
            outcome: facts.contest.outcome,
            sets: facts.contest.sets,
            flags: facts.contest.flags,
        };
        appearance.check_match_shape()?;
        Ok(appearance)
    }

    pub fn appearance_id(&self) -> &Identifier {
        &self.appearance_id
    }

    pub fn player_id(&self) -> &Identifier {
        &self.player_id
    }

    pub fn matchday_id(&self) -> &Identifier {
        &self.matchday_id
    }

    pub fn tie_id(&self) -> &Identifier {
        &self.tie_id
    }

    pub fn kind(&self) -> AppearanceKind {
        self.kind
    }

    pub fn matchup(&self) -> &Matchup {
        &self.matchup
    }

    pub fn played(&self) -> bool {
        self.played
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    pub fn sets(&self) -> &Sets {
        &self.sets
    }

    pub fn flags(&self) -> AppearanceFlags {
        self.flags
    }

    /// Singles matches name an opponent and no partner; doubles the reverse.
    fn check_match_shape(&self) -> Result<(), DomainError> {
        let (partner, opponent) = (self.matchup.partner(), self.matchup.opponent());
        let shaped = match self.kind {
            AppearanceKind::Singles => partner.is_none() && opponent.is_some(),
            AppearanceKind::Doubles => partner.is_some(),
        };
        if !shaped {
            return Err(DomainError::InvalidAppearance {
                appearance: self.appearance_id.clone(),
            });
        }
        if self.played && self.outcome.is_none() {
            return Err(DomainError::InvalidAppearance {
                appearance: self.appearance_id.clone(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use proptest::prelude::*;

    fn id(prefix: DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    fn key() -> AppearanceKey {
        AppearanceKey::new(
            id(DomainPrefix::Appearance, 'A'),
            AppearanceContext::new(
                id(DomainPrefix::Player, 'B'),
                id(DomainPrefix::Matchday, 'C'),
                id(DomainPrefix::Tie, 'D'),
            ),
        )
    }

    fn contest(
        kind: AppearanceKind,
        partner: Option<Identifier>,
        played: bool,
    ) -> AppearanceContest {
        AppearanceContest {
            kind,
            matchup: Matchup::new(partner, Some(id(DomainPrefix::Player, 'F')), Some(2100)),
            played,
            outcome: played.then_some(Outcome::Win),
            sets: Sets::new(vec![SetScore::new(11, 8), SetScore::new(11, 9)]),
            flags: AppearanceFlags {
                is_team_win: true,
                ..AppearanceFlags::default()
            },
        }
    }

    fn facts(kind: AppearanceKind, partner: Option<Identifier>, played: bool) -> AppearanceFacts {
        AppearanceFacts::new(key(), contest(kind, partner, played))
    }

    #[test]
    fn singles_appearance_names_an_opponent() {
        let appearance = PlayerAppearance::new(facts(AppearanceKind::Singles, None, true))
            .expect("valid singles appearance");
        assert_eq!(appearance.kind(), AppearanceKind::Singles);
        assert_eq!(appearance.outcome(), Some(Outcome::Win));
        assert_eq!(appearance.sets().won(), 2);
        assert_eq!(appearance.sets().lost(), 0);
        assert!(appearance.flags().is_team_win);
    }

    #[test]
    fn doubles_appearance_names_a_partner() {
        let appearance = PlayerAppearance::new(facts(
            AppearanceKind::Doubles,
            Some(id(DomainPrefix::Player, 'E')),
            true,
        ))
        .expect("valid doubles appearance");
        assert_eq!(
            appearance.appearance_id().prefix(),
            DomainPrefix::Appearance
        );
        assert_eq!(appearance.player_id(), &id(DomainPrefix::Player, 'B'));
    }

    #[test]
    fn an_appearance_reads_back_its_contest() {
        let appearance = PlayerAppearance::new(facts(
            AppearanceKind::Doubles,
            Some(id(DomainPrefix::Player, 'E')),
            true,
        ))
        .expect("valid doubles appearance");
        assert_eq!(appearance.matchday_id(), &id(DomainPrefix::Matchday, 'C'));
        assert_eq!(appearance.tie_id(), &id(DomainPrefix::Tie, 'D'));
        assert!(appearance.played());
        assert_eq!(
            appearance.matchup().partner(),
            Some(&id(DomainPrefix::Player, 'E'))
        );
        assert_eq!(appearance.matchup().opponent_rating(), Some(2100));
    }

    #[test]
    fn singles_with_a_partner_is_rejected() {
        let error = PlayerAppearance::new(facts(
            AppearanceKind::Singles,
            Some(id(DomainPrefix::Player, 'E')),
            true,
        ))
        .expect_err("singles cannot name a partner");
        assert_eq!(error.code(), "INVALID_APPEARANCE");
    }

    #[test]
    fn doubles_without_a_partner_is_rejected() {
        let error = PlayerAppearance::new(facts(AppearanceKind::Doubles, None, true))
            .expect_err("doubles require a partner");
        assert_eq!(error.code(), "INVALID_APPEARANCE");
    }

    #[test]
    fn played_without_outcome_is_rejected() {
        let contest = AppearanceContest {
            outcome: None,
            ..contest(AppearanceKind::Singles, None, true)
        };
        assert!(PlayerAppearance::new(AppearanceFacts::new(key(), contest)).is_err());
    }

    proptest! {
        #[test]
        fn set_counts_match_the_scoreline(
            scoreline in prop::collection::vec((0u8..12, 0u8..12), 1..=5),
        ) {
            let sets: Vec<SetScore> = scoreline
                .iter()
                .map(|(player, opponent)| SetScore::new(*player, *opponent))
                .collect();
            let sets = Sets::new(sets);
            let won = sets
                .scoreline()
                .iter()
                .filter(|set| set.player() > set.opponent())
                .count() as u16;
            prop_assert_eq!(sets.won(), won);
            prop_assert_eq!(sets.lost(), sets.scoreline().len() as u16 - won);
        }
    }
}
