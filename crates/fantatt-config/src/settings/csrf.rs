use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Csrf {
    pub enabled: bool,
    pub header: String,
}

impl Csrf {
    pub fn names_a_header(&self) -> bool {
        !self.header.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_header_name_is_incoherent() {
        let csrf = Csrf {
            enabled: true,
            header: "X-CSRF-Token".to_owned(),
        };
        assert!(csrf.names_a_header());
        let blank = Csrf {
            enabled: true,
            header: "  ".to_owned(),
        };
        assert!(!blank.names_a_header());
    }
}
