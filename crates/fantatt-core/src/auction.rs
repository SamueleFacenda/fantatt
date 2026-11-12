use serde::{Deserialize, Serialize};
use time::Date;

use crate::error::DomainError;
use crate::ids::Identifier;
use crate::money::Money;

/// Lifecycle of an auctioned player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LotStatus {
    Open,
    Sold,
    Unsold,
}

/// The lot, its league, and the player under auction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotIdentity {
    lot_id: Identifier,
    league_id: Identifier,
    player_id: Identifier,
}

impl LotIdentity {
    pub fn new(lot_id: Identifier, league_id: Identifier, player_id: Identifier) -> Self {
        Self {
            lot_id,
            league_id,
            player_id,
        }
    }
}

/// Who bid how much.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bid {
    bidder: Identifier,
    amount: Money,
}

impl Bid {
    pub fn new(bidder: Identifier, amount: Money) -> Self {
        Self { bidder, amount }
    }
}

/// A bid together with the moment it was placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferPlacement {
    bid: Bid,
    offered_at: Date,
}

impl OfferPlacement {
    pub fn new(bid: Bid, offered_at: Date) -> Self {
        Self { bid, offered_at }
    }
}

/// One bid or counter-offer on a lot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    offer_id: Identifier,
    lot_id: Identifier,
    bid: Bid,
    offered_at: Date,
}

impl Offer {
    pub fn new(offer_id: Identifier, lot_id: Identifier, placement: OfferPlacement) -> Self {
        Self {
            offer_id,
            lot_id,
            bid: placement.bid,
            offered_at: placement.offered_at,
        }
    }

    pub fn offer_id(&self) -> &Identifier {
        &self.offer_id
    }

    pub fn bidder(&self) -> &Identifier {
        &self.bid.bidder
    }

    pub fn amount(&self) -> &Money {
        &self.bid.amount
    }
}

/// One player under auction, with its offers and close window.
///
/// A lot accepts an offer only while the window is open and only when the
/// offer is strictly the highest (INV-11), so overbidding an accepted bid is
/// impossible even under concurrent bidders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lot {
    lot_id: Identifier,
    league_id: Identifier,
    player_id: Identifier,
    status: LotStatus,
    deadline: Date,
    offers: Vec<Offer>,
    current_best: Option<Offer>,
}

impl Lot {
    pub fn new(identity: LotIdentity, deadline: Date) -> Self {
        Self {
            lot_id: identity.lot_id,
            league_id: identity.league_id,
            player_id: identity.player_id,
            status: LotStatus::Open,
            deadline,
            offers: Vec::new(),
            current_best: None,
        }
    }

    pub fn lot_id(&self) -> &Identifier {
        &self.lot_id
    }

    pub fn player_id(&self) -> &Identifier {
        &self.player_id
    }

    pub fn status(&self) -> LotStatus {
        self.status
    }

    pub fn current_best(&self) -> Option<&Offer> {
        self.current_best.as_ref()
    }

    /// Places an offer while the window is open and it beats the best bid.
    pub fn place_offer(&mut self, offer: Offer, as_of: Date) -> Result<(), DomainError> {
        self.check_window_open(as_of)?;
        self.check_highest(offer.amount())?;
        self.offers.push(offer.clone());
        self.current_best = Some(offer);
        Ok(())
    }

    fn check_window_open(&self, as_of: Date) -> Result<(), DomainError> {
        if self.status != LotStatus::Open || as_of > self.deadline {
            return Err(DomainError::AuctionWindowClosed {
                lot: self.lot_id.clone(),
            });
        }
        Ok(())
    }

    fn check_highest(&self, amount: &Money) -> Result<(), DomainError> {
        let Some(best) = &self.current_best else {
            return Ok(());
        };
        if amount <= best.amount() {
            return Err(DomainError::OfferNotHighest {
                lot: self.lot_id.clone(),
                offered: amount.clone(),
                best: best.amount().clone(),
            });
        }
        Ok(())
    }

    /// Closes the window: the best offer wins, or the lot goes unsold.
    pub fn close(&mut self) -> Option<Offer> {
        self.status = match &self.current_best {
            Some(_) => LotStatus::Sold,
            None => LotStatus::Unsold,
        };
        self.current_best.clone()
    }

    /// A lot is still bidding while open and before its deadline.
    pub fn is_open_at(&self, as_of: Date) -> bool {
        self.status == LotStatus::Open && as_of <= self.deadline
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

    fn money(minor_units: u64) -> Money {
        Money::new("EUR", minor_units).expect("valid money")
    }

    fn lot() -> Lot {
        Lot::new(
            LotIdentity::new(
                id(DomainPrefix::Lot, 'A'),
                id(DomainPrefix::League, 'B'),
                id(DomainPrefix::Player, 'C'),
            ),
            Date::from_ordinal_date(2026, 100).expect("deadline"),
        )
    }

    fn offer(last: char, minor_units: u64) -> Offer {
        Offer::new(
            id(DomainPrefix::Offer, last),
            id(DomainPrefix::Lot, 'A'),
            OfferPlacement::new(
                Bid::new(id(DomainPrefix::User, last), money(minor_units)),
                Date::from_ordinal_date(2026, 90).expect("valid date"),
            ),
        )
    }

    fn deadline() -> Date {
        Date::from_ordinal_date(2026, 100).expect("deadline")
    }

    #[test]
    fn first_offer_becomes_the_best() {
        let mut lot = lot();
        lot.place_offer(offer('A', 100), deadline())
            .expect("first offer accepted");
        assert_eq!(lot.current_best().expect("best").amount(), &money(100));
    }

    #[test]
    fn higher_offers_replace_the_best() {
        let mut lot = lot();
        lot.place_offer(offer('A', 100), deadline()).expect("first");
        lot.place_offer(offer('B', 150), deadline())
            .expect("higher");
        assert_eq!(lot.current_best().expect("best").amount(), &money(150));
    }

    #[test]
    fn equal_or_lower_offers_are_rejected() {
        let mut lot = lot();
        lot.place_offer(offer('A', 150), deadline()).expect("first");
        let error = lot
            .place_offer(offer('B', 150), deadline())
            .expect_err("a tie cannot replace the best");
        assert_eq!(error.code(), "OFFER_NOT_HIGHEST");
        let error = lot
            .place_offer(offer('C', 149), deadline())
            .expect_err("a lower offer cannot replace the best");
        assert_eq!(error.code(), "OFFER_NOT_HIGHEST");
    }

    #[test]
    fn offers_after_the_deadline_are_rejected() {
        let mut lot = lot();
        let after = Date::from_ordinal_date(2026, 101).expect("valid date");
        let error = lot
            .place_offer(offer('A', 100), after)
            .expect_err("the window closed");
        assert_eq!(error.code(), "AUCTION_WINDOW_CLOSED");
    }

    #[test]
    fn closing_sells_to_the_best_offer() {
        let mut lot = lot();
        lot.place_offer(offer('A', 100), deadline()).expect("first");
        lot.place_offer(offer('B', 200), deadline())
            .expect("second");
        let winner = lot.close().expect("sold");
        assert_eq!(winner.amount(), &money(200));
        assert_eq!(lot.status(), LotStatus::Sold);
        assert!(lot.place_offer(offer('C', 300), deadline()).is_err());
    }

    #[test]
    fn closing_without_offers_leaves_the_lot_unsold() {
        let mut lot = lot();
        assert!(lot.close().is_none());
        assert_eq!(lot.status(), LotStatus::Unsold);
    }

    proptest! {
        #[test]
        fn only_strictly_higher_offers_win(
            amounts in prop::collection::vec(1u64..1_000, 1..=10),
        ) {
            let mut lot = lot();
            let mut best = 0u64;
            for (index, amount) in amounts.iter().enumerate() {
                let letter = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"[index] as char;
                let attempt = offer(letter, *amount);
                if *amount > best {
                    lot.place_offer(attempt, deadline())?;
                    best = *amount;
                } else {
                    prop_assert!(lot.place_offer(attempt, deadline()).is_err());
                }
            }
            prop_assert_eq!(
                lot.current_best().map(|offer| offer.amount().clone()),
                Some(money(best))
            );
        }
    }
}
