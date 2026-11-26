//! Golden tests for the calendar parser (G1.5): the recorded calendar
//! becomes exactly the rounds it schedules, and the team score it prints for
//! a matchday is the count of ties the recorded giornata of that matchday
//! carries. No test reaches the network.

use fantatt_fixtures::FixtureSet;
use fantatt_ingest::uri::Uri;
use fantatt_ingest::{
    CalendarCoordinates, CalendarMatch, GiornataCoordinates, ParsedCalendar, ParsedGiornata,
    parse_calendar, parse_giornata,
};

/// The recorded calendar parses to the seven rounds of four matches the
/// championship schedules.
#[test]
fn the_recorded_calendar_parses_its_rounds_and_matches() {
    let parsed = recorded_calendar().expect("calendar parses");
    assert_eq!(parsed.championship_id, "1");
    assert_eq!(parsed.season_year, "41");
    assert_eq!(parsed.label, "Serie A/1 Gir.U");
    let numbers: Vec<u32> = parsed.rounds.iter().map(|round| round.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5, 6, 7]);
    for round in &parsed.rounds {
        assert_eq!(round.matches.len(), 4, "round {}", round.number);
    }
}

/// The second match of the first round states both legs, and the first leg
/// carries the score the recorded referto confirms.
#[test]
fn the_second_match_of_the_first_round_states_its_two_legs() {
    let parsed = recorded_calendar().expect("calendar parses");
    let found = &parsed.rounds[0].matches[1];
    assert_eq!(found.home_team, "TENNISTAVOLO SASSARI");
    assert_eq!(found.away_team, "ASD MURAVERA TT");
    assert_eq!(found.home.contest_id, "370793");
    assert_eq!(found.home.score, Some((0, 3)));
    assert_eq!(found.away.contest_id, "392017");
    assert_eq!(found.away.score, None);
    assert_eq!(found.away.date, None);
}

/// The opening match of the first round states the first society of the
/// championship and its played first leg.
#[test]
fn the_opening_match_states_its_played_first_leg() {
    let parsed = recorded_calendar().expect("calendar parses");
    let found = &parsed.rounds[0].matches[0];
    assert_eq!(found.home_team, "ASD TT CITTA' DI SIRACUSA");
    assert_eq!(found.away_team, "USD APUANIA CARRARA TENNISTAVOLO");
    assert_eq!(found.home.contest_id, "370792");
    assert_eq!(found.home.score, Some((3, 0)));
    assert_eq!(found.away.score, None);
}

/// The team score the calendar prints for a matchday is the count of ties
/// the recorded giornata of that matchday carries, framed by the societies
/// the calendar names: the return leg swaps them.
#[test]
fn the_calendar_team_scores_agree_with_the_recorded_giornate() {
    let parsed = recorded_calendar().expect("calendar parses");
    let mut compared = 0;
    for found in matches(&parsed) {
        for leg in [&found.home, &found.away] {
            let Some(giornata) = recorded_giornata(&leg.contest_id) else {
                continue;
            };
            let wins = tie_wins(&giornata);
            match leg.score {
                Some(printed) => {
                    let expected = score_ref(&wins, found, &giornata);
                    assert_eq!(printed, expected, "{}", leg.contest_id);
                }
                None => assert_eq!(wins, (0, 0), "{} counts no played tie", leg.contest_id),
            }
            compared += 1;
        }
    }
    assert_eq!(compared, 8);
}

/// The tie wins of one matchday, framed by the societies the calendar names:
/// the return leg swaps the two.
fn score_ref(wins: &(u8, u8), found: &CalendarMatch, giornata: &ParsedGiornata) -> (u8, u8) {
    if giornata.giornata.home_team == found.home_team {
        *wins
    } else {
        (wins.1, wins.0)
    }
}

/// The ties each society won in one matchday, counting the ties the referto
/// kept.
fn tie_wins(giornata: &ParsedGiornata) -> (u8, u8) {
    giornata
        .giornata
        .rows
        .iter()
        .fold((0, 0), |(home, away), row| {
            let home_sets = row.sets.iter().filter(|(a, b)| a > b).count();
            let away_sets = row.sets.iter().filter(|(a, b)| b > a).count();
            if home_sets > away_sets {
                (home + 1, away)
            } else {
                (home, away + 1)
            }
        })
}

fn matches(parsed: &ParsedCalendar) -> Vec<&CalendarMatch> {
    parsed
        .rounds
        .iter()
        .flat_map(|round| &round.matches)
        .collect()
}

fn recorded_calendar() -> Result<ParsedCalendar, fantatt_ingest::CalendarParseError> {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| surface.url.contains("Calendario.asp"))
        .expect("calendar surface");
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = CalendarCoordinates::from_uri(&uri).expect("coordinates");
    parse_calendar(&bytes, &coordinates)
}

fn recorded_giornata(contest_id: &str) -> Option<ParsedGiornata> {
    let set = FixtureSet::discover().expect("fixture set");
    let surface = set
        .html_surfaces()
        .expect("recorded surfaces")
        .into_iter()
        .find(|surface| {
            surface
                .url
                .contains(&format!("giornata.php?INCONTRO={contest_id}"))
        })?;
    let bytes = std::fs::read(set.file_path(&surface.file)).expect("page bytes");
    let uri = Uri::parse(&surface.url).expect("page uri");
    let coordinates = GiornataCoordinates::from_uri(&uri).expect("coordinates");
    parse_giornata(&bytes, &coordinates).ok()
}
