//! Crons (`crön`): timed events the day's second participant runs, as the
//! original's `_CronEventHandler` (@0x3874f) does each day after the date
//! moves on (@0xb58b).
//!
//! # The daily pass
//!
//! The original keeps 512 slots, crön 128 + i (`_InitArrays` @0x1bb3d;
//! `_LoadObjectData` @0x7d76f-0x7da23), each with `active`, `days` (+0x26:
//! the days left of the duration, -1 marking the post-holdoff) and
//! `holdoff` (+0x28). The port keeps a [`CronState`] for each crön
//! running or holding off, on the pilot, keyed by ID, so there is no
//! cap. A new pilot has none, and a save keeps them.
//!
//! The pass (`pass`, through the `CronDay` port the session implements)
//! goes over the crons once a day by ascending ID
//! (@0x38762-0x38912), each one this way:
//!
//! 1. A crön whose `Duration` is below 0 is skipped, running or not
//!    (@0x38766).
//! 2. Holding off (`holdoff` above 0): the holdoff counts down a day. At
//!    0 with `days` 0 or more the pre-holdoff is over: `OnStart` runs,
//!    unconditionally, even if its `EnableOn` no longer holds, and with
//!    `days` 0 it ends the same day (@0x38784-0x387c1). At 0 with `days`
//!    -1 the post-holdoff is over: inactive (@0x387cb).
//! 3. Running: `days` counts down; at 0 or below it ends (@0x387d3-0x387e8).
//! 4. Inactive: when [`eligibility`] passes it starts, `days` its
//!    `Duration` (@0x38867-0x38871). With `PreHoldoff` above 0 it holds
//!    off that many days and nothing runs yet (@0x388c5); otherwise
//!    `OnStart` runs, and with `Duration` 0 it ends the same day.
//!
//! A crön ends by running `OnEnd`, then, with `PostHoldoff` above 0,
//! `days` -1 and `holdoff` its `PostHoldoff`, counted down by step 2;
//! otherwise it is inactive. Each state is written before the crön's
//! scripts run, and each script runs at once, so a later crön in the
//! pass sees what an earlier one set. A running crön never rechecks its
//! dates, `Require` or `EnableOn`; one that goes inactive may start again
//! the next day. `_ActivateCron` and `_TerminateCron` have no other
//! callers: no operator, mission or new pilot starts or ends a crön.
//!
//! These are the intended ends, decided 2026-10-11, where the engine has
//! bugs that crons-news-ranks phase 7 adds as opt-in Rulebook readings:
//! after a running crön's normal end the engine loads its `PreHoldoff`
//! into the holdoff rather than its `PostHoldoff` (@0x387f1 → 0x388c5),
//! and a `Duration` 0 crön whose pre-holdoff ends runs `OnEnd` again the
//! next day. The port runs `OnEnd` once and waits out the `PostHoldoff`.
//! The `Flags` that run `OnStart` or `OnEnd` each day (0x1, 0x2) are
//! crons-news-ranks phase 6's: here every crön runs each script once.
//!
//! The session runs each script as a participant's effect (its `M`, `N`,
//! `Q` and `S` deferred to the next settling), named by [`cron_hook`]
//! and never registered, so it keeps no once-fired entry.
//!
//! # Contribute
//!
//! A running crön's `Contribute` bits are among those the player holds
//! only outside its holdoffs, at a `holdoff` of 0 or below
//! ([`contributes`]; `_GetPlayerContributeBits` @0x770f-0x7724): a crön
//! in its pre-holdoff or its post-holdoff contributes nothing, and a
//! running crön's own bits count towards a later crön's `Require`. Every
//! reader sums them through [`wares::contributed`].
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

use crate::catalog::{CronId, CronRecord};
use crate::chance::Chance;
use crate::control::{Gate, Script};
use crate::date::GameDate;
use crate::rulebook::{RuleKey, Rulebook};
use crate::rules::{Check, Checks, Context};
use crate::story::{Consumer, SubscriptionId};
use crate::wares;

/// A crön's `OnStart` hook.
pub const START: u8 = 0;
/// A crön's `OnEnd` hook.
pub const END: u8 = 1;

/// The name of crön `cron`'s `hook` ([`START`] or [`END`]) as it runs:
/// never registered, and never kept as fired, since a crön recurs.
#[must_use]
pub const fn cron_hook(cron: CronId, hook: u8) -> SubscriptionId {
    SubscriptionId {
        consumer: Consumer::Crons,
        record: cron.0,
        hook,
    }
}

/// What the daily pass asks of the session it runs on: the crons'
/// states, whether an inactive crön may start, and running a crön's
/// script.
pub(crate) trait CronDay {
    /// Crön `cron`'s counters, or `None` while it is inactive.
    fn state(&self, cron: CronId) -> Option<CronState>;
    /// Sets crön `cron`'s counters, `None` making it inactive.
    fn set_state(&mut self, cron: CronId, state: Option<CronState>);
    /// Whether inactive crön `record` starts today: runs
    /// [`eligibility`] now, the roll and then the dates, `Require` (the
    /// bits contributed as they stand) and `EnableOn`.
    fn eligible(&mut self, record: &CronRecord) -> bool;
    /// Runs `script`, named `id` ([`cron_hook`]), at once.
    fn run(&mut self, id: SubscriptionId, script: &Script);
}

/// One day's pass over the crons, by ascending ID, as
/// `_CronEventHandler` (@0x3874f) makes it, with the intended ends (see
/// the module docs). Each state is written before the crön's scripts
/// run, and each script runs at once, so a later crön in the pass sees
/// what an earlier one set.
pub(crate) fn pass(records: &[CronRecord], day: &mut (impl CronDay + ?Sized)) {
    let mut sorted: Vec<&CronRecord> = records.iter().collect();
    sorted.sort_by_key(|record| record.id);
    for record in sorted {
        if disabled(record) {
            continue;
        }
        let id = record.id;
        match day.state(id) {
            // The holdoff counts down (@0x38784-0x387cb).
            Some(mut state) if state.holdoff > 0 => {
                state.holdoff = state.holdoff.saturating_sub(1);
                if state.holdoff > 0 {
                    day.set_state(id, Some(state));
                } else if state.days >= 0 {
                    day.set_state(id, Some(state));
                    start(record, state.days, day);
                } else {
                    day.set_state(id, None);
                }
            }
            // Running: a day of its duration (@0x387d3-0x387fc).
            Some(mut state) => {
                state.days = state.days.saturating_sub(1);
                day.set_state(id, Some(state));
                if state.days <= 0 {
                    end(record, day);
                }
            }
            // Inactive: it may start (@0x38804-0x38901).
            None => {
                if !day.eligible(record) {
                    continue;
                }
                if record.pre_holdoff > 0 {
                    let state = CronState {
                        days: record.duration,
                        holdoff: record.pre_holdoff,
                    };
                    day.set_state(id, Some(state));
                } else {
                    let state = CronState {
                        days: record.duration,
                        holdoff: 0,
                    };
                    day.set_state(id, Some(state));
                    start(record, record.duration, day);
                }
            }
        }
    }
}

/// Runs `record`'s `OnStart`, and with `days` 0 left its `OnEnd` and its
/// end the same day (@0x388d4-0x388e6).
fn start(record: &CronRecord, days: i16, day: &mut (impl CronDay + ?Sized)) {
    day.run(cron_hook(record.id, START), &record.on_start);
    if days == 0 {
        end(record, day);
    }
}

/// Runs `record`'s `OnEnd`, then its intended end: its post-holdoff when
/// `PostHoldoff` is above 0, else inactive.
fn end(record: &CronRecord, day: &mut (impl CronDay + ?Sized)) {
    day.run(cron_hook(record.id, END), &record.on_end);
    let after = (record.post_holdoff > 0).then_some(CronState {
        days: -1,
        holdoff: record.post_holdoff,
    });
    day.set_state(record.id, after);
}

/// One crön's counters while it runs or holds off, as the original keeps
/// them in its slot (+0x26, +0x28). A crön with none is inactive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CronState {
    /// The days left of its duration; -1 in its post-holdoff.
    pub days: i16,
    /// The days left of its holdoff: its pre-holdoff while `days` is 0 or
    /// more, its post-holdoff once `days` is -1.
    pub holdoff: i16,
}

impl CronState {
    /// Whether a crön saved with these counters is active, as a load
    /// derives it: `days` or `holdoff` 0 or more (`_LoadPilotData`
    /// @0x75fbc-0x76025).
    #[must_use]
    pub fn active(self) -> bool {
        self.days >= 0 || self.holdoff >= 0
    }
}

/// Whether a crön with counters `state` contributes its `Contribute`
/// bits: only outside its holdoffs, at a `holdoff` of 0 or below
/// (`_GetPlayerContributeBits` @0x770f-0x7724).
#[must_use]
pub fn contributes(state: CronState) -> bool {
    state.holdoff <= 0
}

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
    use std::collections::BTreeMap;

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
    fn a_cron_contributes_only_outside_its_holdoffs() {
        let state = |days, holdoff| CronState { days, holdoff };
        assert!(contributes(state(3, 0)));
        assert!(contributes(state(0, -1)));
        assert!(!contributes(state(3, 1)));
        assert!(!contributes(state(-1, 2)));
    }

    #[test]
    fn a_saved_cron_is_active_with_days_or_holdoff_0_or_more() {
        let state = |days, holdoff| CronState { days, holdoff };
        assert!(state(0, -1).active());
        assert!(state(-1, 0).active());
        assert!(state(5, 2).active());
        assert!(!state(-1, -1).active());
        assert!(!state(-2, -3).active());
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
                // A year of 0 is skipped even before year 0, which a
                // plug-in's start date can reach.
                ((0, 0, 0), (0, 0, -1), date(-5, 6, 15), true),
                ((0, 0, -1), (0, 0, 0), date(-5, 6, 15), true),
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
                // Equal yearless ends are one day, not a wrap.
                ((15, 6, 0), (15, 6, 0), date(1177, 6, 15), true),
                ((15, 6, 0), (15, 6, 0), date(1177, 6, 16), false),
                ((15, 6, 0), (15, 6, 0), date(1177, 6, 14), false),
                ((15, 0, 0), (15, 0, 0), date(1177, 6, 15), true),
                ((15, 0, 0), (15, 0, 0), date(1177, 7, 16), false),
                ((15, 0, 0), (15, 0, 0), date(1177, 7, 14), false),
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

    #[test]
    fn a_crons_scripts_are_named_by_its_id_and_hook() {
        assert_eq!(
            cron_hook(CronId(156), START),
            SubscriptionId {
                consumer: Consumer::Crons,
                record: 156,
                hook: 0,
            }
        );
        assert_eq!(cron_hook(CronId(129), END).hook, 1);
        assert_eq!(cron_hook(CronId(129), END).record, 129);
    }

    /// What the pass did, in order.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Did {
        /// Asked whether crön `.0` is eligible.
        Asked(i16),
        /// Ran crön `.0`'s `OnStart`.
        Started(i16),
        /// Ran crön `.0`'s `OnEnd`.
        Ended(i16),
    }

    /// A hand-written day: the crons' states in a map, each crön's
    /// eligibility a canned answer (no by default), and a log of each ask
    /// and each script run, in order.
    #[derive(Default)]
    struct MockDay {
        states: BTreeMap<CronId, CronState>,
        eligible: BTreeMap<i16, bool>,
        log: Vec<Did>,
    }

    impl CronDay for MockDay {
        fn state(&self, cron: CronId) -> Option<CronState> {
            self.states.get(&cron).copied()
        }

        fn set_state(&mut self, cron: CronId, state: Option<CronState>) {
            match state {
                Some(state) => self.states.insert(cron, state),
                None => self.states.remove(&cron),
            };
        }

        fn eligible(&mut self, record: &CronRecord) -> bool {
            self.log.push(Did::Asked(record.id.0));
            self.eligible.get(&record.id.0).copied().unwrap_or(false)
        }

        fn run(&mut self, id: SubscriptionId, script: &Script) {
            assert_eq!(id.consumer, Consumer::Crons);
            let expected = match id.hook {
                START => Script::parse(&format!("b{}", id.record)),
                END => Script::parse(&format!("!b{}", id.record)),
                hook => panic!("no hook {hook}"),
            };
            assert_eq!(*script, expected, "the record's own script");
            self.log.push(match id.hook {
                START => Did::Started(id.record),
                _ => Did::Ended(id.record),
            });
        }
    }

    impl MockDay {
        /// A day with crön `id` eligible.
        fn eligible(id: i16) -> Self {
            Self {
                eligible: BTreeMap::from([(id, true)]),
                ..Self::default()
            }
        }

        /// Runs a day's pass over `records`, and what it did.
        fn day(&mut self, records: &[CronRecord]) -> Vec<Did> {
            pass(records, self);
            std::mem::take(&mut self.log)
        }

        /// Crön `id`'s state.
        fn of(&self, id: i16) -> Option<(i16, i16)> {
            self.states
                .get(&CronId(id))
                .map(|state| (state.days, state.holdoff))
        }
    }

    /// Crön `id` of `Duration` `duration` and these holdoffs, its
    /// `OnStart` setting `b<id>` and its `OnEnd` clearing it.
    fn timed(id: i16, duration: i16, pre_holdoff: i16, post_holdoff: i16) -> CronRecord {
        CronRecord {
            duration,
            pre_holdoff,
            post_holdoff,
            on_start: Script::parse(&format!("b{id}")),
            on_end: Script::parse(&format!("!b{id}")),
            ..cron(id)
        }
    }

    use Did::{Asked, Ended, Started};

    #[test]
    fn a_disabled_cron_is_skipped_running_or_not() {
        let records = [timed(128, -1, 0, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), None);
        day.states.insert(
            CronId(128),
            CronState {
                days: 3,
                holdoff: 0,
            },
        );
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((3, 0)), "untouched");
        day.states.insert(
            CronId(128),
            CronState {
                days: 3,
                holdoff: 2,
            },
        );
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((3, 2)), "untouched");
    }

    #[test]
    fn a_cron_starts_runs_its_duration_and_ends() {
        let records = [timed(128, 3, 0, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128), Started(128)]);
        assert_eq!(day.of(128), Some((3, 0)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((2, 0)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((1, 0)));
        assert_eq!(day.day(&records), [Ended(128)]);
        assert_eq!(day.of(128), None);
    }

    #[test]
    fn a_cron_of_duration_0_starts_and_ends_the_same_day() {
        let records = [timed(128, 0, 0, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128), Started(128), Ended(128)]);
        assert_eq!(day.of(128), None);
        assert_eq!(
            day.day(&records),
            [Asked(128), Started(128), Ended(128)],
            "asked again the next day"
        );
    }

    #[test]
    fn a_cron_not_eligible_stays_inactive() {
        let records = [timed(128, 3, 0, 0)];
        let mut day = MockDay::default();
        assert_eq!(day.day(&records), [Asked(128)]);
        assert_eq!(day.of(128), None);
    }

    #[test]
    fn a_pre_holdoff_delays_on_start_with_no_recheck() {
        let records = [timed(128, 1, 2, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128)]);
        assert_eq!(day.of(128), Some((1, 2)));
        day.eligible.clear();
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((1, 1)));
        assert_eq!(day.day(&records), [Started(128)], "not eligible now");
        assert_eq!(day.of(128), Some((1, 0)));
        assert_eq!(day.day(&records), [Ended(128)]);
        assert_eq!(day.of(128), None);
    }

    #[test]
    fn a_running_cron_never_rechecks() {
        let records = [timed(128, 4, 0, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128), Started(128)]);
        day.eligible.clear();
        for _ in 0..3 {
            assert_eq!(day.day(&records), []);
        }
        assert_eq!(day.day(&records), [Ended(128)]);
    }

    #[test]
    fn a_cron_that_ended_starts_again_on_a_later_day() {
        let records = [timed(128, 1, 0, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128), Started(128)]);
        assert_eq!(day.day(&records), [Ended(128)]);
        assert_eq!(day.day(&records), [Asked(128), Started(128)]);
        assert_eq!(day.of(128), Some((1, 0)));
    }

    #[test]
    fn a_post_holdoff_follows_the_end_and_then_the_cron_is_inactive() {
        let records = [timed(128, 2, 0, 3)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128), Started(128)]);
        assert_eq!(day.day(&records), []);
        assert_eq!(day.day(&records), [Ended(128)]);
        assert_eq!(day.of(128), Some((-1, 3)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((-1, 2)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((-1, 1)));
        assert_eq!(day.day(&records), [], "OnEnd ran once");
        assert_eq!(day.of(128), None);
        assert_eq!(day.day(&records), [Asked(128), Started(128)]);
    }

    #[test]
    fn a_duration_0_cron_after_a_pre_holdoff_ends_once() {
        let records = [timed(128, 0, 2, 0)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128)]);
        assert_eq!(day.of(128), Some((0, 2)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.day(&records), [Started(128), Ended(128)]);
        assert_eq!(day.of(128), None);
        day.eligible.clear();
        assert_eq!(day.day(&records), [Asked(128)], "no second OnEnd");
    }

    #[test]
    fn a_duration_0_cron_after_a_pre_holdoff_takes_its_post_holdoff() {
        let records = [timed(128, 0, 2, 2)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128)]);
        assert_eq!(day.day(&records), []);
        assert_eq!(day.day(&records), [Started(128), Ended(128)]);
        assert_eq!(day.of(128), Some((-1, 2)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), Some((-1, 1)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), None);
    }

    #[test]
    fn a_duration_0_cron_takes_its_post_holdoff() {
        let records = [timed(128, 0, 0, 1)];
        let mut day = MockDay::eligible(128);
        assert_eq!(day.day(&records), [Asked(128), Started(128), Ended(128)]);
        assert_eq!(day.of(128), Some((-1, 1)));
        assert_eq!(day.day(&records), []);
        assert_eq!(day.of(128), None);
    }

    #[test]
    fn the_pass_goes_by_ascending_id_each_crons_scripts_before_the_next_ask() {
        let records = [timed(130, 0, 0, 0), timed(129, 1, 0, 0)];
        let mut day = MockDay {
            eligible: BTreeMap::from([(129, true), (130, true)]),
            ..MockDay::default()
        };
        assert_eq!(
            day.day(&records),
            [
                Asked(129),
                Started(129),
                Asked(130),
                Started(130),
                Ended(130)
            ]
        );
    }

    /// A day whose eligibility asks write the day's state: the pass writes
    /// each state before it runs the crön's scripts.
    struct Watching {
        day: MockDay,
        seen: Vec<Option<(i16, i16)>>,
    }

    impl CronDay for Watching {
        fn state(&self, cron: CronId) -> Option<CronState> {
            self.day.state(cron)
        }

        fn set_state(&mut self, cron: CronId, state: Option<CronState>) {
            self.day.set_state(cron, state);
        }

        fn eligible(&mut self, record: &CronRecord) -> bool {
            self.day.eligible(record)
        }

        fn run(&mut self, id: SubscriptionId, script: &Script) {
            self.seen.push(self.day.of(id.record));
            self.day.run(id, script);
        }
    }

    #[test]
    fn each_state_is_written_before_the_crons_scripts_run() {
        let records = [timed(128, 0, 0, 0), timed(129, 2, 0, 0)];
        let mut watching = Watching {
            day: MockDay {
                eligible: BTreeMap::from([(128, true), (129, true)]),
                ..MockDay::default()
            },
            seen: Vec::new(),
        };
        pass(&records, &mut watching);
        assert_eq!(watching.seen, [Some((0, 0)), Some((0, 0)), Some((2, 0))]);
    }

    #[test]
    fn a_saved_counter_at_its_floor_does_not_overflow() {
        let records = [timed(128, 3, 0, 0)];
        let mut day = MockDay::default();
        day.states.insert(
            CronId(128),
            CronState {
                days: i16::MIN,
                holdoff: 0,
            },
        );
        assert_eq!(day.day(&records), [Ended(128)]);
        assert_eq!(day.of(128), None);
    }
}
