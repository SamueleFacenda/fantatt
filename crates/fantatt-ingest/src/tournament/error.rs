//! Why a tournament page could not become the canonical record. Every
//! variant names the selector, row, or link at fault, so a portal structure
//! change reports where it happened instead of failing silently
//! (docs/06-data-pipeline.md §5).

/// The failure of one tournament parse.
#[derive(Debug, thiserror::Error)]
pub enum TournamentParseError {
    #[error("the {selector} element naming {purpose} is missing from the page")]
    MissingSelector {
        selector: &'static str,
        purpose: &'static str,
    },

    #[error("row {row} names no {param} parameter in its link: {href:?}")]
    MissingParam {
        row: usize,
        param: &'static str,
        href: String,
    },

    #[error("row {row} prints no {purpose}")]
    MissingCell { row: usize, purpose: &'static str },

    #[error("row {row} names no calendar date: {text:?}")]
    MalformedDate { row: usize, text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_selector_names_the_selector_and_its_purpose() {
        let error = TournamentParseError::MissingSelector {
            selector: "p.stile",
            purpose: "the tournament family and its season",
        };
        let message = error.to_string();
        assert!(message.contains("p.stile"), "{message}");
        assert!(message.contains("the tournament family"), "{message}");
    }

    #[test]
    fn a_link_without_a_tournament_names_the_row_and_the_href() {
        let error = TournamentParseError::MissingParam {
            row: 3,
            param: "IDT",
            href: "visualizza_risultati.php?ANNO=41&COM=6".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("row 3"), "{message}");
        assert!(message.contains("IDT"), "{message}");
        assert!(message.contains("ANNO=41"), "{message}");
    }

    #[test]
    fn a_row_without_a_place_names_the_row_and_the_purpose() {
        let error = TournamentParseError::MissingCell {
            row: 2,
            purpose: "the place the tournament was played in",
        };
        assert!(error.to_string().contains("row 2"), "{error}");
        assert!(error.to_string().contains("the place"), "{error}");
    }

    #[test]
    fn a_row_without_a_date_names_the_row_and_the_text() {
        let error = TournamentParseError::MalformedDate {
            row: 1,
            text: "domenica".to_string(),
        };
        assert!(error.to_string().contains("domenica"), "{error}");
    }
}
