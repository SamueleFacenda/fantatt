//! Why a classification page could not become the canonical record. Every
//! variant names the selector, row, or cell at fault, so a portal structure
//! change reports where it happened instead of failing silently
//! (docs/06-data-pipeline.md §5).

/// The failure of one classification parse.
#[derive(Debug, thiserror::Error)]
pub enum ClassificationParseError {
    #[error("the {selector} element naming {purpose} is missing from the page")]
    MissingSelector {
        selector: &'static str,
        purpose: &'static str,
    },

    #[error("row {row} names no athlete in its link: {onclick:?}")]
    MissingAthlete { row: usize, onclick: String },

    #[error("row {row} cell {cell} is missing from the ranking row")]
    MissingCell { row: usize, cell: usize },

    #[error("the heading names no athlete count: {text:?}")]
    MalformedCount { text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_selector_names_the_selector_and_its_purpose() {
        let error = ClassificationParseError::MissingSelector {
            selector: "p.titoli",
            purpose: "the title, the snapshot day, and the athlete count",
        };
        let message = error.to_string();
        assert!(message.contains("p.titoli"), "{message}");
        assert!(message.contains("the snapshot day"), "{message}");
    }

    #[test]
    fn a_row_without_an_athlete_names_the_row_and_the_handler() {
        let error = ClassificationParseError::MissingAthlete {
            row: 6,
            onclick: "getScheda()".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("row 6"), "{message}");
        assert!(message.contains("getScheda()"), "{message}");
    }

    #[test]
    fn a_heading_without_a_count_names_the_text() {
        let error = ClassificationParseError::MalformedCount {
            text: "Classifica al 25/10/2026".to_string(),
        };
        assert!(error.to_string().contains("25/10/2026"), "{error}");
    }
}
