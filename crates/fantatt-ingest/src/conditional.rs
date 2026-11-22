//! Conditional requests: the validators that let the portal answer 304 Not
//! Modified instead of resending a page.
//!
//! Validators pass through verbatim: the facade extracts the portal's `ETag`
//! and `Last-Modified` and sends them back as `If-None-Match` and
//! `If-Modified-Since` (docs/06-data-pipeline.md §3.3). Comparing dates
//! belongs to the diff stage that stores them (G1.3).

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// The cache validators of one page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheValidators {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

impl CacheValidators {
    pub fn is_empty(&self) -> bool {
        self.etag.is_none() && self.last_modified.is_none()
    }

    /// The conditional headers to attach to the request.
    pub fn headers(&self) -> Vec<(&'static str, &str)> {
        let mut headers = Vec::new();
        if let Some(etag) = &self.etag {
            headers.push(("if-none-match", etag.as_str()));
        }
        if let Some(last_modified) = &self.last_modified {
            headers.push(("if-modified-since", last_modified.as_str()));
        }
        headers
    }

    /// The validators a response carries for the next request.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        Self {
            etag: header_value(headers, "etag"),
            last_modified: header_value(headers, "last-modified"),
        }
    }

    /// The headers to insert into a prepared request.
    pub fn insert_into(&self, headers: &mut HeaderMap) {
        for (name, value) in self.headers() {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                headers.insert(name, value);
            }
        }
    }
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).expect("name"),
                HeaderValue::from_str(value).expect("value"),
            );
        }
        headers
    }

    #[test]
    fn empty_validators_send_no_conditional_headers() {
        assert!(CacheValidators::default().is_empty());
        assert!(CacheValidators::default().headers().is_empty());
    }

    #[test]
    fn both_validators_are_sent_back() {
        let validators = CacheValidators {
            etag: Some("\"abc123\"".to_owned()),
            last_modified: Some("Wed, 21 Oct 2026 07:28:00 GMT".to_owned()),
        };
        assert_eq!(
            validators.headers(),
            vec![
                ("if-none-match", "\"abc123\""),
                ("if-modified-since", "Wed, 21 Oct 2026 07:28:00 GMT"),
            ]
        );
    }

    #[test]
    fn a_response_supplies_the_next_validators() {
        let response = headers(&[
            ("etag", "\"deadbeef\""),
            ("last-modified", "Wed, 21 Oct 2026 07:28:00 GMT"),
            ("content-type", "text/html"),
        ]);
        let validators = CacheValidators::from_headers(&response);
        assert_eq!(validators.etag.as_deref(), Some("\"deadbeef\""));
        assert_eq!(
            validators.last_modified.as_deref(),
            Some("Wed, 21 Oct 2026 07:28:00 GMT")
        );
    }

    #[test]
    fn a_response_without_validators_yields_empty_ones() {
        let response = headers(&[("content-type", "text/html")]);
        assert!(CacheValidators::from_headers(&response).is_empty());
    }

    fn value_of<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
        headers.get(name).and_then(|value| value.to_str().ok())
    }

    #[test]
    fn inserting_replaces_a_stale_validator_and_keeps_the_rest() {
        let mut request = headers(&[
            ("if-none-match", "\"old\""),
            ("user-agent", "fantatt-ingest/0.1.0"),
        ]);
        CacheValidators {
            etag: Some("\"new\"".to_owned()),
            last_modified: None,
        }
        .insert_into(&mut request);
        assert_eq!(value_of(&request, "if-none-match"), Some("\"new\""));
        assert_eq!(
            value_of(&request, "user-agent"),
            Some("fantatt-ingest/0.1.0")
        );
    }

    #[test]
    fn inserting_writes_no_validator_the_cache_does_not_carry() {
        let mut request = headers(&[("user-agent", "fantatt-ingest/0.1.0")]);
        CacheValidators {
            etag: Some("\"new\"".to_owned()),
            last_modified: None,
        }
        .insert_into(&mut request);
        assert_eq!(value_of(&request, "if-none-match"), Some("\"new\""));
        assert!(request.get("if-modified-since").is_none());
    }

    #[test]
    fn inserting_writes_both_validators_when_the_cache_carries_them() {
        let mut request = headers(&[]);
        CacheValidators {
            etag: Some("\"new\"".to_owned()),
            last_modified: Some("Wed, 21 Oct 2026 07:28:00 GMT".to_owned()),
        }
        .insert_into(&mut request);
        assert_eq!(value_of(&request, "if-none-match"), Some("\"new\""));
        assert_eq!(
            value_of(&request, "if-modified-since"),
            Some("Wed, 21 Oct 2026 07:28:00 GMT")
        );
    }
}
