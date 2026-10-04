//! The in-game date: a day of the Gregorian calendar, which advances one
//! day for each hyperspace jump.
//!
//! A new pilot's date comes from the first `chär`'s starting day, month and
//! year ([`StartDate`]), raw. A month outside 1-12 means January, and a day
//! that the month does not have means the 1st.

use crate::catalog::StartDate;

/// A day of the Gregorian calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameDate {
    year: i32,
    month: u8,
    day: u8,
}

impl GameDate {
    /// The date `start` names, with a month outside 1-12 read as 1 and a
    /// day outside that month read as 1.
    #[must_use]
    pub fn from_start(start: StartDate) -> Self {
        let year = i32::from(start.year);
        let month = u8::try_from(start.month)
            .ok()
            .filter(|month| (1..=12).contains(month))
            .unwrap_or(1);
        let day = u8::try_from(start.day)
            .ok()
            .filter(|&day| day >= 1 && day <= days_in_month(year, month))
            .unwrap_or(1);
        Self { year, month, day }
    }

    /// The day after this one.
    #[must_use]
    pub fn next_day(self) -> Self {
        if self.day < days_in_month(self.year, self.month) {
            Self {
                day: self.day + 1,
                ..self
            }
        } else if self.month < 12 {
            Self {
                month: self.month + 1,
                day: 1,
                ..self
            }
        } else {
            Self {
                year: self.year + 1,
                month: 1,
                day: 1,
            }
        }
    }

    /// The year.
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// The month, 1 (January) to 12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, from 1.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }
}

/// Whether `year` is a Gregorian leap year.
fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// How many days `month` (1-12) of `year` has.
fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: i16, month: i16, year: i16) -> GameDate {
        GameDate::from_start(StartDate { day, month, year })
    }

    fn dmy(date: GameDate) -> (u8, u8, i32) {
        (date.day(), date.month(), date.year())
    }

    #[test]
    fn the_stock_start_is_23_june_1177_and_the_next_day_the_24th() {
        let start = date(23, 6, 1177);
        assert_eq!(dmy(start), (23, 6, 1177));
        assert_eq!(dmy(start.next_day()), (24, 6, 1177));
    }

    #[test]
    fn the_last_day_of_a_month_runs_into_the_next() {
        assert_eq!(dmy(date(30, 6, 1177).next_day()), (1, 7, 1177));
        assert_eq!(dmy(date(30, 4, 1177).next_day()), (1, 5, 1177));
        assert_eq!(dmy(date(30, 9, 1177).next_day()), (1, 10, 1177));
        assert_eq!(dmy(date(30, 11, 1177).next_day()), (1, 12, 1177));
        assert_eq!(dmy(date(30, 7, 1177).next_day()), (31, 7, 1177));
        assert_eq!(dmy(date(31, 7, 1177).next_day()), (1, 8, 1177));
        assert_eq!(dmy(date(31, 1, 1177).next_day()), (1, 2, 1177));
    }

    #[test]
    fn the_last_day_of_the_year_runs_into_the_next_year() {
        assert_eq!(dmy(date(31, 12, 1177).next_day()), (1, 1, 1178));
        assert_eq!(dmy(date(30, 12, 1177).next_day()), (31, 12, 1177));
    }

    #[test]
    fn february_has_29_days_in_a_leap_year() {
        assert_eq!(dmy(date(28, 2, 1176).next_day()), (29, 2, 1176));
        assert_eq!(dmy(date(29, 2, 1176).next_day()), (1, 3, 1176));
        assert_eq!(dmy(date(28, 2, 1177).next_day()), (1, 3, 1177));
    }

    #[test]
    fn centuries_are_leap_years_only_every_400_years() {
        assert_eq!(dmy(date(28, 2, 1900).next_day()), (1, 3, 1900));
        assert_eq!(dmy(date(28, 2, 2000).next_day()), (29, 2, 2000));
        assert_eq!(dmy(date(28, 2, 1904).next_day()), (29, 2, 1904));
    }

    #[test]
    fn an_invalid_month_or_day_falls_back_to_1() {
        assert_eq!(dmy(date(23, 0, 1177)), (23, 1, 1177));
        assert_eq!(dmy(date(23, 13, 1177)), (23, 1, 1177));
        assert_eq!(dmy(date(23, -1, 1177)), (23, 1, 1177));
        assert_eq!(dmy(date(23, 300, 1177)), (23, 1, 1177));
        assert_eq!(dmy(date(0, 6, 1177)), (1, 6, 1177));
        assert_eq!(dmy(date(31, 6, 1177)), (1, 6, 1177));
        assert_eq!(dmy(date(30, 6, 1177)), (30, 6, 1177));
        assert_eq!(dmy(date(-5, 6, 1177)), (1, 6, 1177));
        assert_eq!(dmy(date(300, 6, 1177)), (1, 6, 1177));
        assert_eq!(dmy(date(29, 2, 1177)), (1, 2, 1177));
        assert_eq!(dmy(date(29, 2, 1176)), (29, 2, 1176));
        assert_eq!(dmy(date(1, 12, 1177)), (1, 12, 1177));
        assert_eq!(dmy(date(31, 12, -3)), (31, 12, -3));
    }
}
