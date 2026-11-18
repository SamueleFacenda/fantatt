use serde::{Deserialize, Serialize};

/// The explicit superadmin grant list of docs/10-security.md §4: nothing is
/// implied by account creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authz {
    pub superadmins: Vec<String>,
}

impl Authz {
    /// Whether the user id holds the platform grant.
    pub fn is_superadmin(&self, user_id: &str) -> bool {
        self.superadmins.iter().any(|entry| entry == user_id)
    }

    /// Whether the text names a user id: the `usr_` prefix plus a 26-character
    /// ULID, matching the identifiers `fantatt-core` mints.
    pub fn names_a_user_id(text: &str) -> bool {
        text.starts_with("usr_") && text.len() == USER_ID_LENGTH
    }
}

const USER_ID_LENGTH: usize = 30;

#[cfg(test)]
mod tests {
    use super::*;

    const USER_ID: &str = "usr_01ARZ3XK7M0C4V6B8N2QWE9RTA";

    fn authz(entries: &[&str]) -> Authz {
        Authz {
            superadmins: entries.iter().map(|entry| (*entry).to_owned()).collect(),
        }
    }

    #[test]
    fn the_grant_list_decides_platform_access() {
        assert!(authz(&[USER_ID]).is_superadmin(USER_ID));
        assert!(!authz(&[USER_ID]).is_superadmin("usr_01ARZ3XK7M0C4V6B8N2QWE9RTB"));
        assert!(!authz(&[]).is_superadmin(USER_ID));
    }

    #[test]
    fn only_user_ids_name_a_grant() {
        assert!(Authz::names_a_user_id(USER_ID));
        assert!(!Authz::names_a_user_id("usr_short"));
        assert!(!Authz::names_a_user_id("lg_01ARZ3XK7M0C4V6B8N2QWE9RTA"));
        assert!(!Authz::names_a_user_id(""));
    }
}
