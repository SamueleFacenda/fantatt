//! The calendar parser: one `Calendario.asp` page becomes the rounds of
//! matches it schedules. Every extraction names the portal structure it
//! reads, and every failure is a typed error that locates the selector, row,
//! or link at fault instead of a panic (docs/06-data-pipeline.md §2, §5).

use scraper::{ElementRef, Html, Selector};
use time::{Date, Month};

use super::error::CalendarParseError;
use super::model::{CalendarLeg, CalendarMatch, CalendarRound, ParsedCalendar};
use crate::html::parse_page;
use crate::names::{collapse_whitespace, normalize_team};
use crate::uri::Uri;

/// The selector naming the heading lines of the calendar.
const HEADING: &str = "p.intestazione";

/// The prefix the championship heading carries.
const HEADING_PREFIX: &str = "Calendario Incontri ";

/// The selector naming a row of the calendar tables.
const ROW: &str = "tr";

/// The selector naming one cell of a row.
const CELL: &str = "td";

/// The selector naming the score link of one leg.
const SCORE_LINK: &str = "p.risultato a";

/// The cells one match row prints: the two legs around the two societies.
/// why: the score of a leg is read off its link, not off a cell index, so
/// only the date, time, and society cells need one.
const HOME_DATE_CELL: usize = 0;
const HOME_TIME_CELL: usize = 1;
const HOME_TEAM_CELL: usize = 3;
const AWAY_TEAM_CELL: usize = 4;
const AWAY_TIME_CELL: usize = 6;
const AWAY_DATE_CELL: usize = 7;

/// The coordinates a calendar page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarCoordinates {
    /// The `CAM` query value of the championship.
    pub championship_id: String,
    /// The `ANNO` query value naming the season year.
    pub season_year: String,
}

impl CalendarCoordinates {
    /// Read the coordinates off a calendar URI; none when the query names no
    /// championship or no season.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        Some(Self {
            championship_id: uri.param("CAM")?.to_string(),
            season_year: uri.param("ANNO")?.to_string(),
        })
    }
}

/// Parse one calendar page into the rounds of matches it schedules.
pub fn parse_calendar(
    page: &[u8],
    coordinates: &CalendarCoordinates,
) -> Result<ParsedCalendar, CalendarParseError> {
    let document = parse_page(page);
    Ok(ParsedCalendar {
        championship_id: coordinates.championship_id.clone(),
        season_year: coordinates.season_year.clone(),
        label: label(&document)?,
        rounds: rounds(&document)?,
    })
}

/// The championship label the heading prints above the first table.
fn label(document: &Html) -> Result<String, CalendarParseError> {
    let selector = Selector::parse(HEADING).expect("static selector");
    document
        .select(&selector)
        .map(|heading| collapse_whitespace(&heading.text().collect::<String>()))
        .find_map(|text| text.strip_prefix(HEADING_PREFIX).map(str::to_string))
        .ok_or(CalendarParseError::MissingSelector {
            selector: HEADING,
            purpose: "the championship the calendar covers",
        })
}

/// Every round the page prints, in document order, with the matches
/// scheduled in each.
fn rounds(document: &Html) -> Result<Vec<CalendarRound>, CalendarParseError> {
    let selector = Selector::parse(ROW).expect("static selector");
    let mut rounds: Vec<CalendarRound> = Vec::new();
    for row in document.select(&selector) {
        if let Some(number) = round_number(row) {
            rounds.push(CalendarRound {
                number,
                matches: Vec::new(),
            });
        } else if let Some(found) = calendar_match(row)? {
            push_match(&mut rounds, found)?;
        }
    }
    Ok(rounds)
}

/// Add one match to the round that named it, or fail when the page printed
/// it above any round heading.
fn push_match(
    rounds: &mut [CalendarRound],
    found: CalendarMatch,
) -> Result<(), CalendarParseError> {
    let round = rounds
        .last_mut()
        .ok_or_else(|| CalendarParseError::OrphanMatch {
            home_team: found.home_team.clone(),
            away_team: found.away_team.clone(),
        })?;
    round.matches.push(found);
    Ok(())
}

/// The ordinal a round heading row prints, or None for any other row.
fn round_number(row: ElementRef<'_>) -> Option<u32> {
    let text = single_cell_text(row)?;
    let ordinal = text.strip_suffix(" Giornata")?.strip_suffix('^')?;
    ordinal.trim().parse().ok()
}

/// The text of the one cell a heading row prints.
fn single_cell_text(row: ElementRef<'_>) -> Option<String> {
    let selector = Selector::parse(CELL).expect("static selector");
    let mut cells = row.select(&selector);
    let text = cells
        .next()
        .map(|cell| collapse_whitespace(&cell.text().collect::<String>()))?;
    cells.next().is_none().then_some(text)
}

/// The match a row prints, or None for a row that schedules no match.
fn calendar_match(row: ElementRef<'_>) -> Result<Option<CalendarMatch>, CalendarParseError> {
    let links = leg_links(row);
    let [home, away] = links.as_slice() else {
        return Ok(None);
    };
    let home_team = cell_text(row, HOME_TEAM_CELL).unwrap_or_default();
    let away_team = cell_text(row, AWAY_TEAM_CELL).unwrap_or_default();
    Ok(Some(CalendarMatch {
        home_team: normalize_team(&home_team),
        away_team: normalize_team(&away_team),
        home: leg(row, *home, HOME_CELLS)?,
        away: leg(row, *away, AWAY_CELLS)?,
    }))
}

/// The score links of one row, in the order the page prints them.
fn leg_links(row: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    let selector = Selector::parse(SCORE_LINK).expect("static selector");
    row.select(&selector).collect()
}

/// The cells one leg reads its date and its clock time from.
struct LegCells {
    date: usize,
    time: usize,
}

const HOME_CELLS: LegCells = LegCells {
    date: HOME_DATE_CELL,
    time: HOME_TIME_CELL,
};

const AWAY_CELLS: LegCells = LegCells {
    date: AWAY_DATE_CELL,
    time: AWAY_TIME_CELL,
};

fn leg(
    row: ElementRef<'_>,
    link: ElementRef<'_>,
    cells: LegCells,
) -> Result<CalendarLeg, CalendarParseError> {
    let text = collapse_whitespace(&link.text().collect::<String>());
    Ok(CalendarLeg {
        contest_id: contest_id(link)?,
        date: cell_date(row, cells.date)?,
        time: cell_text(row, cells.time).filter(|time| !time.is_empty()),
        score: score(&text)?,
    })
}

/// The matchday a leg link names, read off the `INCONTRO` query value of its
/// href.
fn contest_id(link: ElementRef<'_>) -> Result<String, CalendarParseError> {
    let href = link.value().attr("href").unwrap_or_default();
    let (_, query) = href.split_once('?').unwrap_or_default();
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("INCONTRO")?.strip_prefix('='))
        .map(str::to_string)
        .ok_or_else(|| CalendarParseError::MissingParam {
            param: "INCONTRO",
            href: href.to_string(),
        })
}

/// The team set score a leg link prints, or None while the leg is unplayed.
fn score(text: &str) -> Result<Option<(u8, u8)>, CalendarParseError> {
    let (home, away) = text.split_once('-').unwrap_or((text, ""));
    let home = home.trim();
    let away = away.trim();
    if home.is_empty() || away.is_empty() {
        return Ok(None);
    }
    match (home.parse::<u8>(), away.parse::<u8>()) {
        (Ok(home), Ok(away)) => Ok(Some((home, away))),
        _ => Err(CalendarParseError::MalformedScore {
            text: text.to_string(),
        }),
    }
}

fn cell_text(row: ElementRef<'_>, cell: usize) -> Option<String> {
    let selector = Selector::parse(CELL).expect("static selector");
    row.select(&selector)
        .nth(cell)
        .map(|found| collapse_whitespace(&found.text().collect::<String>()))
        .filter(|text| !text.is_empty())
}

/// The day a cell names, or None when the cell holds no calendar date: a
/// postponed leg prints `Rinviata` where its day would stand.
fn cell_date(row: ElementRef<'_>, cell: usize) -> Result<Option<Date>, CalendarParseError> {
    let Some(text) = cell_text(row, cell) else {
        return Ok(None);
    };
    let mut fields = text.split('/');
    let (Some(day), Some(month), Some(year)) = (fields.next(), fields.next(), fields.next()) else {
        return Ok(None);
    };
    let (Ok(day), Ok(month), Ok(year)) =
        (day.parse::<u8>(), month.parse::<u8>(), year.parse::<i32>())
    else {
        return Ok(None);
    };
    let Some(month) = Month::try_from(month).ok() else {
        return Ok(None);
    };
    Ok(Date::from_calendar_date(year, month, day).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(input: &str) -> Uri {
        Uri::parse(input).expect("url")
    }

    fn coordinates() -> CalendarCoordinates {
        CalendarCoordinates {
            championship_id: "1".to_string(),
            season_year: "41".to_string(),
        }
    }

    /// One round of a calendar as the page prints it: a heading row, then
    /// the matches with both legs around the two societies.
    const CALENDAR: &[u8] = br#"<html><body><table width="668">
        <tr><td><p class="intestazione">Calendario Incontri Serie A/1 Gir.U</p></td>
        <td><p class="intestazione"><a href="../../fpdf2/stampa_calend.php?ANNO=41&CAM=1">STAMPA</a></p></td></tr>
    </table><TABLE width="668">
        <tr><td colspan="8"><p class="intestazione">1^ Giornata</p></td></tr>
        <tr><td width=53><p class="stile">26/09/2026</p></td><td width=26><p class="stile">16.00</p></td>
        <td width=23><b><p class="risultato"><a href='giornata_home.php?INCONTRO=370793&CAM=1&FORMULA=17'>0 - 3</a></p></b></td>
        <td width=198><p class="stile">TENNISTAVOLO SASSARI</p></td><td width=198><p class="stile">ASD MURAVERA TT</p></td>
        <td width=23><b><p class="risultato"><a href='giornata_home.php?INCONTRO=392017&CAM=1&FORMULA=17'> - </a></p></b></td>
        <td width=26><p class="stile"></p></td><td width=52><p class="stile">Rinviata</p></td></tr>
        <tr><td colspan="8"><p class="intestazione">2^ Giornata</p></td></tr>
        <tr><td width=53><p class="stile">03/10/2026</p></td><td width=26><p class="stile">16.00</p></td>
        <td width=23><b><p class="risultato"><a href='giornata_home.php?INCONTRO=370794&CAM=1&FORMULA=17'>2 - 3</a></p></b></td>
        <td width=198><p class="stile">ASD MARCOZZI</p></td><td width=198><p class="stile">T.T. REGGIO EMILIA</p></td>
        <td width=23><b><p class="risultato"><a href='giornata_home.php?INCONTRO=392018&CAM=1&FORMULA=17'>1 - 3</a></p></b></td>
        <td width=26><p class="stile">18.45</p></td><td width=52><p class="stile">09/01/2027</p></td></tr>
    </TABLE></body></html>"#;

    #[test]
    fn a_calendar_parses_its_championship_and_rounds() {
        let parsed = parse_calendar(CALENDAR, &coordinates()).expect("calendar parses");
        assert_eq!(parsed.championship_id, "1");
        assert_eq!(parsed.season_year, "41");
        assert_eq!(parsed.label, "Serie A/1 Gir.U");
        assert_eq!(parsed.rounds.len(), 2);
        let first = &parsed.rounds[0];
        assert_eq!(first.number, 1);
        assert_eq!(first.matches.len(), 1);
    }

    #[test]
    fn a_match_parses_its_two_legs() {
        let parsed = parse_calendar(CALENDAR, &coordinates()).expect("calendar parses");
        let found = &parsed.rounds[0].matches[0];
        assert_eq!(found.home_team, "TENNISTAVOLO SASSARI");
        assert_eq!(found.away_team, "ASD MURAVERA TT");
        assert_eq!(found.home.contest_id, "370793");
        assert_eq!(found.home.score, Some((0, 3)));
        assert_eq!(found.home.time.as_deref(), Some("16.00"));
        assert_eq!(found.away.contest_id, "392017");
        assert_eq!(found.away.score, None);
        assert_eq!(found.away.date, None);
        assert_eq!(found.away.time, None);
    }

    #[test]
    fn a_played_return_leg_parses_its_date_and_score() {
        let parsed = parse_calendar(CALENDAR, &coordinates()).expect("calendar parses");
        let found = &parsed.rounds[1].matches[0];
        assert_eq!(found.away.contest_id, "392018");
        assert_eq!(found.away.score, Some((1, 3)));
        assert_eq!(found.away.time.as_deref(), Some("18.45"));
        assert_eq!(
            found.away.date,
            time::Date::from_calendar_date(2027, time::Month::January, 9).ok()
        );
    }

    #[test]
    fn a_page_without_the_championship_heading_fails_naming_the_selector() {
        let page = br#"<html><body><TABLE><tr><td colspan="8">
            <p class="intestazione">1^ Giornata</p></td></tr></TABLE></body></html>"#;
        let error = parse_calendar(page, &coordinates()).expect_err("no heading");
        assert!(
            matches!(
                error,
                CalendarParseError::MissingSelector {
                    selector: "p.intestazione",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_match_outside_any_round_fails_naming_the_two_societies() {
        let page = br#"<html><body><TABLE><tr><td colspan="8">
            <p class="intestazione">Calendario Incontri Serie A/1</p></td></tr>
            <tr><td><p class="stile">26/09/2026</p></td><td><p class="stile">16.00</p></td>
            <td><p class="risultato"><a href="giornata_home.php?INCONTRO=1">3 - 0</a></p></td>
            <td><p class="stile">HOME</p></td><td><p class="stile">AWAY</p></td>
            <td><p class="risultato"><a href="giornata_home.php?INCONTRO=2"> - </a></p></td>
            <td></td><td></td></tr></TABLE></body></html>"#;
        let error = parse_calendar(page, &coordinates()).expect_err("orphan match");
        assert!(
            matches!(error, CalendarParseError::OrphanMatch { .. }),
            "{error}"
        );
    }

    #[test]
    fn a_leg_link_without_a_matchday_fails_naming_the_href() {
        let page = br#"<html><body><TABLE><tr><td colspan="8">
            <p class="intestazione">Calendario Incontri Serie A/1</p></td></tr>
            <tr><td colspan="8"><p class="intestazione">1^ Giornata</p></td></tr>
            <tr><td><p class="stile">26/09/2026</p></td><td><p class="stile">16.00</p></td>
            <td><p class="risultato"><a href="giornata_home.php?CAM=1">3 - 0</a></p></td>
            <td><p class="stile">HOME</p></td><td><p class="stile">AWAY</p></td>
            <td><p class="risultato"><a href="giornata_home.php?INCONTRO=2"> - </a></p></td>
            <td></td><td></td></tr></TABLE></body></html>"#;
        let error = parse_calendar(page, &coordinates()).expect_err("no INCONTRO");
        assert!(
            matches!(error, CalendarParseError::MissingParam { .. }),
            "{error}"
        );
    }

    #[test]
    fn the_coordinates_read_the_championship_and_season_off_the_uri() {
        let found = CalendarCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/campionati/Calendario.asp?CAM=1&ANNO=41",
        ));
        assert_eq!(found, Some(coordinates()));
        let without = CalendarCoordinates::from_uri(&uri(
            "https://portale.fitet.org/risultati/campionati/Calendario.asp?CAM=1",
        ));
        assert_eq!(without, None);
    }
}
