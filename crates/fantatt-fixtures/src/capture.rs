//! The capture index: every recorded surface, its kind, file, and source URL.

use serde::{Deserialize, Deserializer, Serialize};
use time::{Date, Month};

/// Which source surface a recorded page belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceKind {
    Menu,
    RegionMenu,
    RegionHome,
    RegionHeader,
    ChampionshipIndex,
    ChampionshipList,
    ChampionshipPage,
    ChampionshipHeader,
    Calendar,
    GiornataHome,
    Giornata,
    MatchDetail,
    PlayerSearch,
    LegacyMatches,
    MatchSet,
}

impl SurfaceKind {
    /// True for the canonical match arrays, not for a recorded HTML page.
    pub fn is_match_set(self) -> bool {
        matches!(self, Self::LegacyMatches | Self::MatchSet)
    }
}

/// One recorded surface: a file, its kind, and the URL it was captured from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Surface {
    pub kind: SurfaceKind,
    pub file: String,
    pub url: String,
}

/// The `capture.json` document describing the fixture tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureIndex {
    pub version: u32,
    #[serde(deserialize_with = "iso_date")]
    pub captured_on: Date,
    pub surfaces: Vec<Surface>,
}

impl CaptureIndex {
    /// The canonical match arrays held by the set.
    pub fn match_sets(&self) -> impl Iterator<Item = &Surface> {
        self.surfaces
            .iter()
            .filter(|surface| surface.kind.is_match_set())
    }

    /// The recorded HTML and JSON pages held by the set.
    pub fn html_surfaces(&self) -> impl Iterator<Item = &Surface> {
        self.surfaces
            .iter()
            .filter(|surface| !surface.kind.is_match_set())
    }
}

/// Deserialize the capture date from an ISO calendar date string.
fn iso_date<'de, D>(deserializer: D) -> Result<Date, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    parse_iso(&text)
        .ok_or_else(|| serde::de::Error::custom(format!("{text} is not an ISO calendar date")))
}

fn parse_iso(date: &str) -> Option<Date> {
    let mut fields = date.split('-');
    let year: i32 = fields.next()?.parse().ok()?;
    let month: u8 = fields.next()?.parse().ok()?;
    let day: u8 = fields.next()?.parse().ok()?;
    if fields.next().is_some() {
        return None;
    }
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}
