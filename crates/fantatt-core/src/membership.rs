use serde::{Deserialize, Serialize};
use time::Date;

use crate::error::DomainError;
use crate::ids::Identifier;
use crate::money::Money;

/// What a member may do inside a league.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MembershipRole {
    Owner,
    Manager,
}

/// Lifecycle of a membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MembershipState {
    Invited,
    Active,
    Removed,
}

/// A user's participation in one league, owning zero or one squad.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    league_id: Identifier,
    manager_id: Identifier,
    role: MembershipRole,
    joined_at: Date,
    remaining_budget: Money,
    state: MembershipState,
}

impl Membership {
    /// The league creator's membership.
    pub fn owner(league_id: Identifier, manager_id: Identifier, starting_budget: Money) -> Self {
        Self {
            league_id,
            manager_id,
            role: MembershipRole::Owner,
            joined_at: Membership::epoch(),
            remaining_budget: starting_budget,
            state: MembershipState::Active,
        }
    }

    /// A joining manager's membership.
    pub fn manager(league_id: Identifier, manager_id: Identifier, starting_budget: Money) -> Self {
        Self {
            league_id,
            manager_id,
            role: MembershipRole::Manager,
            joined_at: Membership::epoch(),
            remaining_budget: starting_budget,
            state: MembershipState::Active,
        }
    }

    fn epoch() -> Date {
        Date::from_ordinal_date(2026, 1).expect("epoch date")
    }

    pub fn league_id(&self) -> &Identifier {
        &self.league_id
    }

    pub fn manager_id(&self) -> &Identifier {
        &self.manager_id
    }

    pub fn role(&self) -> MembershipRole {
        self.role
    }

    pub fn state(&self) -> MembershipState {
        self.state
    }

    pub fn remaining_budget(&self) -> &Money {
        &self.remaining_budget
    }

    /// Spends from the remaining budget, rejecting overdrafts (INV-12).
    pub fn spend(&mut self, amount: Money) -> Result<(), DomainError> {
        self.check_active()?;
        let remaining =
            self.remaining_budget
                .checked_sub(&amount)
                .ok_or(DomainError::BudgetExceeded {
                    spent: amount,
                    budget: self.remaining_budget.clone(),
                })?;
        self.remaining_budget = remaining;
        Ok(())
    }

    /// Returns money to the budget, for example after a lost resell.
    pub fn refund(&mut self, amount: Money) -> Result<(), DomainError> {
        self.check_active()?;
        let remaining =
            self.remaining_budget
                .checked_add(&amount)
                .ok_or(DomainError::BudgetExceeded {
                    spent: amount,
                    budget: self.remaining_budget.clone(),
                })?;
        self.remaining_budget = remaining;
        Ok(())
    }

    pub fn remove(&mut self) {
        self.state = MembershipState::Removed;
    }

    fn check_active(&self) -> Result<(), DomainError> {
        if self.state != MembershipState::Active {
            return Err(DomainError::MembershipNotActive {
                manager: self.manager_id.clone(),
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

    fn money(minor_units: u64) -> Money {
        Money::new("EUR", minor_units).expect("valid money")
    }

    fn membership() -> Membership {
        Membership::owner(
            id(DomainPrefix::League, 'A'),
            id(DomainPrefix::User, 'B'),
            money(1000),
        )
    }

    #[test]
    fn owner_membership_starts_with_the_full_budget() {
        let membership = membership();
        assert_eq!(membership.role(), MembershipRole::Owner);
        assert_eq!(membership.remaining_budget(), &money(1000));
    }

    #[test]
    fn spending_within_budget_decrements_the_remainder() {
        let mut membership = membership();
        membership.spend(money(400)).expect("within budget");
        assert_eq!(membership.remaining_budget(), &money(600));
    }

    #[test]
    fn overspending_is_rejected() {
        let mut membership = membership();
        let error = membership.spend(money(1001)).expect_err("over budget");
        assert_eq!(error.code(), "BUDGET_EXCEEDED");
        assert_eq!(membership.remaining_budget(), &money(1000));
    }

    #[test]
    fn refunds_return_money() {
        let mut membership = membership();
        membership.spend(money(400)).expect("spend");
        membership.refund(money(150)).expect("refund");
        assert_eq!(membership.remaining_budget(), &money(750));
    }

    #[test]
    fn removed_members_cannot_spend() {
        let mut membership = membership();
        membership.remove();
        assert_eq!(
            membership.spend(money(10)),
            Err(DomainError::MembershipNotActive {
                manager: id(DomainPrefix::User, 'B')
            })
        );
    }

    proptest! {
        #[test]
        fn spend_never_exceeds_the_starting_budget(
            amounts in prop::collection::vec(0u64..300, 1..=12),
        ) {
            let budget = 1000u64;
            let mut membership = membership();
            let mut spent = 0u64;
            for amount in amounts {
                let attempt = money(amount);
                if spent + amount <= budget {
                    membership.spend(attempt)?;
                    spent += amount;
                } else {
                    prop_assert!(membership.spend(attempt).is_err());
                }
            }
            prop_assert!(membership.remaining_budget().minor_units() <= budget);
            prop_assert_eq!(spent + membership.remaining_budget().minor_units(), budget);
        }
    }
}
