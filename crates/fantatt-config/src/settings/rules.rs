use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub active_bundle: String,
    pub bundle_path: String,
    pub verify_proofs: bool,
}

impl Rules {
    pub fn names_a_bundle(&self) -> bool {
        !self.active_bundle.trim().is_empty() && !self.bundle_path.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_bundle_identifiers_must_be_present() {
        let mut rules = Rules {
            active_bundle: "fantatt-core/1.4.0".to_owned(),
            bundle_path: "/var/lib/fantatt/rules".to_owned(),
            verify_proofs: true,
        };
        assert!(rules.names_a_bundle());
        rules.active_bundle = "  ".to_owned();
        assert!(!rules.names_a_bundle());
    }
}
