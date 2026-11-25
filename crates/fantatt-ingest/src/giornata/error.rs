//! Why a referto page could not become a canonical matchday record. Every
//! variant names the selector, row, or cell at fault, so a portal structure
//! change reports where it happened instead of failing silently
//! (docs/06-data-pipeline.md §5).

/// The failure of one giornata parse.
#[derive(Debug, thiserror::Error)]
pub enum GiornataParseError {
    #[error("the {selector} element naming {purpose} is missing from the referto")]
    MissingSelector {
        selector: &'static str,
        purpose: &'static str,
    },

    #[error("row {row} has no cell {cell}")]
    MissingCell { row: usize, cell: usize },

    #[error("row {row} cell {cell} does not hold a point value: {text:?}")]
    MalformedPoints {
        row: usize,
        cell: usize,
        text: String,
    },

    #[error("row {row} carries {sets} sets; a tie is decided in three to five")]
    SetCount { row: usize, sets: usize },

    #[error(
        "row {row} declares {declared} sets won for the {side} player but the scores show {computed}"
    )]
    DeclaredSets {
        row: usize,
        side: &'static str,
        declared: usize,
        computed: usize,
    },

    #[error("row {row} leaves the {side} player unnamed")]
    EmptyPlayer { row: usize, side: &'static str },

    #[error("the referto reports {played} of {total} ties played; the page is incomplete")]
    PartialBracket { played: u32, total: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_selector_names_the_selector_and_its_purpose() {
        let error = GiornataParseError::MissingSelector {
            selector: "p.incontro",
            purpose: "the two societies",
        };
        let message = error.to_string();
        assert!(message.contains("p.incontro"), "{message}");
        assert!(message.contains("the two societies"), "{message}");
    }

    #[test]
    fn a_malformed_point_names_the_row_and_the_cell() {
        let error = GiornataParseError::MalformedPoints {
            row: 2,
            cell: 7,
            text: "undici".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("row 2"), "{message}");
        assert!(message.contains("cell 7"), "{message}");
        assert!(message.contains("undici"), "{message}");
    }

    #[test]
    fn a_partial_bracket_reports_its_counts() {
        let error = GiornataParseError::PartialBracket {
            played: 63,
            total: 72,
        };
        let message = error.to_string();
        assert!(message.contains("63"), "{message}");
        assert!(message.contains("72"), "{message}");
    }
}
