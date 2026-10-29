//! The in-game date: a day of the Gregorian calendar, which advances one
//! day for each hyperspace jump.
//!
//! A new pilot's date comes from the first `chär`'s starting day, month and
//! year ([`StartDate`]), raw. A month outside 1-12 means January, and a day
//! that the month does not have means the 1st.
//!
//! [`long_date_text`] is the original's long date (`_DateToLongString`
//! @0x8ef8): `{DatePrefix}{Month} {day}{ordinal}, {year}{DateSuffix}`, as
//! in stock's "June 23rd, 1177 NC". The arrival messages of a jump, a
//! hypergate and a wormhole, the take-off greeting and a mission's `<DL>`
//! show it. The Bible says the `chär`'s `DatePrefix` and `DateSuffix` are
//! "appended to the start/end of the date whenever it's displayed", so
//! they are used verbatim, with no spaces added; the suffix is an era
//! marker, so the year comes last. The months are `STR#` 137 #1-12 of
//! `Nova-DF.rsrc`, "January" to "December", with no weekday, no
//! zero-padding, and the year a plain integer that keeps its sign.
//!
//! [`date_text`] is the same without the ordinal, "June 23, 1177 NC". The
//! original has no counterpart: it is kept only for the developer tools.
//!
//! [`med_date_text`] is the original's medium date (`_DateToMedString`
//! @0x8cb6), which the mission computer and the mission info screen
//! show: the long date with the month's short name, "Jan. 1st, 1177 NC".

use crate::catalog::{DateAffixes, StartDate};

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

    /// Day `day` of month `month` (1-12) of `year`, if the calendar has
    /// it.
    #[must_use]
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        let exists = (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month);
        exists.then_some(Self { year, month, day })
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

/// The months' English names, January first.
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `date` wrapped in `affixes`, without the ordinal: for example
/// "June 23, 1177 NC". The original writes no date this way; only the
/// developer tools show it. The player sees [`long_date_text`].
#[must_use]
pub fn date_text(date: GameDate, affixes: &DateAffixes) -> String {
    let month = MONTH_NAMES[usize::from(date.month - 1)];
    format!(
        "{}{month} {}, {}{}",
        affixes.prefix, date.day, date.year, affixes.suffix
    )
}

/// `date` as the original's long date (`_DateToLongString` @0x8ef8),
/// wrapped in `affixes`: as [`date_text`], with the day's English ordinal
/// (`STR#` 137 #25-28), for example "June 23rd, 1177 NC".
#[must_use]
pub fn long_date_text(date: GameDate, affixes: &DateAffixes) -> String {
    let mut text = String::new();
    write_long_date_text(&mut text, date, affixes);
    text
}

/// [`long_date_text`], written into `into` in place of what it held, so a
/// kept buffer is reused rather than a fresh one allocated: what a frame's
/// fill writes the date with.
pub fn write_long_date_text(into: &mut String, date: GameDate, affixes: &DateAffixes) {
    use std::fmt::Write as _;
    into.clear();
    let month = MONTH_NAMES[usize::from(date.month - 1)];
    // Writing to a `String` never fails.
    let _ = write!(
        into,
        "{}{month} {}{}, {}{}",
        affixes.prefix,
        date.day,
        ordinal(date.day),
        date.year,
        affixes.suffix
    );
}

/// The months' short names (`STR#` 137 #13-24), January first.
const SHORT_MONTH_NAMES: [&str; 12] = [
    "Jan.", "Feb.", "Mar.", "Apr.", "May", "June", "July", "Aug.", "Sept.", "Oct.", "Nov.", "Dec.",
];

/// `date` as the original's medium date (`_DateToMedString` @0x8cb6),
/// which the mission screens show, wrapped in `affixes`: the short month,
/// then the day with its English ordinal, for example "Jan. 1st, 1177
/// NC".
#[must_use]
pub fn med_date_text(date: GameDate, affixes: &DateAffixes) -> String {
    let month = SHORT_MONTH_NAMES[usize::from(date.month - 1)];
    format!(
        "{}{month} {}{}, {}{}",
        affixes.prefix,
        date.day,
        ordinal(date.day),
        date.year,
        affixes.suffix
    )
}

/// The English ordinal of day `day` (`STR#` 137 #25-28): "st", "nd",
/// "rd" or "th".
fn ordinal(day: u8) -> &'static str {
    match (day % 10, day) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
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
    fn a_date_from_its_parts_must_exist() {
        let made = |year, month, day| GameDate::new(year, month, day).map(dmy);
        assert_eq!(made(1177, 6, 23), Some((23, 6, 1177)));
        assert_eq!(made(1176, 2, 29), Some((29, 2, 1176)));
        assert_eq!(made(1177, 2, 29), None);
        assert_eq!(made(1177, 12, 31), Some((31, 12, 1177)));
        assert_eq!(made(1177, 1, 1), Some((1, 1, 1177)));
        assert_eq!(made(-40_000, 1, 1), Some((1, 1, -40_000)));
        for (month, day) in [(0, 1), (13, 1), (6, 0), (6, 31), (4, 31), (1, 32)] {
            assert_eq!(made(1177, month, day), None, "{month}/{day}");
        }
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

    // Formatting.

    fn affixes(prefix: &str, suffix: &str) -> DateAffixes {
        DateAffixes {
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
        }
    }

    fn day(year: i32, month: u8, day: u8) -> GameDate {
        GameDate::new(year, month, day).expect("exists")
    }

    #[test]
    fn the_stock_start_reads_june_23_1177_nc() {
        assert_eq!(
            date_text(date(23, 6, 1177), &affixes("", " NC")),
            "June 23, 1177 NC"
        );
    }

    #[test]
    fn each_month_reads_as_its_english_name() {
        let names = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        for (month, name) in (1..=12).zip(names) {
            assert_eq!(
                date_text(day(1177, month, 9), &DateAffixes::default()),
                format!("{name} 9, 1177")
            );
        }
    }

    #[test]
    fn a_one_digit_day_is_not_padded() {
        assert_eq!(
            date_text(day(1177, 7, 1), &affixes("", " NC")),
            "July 1, 1177 NC"
        );
    }

    #[test]
    fn the_prefix_and_suffix_wrap_the_date_verbatim() {
        assert_eq!(
            date_text(date(23, 6, 1177), &affixes("<", ">")),
            "<June 23, 1177>"
        );
        assert_eq!(
            date_text(date(23, 6, 1177), &affixes("Year ", " NC")),
            "Year June 23, 1177 NC"
        );
    }

    #[test]
    fn without_affixes_the_date_is_bare() {
        assert_eq!(
            date_text(date(23, 6, 1177), &DateAffixes::default()),
            "June 23, 1177"
        );
    }

    #[test]
    fn a_negative_year_keeps_its_sign() {
        assert_eq!(
            date_text(day(-735, 12, 31), &affixes("", " NC")),
            "December 31, -735 NC"
        );
    }

    // The long date.

    #[test]
    fn the_long_date_reads_june_23rd_1177_nc() {
        assert_eq!(
            long_date_text(date(23, 6, 1177), &affixes("", " NC")),
            "June 23rd, 1177 NC"
        );
    }

    #[test]
    fn the_long_dates_day_takes_its_english_ordinal() {
        let read = |n| long_date_text(day(1177, 7, n), &DateAffixes::default());
        for (n, ordinal) in [
            (1, "1st"),
            (2, "2nd"),
            (3, "3rd"),
            (4, "4th"),
            (10, "10th"),
            (11, "11th"),
            (12, "12th"),
            (13, "13th"),
            (14, "14th"),
            (20, "20th"),
            (21, "21st"),
            (22, "22nd"),
            (23, "23rd"),
            (24, "24th"),
            (30, "30th"),
            (31, "31st"),
        ] {
            assert_eq!(read(n), format!("July {ordinal}, 1177"), "day {n}");
        }
    }

    #[test]
    fn the_long_date_written_into_a_buffer_replaces_what_it_held() {
        let mut into = "a stale date, longer than the new one".to_owned();
        let affixes = affixes("", " NC");
        write_long_date_text(&mut into, date(23, 6, 1177), &affixes);
        assert_eq!(into, long_date_text(date(23, 6, 1177), &affixes));
        assert_eq!(into, "June 23rd, 1177 NC");
    }

    #[test]
    fn the_long_dates_prefix_and_suffix_wrap_it_verbatim() {
        assert_eq!(
            long_date_text(day(-735, 12, 2), &affixes("Year ", " NC")),
            "Year December 2nd, -735 NC"
        );
    }

    // The medium date.

    #[test]
    fn the_medium_date_reads_june_23rd_1177_nc() {
        assert_eq!(
            med_date_text(date(23, 6, 1177), &affixes("", " NC")),
            "June 23rd, 1177 NC"
        );
        assert_eq!(
            med_date_text(date(1, 1, 1177), &affixes("", " NC")),
            "Jan. 1st, 1177 NC"
        );
    }

    #[test]
    fn the_medium_dates_month_is_short_and_its_day_takes_its_ordinal() {
        let months = [
            "Jan.", "Feb.", "Mar.", "Apr.", "May", "June", "July", "Aug.", "Sept.", "Oct.", "Nov.",
            "Dec.",
        ];
        for (month, short) in (1..=12).zip(months) {
            assert_eq!(
                med_date_text(day(1177, month, 11), &DateAffixes::default()),
                format!("{short} 11th, 1177"),
                "month {month}"
            );
        }
        for (n, ordinal) in [
            (2, "2nd"),
            (12, "12th"),
            (13, "13th"),
            (22, "22nd"),
            (31, "31st"),
        ] {
            assert_eq!(
                med_date_text(day(1177, 7, n), &DateAffixes::default()),
                format!("July {ordinal}, 1177")
            );
        }
    }

    #[test]
    fn the_medium_dates_prefix_and_suffix_wrap_it_verbatim() {
        assert_eq!(
            med_date_text(day(-735, 12, 2), &affixes("Year ", " NC")),
            "Year Dec. 2nd, -735 NC"
        );
    }
}
