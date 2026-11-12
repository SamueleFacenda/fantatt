use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::Date;

/// Error produced when a calendar value is out of range.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CalendarError {
    #[error("month must be 1..=12, got {0}")]
    InvalidMonth(u8),
}

/// A roster month identified by its calendar year and 1-based month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MonthYear {
    year: i32,
    month: u8,
}

impl MonthYear {
    pub fn new(year: i32, month: u8) -> Result<Self, CalendarError> {
        if !(1..=12).contains(&month) {
            return Err(CalendarError::InvalidMonth(month));
        }
        Ok(Self { year, month })
    }

    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn next(&self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }

    pub fn previous(&self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }
}

/// ISO week number of a date, 1..=53.
pub fn iso_week(date: Date) -> u8 {
    date.iso_week()
}

/// Week 1 of the calendar year scores zero for every player (INV-8):
/// league assignment is unknown before the first matchday.
pub fn apply_week_one_rule(points: i32, date: Date) -> i32 {
    if iso_week(date) == 1 { 0 } else { points }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use time::Month;

    #[test]
    fn month_year_rejects_out_of_range_months() {
        assert_eq!(MonthYear::new(2026, 0), Err(CalendarError::InvalidMonth(0)));
        assert_eq!(
            MonthYear::new(2026, 13),
            Err(CalendarError::InvalidMonth(13))
        );
    }

    #[test]
    fn month_year_rolls_over_year_boundaries() {
        let december = MonthYear::new(2026, 12).expect("valid");
        assert_eq!(december.next(), MonthYear::new(2027, 1).expect("valid"));
        assert_eq!(
            december.previous(),
            MonthYear::new(2026, 11).expect("valid")
        );
        let january = MonthYear::new(2027, 1).expect("valid");
        assert_eq!(january.previous(), MonthYear::new(2026, 12).expect("valid"));
        assert_eq!(january.next(), MonthYear::new(2027, 2).expect("valid"));
    }

    #[test]
    fn week_one_dates_score_zero() {
        let new_years_day = Date::from_calendar_date(2026, Month::January, 1).expect("valid date");
        assert_eq!(iso_week(new_years_day), 1);
        assert_eq!(apply_week_one_rule(42, new_years_day), 0);
    }

    #[test]
    fn later_weeks_keep_their_points() {
        let mid_june = Date::from_calendar_date(2026, Month::June, 15).expect("valid date");
        assert_ne!(iso_week(mid_june), 1);
        assert_eq!(apply_week_one_rule(42, mid_june), 42);
    }

    proptest! {
        #[test]
        fn every_date_in_week_one_scores_zero(year in 2020i32..2030, day in 1u8..7) {
            let date = Date::from_calendar_date(year, Month::January, day).expect("valid date");
            if iso_week(date) == 1 {
                prop_assert_eq!(apply_week_one_rule(999, date), 0);
            }
        }

        #[test]
        fn month_year_next_previous_round_trips(year in 2000i32..2100, month in 1u8..=12) {
            let month_year = MonthYear::new(year, month)?;
            prop_assert_eq!(month_year.next().previous(), month_year);
        }
    }
}
