use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::Date;

use crate::division::DivisionTier;
use crate::error::DomainError;
use crate::identity::{Alias, PlayerIdentity};
use crate::ids::Identifier;
use crate::rating::{PROVISIONAL_RATING, Rating};

/// Gender band of a FITET competition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
}

/// A current-season affiliation: which competition, tier, and team a player
/// actively plays in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Affiliation {
    competition: Identifier,
    tier: DivisionTier,
    team: Identifier,
}

impl Affiliation {
    pub fn new(competition: Identifier, tier: DivisionTier, team: Identifier) -> Self {
        Self {
            competition,
            tier,
            team,
        }
    }

    pub fn competition(&self) -> &Identifier {
        &self.competition
    }

    pub fn tier(&self) -> DivisionTier {
        self.tier
    }

    pub fn team(&self) -> &Identifier {
        &self.team
    }
}

/// A FITET athlete: canonical identity, rated history, and affiliations.
///
/// Ratings are keyed by their effective date, so a player holds exactly one
/// rating per date (INV-1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    player_id: Identifier,
    fitet_person_id: Option<String>,
    full_name: String,
    gender: Gender,
    ratings: BTreeMap<Date, Rating>,
    affiliations: Vec<Affiliation>,
    identity: PlayerIdentity,
}

impl Player {
    /// A provisional player: unresolved identity, 2000 points, no affiliations.
    pub fn provisional(player_id: Identifier, full_name: &str, gender: Gender) -> Self {
        let identity = PlayerIdentity::unresolved(player_id.clone(), Alias::new(full_name));
        Self {
            player_id,
            fitet_person_id: None,
            full_name: full_name.to_owned(),
            gender,
            ratings: BTreeMap::new(),
            affiliations: Vec::new(),
            identity,
        }
    }

    pub fn player_id(&self) -> &Identifier {
        &self.player_id
    }

    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    pub fn gender(&self) -> Gender {
        self.gender
    }

    pub fn identity(&self) -> &PlayerIdentity {
        &self.identity
    }

    /// Records a rating for its effective date, rejecting duplicates (INV-1).
    pub fn record_rating(&mut self, rating: Rating) -> Result<(), DomainError> {
        if self.ratings.contains_key(&rating.as_of()) {
            return Err(DomainError::DuplicateRatingForDate {
                player: self.player_id.clone(),
                as_of: rating.as_of(),
            });
        }
        self.ratings.insert(rating.as_of(), rating);
        Ok(())
    }

    /// The rating in force on a date, or the latest one before it.
    pub fn rating_as_of(&self, as_of: Date) -> Option<&Rating> {
        self.ratings.range(..=as_of).next_back().map(|(_, r)| r)
    }

    /// Latest known rating; unresolved players rate at the provisional default.
    pub fn current_rating(&self) -> Option<&Rating> {
        self.ratings.values().next_back()
    }

    pub fn add_affiliation(&mut self, affiliation: Affiliation) {
        if !self.affiliations.contains(&affiliation) {
            self.affiliations.push(affiliation);
        }
    }

    /// The tier the player actively plays in, preferring a named competition.
    pub fn active_tier(&self, competition: Option<&Identifier>) -> Option<DivisionTier> {
        self.affiliations
            .iter()
            .find(|affiliation| match competition {
                Some(competition) => affiliation.competition() == competition,
                None => true,
            })
            .map(Affiliation::tier)
    }
}

/// The rating an unresolved player carries until identity resolves.
pub fn provisional_rating_value() -> i64 {
    PROVISIONAL_RATING
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use crate::rating::{Rating, RatingConfidence, RatingProvenance};
    use proptest::prelude::*;

    fn id(prefix: DomainPrefix, last: char) -> Identifier {
        let suffix = format!("01ARZ3XK7M0C4V6B8N2QWE9RT{last}");
        Identifier::new(prefix, &suffix).expect("valid id")
    }

    fn player() -> Player {
        Player::provisional(
            id(DomainPrefix::Player, 'A'),
            "Bendoni Lorenzo",
            Gender::Male,
        )
    }

    fn rating(points: i64, day: u8) -> Rating {
        let date =
            time::Date::from_calendar_date(2026, time::Month::June, day).expect("valid date");
        let provenance = RatingProvenance::new("classifica", RatingConfidence::High);
        Rating::confirmed(points, date, provenance).expect("valid rating")
    }

    #[test]
    fn provisional_player_starts_unresolved() {
        let player = player();
        assert_eq!(
            player.identity().state(),
            crate::identity::IdentityState::Unresolved
        );
        assert_eq!(player.current_rating(), None);
        assert_eq!(provisional_rating_value(), 2000);
    }

    #[test]
    fn one_rating_per_effective_date() {
        let mut player = player();
        player.record_rating(rating(2400, 1)).expect("first rating");
        let error = player
            .record_rating(rating(2450, 1))
            .expect_err("duplicate date is rejected");
        assert_eq!(error.code(), "DUPLICATE_RATING_FOR_DATE");
    }

    #[test]
    fn distinct_dates_accumulate() {
        let mut player = player();
        player.record_rating(rating(2400, 1)).expect("first");
        player.record_rating(rating(2450, 15)).expect("second");
        assert_eq!(player.current_rating().expect("latest").value(), 2450);
    }

    #[test]
    fn rating_as_of_falls_back_to_the_latest_before_the_date() {
        let mut player = player();
        player.record_rating(rating(2400, 1)).expect("first");
        player.record_rating(rating(2500, 20)).expect("second");
        let mid = time::Date::from_calendar_date(2026, time::Month::June, 10).expect("valid date");
        assert_eq!(player.rating_as_of(mid).expect("prior").value(), 2400);
        let late = time::Date::from_calendar_date(2026, time::Month::July, 1).expect("valid date");
        assert_eq!(player.rating_as_of(late).expect("prior").value(), 2500);
    }

    #[test]
    fn active_tier_prefers_the_named_competition() {
        let mut player = player();
        let competition = id(DomainPrefix::Competition, 'B');
        let other = id(DomainPrefix::Competition, 'C');
        let team = id(DomainPrefix::Team, 'D');
        player.add_affiliation(Affiliation::new(
            competition.clone(),
            DivisionTier::C2,
            team.clone(),
        ));
        player.add_affiliation(Affiliation::new(other, DivisionTier::D1, team));
        assert_eq!(
            player
                .active_tier(Some(&competition))
                .expect("named competition wins"),
            DivisionTier::C2
        );
        assert!(player.active_tier(None).is_some());
    }

    #[test]
    fn player_exposes_its_record() {
        let player = player();
        assert_eq!(player.player_id(), &id(DomainPrefix::Player, 'A'));
        assert_eq!(player.full_name(), "Bendoni Lorenzo");
        assert_eq!(player.gender(), Gender::Male);
        assert_eq!(player.identity().internal_id(), player.player_id());
    }

    #[test]
    fn affiliation_exposes_competition_tier_and_team() {
        let affiliation = Affiliation::new(
            id(DomainPrefix::Competition, 'B'),
            DivisionTier::C1,
            id(DomainPrefix::Team, 'D'),
        );
        assert_eq!(
            affiliation.competition(),
            &id(DomainPrefix::Competition, 'B')
        );
        assert_eq!(affiliation.tier(), DivisionTier::C1);
        assert_eq!(affiliation.team(), &id(DomainPrefix::Team, 'D'));
    }

    proptest! {
        #[test]
        fn ratings_stay_unique_per_date(points_a in 0i64..4000, points_b in 0i64..4000, day in 1u8..28) {
            let mut player = player();
            let first = rating(points_a, day);
            player.record_rating(first)?;
            let second = rating(points_b, day);
            prop_assert!(player.record_rating(second).is_err());
            prop_assert_eq!(player.ratings.len(), 1);
        }
    }
}
