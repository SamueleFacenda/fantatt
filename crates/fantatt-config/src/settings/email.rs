use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmailProvider {
    Smtp,
    HttpApi,
    Log,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email {
    pub provider: EmailProvider,
    pub from: String,
}

impl Email {
    pub fn names_a_sender(&self) -> bool {
        !self.from.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_variants_use_kebab_case() {
        assert_eq!(
            serde_json::to_string(&EmailProvider::HttpApi).unwrap(),
            "\"http-api\""
        );
        assert_eq!(
            serde_json::to_string(&EmailProvider::Log).unwrap(),
            "\"log\""
        );
    }

    #[test]
    fn a_blank_sender_address_is_rejected() {
        assert!(email("Fantatt <noreply@fantatt.example.org>").names_a_sender());
        assert!(!email("").names_a_sender());
    }

    fn email(from: &str) -> Email {
        Email {
            provider: EmailProvider::Log,
            from: from.to_owned(),
        }
    }
}
