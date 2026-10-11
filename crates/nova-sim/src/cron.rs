//! Crons (`crön`): timed events the day's second participant runs, as the
//! original's `_CronEventHandler` (@0x3874f) does each day after the date
//! moves on (@0xb58b).
//!
//! # Eligibility
//!
//! An inactive crön starts when every check of [`eligibility`] passes, in
//! this order (`_CronEventHandler` @0x38766-0x38856):
//!
//! 1. Its `Duration` is 0 or more: below 0 disables it, running or not
//!    (@0x38766).
//! 2. `Random`: a roll drawn for every inactive crön not disabled, before
//!    any other check, so a crön refused by its dates or bits still draws
//!    (@0x38804-0x38818). By the engine ([`RuleKey::CronRandomScale`]'s
//!    default) it passes when `_Rand(101)` (0-100) is at most `Random`:
//!    `Random` 0 still fires 1 day in 101, and below 0 never. By the
//!    Bible `Random` is a percent chance, drawn as `_Rand(100) < Random`.
//! 3. The date range ([`RuleKey::CronDateCompare`]): by the engine, as
//!    `_CronDateInRange` (@0x74d3-0x75bf) compares it, the year on its
//!    own and the month and day on their own ([`dates_engine`]); by the
//!    other reading, as dates ([`dates_bible`]).
//! 4. `Require`: every bit among those the player holds
//!    (`_PlayerMeetsRequirements` @0x3883f, over 64 bits), the crons
//!    running among them.
//! 5. `EnableOn`, through the control bits (@0x38856).
//!
//! ## The engine's date range
//!
//! Each end is tested on its own, both inclusive. The start fails when
//! its `FirstYear` is above 0 and above this year (@0x74e2-0x74ec). Its
//! month and day: `FirstMonth` 0 compares only the day, when `FirstDay`
//! is above 0 (@0x74fc); `FirstMonth` below 0 skips them; `FirstMonth`
//! above 0 with `FirstDay` 0 compares the month; with `FirstDay` below 0
//! it skips them; both above 0 compare `month*32+day` (@0x7533-0x754d).
//! The end mirrors it (@0x754f-0x75bf). So a range across a year end
//! with month and day set never matches, even a dated one such as 1 Jun
//! 1177 to 1 Mar 1178, which is out on 15 Jan 1178. No stock crön has
//! one, and crön 156 relies on the field-wise test for a yearly 1 Sep to
//! 30 Dec window.
//!
//! ## The other date range
//!
//! The Bible says only that a field of 0 or -1 matches any; this reading
//! is the port's choice. A field of 0 or less is unset. An end with a
//! year compares the whole date, its month and day filled from the
//! year's first day at the start and its last at the end. An end with a
//! month and no year compares the month and day within the year (its day
//! filled the same way), one with only a day compares the day of the
//! month, and one with nothing set is open. When both ends are of the
//! same yearless kind and the start comes after the end, the range wraps
//! over the year's (or the month's) end.

use std::cmp::Ordering;

use crate::catalog::CronRecord;
use crate::chance::Chance;
use crate::control::Gate;
use crate::date::GameDate;
use crate::rulebook::{RuleKey, Rulebook};
use crate::rules::{Check, Checks, Context};
use crate::wares;

/// Whether `record` is disabled: a `Duration` below 0 (@0x38766), which
/// the pass skips whether the crön runs or not.
#[must_use]
pub fn disabled(record: &CronRecord) -> bool {
    record.duration < 0
}

/// Why an inactive crön does not start today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CronRefusal {
    /// Its `Duration` is below 0.
    Disabled,
    /// Its `Random` roll failed.
    Random,
    /// Today is outside its date range.
    Dates,
    /// The player lacks a bit of its `Require`.
    Require,
    /// Its `EnableOn` does not hold.
    EnableOn,
}

/// What a crön's eligibility reads: the crön, today, the `Contribute`
/// bits the player holds, the gate its `EnableOn` is asked through, and
/// the rule set.
pub struct CronFacts<'a> {
    /// The crön.
    pub record: &'a CronRecord,
    /// Today, the date moved on.
    pub today: GameDate,
    /// The `Contribute` bits the player holds, as they stand.
    pub contributed: u64,
    /// The control bits `EnableOn` is asked through.
    pub gate: Gate<'a>,
    /// The rule set in force.
    pub rules: &'a Rulebook,
}

impl Context for CronFacts<'_> {
    fn rulebook(&self) -> &Rulebook {
        self.rules
    }
}

/// The checks an inactive crön passes to start, in `_CronEventHandler`'s
/// order (see the module docs).
#[must_use]
pub fn eligibility<'a>() -> Checks<CronFacts<'a>, CronRefusal> {
    [
        Check::plain("Duration", "@0x38766", |facts: &CronFacts| {
            if disabled(facts.record) {
                Err(CronRefusal::Disabled)
            } else {
                Ok(())
            }
        }),
        Check::disputed_drawing(
            "Random",
            "_Rand(101) @0x38804-0x38818",
            RuleKey::CronRandomScale,
            |facts: &CronFacts, chance: &mut dyn Chance| {
                refuse_unless(roll_engine(facts.record, chance), CronRefusal::Random)
            },
            |facts: &CronFacts, chance: &mut dyn Chance| {
                refuse_unless(roll_bible(facts.record, chance), CronRefusal::Random)
            },
        ),
        Check::disputed(
            "date range",
            "_CronDateInRange @0x74d3-0x75bf",
            RuleKey::CronDateCompare,
            |facts: &CronFacts| {
                refuse_unless(dates_engine(facts.record, facts.today), CronRefusal::Dates)
            },
            |facts: &CronFacts| {
                refuse_unless(dates_bible(facts.record, facts.today), CronRefusal::Dates)
            },
        ),
        Check::plain(
            "Require",
            "_PlayerMeetsRequirements @0x3883f",
            |facts: &CronFacts| {
                refuse_unless(
                    wares::requirement_met(facts.record.require, facts.contributed),
                    CronRefusal::Require,
                )
            },
        ),
        Check::plain("EnableOn", "@0x38856", |facts: &CronFacts| {
            refuse_unless(
                facts.gate.allows(&facts.record.enable_on),
                CronRefusal::EnableOn,
            )
        }),
    ]
    .into()
}

/// `Ok` when `passes`, else `refusal`.
fn refuse_unless(passes: bool, refusal: CronRefusal) -> Result<(), CronRefusal> {
    if passes { Ok(()) } else { Err(refusal) }
}

/// The engine's `Random`: `_Rand(101)` at most `Random` (@0x38818).
fn roll_engine(record: &CronRecord, chance: &mut dyn Chance) -> bool {
    i32::from(chance.roll(101)) <= i32::from(record.random)
}

/// The Bible's `Random`: a percent chance, `_Rand(100)` below `Random`.
fn roll_bible(record: &CronRecord, chance: &mut dyn Chance) -> bool {
    i32::from(chance.roll(100)) < i32::from(record.random)
}

/// Whether `today` is in `record`'s date range as the engine compares it
/// (`_CronDateInRange` @0x74d3-0x75bf; see the module docs).
#[must_use]
pub fn dates_engine(record: &CronRecord, today: GameDate) -> bool {
    let year = today.year();
    let (month, day) = (i32::from(today.month()), i32::from(today.day()));
    let start_year = i32::from(record.first_year);
    let end_year = i32::from(record.last_year);
    // @0x74e2-0x74ec, and the end's mirror.
    let after_start = !(start_year > 0 && start_year > year)
        && field_wise(record.first_month, record.first_day, month, day).is_none_or(Ordering::is_le);
    let before_end = !(end_year > 0 && end_year < year)
        && field_wise(record.last_month, record.last_day, month, day).is_none_or(Ordering::is_ge);
    after_start && before_end
}

/// How an end's `month` and `day` compare with today's, as
/// `_CronDateInRange` compares them (@0x74fc-0x754d), or `None` when it
/// skips them.
fn field_wise(month: i16, day: i16, today_month: i32, today_day: i32) -> Option<Ordering> {
    let (month, day) = (i32::from(month), i32::from(day));
    match (month.cmp(&0), day.cmp(&0)) {
        (Ordering::Equal, Ordering::Greater) => Some(day.cmp(&today_day)),
        (Ordering::Greater, Ordering::Equal) => Some(month.cmp(&today_month)),
        (Ordering::Greater, Ordering::Greater) => {
            Some((month * 32 + day).cmp(&(today_month * 32 + today_day)))
        }
        _ => None,
    }
}

/// Whether `today` is in `record`'s date range compared as dates (the
/// port's reading; see the module docs).
#[must_use]
pub fn dates_bible(record: &CronRecord, today: GameDate) -> bool {
    let start = Bound::of(
        record.first_year,
        record.first_month,
        record.first_day,
        (1, 1),
    );
    let end = Bound::of(
        record.last_year,
        record.last_month,
        record.last_day,
        (12, 31),
    );
    let after_start = start.compare(today).is_none_or(Ordering::is_le);
    let before_end = end.compare(today).is_none_or(Ordering::is_ge);
    match (start, end) {
        (Bound::MonthDay(first_month, first_day), Bound::MonthDay(last_month, last_day))
            if (first_month, first_day) > (last_month, last_day) =>
        {
            after_start || before_end
        }
        (Bound::Day(first), Bound::Day(last)) if first > last => after_start || before_end,
        _ => after_start && before_end,
    }
}

/// One end of a range compared as dates, at the precision it gives: a
/// field of 0 or less is unset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bound {
    /// A year, month and day.
    Date(i32, i32, i32),
    /// A month and day, in any year.
    MonthDay(i32, i32),
    /// A day, in any month.
    Day(i32),
    /// Nothing set.
    Open,
}

impl Bound {
    /// The end of `year`, `month` and `day`, an unset month and day
    /// filled from `fill` when it gives a year or a month.
    fn of(year: i16, month: i16, day: i16, fill: (i32, i32)) -> Self {
        let set = |field: i16| (field > 0).then_some(i32::from(field));
        match (set(year), set(month), set(day)) {
            (Some(year), month, day) => {
                Self::Date(year, month.unwrap_or(fill.0), day.unwrap_or(fill.1))
            }
            (None, Some(month), day) => Self::MonthDay(month, day.unwrap_or(fill.1)),
            (None, None, Some(day)) => Self::Day(day),
            (None, None, None) => Self::Open,
        }
    }

    /// How the end compares with `today`, or `None` when it is open.
    fn compare(self, today: GameDate) -> Option<Ordering> {
        let (year, month, day) = (
            today.year(),
            i32::from(today.month()),
            i32::from(today.day()),
        );
        match self {
            Self::Date(y, m, d) => Some((y, m, d).cmp(&(year, month, day))),
            Self::MonthDay(m, d) => Some((m, d).cmp(&(month, day))),
            Self::Day(d) => Some(d.cmp(&day)),
            Self::Open => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    use crate::control::{ControlBits, FreshPilot, PilotFacts, Test, TestExpr};
    use crate::rulebook::RuleSource;
    use crate::rules::Descriptor;
    use crate::testkit::{Scripted, cron};

    /// A hand-written control-bit port with a canned answer, recording
    /// each test it is asked.
    #[derive(Debug)]
    struct MockBits {
        answer: bool,
        asked: RefCell<Vec<TestExpr>>,
    }

    impl MockBits {
        fn answering(answer: bool) -> Self {
            Self {
                answer,
                asked: RefCell::default(),
            }
        }
    }

    impl ControlBits for MockBits {
        fn allows(&self, test: &TestExpr, _pilot: &dyn PilotFacts) -> bool {
            self.asked.borrow_mut().push(test.clone());
            self.answer
        }
    }

    fn date(year: i32, month: u8, day: u8) -> GameDate {
        GameDate::new(year, month, day).expect("a real date")
    }

    /// The rule set with both cron keys following `source`.
    fn rules(source: RuleSource) -> Rulebook {
        Rulebook::default()
            .with_override(RuleKey::CronRandomScale, source)
            .with_override(RuleKey::CronDateCompare, source)
    }

    /// `record`'s first refusal on 15 Jun 1177, holding `contributed`,
    /// its `EnableOn` answered by `bits`, under `source`, drawing on
    /// `chance`.
    fn first_refusal(
        record: &CronRecord,
        contributed: u64,
        bits: &MockBits,
        source: RuleSource,
        chance: &mut Scripted,
    ) -> Result<(), CronRefusal> {
        let rules = rules(source);
        let facts = CronFacts {
            record,
            today: date(1177, 6, 15),
            contributed,
            gate: Gate {
                control_bits: bits,
                pilot: &FreshPilot,
            },
            rules: &rules,
        };
        eligibility().first_refusal(&facts, chance)
    }

    #[test]
    fn the_checks_are_cron_event_handlers_in_its_order() {
        let checks = eligibility();
        let listed: Vec<_> = checks
            .descriptors()
            .map(|about| (about.name(), about.reproduces(), about.draws()))
            .collect();
        assert_eq!(
            listed,
            [
                ("Duration", "@0x38766", false),
                ("Random", "_Rand(101) @0x38804-0x38818", true),
                ("date range", "_CronDateInRange @0x74d3-0x75bf", false),
                ("Require", "_PlayerMeetsRequirements @0x3883f", false),
                ("EnableOn", "@0x38856", false),
            ]
        );
        assert_eq!(
            checks
                .descriptors()
                .map(Descriptor::reads)
                .collect::<Vec<_>>(),
            [
                None,
                Some(RuleKey::CronRandomScale),
                Some(RuleKey::CronDateCompare),
                None,
                None
            ]
        );
    }

    #[test]
    fn the_disabling_duration_refuses_before_the_roll() {
        let record = CronRecord {
            duration: -1,
            ..cron(128)
        };
        assert!(disabled(&record));
        assert!(!disabled(&cron(128)));
        for source in RuleSource::ALL {
            let mut chance = Scripted::rolling(&[0]);
            assert_eq!(
                first_refusal(&record, 0, &MockBits::answering(true), source, &mut chance),
                Err(CronRefusal::Disabled),
                "{source:?}"
            );
            assert!(chance.sides_asked.is_empty(), "{source:?}");
        }
    }

    #[test]
    fn the_roll_comes_before_every_other_check() {
        let out_of_range = CronRecord {
            first_month: 7,
            ..cron(128)
        };
        let unmet = CronRecord {
            require: 0x4,
            ..cron(128)
        };
        let gated = CronRecord {
            enable_on: Test::parse("b5"),
            ..cron(128)
        };
        for (source, sides) in [(RuleSource::Engine, 101), (RuleSource::Bible, 100)] {
            for (record, refusal) in [
                (&out_of_range, CronRefusal::Dates),
                (&unmet, CronRefusal::Require),
                (&gated, CronRefusal::EnableOn),
            ] {
                let mut chance = Scripted::rolling(&[0]);
                assert_eq!(
                    first_refusal(record, 0, &MockBits::answering(false), source, &mut chance),
                    Err(refusal),
                    "{source:?}"
                );
                assert_eq!(chance.sides_asked, [sides], "{source:?} {refusal:?}");
            }
        }
    }

    /// Whether `record` passes its roll on `roll` under `source`, its
    /// other checks passing.
    fn rolls(random: i16, roll: u16, source: RuleSource) -> bool {
        let record = CronRecord {
            random,
            ..cron(128)
        };
        let outcome = first_refusal(
            &record,
            0,
            &MockBits::answering(true),
            source,
            &mut Scripted::rolling(&[roll]),
        );
        match outcome {
            Ok(()) => true,
            Err(CronRefusal::Random) => false,
            Err(other) => panic!("refused by {other:?}"),
        }
    }

    #[test]
    fn random_by_the_engine() {
        for (random, roll, passes) in [
            (0, 0, true),
            (0, 1, false),
            (100, 100, true),
            (50, 50, true),
            (50, 51, false),
            (-1, 0, false),
        ] {
            assert_eq!(
                rolls(random, roll, RuleSource::Engine),
                passes,
                "Random {random}, roll {roll}"
            );
        }
    }

    #[test]
    fn random_by_the_bible() {
        for (random, roll, passes) in [
            (0, 0, false),
            (1, 0, true),
            (1, 1, false),
            (100, 99, true),
            (50, 49, true),
            (50, 50, false),
            (-1, 0, false),
        ] {
            assert_eq!(
                rolls(random, roll, RuleSource::Bible),
                passes,
                "Random {random}, roll {roll}"
            );
        }
    }

    /// A crön from `first` (day, month, year) to `last`.
    fn ranged(first: (i16, i16, i16), last: (i16, i16, i16)) -> CronRecord {
        CronRecord {
            first_day: first.0,
            first_month: first.1,
            first_year: first.2,
            last_day: last.0,
            last_month: last.1,
            last_year: last.2,
            ..cron(128)
        }
    }

    /// Asserts `reading` on each case: a crön from `first` to `last`,
    /// whether it matches on `today`.
    #[allow(clippy::type_complexity)]
    fn assert_dates(
        reading: fn(&CronRecord, GameDate) -> bool,
        cases: &[((i16, i16, i16), (i16, i16, i16), GameDate, bool)],
    ) {
        for &(first, last, today, matches) in cases {
            assert_eq!(
                reading(&ranged(first, last), today),
                matches,
                "{first:?} to {last:?} on {today:?}"
            );
        }
    }

    #[test]
    fn dates_by_the_engine_compare_the_start_field_by_field() {
        const OPEN: (i16, i16, i16) = (0, 0, 0);
        let today = date(1177, 6, 15);
        assert_dates(
            dates_engine,
            &[
                // The year on its own: at or below 0 skipped.
                ((0, 0, 1178), OPEN, today, false),
                ((0, 0, 1177), OPEN, today, true),
                ((0, 0, 0), OPEN, today, true),
                ((0, 0, -1), OPEN, today, true),
                // The month and day on their own, even past a year
                // before.
                ((31, 12, 1176), OPEN, today, false),
                // Month 0: the day alone, when it is above 0.
                ((16, 0, 0), OPEN, today, false),
                ((15, 0, 0), OPEN, today, true),
                ((-1, 0, 0), OPEN, today, true),
                // Month below 0: no month or day test.
                ((31, -1, 0), OPEN, today, true),
                // Month above 0, day 0: the month.
                ((0, 7, 0), OPEN, today, false),
                ((0, 6, 0), OPEN, today, true),
                // Month above 0, day below 0: skipped.
                ((-1, 7, 0), OPEN, today, true),
                // Both above 0: month*32+day, inclusive.
                ((16, 6, 0), OPEN, today, false),
                ((15, 6, 0), OPEN, today, true),
                ((1, 7, 0), OPEN, today, false),
                ((31, 5, 0), OPEN, today, true),
            ],
        );
    }

    #[test]
    fn dates_by_the_engine_compare_the_end_field_by_field() {
        const OPEN: (i16, i16, i16) = (0, 0, 0);
        let today = date(1177, 6, 15);
        assert_dates(
            dates_engine,
            &[
                ((0, 0, 0), (0, 0, 1176), today, false),
                ((0, 0, 0), (0, 0, 1177), today, true),
                (OPEN, (0, 0, -1), today, true),
                (OPEN, (1, 1, 1178), today, false),
                (OPEN, (14, 0, 0), today, false),
                (OPEN, (15, 0, 0), today, true),
                (OPEN, (-1, 0, 0), today, true),
                (OPEN, (1, -1, 0), today, true),
                (OPEN, (0, 5, 0), today, false),
                (OPEN, (0, 6, 0), today, true),
                (OPEN, (-1, 5, 0), today, true),
                (OPEN, (14, 6, 0), today, false),
                (OPEN, (15, 6, 0), today, true),
                (OPEN, (31, 5, 0), today, false),
                (OPEN, (1, 7, 0), today, true),
            ],
        );
    }

    #[test]
    fn dates_by_the_engine_keep_the_field_wise_quirks() {
        let yearly = ((1, 9, -1), (30, 12, -1));
        let dated = ((1, 6, 1177), (1, 3, 1178));
        let across = ((1, 12, 0), (31, 1, 0));
        assert_dates(
            dates_engine,
            &[
                (yearly.0, yearly.1, date(1177, 9, 1), true),
                (yearly.0, yearly.1, date(1177, 12, 30), true),
                (yearly.0, yearly.1, date(1177, 8, 31), false),
                (yearly.0, yearly.1, date(1177, 12, 31), false),
                // A dated range whose end month comes before its start
                // month never matches, in either year.
                (dated.0, dated.1, date(1178, 1, 15), false),
                (dated.0, dated.1, date(1177, 6, 1), false),
                (dated.0, dated.1, date(1177, 12, 31), false),
                ((1, 3, 1177), (1, 6, 1178), date(1178, 4, 1), true),
                ((1, 3, 1177), (1, 6, 1178), date(1177, 12, 1), false),
                (across.0, across.1, date(1177, 12, 15), false),
                (across.0, across.1, date(1178, 1, 15), false),
                (across.0, across.1, date(1177, 6, 15), false),
            ],
        );
    }

    #[test]
    fn dates_by_the_other_reading_compare_each_end_at_its_precision() {
        const OPEN: (i16, i16, i16) = (0, 0, 0);
        let today = date(1177, 6, 15);
        assert_dates(
            dates_bible,
            &[
                // An end with a year: the whole date.
                ((0, 0, 1177), OPEN, today, true),
                ((0, 0, 1178), OPEN, today, false),
                ((0, 7, 1176), OPEN, today, true),
                ((16, 6, 1177), OPEN, today, false),
                ((15, 6, 1177), OPEN, today, true),
                ((0, 6, 1177), OPEN, today, true),
                ((0, 7, 1177), OPEN, today, false),
                (OPEN, (0, 0, 1177), today, true),
                (OPEN, (0, 0, 1176), today, false),
                (OPEN, (1, 1, 1178), today, true),
                (OPEN, (14, 6, 1177), today, false),
                (OPEN, (15, 6, 1177), today, true),
                (OPEN, (0, 6, 1177), today, true),
                (OPEN, (0, 5, 1177), today, false),
                (OPEN, (1, 0, 1177), today, true),
                (OPEN, (1, 0, 1176), today, false),
                // A month and no year: within the year.
                ((16, 6, 0), OPEN, today, false),
                ((15, 6, 0), OPEN, today, true),
                ((0, 6, 0), OPEN, today, true),
                ((0, 7, -1), OPEN, today, false),
                (OPEN, (14, 6, 0), today, false),
                (OPEN, (15, 6, 0), today, true),
                (OPEN, (0, 6, 0), today, true),
                (OPEN, (0, 5, -1), today, false),
                // Only a day: the day of the month.
                ((16, 0, 0), OPEN, today, false),
                ((15, -1, 0), OPEN, today, true),
                (OPEN, (14, 0, 0), today, false),
                (OPEN, (15, 0, -1), today, true),
                // Nothing set: open.
                (OPEN, OPEN, today, true),
                ((-1, -1, -1), (-1, -1, -1), today, true),
                // Ends of mixed kinds each on their own.
                ((0, 0, 1177), (10, 0, 0), today, false),
                ((20, 0, 0), (0, 0, 1177), today, false),
                ((0, 7, 0), (0, 0, 1178), today, false),
            ],
        );
    }

    #[test]
    fn dates_by_the_other_reading_wrap_a_yearless_range() {
        let yearly = ((1, 9, -1), (30, 12, -1));
        let dated = ((1, 6, 1177), (1, 3, 1178));
        let across = ((1, 12, 0), (31, 1, 0));
        let month_end = ((25, 0, 0), (5, 0, 0));
        assert_dates(
            dates_bible,
            &[
                (yearly.0, yearly.1, date(1177, 9, 1), true),
                (yearly.0, yearly.1, date(1177, 12, 30), true),
                (yearly.0, yearly.1, date(1177, 8, 31), false),
                (yearly.0, yearly.1, date(1177, 12, 31), false),
                (dated.0, dated.1, date(1178, 1, 15), true),
                (dated.0, dated.1, date(1178, 3, 2), false),
                (dated.0, dated.1, date(1177, 5, 31), false),
                (across.0, across.1, date(1178, 1, 15), true),
                (across.0, across.1, date(1177, 12, 15), true),
                (across.0, across.1, date(1177, 12, 1), true),
                (across.0, across.1, date(1178, 1, 31), true),
                (across.0, across.1, date(1177, 6, 15), false),
                (across.0, across.1, date(1177, 2, 1), false),
                (across.0, across.1, date(1177, 11, 30), false),
                (month_end.0, month_end.1, date(1177, 6, 28), true),
                (month_end.0, month_end.1, date(1177, 7, 3), true),
                (month_end.0, month_end.1, date(1177, 6, 25), true),
                (month_end.0, month_end.1, date(1177, 6, 5), true),
                (month_end.0, month_end.1, date(1177, 6, 15), false),
                (month_end.0, month_end.1, date(1177, 6, 6), false),
                (month_end.0, month_end.1, date(1177, 6, 24), false),
            ],
        );
    }

    #[test]
    fn the_date_range_follows_the_rule_set() {
        let dated = ranged((1, 6, 1177), (1, 3, 1178));
        for (source, expected) in [
            (RuleSource::Engine, Err(CronRefusal::Dates)),
            (RuleSource::Bible, Ok(())),
        ] {
            let rules = Rulebook::default().with_override(RuleKey::CronDateCompare, source);
            let facts = CronFacts {
                record: &dated,
                today: date(1178, 1, 15),
                contributed: 0,
                gate: Gate::FRESH,
                rules: &rules,
            };
            assert_eq!(
                eligibility().first_refusal(&facts, &mut Scripted::rolling(&[0])),
                expected,
                "{source:?}"
            );
        }
    }

    #[test]
    fn require_needs_every_bit_contributed() {
        let record = CronRecord {
            require: 0x6,
            ..cron(128)
        };
        for (contributed, expected) in [
            (0x6, Ok(())),
            (0xF, Ok(())),
            (0x4, Err(CronRefusal::Require)),
            (0x0, Err(CronRefusal::Require)),
        ] {
            assert_eq!(
                first_refusal(
                    &record,
                    contributed,
                    &MockBits::answering(true),
                    RuleSource::Engine,
                    &mut Scripted::rolling(&[0])
                ),
                expected,
                "{contributed:#x}"
            );
        }
    }

    #[test]
    fn enable_on_is_asked_through_the_gate() {
        let record = CronRecord {
            enable_on: Test::parse("b5"),
            ..cron(128)
        };
        for answer in [true, false] {
            let bits = MockBits::answering(answer);
            let outcome = first_refusal(
                &record,
                0,
                &bits,
                RuleSource::Engine,
                &mut Scripted::rolling(&[0]),
            );
            assert_eq!(outcome.is_ok(), answer);
            assert_eq!(
                *bits.asked.borrow(),
                [TestExpr::parse("b5").expect("parses")]
            );
        }
    }
}
