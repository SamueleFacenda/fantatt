use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notifications {
    pub push_enabled: bool,
    pub vapid_public_key: String,
    pub vapid_private_key: SecretRef,
}

impl Notifications {
    /// Web push needs the public half of the VAPID pair in config; the private
    /// half is a reference resolved at startup.
    pub fn is_coherent(&self) -> bool {
        !self.push_enabled || !self.vapid_public_key.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_needs_the_public_vapid_key() {
        let mut notifications = notifications(true, "BPublicKey");
        assert!(notifications.is_coherent());
        notifications.vapid_public_key = String::new();
        assert!(!notifications.is_coherent());
    }

    #[test]
    fn a_silent_deployment_accepts_no_keys() {
        assert!(notifications(false, "").is_coherent());
    }

    fn notifications(push_enabled: bool, public_key: &str) -> Notifications {
        Notifications {
            push_enabled,
            vapid_public_key: public_key.to_owned(),
            vapid_private_key: SecretRef::try_from(
                "env:FANTATT__NOTIFICATIONS__VAPID_PRIVATE_KEY".to_owned(),
            )
            .unwrap(),
        }
    }
}
