//! Conditional requests: the validators that let the portal answer 304 Not
//! Modified instead of resending a page.
//!
//! Validators pass through verbatim: the facade extracts the portal's `ETag`
//! and `Last-Modified` and sends them back as `If-None-Match` and
//! `If-Modified-Since` (docs/06-data-pipeline.md §3.3). Comparing dates
//! belongs to the diff stage that stores them (G1.3).

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use time::{Date, Month, OffsetDateTime, Time, Weekday};

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

    /// The `Last-Modified` header as an instant, when it parses.
    pub fn modified_at(&self) -> Option<OffsetDateTime> {
        self.last_modified.as_deref().and_then(parse_http_date)
    }

    /// The validators a stored instant and ETag describe.
    pub fn stored(etag: Option<String>, modified_at: Option<OffsetDateTime>) -> Self {
        Self {
            etag,
            last_modified: modified_at.map(http_date),
        }
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

/// Parse an HTTP `Last-Modified` header, which is always GMT. why: a hand-rolled
/// parser keeps the workspace `time` feature set flat; the IMF-fixdate shape is
/// fixed by RFC 7231.
pub fn parse_http_date(text: &str) -> Option<OffsetDateTime> {
    let (_, rest) = text.split_once(", ")?;
    let mut fields = rest.split(' ');
    let day: u8 = fields.next()?.parse().ok()?;
    let month = month_of(fields.next()?)?;
    let year: i32 = fields.next()?.parse().ok()?;
    let (hour, minute, second) = clock(fields.next()?)?;
    let zone = fields.next()?;
    if zone != "GMT" || fields.next().is_some() {
        return None;
    }
    let date = Date::from_calendar_date(year, month, day).ok()?;
    let time_of_day = Time::from_hms(hour, minute, second).ok()?;
    Some(date.with_time(time_of_day).assume_utc())
}

/// Render an instant as an HTTP date header.
pub fn http_date(instant: OffsetDateTime) -> String {
    let date = instant.date();
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        weekday_name(date.weekday()),
        date.day(),
        month_name(date.month()),
        date.year(),
        instant.hour(),
        instant.minute(),
        instant.second(),
    )
}

fn clock(text: &str) -> Option<(u8, u8, u8)> {
    let mut parts = text.split(':');
    let hour: u8 = parts.next()?.parse().ok()?;
    let minute: u8 = parts.next()?.parse().ok()?;
    let second: u8 = parts.next()?.parse().ok()?;
    (parts.next().is_none()).then_some((hour, minute, second))
}

fn month_of(text: &str) -> Option<Month> {
    Some(match text {
        "Jan" => Month::January,
        "Feb" => Month::February,
        "Mar" => Month::March,
        "Apr" => Month::April,
        "May" => Month::May,
        "Jun" => Month::June,
        "Jul" => Month::July,
        "Aug" => Month::August,
        "Sep" => Month::September,
        "Oct" => Month::October,
        "Nov" => Month::November,
        "Dec" => Month::December,
        _ => return None,
    })
}

fn month_name(month: Month) -> &'static str {
    match month {
        Month::January => "Jan",
        Month::February => "Feb",
        Month::March => "Mar",
        Month::April => "Apr",
        Month::May => "May",
        Month::June => "Jun",
        Month::July => "Jul",
        Month::August => "Aug",
        Month::September => "Sep",
        Month::October => "Oct",
        Month::November => "Nov",
        Month::December => "Dec",
    }
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Mon",
        Weekday::Tuesday => "Tue",
        Weekday::Wednesday => "Wed",
        Weekday::Thursday => "Thu",
        Weekday::Friday => "Fri",
        Weekday::Saturday => "Sat",
        Weekday::Sunday => "Sun",
    }
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
    fn a_last_modified_header_parses_into_an_instant() {
        let validators = CacheValidators {
            etag: None,
            last_modified: Some("Wed, 21 Oct 2026 07:28:00 GMT".to_owned()),
        };
        let instant = validators.modified_at().expect("instant");
        assert_eq!(instant.year(), 2026);
        assert_eq!(instant.month(), Month::October);
        assert_eq!(instant.day(), 21);
        assert_eq!(instant.hour(), 7);
        assert_eq!(http_date(instant), "Wed, 21 Oct 2026 07:28:00 GMT");
    }

    #[test]
    fn an_unparseable_date_yields_no_instant() {
        let validators = CacheValidators {
            etag: Some("\"x\"".to_owned()),
            last_modified: Some("someday".to_owned()),
        };
        assert!(validators.modified_at().is_none());
    }

    #[test]
    fn stored_validators_round_trip_through_an_instant() {
        let instant = parse_http_date("Tue, 17 Nov 2026 22:31:04 GMT").expect("instant");
        let validators = CacheValidators::stored(Some("\"tag\"".to_owned()), Some(instant));
        assert_eq!(validators.modified_at(), Some(instant));
        assert_eq!(
            validators.headers(),
            vec![
                ("if-none-match", "\"tag\""),
                ("if-modified-since", "Tue, 17 Nov 2026 22:31:04 GMT"),
            ]
        );
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
