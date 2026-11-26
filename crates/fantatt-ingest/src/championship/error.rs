//! Why a menu or list page could not become a championship catalog. Every
//! variant names the selector, row, or link at fault, so a portal structure
//! change reports where it happened instead of failing silently
//! (docs/06-data-pipeline.md §5).

/// The failure of one championship catalog parse.
#[derive(Debug, thiserror::Error)]
pub enum ChampionshipParseError {
    #[error("the {selector} element naming {purpose} is missing from the page")]
    MissingSelector {
        selector: &'static str,
        purpose: &'static str,
    },

    #[error("the championship link on row {row} names no {param} parameter: {href:?}")]
    MissingParam {
        row: usize,
        param: &'static str,
        href: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_selector_names_the_selector_and_its_purpose() {
        let error = ChampionshipParseError::MissingSelector {
            selector: "p.stile",
            purpose: "the season the championships belong to",
        };
        let message = error.to_string();
        assert!(message.contains("p.stile"), "{message}");
        assert!(message.contains("the season"), "{message}");
    }

    #[test]
    fn a_link_without_a_championship_names_the_row_and_the_href() {
        let error = ChampionshipParseError::MissingParam {
            row: 4,
            param: "CAM",
            href: "campionati.php?CONC=0".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("row 4"), "{message}");
        assert!(message.contains("CAM"), "{message}");
        assert!(message.contains("CONC=0"), "{message}");
    }
}
