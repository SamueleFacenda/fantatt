//! The giornata parser: one referto page becomes the canonical matchday
//! record of the model module. Every extraction names the portal structure it
//! reads, and every failure is a typed error that locates the row, cell, or
//! selector at fault instead of a panic (docs/06-data-pipeline.md §3.4).

use scraper::{ElementRef, Html, Node, Selector};
use time::{Date, Month};

use super::error::GiornataParseError;
use super::model::{GiornataRow, ParsedGiornata, RawGiornata, SkipReason, SkippedTie};
use crate::html::{parse_page, texts};
use crate::names::{collapse_whitespace, normalize_name, normalize_team};
use crate::uri::Uri;

/// The cells of a tie row, in the order the referto prints them.
const PLAYER_A_CELL: usize = 1;
const PLAYER_B_CELL: usize = 2;
const SET_CELL_OFFSET: usize = 3;
const MAX_SETS: usize = 5;
const SETS_WON_CELL: usize = 13;
const SETS_LOST_CELL: usize = 14;
const TIE_CELLS: usize = 15;

/// The markers the referto prints for a player who did not contest the tie.
const WALKOVER_MARKERS: [&str; 2] = ["assente", "ritirato"];

/// The coordinates a giornata page carries in its query string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GiornataCoordinates {
    pub championship_id: String,
    pub contest_id: String,
}

impl GiornataCoordinates {
    /// Read the coordinates off a giornata URI; none when the query names no
    /// championship or no contest.
    pub fn from_uri(uri: &Uri) -> Option<Self> {
        Some(Self {
            championship_id: uri.param("CAM")?.to_string(),
            contest_id: uri.param("INCONTRO")?.to_string(),
        })
    }
}

/// What the parser decided about one tie row.
enum Fate {
    Kept(GiornataRow),
    Skipped(SkipReason),
}

/// Which side of the tie the walkover marker named.
enum Side {
    Home,
    Away,
}

impl Side {
    /// The three sets a walkover is recorded as, in play order.
    fn sets(self) -> Vec<(u8, u8)> {
        let walkover = match self {
            Side::Home => (0, 11),
            Side::Away => (11, 0),
        };
        vec![walkover; 3]
    }
}

/// Parse one referto page into the canonical matchday record.
pub fn parse_giornata(
    page: &[u8],
    coordinates: &GiornataCoordinates,
) -> Result<ParsedGiornata, GiornataParseError> {
    let document = parse_page(page);
    guard_completeness(&document)?;
    let (home_team, away_team) = teams(&document)?;
    let date = matchday_date(&document)?;
    let venue = playing_venue(&document);
    let (rows, skipped) = read_ties(&document)?;
    Ok(ParsedGiornata {
        giornata: RawGiornata {
            championship_id: coordinates.championship_id.clone(),
            contest_id: coordinates.contest_id.clone(),
            date,
            home_team,
            away_team,
            venue,
            rows,
        },
        skipped,
    })
}

// why: a page that reports fewer played ties than it schedules cannot be
// trusted row by row, so the marker fails the parse instead of partial data
fn guard_completeness(document: &Html) -> Result<(), GiornataParseError> {
    let text = document.root_element().text().collect::<String>();
    let Some(start) = text.find("incontri disputati") else {
        return Ok(());
    };
    let mut counts = text[start..]
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .take(2);
    let played = counts
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(0);
    let total = counts
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or(0);
    Err(GiornataParseError::PartialBracket { played, total })
}

fn teams(document: &Html) -> Result<(String, String), GiornataParseError> {
    let selector = Selector::parse("p.incontro").expect("static selector");
    match texts(document, &selector).as_slice() {
        [home, away, ..] => Ok((normalize_team(home), normalize_team(away))),
        _ => Err(GiornataParseError::MissingSelector {
            selector: "p.incontro",
            purpose: "the two societies",
        }),
    }
}

// why: a paragraph closes itself before a block element, so the heading the
// referto prints in <p><center><b> lives on the b, not inside the paragraph
fn matchday_date(document: &Html) -> Result<Option<Date>, GiornataParseError> {
    let selector = Selector::parse("b").expect("static selector");
    let heading = texts(document, &selector)
        .into_iter()
        .find(|text| text.to_lowercase().contains("giornata"))
        .ok_or(GiornataParseError::MissingSelector {
            selector: "b",
            purpose: "the giornata heading",
        })?;
    Ok(day_first_date(&heading))
}

fn day_first_date(text: &str) -> Option<Date> {
    let first_slash = text.find('/')?;
    let day_start = text[..first_slash]
        .rfind(|c: char| !c.is_ascii_digit())
        .map_or(0, |index| index + 1);
    let day: u8 = text[day_start..first_slash].parse().ok()?;
    let rest = &text[first_slash + 1..];
    let second_slash = rest.find('/')?;
    let month: u8 = rest[..second_slash].parse().ok()?;
    let year_end = rest[second_slash + 1..]
        .find(|c: char| !c.is_ascii_digit())
        .map_or(rest.len(), |offset| second_slash + 1 + offset);
    let year: i32 = rest[second_slash + 1..year_end].parse().ok()?;
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

fn playing_venue(document: &Html) -> Option<String> {
    let selector = Selector::parse("p.campo_gara").expect("static selector");
    let element = document.select(&selector).next()?;
    let lines = element_lines(&element);
    let address: Vec<String> = lines
        .into_iter()
        .skip_while(|line| line.to_lowercase().starts_with("campo gara"))
        .collect();
    (!address.is_empty()).then(|| address.join(", "))
}

// why: the venue block separates its lines with tags, which a flat text
// concatenation would glue together
fn element_lines(element: &ElementRef<'_>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for node in element.children() {
        match node.value() {
            Node::Text(text) => current.push_str(&text.text),
            Node::Element(tag) if tag.name() == "br" => lines.push(std::mem::take(&mut current)),
            _ => {}
        }
    }
    lines.push(current);
    lines
        .iter()
        .map(|line| collapse_whitespace(line))
        .filter(|line| !line.is_empty())
        .collect()
}

fn read_ties(document: &Html) -> Result<(Vec<GiornataRow>, Vec<SkippedTie>), GiornataParseError> {
    let selector = Selector::parse("tr").expect("static selector");
    let header = document
        .select(&selector)
        .find(|row| is_tie_header(row))
        .ok_or(GiornataParseError::MissingSelector {
            selector: "p.titolo_scheda",
            purpose: "the tie table header",
        })?;
    let header_id = header.id();
    let ties = document
        .select(&selector)
        .skip_while(|row| row.id() != header_id)
        .skip(1);
    read_tie_rows(ties)
}

fn read_tie_rows<'a>(
    ties: impl Iterator<Item = ElementRef<'a>>,
) -> Result<(Vec<GiornataRow>, Vec<SkippedTie>), GiornataParseError> {
    let mut rows = Vec::new();
    let mut skipped = Vec::new();
    for (index, tie) in ties.enumerate() {
        match Tie::read(tie, index)? {
            Fate::Kept(row) => rows.push(row),
            Fate::Skipped(reason) => skipped.push(SkippedTie { row: index, reason }),
        }
    }
    Ok((rows, skipped))
}

fn is_tie_header(row: &ElementRef<'_>) -> bool {
    let selector = Selector::parse("p.titolo_scheda").expect("static selector");
    row.select(&selector)
        .any(|cell| collapse_whitespace(&cell.text().collect::<String>()) == "1^ set")
}

/// One tie row while the parser reads it: the element it lives on, its cells
/// in print order, and where it sits in the table.
struct Tie<'a> {
    element: ElementRef<'a>,
    cells: Vec<String>,
    index: usize,
}

impl<'a> Tie<'a> {
    fn read(element: ElementRef<'a>, index: usize) -> Result<Fate, GiornataParseError> {
        let tie = Self {
            element,
            cells: cell_texts(&element),
            index,
        };
        let Some(match_number) = tie.number() else {
            return Ok(Fate::Skipped(SkipReason::NotATie));
        };
        if tie.cells.len() < TIE_CELLS {
            let cell = tie.cells.len();
            return Err(GiornataParseError::MissingCell { row: index, cell });
        }
        let sets = tie.sets()?;
        if sets.is_empty() {
            return Ok(Fate::Skipped(SkipReason::Unplayed));
        }
        tie.kept(match_number, sets)
    }

    // why: a row whose first cell does not carry the tie's ordinal is a header,
    // a spacer, or prose the referto prints between tables
    fn number(&self) -> Option<u32> {
        self.cells.first()?.trim().parse().ok()
    }

    fn sets(&self) -> Result<Vec<(u8, u8)>, GiornataParseError> {
        match walkover_side(&self.cells) {
            Some(side) => Ok(side.sets()),
            None => self.point_pairs(),
        }
    }

    // why: the referto pads unused sets with a 0/0 pair, and the first such
    // pair ends the tie's sets
    fn point_pairs(&self) -> Result<Vec<(u8, u8)>, GiornataParseError> {
        let mut sets = Vec::with_capacity(MAX_SETS);
        for pair in 0..MAX_SETS {
            let home = self.point(SET_CELL_OFFSET + pair * 2)?;
            let away = self.point(SET_CELL_OFFSET + pair * 2 + 1)?;
            if home == 0 && away == 0 {
                break;
            }
            sets.push((home, away));
        }
        Ok(sets)
    }

    fn point(&self, cell: usize) -> Result<u8, GiornataParseError> {
        let text = self
            .cells
            .get(cell)
            .ok_or(GiornataParseError::MissingCell {
                row: self.index,
                cell,
            })?;
        text.trim()
            .parse()
            .map_err(|_| GiornataParseError::MalformedPoints {
                row: self.index,
                cell,
                text: text.clone(),
            })
    }

    fn kept(&self, match_number: u32, sets: Vec<(u8, u8)>) -> Result<Fate, GiornataParseError> {
        let (player_a, player_b) = self.players()?;
        check_result_shape(self.index, &sets)?;
        check_declared_sets(self.index, &self.cells, &sets)?;
        Ok(Fate::Kept(GiornataRow {
            match_number,
            detail_id: detail_link(&self.element),
            player_a,
            player_b,
            sets,
        }))
    }

    fn players(&self) -> Result<(String, String), GiornataParseError> {
        let player_a = normalize_name(&self.cells[PLAYER_A_CELL]);
        let player_b = normalize_name(&self.cells[PLAYER_B_CELL]);
        if player_a.is_empty() || player_b.is_empty() {
            let side = if player_a.is_empty() {
                "player_a"
            } else {
                "player_b"
            };
            return Err(GiornataParseError::EmptyPlayer {
                row: self.index,
                side,
            });
        }
        Ok((player_a, player_b))
    }
}

fn cell_texts(row: &ElementRef<'_>) -> Vec<String> {
    let selector = Selector::parse("td").expect("static selector");
    row.select(&selector)
        .map(|cell| collapse_whitespace(&cell.text().collect::<String>()))
        .collect()
}

fn walkover_side(cells: &[String]) -> Option<Side> {
    let marked = |text: &str| {
        let lower = text.to_lowercase();
        WALKOVER_MARKERS.iter().any(|marker| lower.contains(marker))
    };
    if marked(&cells[PLAYER_A_CELL]) {
        return Some(Side::Home);
    }
    if marked(&cells[PLAYER_B_CELL]) {
        return Some(Side::Away);
    }
    marked_set_cell(cells, &marked)
}

// why: a marker in a set cell attributes by column: the odd offsets are the
// home side's cells
fn marked_set_cell(cells: &[String], marked: &impl Fn(&str) -> bool) -> Option<Side> {
    let cell = cells
        .iter()
        .enumerate()
        .skip(SET_CELL_OFFSET)
        .take(MAX_SETS * 2)
        .find(|(_, cell)| marked(cell))?
        .0;
    Some(if cell % 2 == 1 {
        Side::Home
    } else {
        Side::Away
    })
}

// why: a tie is decided once a side reaches three set wins, so it runs three
// to five sets; anything else means the row misaligned, which is a structure
// change rather than a result
fn check_result_shape(row: usize, sets: &[(u8, u8)]) -> Result<(), GiornataParseError> {
    if (3..=5).contains(&sets.len()) {
        return Ok(());
    }
    Err(GiornataParseError::SetCount {
        row,
        sets: sets.len(),
    })
}

// why: the referto prints the sets each side won; a disagreement with the
// scores is the cheapest drift signal the page offers
fn check_declared_sets(
    row: usize,
    cells: &[String],
    sets: &[(u8, u8)],
) -> Result<(), GiornataParseError> {
    let sides = [
        (SETS_WON_CELL, "home", home_sets_won(sets)),
        (SETS_LOST_CELL, "away", away_sets_won(sets)),
    ];
    for (cell, side, computed) in sides {
        let declared = declared_count(cells, cell, row)?;
        if declared != computed {
            return Err(GiornataParseError::DeclaredSets {
                row,
                side,
                declared,
                computed,
            });
        }
    }
    Ok(())
}

fn declared_count(cells: &[String], cell: usize, row: usize) -> Result<usize, GiornataParseError> {
    let text = cells
        .get(cell)
        .ok_or(GiornataParseError::MissingCell { row, cell })?;
    let count = text
        .trim()
        .parse::<u8>()
        .map_err(|_| GiornataParseError::MalformedPoints {
            row,
            cell,
            text: text.clone(),
        })?;
    Ok(usize::from(count))
}

fn home_sets_won(sets: &[(u8, u8)]) -> usize {
    sets.iter().filter(|(home, away)| home > away).count()
}

fn away_sets_won(sets: &[(u8, u8)]) -> usize {
    sets.iter().filter(|(home, away)| away > home).count()
}

// why: the row links its detail page with the tie's ID; a relative href is
// scanned for the query value rather than resolved against a base
fn detail_link(row: &ElementRef<'_>) -> Option<String> {
    let selector = Selector::parse("a").expect("static selector");
    let href = row.select(&selector).next()?.value().attr("href")?;
    let (_, query) = href.split_once('?')?;
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("ID="))
        .map(str::to_string)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn coordinates() -> GiornataCoordinates {
        GiornataCoordinates {
            championship_id: "1".to_string(),
            contest_id: "370793".to_string(),
        }
    }

    fn tie(number: u32, players: (&str, &str), sets: &[(u8, u8)]) -> String {
        let (home, away) = players;
        let won = home_sets_won(sets);
        let ordinal = format!(
            "<td><p class=dettagli><a href=\"dettaglio_partita.php?ID={id}\">{number}</a></p></td>\
             <td><p class=dettagli>{home}</p></td><td><p class=dettagli>{away}</p></td>",
            id = 1594361 + number
        );
        let summary = format!(
            "<td align=center><p class=dettagli><b>{won}</b></p></td>\
             <td align=center><p class=dettagli><b>{}</b></p></td>",
            sets.len() - won
        );
        format!("<tr>{ordinal}{}{summary}</tr>", set_cells(sets))
    }

    fn set_cells(sets: &[(u8, u8)]) -> String {
        let mut played = sets.iter();
        let mut cells = String::new();
        for _ in 0..MAX_SETS {
            let (home, away) = played.next().copied().unwrap_or((0, 0));
            cells.push_str(&format!(
                "<td align=center><p class=dettagli>{home}</p></td>\
                 <td align=center><p class=dettagli>{away}</p></td>"
            ));
        }
        cells
    }

    // why: the template mirrors the referto's two-block layout, where the header
    // paragraph closes itself before its center so the heading lands on the b
    const PAGE_PREFIX: &str = r#"<html><head><meta charset=windows-1252></head><body><div align=center><table><tr><td><table width=466><tr><td><p class="incontro">TENNISTAVOLO SASSARI</p></td><td><p class="incontro">ASD MURAVERA TT</p></td></tr></table></td><td><p class=stile><center><b>"#;

    const PAGE_SUFFIX: &str = r#"</b></center><br>Arbitro: BASSANI ROBERTO</p><table><tr><td><p class="campo_gara">Campo gara principale:<br>PALESTRA SCUOLA MEDIA N 2<br>07100 Sassari (SS)</p></td></tr></table></td></tr></table></div><div align=center><table><tr><td><p class=titolo_scheda>1^ set</p></td><td><p class=titolo_scheda>2^ set</p></td><td><p class=titolo_scheda>3^ set</p></td><td><p class=titolo_scheda>4^ set</p></td><td><p class=titolo_scheda>5^ set</p></td><td><p class=titolo_scheda>SV</p></td><td><p class=titolo_scheda>SP</p></td></tr>"#;

    fn page(header: &str, ties: &[String]) -> Vec<u8> {
        let rows = ties.join("");
        format!("{PAGE_PREFIX}{header}{PAGE_SUFFIX}{rows}</table></div></body></html>").into_bytes()
    }

    fn parse(page: &[u8]) -> ParsedGiornata {
        parse_giornata(page, &coordinates()).expect("parse")
    }

    fn two_tie_page() -> Vec<u8> {
        page(
            "1^ Giornata&nbsp;del 26/09/2026",
            &[
                tie(
                    1,
                    ("PUPPO ANDREA", "KOZUL DENI"),
                    &[(6, 11), (5, 11), (9, 11)],
                ),
                tie(
                    2,
                    ("GACINA ANDREJ", "ABUSEV ARTUR"),
                    &[(11, 7), (9, 11), (11, 5), (9, 11), (11, 8)],
                ),
            ],
        )
    }

    #[test]
    fn a_played_matchday_carries_its_header_facts() {
        let parsed = parse(&two_tie_page());
        assert_eq!(parsed.giornata.championship_id, "1");
        assert_eq!(parsed.giornata.contest_id, "370793");
        assert_eq!(
            parsed.giornata.date,
            time::Date::from_calendar_date(2026, time::Month::September, 26).ok()
        );
        assert_eq!(parsed.giornata.home_team, "TENNISTAVOLO SASSARI");
        assert_eq!(parsed.giornata.away_team, "ASD MURAVERA TT");
        assert_eq!(
            parsed.giornata.venue.as_deref(),
            Some("PALESTRA SCUOLA MEDIA N 2, 07100 Sassari (SS)")
        );
        assert!(parsed.skipped.is_empty());
    }

    #[test]
    fn a_played_matchday_keeps_every_tie_with_its_detail_link() {
        let parsed = parse(&two_tie_page());
        let [first, second] = parsed.giornata.rows.as_slice() else {
            panic!("two ties expected");
        };
        assert_eq!(first.match_number, 1);
        assert_eq!(first.detail_id.as_deref(), Some("1594362"));
        assert_eq!(first.player_a, "Puppo Andrea");
        assert_eq!(first.player_b, "Kozul Deni");
        assert_eq!(first.sets, vec![(6, 11), (5, 11), (9, 11)]);
        assert_eq!(
            second.sets,
            vec![(11, 7), (9, 11), (11, 5), (9, 11), (11, 8)]
        );
        assert!(parsed.skipped.is_empty());
    }

    #[test]
    fn a_doubles_pair_stays_one_player_cell() {
        let page = page(
            "1^ Giornata del 10/10/2026",
            &[tie(
                4,
                ("MOTTA L - RIZZO A", "ANTONIETTI E - DELAI M"),
                &[(11, 9), (7, 11), (11, 4)],
            )],
        );
        let parsed = parse(&page);
        assert_eq!(parsed.giornata.rows[0].player_a, "Motta L - Rizzo A");
        assert_eq!(parsed.giornata.rows[0].player_b, "Antonietti E - Delai M");
    }

    #[test]
    fn an_unplayed_tie_is_skipped_with_its_reason() {
        let page = page(
            "1^ Giornata del 10/10/2026",
            &[
                tie(
                    1,
                    ("ROSSI CARLO", "BIANCHI LUCA"),
                    &[(11, 9), (11, 7), (11, 6)],
                ),
                tie(2, ("VERDI MARCO", "NERI LUCA"), &[]),
            ],
        );
        let parsed = parse(&page);
        assert_eq!(parsed.giornata.rows.len(), 1);
        assert_eq!(
            parsed.skipped,
            vec![SkippedTie {
                row: 1,
                reason: SkipReason::Unplayed
            }]
        );
    }

    #[test]
    fn a_row_that_is_not_a_tie_is_skipped_with_its_reason() {
        let ties = format!(
            "<tr><td><p class=dettagli>note</p></td></tr>{}",
            tie(
                1,
                ("ROSSI CARLO", "BIANCHI LUCA"),
                &[(11, 9), (11, 7), (11, 6)]
            )
        );
        let parsed = parse(&page("1^ Giornata del 10/10/2026", &[ties]));
        assert_eq!(
            parsed.skipped,
            vec![SkippedTie {
                row: 0,
                reason: SkipReason::NotATie
            }]
        );
        assert_eq!(parsed.giornata.rows.len(), 1);
    }

    #[test]
    fn a_scheduled_matchday_without_ties_is_a_valid_record() {
        let parsed = parse(&page("8^ Giornata&nbsp;del 09/01/2027", &[]));
        assert!(parsed.giornata.rows.is_empty());
        assert!(parsed.skipped.is_empty());
        assert_eq!(
            parsed.giornata.date,
            time::Date::from_calendar_date(2027, time::Month::January, 9).ok()
        );
    }

    #[test]
    fn a_matchday_without_a_confirmed_date_stays_undated() {
        let parsed = parse(&page("8^ Giornata&nbsp;del ", &[]));
        assert_eq!(parsed.giornata.date, None);
    }

    #[test]
    fn an_absent_home_player_records_a_walkover_to_the_away_side() {
        let page = page(
            "1^ Giornata del 10/10/2026",
            &[tie(
                1,
                ("ASSENTE", "KOZUL DENI"),
                &[(0, 11), (0, 11), (0, 11)],
            )],
        );
        let parsed = parse(&page);
        assert_eq!(parsed.skipped, vec![]);
        assert_eq!(
            parsed.giornata.rows[0].sets,
            vec![(0, 11), (0, 11), (0, 11)]
        );
        assert_eq!(parsed.giornata.rows[0].player_a, "Assente");
    }

    #[test]
    fn an_absent_away_player_records_a_walkover_to_the_home_side() {
        let page = page(
            "1^ Giornata del 10/10/2026",
            &[tie(
                1,
                ("PUPPO ANDREA", "RITIRATO"),
                &[(11, 0), (11, 0), (11, 0)],
            )],
        );
        let parsed = parse(&page);
        assert_eq!(
            parsed.giornata.rows[0].sets,
            vec![(11, 0), (11, 0), (11, 0)]
        );
    }

    #[test]
    fn an_incomplete_referto_fails_with_its_counts() {
        let page = page(
            "1^ Giornata del 10/10/2026 (incontri disputati 63/72, completezza: 87.5%)",
            &[tie(
                1,
                ("ROSSI CARLO", "BIANCHI LUCA"),
                &[(11, 9), (11, 7), (11, 6)],
            )],
        );
        let error = parse_giornata(&page, &coordinates()).expect_err("partial page");
        assert!(
            matches!(
                error,
                GiornataParseError::PartialBracket {
                    played: 63,
                    total: 72
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_page_without_the_team_names_fails_naming_the_selector() {
        let text = String::from_utf8(page("1^ Giornata del 10/10/2026", &[]))
            .expect("utf8")
            .replace("p class=\"incontro\"", "p class=\"societa\"");
        let error = parse_giornata(text.as_bytes(), &coordinates()).expect_err("no teams");
        assert!(
            matches!(
                error,
                GiornataParseError::MissingSelector {
                    selector: "p.incontro",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_page_without_the_tie_table_header_fails_naming_the_selector() {
        let text = String::from_utf8(page("1^ Giornata del 10/10/2026", &[]))
            .expect("utf8")
            .replace("1^ set", "primo set");
        let error = parse_giornata(text.as_bytes(), &coordinates()).expect_err("no header");
        assert!(
            matches!(
                error,
                GiornataParseError::MissingSelector {
                    selector: "p.titolo_scheda",
                    ..
                }
            ),
            "{error}"
        );
    }

    fn prose_row() -> String {
        let cells = "<td align=center><p class=dettagli>{points}</p></td>";
        let mut row = String::from(
            "<tr><td><p class=dettagli><a href=\"dettaglio_partita.php?ID=1594362\">1</a></p></td>\
             <td><p class=dettagli>ROSSI CARLO</p></td><td><p class=dettagli>BIANCHI LUCA</p></td>\
             <td align=center><p class=dettagli>undici</p></td>",
        );
        for points in ["9", "11", "7", "11", "6", "0", "0", "0", "0", "2", "1"] {
            row.push_str(&cells.replace("{points}", points));
        }
        row.push_str("</tr>");
        row
    }

    #[test]
    fn a_point_cell_of_prose_fails_naming_the_row_and_cell() {
        let ties = prose_row();
        let error = parse_giornata(&page("1^ Giornata del 10/10/2026", &[ties]), &coordinates())
            .expect_err("prose in a point cell");
        match error {
            GiornataParseError::MalformedPoints {
                row,
                cell,
                ref text,
            } => {
                assert_eq!(row, 0);
                assert_eq!(cell, 3);
                assert_eq!(text, "undici");
            }
            other => panic!("{other}"),
        }
    }

    #[test]
    fn a_tie_decided_three_to_one_runs_four_sets() {
        let page = page(
            "1^ Giornata del 26/09/2026",
            &[tie(
                3,
                ("CAPPUCCIO MARCO ANTONIO", "TREVISAN FRANCESCO"),
                &[(11, 13), (12, 10), (7, 11), (7, 11)],
            )],
        );
        let parsed = parse(&page);
        assert_eq!(
            parsed.giornata.rows[0].sets,
            vec![(11, 13), (12, 10), (7, 11), (7, 11)]
        );
    }

    #[test]
    fn a_truncated_tie_fails_naming_its_set_count() {
        let page = page(
            "1^ Giornata del 10/10/2026",
            &[tie(1, ("ROSSI CARLO", "BIANCHI LUCA"), &[(11, 9), (11, 7)])],
        );
        let error = parse_giornata(&page, &coordinates()).expect_err("two sets");
        assert!(
            matches!(error, GiornataParseError::SetCount { row: 0, sets: 2 }),
            "{error}"
        );
    }

    #[test]
    fn a_declared_set_count_that_disagrees_fails() {
        let row = tie(
            1,
            ("ROSSI CARLO", "BIANCHI LUCA"),
            &[(11, 9), (11, 7), (11, 6)],
        )
        .replace("<b>3</b>", "<b>2</b>");
        let error = parse_giornata(&page("1^ Giornata del 10/10/2026", &[row]), &coordinates())
            .expect_err("wrong declaration");
        let GiornataParseError::DeclaredSets {
            row,
            side,
            declared,
            computed,
        } = error
        else {
            panic!("{error}");
        };
        assert_eq!((row, side, declared, computed), (0, "home", 2, 3));
    }

    #[test]
    fn the_coordinates_read_off_a_giornata_uri() {
        let uri = Uri::parse(
            "https://portale.fitet.org/risultati/campionati/giornata.php?INCONTRO=370793&CAM=1&FORMULA=17",
        )
        .expect("giornata uri");
        let coordinates = GiornataCoordinates::from_uri(&uri).expect("coordinates");
        assert_eq!(coordinates.championship_id, "1");
        assert_eq!(coordinates.contest_id, "370793");
    }

    #[test]
    fn a_uri_without_contest_carries_no_coordinates() {
        let uri = Uri::parse("https://portale.fitet.org/risultati/campionati/giornata.php?CAM=1")
            .expect("giornata uri");
        assert!(GiornataCoordinates::from_uri(&uri).is_none());
    }
}
