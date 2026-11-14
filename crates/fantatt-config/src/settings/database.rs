use serde::{Deserialize, Serialize};

use crate::secret::SecretRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Database {
    pub url: SecretRef,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u32,
}

impl Database {
    pub fn pool_is_shaped_correctly(&self) -> bool {
        self.min_connections >= 1 && self.min_connections <= self.max_connections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database(max: u32, min: u32) -> Database {
        Database {
            url: SecretRef::try_from("env:FANTATT__DATABASE__URL".to_owned()).unwrap(),
            max_connections: max,
            min_connections: min,
            connect_timeout_seconds: 10,
        }
    }

    #[test]
    fn pool_shape_requires_a_usable_range() {
        assert!(database(20, 2).pool_is_shaped_correctly());
        assert!(!database(2, 20).pool_is_shaped_correctly());
        assert!(!database(20, 0).pool_is_shaped_correctly());
    }
}
