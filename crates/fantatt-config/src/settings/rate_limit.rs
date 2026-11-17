use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RateLimitStore {
    Redis,
    Memory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    pub store: RateLimitStore,
    pub rules: BTreeMap<String, String>,
}

impl RateLimit {
    pub fn covers(&self, route: &str) -> bool {
        self.rules.contains_key(route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit() -> RateLimit {
        let mut rules = BTreeMap::new();
        rules.insert("auth".to_owned(), "10/min per ip".to_owned());
        rules.insert("bid".to_owned(), "20/min per user".to_owned());
        RateLimit {
            store: RateLimitStore::Memory,
            rules,
        }
    }

    #[test]
    fn store_variants_use_kebab_case() {
        assert_eq!(
            serde_json::to_string(&RateLimitStore::Redis).unwrap(),
            "\"redis\""
        );
    }

    #[test]
    fn documented_routes_are_covered() {
        assert!(limit().covers("auth"));
        assert!(!limit().covers("unknown-route"));
    }

    #[test]
    fn defaults_carry_every_documented_budget() {
        let defaults = include_str!("../defaults.toml");
        for budget in ["auth", "auth-account", "bid", "search", "reads"] {
            assert!(
                defaults.contains(&format!("{budget} = \"")),
                "the defaults miss the {budget} budget of docs/07-api-spec.md §14"
            );
        }
    }
}
