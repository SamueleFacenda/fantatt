use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Auth {
    pub google: Google,
    pub passkey: Passkey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Google {
    pub enabled: bool,
    pub client_id: String,
    pub client_secret: SecretRef,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Passkey {
    pub enabled: bool,
    pub rp_id: String,
    pub rp_origin: String,
}

impl Google {
    pub fn is_fully_configured(&self) -> bool {
        !self.enabled
            || (!self.client_id.trim().is_empty()
                && !self.redirect_uri.trim().is_empty()
                && !self.scopes.is_empty())
    }
}

impl Passkey {
    pub fn is_fully_configured(&self) -> bool {
        !self.enabled || (!self.rp_id.trim().is_empty() && !self.rp_origin.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn google(enabled: bool) -> Google {
        Google {
            enabled,
            client_id: "client-id".to_owned(),
            client_secret: SecretRef::try_from(
                "env:FANTATT__AUTH__GOOGLE__CLIENT_SECRET".to_owned(),
            )
            .unwrap(),
            scopes: vec!["openid".to_owned()],
            redirect_uri: "https://fantatt.example.org/callback".to_owned(),
        }
    }

    #[test]
    fn an_enabled_google_needs_every_settable_field() {
        assert!(google(true).is_fully_configured());
        let mut blank_id = google(true);
        blank_id.client_id = String::new();
        assert!(!blank_id.is_fully_configured());
        let mut blank_uri = google(true);
        blank_uri.redirect_uri = "  ".to_owned();
        assert!(!blank_uri.is_fully_configured());
    }

    #[test]
    fn a_disabled_google_needs_nothing() {
        let mut off = google(false);
        off.client_id = String::new();
        off.redirect_uri = String::new();
        off.scopes.clear();
        assert!(off.is_fully_configured());
    }

    #[test]
    fn an_enabled_passkey_needs_the_relying_party_identity() {
        let mut passkey = Passkey {
            enabled: true,
            rp_id: "fantatt.example.org".to_owned(),
            rp_origin: "https://fantatt.example.org".to_owned(),
        };
        assert!(passkey.is_fully_configured());
        passkey.rp_origin = String::new();
        assert!(!passkey.is_fully_configured());
    }
}
