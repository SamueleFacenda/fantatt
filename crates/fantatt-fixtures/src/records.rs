//! The legacy match corpus: recorded results with their source coordinates.

use std::ops::RangeInclusive;
use std::path::Path;

use serde::{Deserialize, Serialize};
use time::{Date, Month};

use super::MatchSet;
use super::error::FixtureError;

/// Highest legal point value in a recorded set.
const MAX_SET_POINTS: u8 = 30;

/// Legal number of sets per recorded match.
const SET_COUNTS: RangeInclusive<usize> = 3..=5;

/// One recorded match: two players, their set scores, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawMatch {
    pub one: String,
    pub two: String,
    pub score: Vec<[u8; 2]>,
    pub source: RecordedSource,
}

/// Which surface produced the match, and its identifiers there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub a: String,
    pub b: String,
    pub date: String,
}

impl RecordedSource {
    /// The recorded play date, when it parses in the prototype's day-first format.
    pub fn play_date(&self) -> Option<Date> {
        parse_day_first(&self.date)
    }
}

/// Read and validate one recorded match array.
pub fn read_match_set(path: &Path, label: &str) -> Result<MatchSet, FixtureError> {
    let text = std::fs::read_to_string(path).map_err(|reason| FixtureError::Io {
        path: label.to_string(),
        reason: reason.to_string(),
    })?;
    parse_match_set(&text, label)
}

/// Parse and validate one recorded match array from its text.
pub fn parse_match_set(text: &str, label: &str) -> Result<MatchSet, FixtureError> {
    let records: Vec<RawMatch> = serde_json::from_str(text).map_err(|reason| {
        if serde_json::from_str::<serde_json::Value>(text).is_ok() {
            FixtureError::UnsupportedFile {
                path: label.to_string(),
            }
        } else {
            FixtureError::Parse {
                path: label.to_string(),
                reason: reason.to_string(),
            }
        }
    })?;
    validate(&records, label)?;
    Ok(MatchSet {
        file: label.to_string(),
        records,
    })
}

fn validate(records: &[RawMatch], label: &str) -> Result<(), FixtureError> {
    if records.is_empty() {
        return Err(FixtureError::EmptySet {
            path: label.to_string(),
        });
    }
    for (index, record) in records.iter().enumerate() {
        if record.one.is_empty() || record.two.is_empty() {
            return Err(FixtureError::EmptyRow {
                path: label.to_string(),
                index,
            });
        }
        validate_score(record, index, label)?;
        validate_date(record, index, label)?;
    }
    Ok(())
}

fn validate_score(record: &RawMatch, index: usize, label: &str) -> Result<(), FixtureError> {
    check_set_count(record, index, label)?;
    check_set_points(record, index, label)?;
    Ok(())
}

fn check_set_count(record: &RawMatch, index: usize, label: &str) -> Result<(), FixtureError> {
    if SET_COUNTS.contains(&record.score.len()) {
        return Ok(());
    }
    Err(FixtureError::SetCount {
        path: label.to_string(),
        index,
        matchup: matchup_label(record),
        sets: record.score.len(),
    })
}

fn check_set_points(record: &RawMatch, index: usize, label: &str) -> Result<(), FixtureError> {
    if !record
        .score
        .iter()
        .flatten()
        .any(|points| *points > MAX_SET_POINTS)
    {
        return Ok(());
    }
    Err(FixtureError::SetScore {
        path: label.to_string(),
        index,
        matchup: matchup_label(record),
        limit: MAX_SET_POINTS,
    })
}

fn validate_date(record: &RawMatch, index: usize, label: &str) -> Result<(), FixtureError> {
    if record.source.play_date().is_none() {
        return Err(FixtureError::Date {
            path: label.to_string(),
            index,
            date: record.source.date.clone(),
        });
    }
    Ok(())
}

fn matchup_label(record: &RawMatch) -> String {
    format!("({} v {})", record.one, record.two)
}

// why: the prototype records dates day-first, so the loader parses that shape
// explicitly instead of depending on time's formatting feature set
fn parse_day_first(date: &str) -> Option<Date> {
    let mut fields = date.split('/');
    let day: u8 = fields.next()?.parse().ok()?;
    let month: u8 = fields.next()?.parse().ok()?;
    let year: i32 = fields.next()?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}
