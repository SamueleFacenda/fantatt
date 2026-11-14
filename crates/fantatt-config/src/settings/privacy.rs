use serde::{Deserialize, Serialize};

pub const MINIMUM_DELETION_GRACE_DAYS: u32 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Privacy {
    pub policy_version: String,
    pub cookie_notice_version: String,
    pub deletion_grace_days: u32,
    pub minimum_age: u16,
}

impl Privacy {
    /// The purge window is a contractual guarantee granted to every manager, so
    /// a shorter window is rejected rather than silently honoured.
    pub fn keeps_the_purge_promise(&self) -> bool {
        self.deletion_grace_days >= MINIMUM_DELETION_GRACE_DAYS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grace_window_may_never_shrink_below_thirty_days() {
        let mut privacy = Privacy {
            policy_version: "2026-11-10".to_owned(),
            cookie_notice_version: "2026-11-10".to_owned(),
            deletion_grace_days: MINIMUM_DELETION_GRACE_DAYS,
            minimum_age: 16,
        };
        assert!(privacy.keeps_the_purge_promise());
        privacy.deletion_grace_days = MINIMUM_DELETION_GRACE_DAYS - 1;
        assert!(!privacy.keeps_the_purge_promise());
        privacy.deletion_grace_days = 90;
        assert!(privacy.keeps_the_purge_promise());
    }
}
