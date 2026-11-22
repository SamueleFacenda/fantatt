//! Failure modes of the recorded fixture set.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FixtureError {
    #[error("fixture set not found: {path}")]
    Missing { path: String },

    #[error("fixture file {path} failed to read: {reason}")]
    Io { path: String, reason: String },

    #[error("match set {path} is neither a legacy match array nor readable: {reason}")]
    Parse { path: String, reason: String },

    #[error("match set {path} holds no legacy match records")]
    UnsupportedFile { path: String },

    #[error("match set {path} holds no records")]
    EmptySet { path: String },

    #[error("match set {path}: record {index} names no players")]
    EmptyRow { path: String, index: usize },

    #[error("match set {path}: record {index} {matchup} holds {sets} sets, expected 3 to 5")]
    SetCount {
        path: String,
        index: usize,
        matchup: String,
        sets: usize,
    },

    #[error("match set {path}: record {index} {matchup} holds a set score above {limit}")]
    SetScore {
        path: String,
        index: usize,
        matchup: String,
        limit: u8,
    },

    #[error("match set {path}: record {index} holds date {date} outside the day-first format")]
    Date {
        path: String,
        index: usize,
        date: String,
    },

    #[error("manifest {path} holds an unreadable line: {entry}")]
    Manifest { path: String, entry: String },

    #[error("file {path} is missing from the manifest")]
    Unlisted { path: String },

    #[error("manifest entry {path} does not resolve to a file")]
    MissingFile { path: String },

    #[error("file {path} failed its manifest hash check: {reason}")]
    Hash { path: String, reason: String },
}
