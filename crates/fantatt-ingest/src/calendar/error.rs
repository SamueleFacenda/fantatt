//! Why a calendar page could not become the rounds it schedules. Every
//! variant names the selector or the text at fault, so a portal structure
//! change reports where it happened instead of failing silently
//! (docs/06-data-pipeline.md §5).

/// The failure of one calendar parse.
#[derive(Debug, thiserror::Error)]
pub enum CalendarParseError {
    #[error("the {selector} element naming {purpose} is missing from the calendar")]
    MissingSelector {
        selector: &'static str,
        purpose: &'static str,
    },

    #[error("the match of {home_team} against {away_team} is printed outside any round heading")]
    OrphanMatch {
        home_team: String,
        away_team: String,
    },

    #[error("the matchday link names no {param} parameter: {href:?}")]
    MissingParam { param: &'static str, href: String },

    #[error("the leg score {text:?} is not a pair of set counts")]
    MalformedScore { text: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_selector_names_the_selector_and_its_purpose() {
        let error = CalendarParseError::MissingSelector {
            selector: "p.intestazione",
            purpose: "the championship the calendar covers",
        };
        let message = error.to_string();
        assert!(message.contains("p.intestazione"), "{message}");
        assert!(message.contains("the championship"), "{message}");
    }

    #[test]
    fn an_orphan_match_names_the_two_societies() {
        let error = CalendarParseError::OrphanMatch {
            home_team: "TENNISTAVOLO SASSARI".to_string(),
            away_team: "ASD MURAVERA TT".to_string(),
        };
        let message = error.to_string();
        assert!(message.contains("TENNISTAVOLO SASSARI"), "{message}");
        assert!(message.contains("ASD MURAVERA TT"), "{message}");
    }

    #[test]
    fn a_malformed_score_names_the_text() {
        let error = CalendarParseError::MalformedScore {
            text: "tre a zero".to_string(),
        };
        assert!(error.to_string().contains("tre a zero"), "{error}");
    }
}
