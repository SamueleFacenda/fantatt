use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crypto {
    /// Session and CSRF signing key. The deployment provides it so nodes can
    /// validate each other's cookies; a single node may generate a fresh value
    /// into its env file at every boot.
    pub session_secret: SecretRef,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_is_held_by_reference_only() {
        let crypto = Crypto {
            session_secret: SecretRef::try_from("env:FANTATT__CRYPTO__SESSION_SECRET".to_owned())
                .unwrap(),
        };
        assert_eq!(
            crypto.session_secret.variable_name(),
            "FANTATT__CRYPTO__SESSION_SECRET"
        );
    }

    #[test]
    fn a_literal_secret_never_fits_the_type() {
        let literal = serde_json::to_string(&"hunter2").unwrap();
        assert!(serde_json::from_str::<Crypto>(&literal).is_err());
    }
}
