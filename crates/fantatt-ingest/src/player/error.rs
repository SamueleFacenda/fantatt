//! Why an athlete page could not become the canonical record. Every variant
//! names the selector or the text at fault, so a portal structure change
//! reports where it happened instead of failing silently
//! (docs/06-data-pipeline.md §5).

/// The failure of one player detail parse.
#[derive(Debug, thiserror::Error)]
pub enum PlayerParseError {
    #[error("the {selector} element naming {purpose} is missing from the sheet")]
    MissingSelector {
        selector: &'static str,
        purpose: &'static str,
    },
}

/// The failure of one player search parse.
#[derive(Debug, thiserror::Error)]
pub enum PlayerSearchParseError {
    #[error("the search answer is not the JSON array the portal serves: {text}")]
    MalformedJson { text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_selector_names_the_selector_and_its_purpose() {
        let error = PlayerParseError::MissingSelector {
            selector: "p.titolo",
            purpose: "the athlete the sheet names",
        };
        let message = error.to_string();
        assert!(message.contains("p.titolo"), "{message}");
        assert!(message.contains("the athlete"), "{message}");
    }

    #[test]
    fn a_malformed_answer_names_the_text() {
        let error = PlayerSearchParseError::MalformedJson {
            text: "not json".to_string(),
        };
        assert!(error.to_string().contains("not json"), "{error}");
    }
}
