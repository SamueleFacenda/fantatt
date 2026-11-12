use serde::{Deserialize, Serialize};
use time::{Date, Duration};

use crate::error::DomainError;
use crate::ids::Identifier;

/// Personal data is purged within this many days of an account deletion.
pub const PURGE_WINDOW_DAYS: i64 = 30;

/// A deletion request that must complete within the purge window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurgeRequest {
    manager: Identifier,
    requested_at: Date,
}

impl PurgeRequest {
    pub fn new(manager: Identifier, requested_at: Date) -> Self {
        Self {
            manager,
            requested_at,
        }
    }

    pub fn manager(&self) -> &Identifier {
        &self.manager
    }

    /// The last day personal data may still exist.
    pub fn due_by(&self) -> Date {
        self.requested_at
            .checked_add(Duration::days(PURGE_WINDOW_DAYS))
            .expect("date within range")
    }

    pub fn is_overdue(&self, as_of: Date) -> bool {
        as_of > self.due_by()
    }
}

/// The anonymised stand-in for a purged manager.
///
/// League history anonymises managers rather than deleting their matchday
/// scores, so standings stay intact (INV-15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    manager: Identifier,
    display_name: String,
}

impl Tombstone {
    pub fn anonymised(manager: Identifier) -> Self {
        Self {
            manager,
            display_name: "Deleted manager".to_owned(),
        }
    }

    pub fn manager(&self) -> &Identifier {
        &self.manager
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

/// The outcome of a purge: the tombstone that replaced the manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeOutcome {
    tombstone: Tombstone,
}

impl PurgeOutcome {
    pub fn tombstone(&self) -> &Tombstone {
        &self.tombstone
    }
}

impl PurgeRequest {
    /// Executes the purge within the window, anonymising the manager.
    ///
    /// Executing after the window means personal data outlived its retention
    /// limit, which is itself a violation (INV-15).
    pub fn execute(&self, as_of: Date) -> Result<PurgeOutcome, DomainError> {
        if self.is_overdue(as_of) {
            return Err(DomainError::PurgeWindowExceeded {
                manager: self.manager.clone(),
                days_retained: retention_days(self.requested_at, as_of),
            });
        }
        Ok(PurgeOutcome {
            tombstone: Tombstone::anonymised(self.manager.clone()),
        })
    }
}

fn retention_days(requested_at: Date, as_of: Date) -> u16 {
    let days = (as_of - requested_at).whole_days();
    days.clamp(0, i64::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DomainPrefix;
    use proptest::prelude::*;

    fn id() -> Identifier {
        Identifier::new(DomainPrefix::User, "01ARZ3XK7M0C4V6B8N2QWE9RTA").expect("valid id")
    }

    fn request() -> PurgeRequest {
        PurgeRequest::new(id(), Date::from_ordinal_date(2026, 1).expect("valid date"))
    }

    fn day(ordinal: i32) -> Date {
        Date::from_ordinal_date(2026, ordinal as u16).expect("valid date")
    }

    #[test]
    fn the_purge_window_is_thirty_days() {
        let request = request();
        assert_eq!(request.due_by(), day(1 + PURGE_WINDOW_DAYS as i32));
        assert_eq!(PURGE_WINDOW_DAYS, 30);
    }

    #[test]
    fn executing_within_the_window_anonymises_the_manager() {
        let request = request();
        let outcome = request.execute(day(20)).expect("within the window");
        assert_eq!(outcome.tombstone().display_name(), "Deleted manager");
        assert_eq!(outcome.tombstone().manager(), &id());
    }

    #[test]
    fn executing_on_the_last_day_is_still_in_time() {
        let request = request();
        assert!(request.execute(request.due_by()).is_ok());
    }

    #[test]
    fn executing_after_the_window_is_a_violation() {
        let request = request();
        let error = request.execute(day(32)).expect_err("too late");
        assert_eq!(error.code(), "PURGE_WINDOW_EXCEEDED");
        assert!(request.is_overdue(day(32)));
    }

    proptest! {
        #[test]
        fn execution_holds_only_inside_the_window(offset in 0i32..60) {
            let request = request();
            let as_of = day(1 + offset);
            let within = offset <= PURGE_WINDOW_DAYS as i32;
            prop_assert_eq!(request.execute(as_of).is_ok(), within);
            prop_assert_eq!(request.is_overdue(as_of), !within);
        }
    }
}
