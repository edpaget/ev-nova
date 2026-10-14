//! The daily cron pass over the stock data: a new pilot's days start
//! stock cröns on the right day and run their expressions. Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - Crön 156 "Auroran Drop Bear Mating Season" runs yearly from 1 Sep
//!   to 30 Dec (`F 1/9/-1`, `L 30/12/-1`), `Random` 100, `Duration` 105,
//!   `EnableOn !b42`, `OnStart b42` and `OnEnd !b42`.
//! - Crön 222 "Advanced Pirate news" has no dates, `EnableOn b8888 &
//!   !b8889` and `OnStart b8889`.
//! - Crön 184, a rare Aurora/Federation border change: `Duration` 14,
//!   `PreHoldoff` 0, `PostHoldoff` 550, `Random` 12, `EnableOn !b6031 &
//!   !b333`, `OnStart b6021`, `OnEnd !b6021`, and government news in
//!   slots 128 and 129 (`STR#` 15027 and 15053). No other crön touches
//!   b6021, and none sets b6031 or b333.
//!
//! Every roll is 0 ([`NeverFires`]), which passes every `Random` of 0 or
//! more by the engine's reading, so the other stock cröns start too:
//! only the bits these two own are asserted.

mod common;

use nova_data::GameData;
use nova_sim::cron::CronState;
use nova_sim::news;
use nova_sim::{
    Bit, CronId, GameDate, GovtId, NeverFires, PilotCatalog, RuleKey, RuleSource, Rulebook, Session,
};

fn bit(n: u16) -> Bit {
    Bit::new(n).expect("a bit")
}

/// A new stock pilot's session, in flight.
fn new_pilot(data: &GameData) -> Session {
    Session::start(data).expect("the stock first chär starts")
}

#[test]
fn a_stock_cron_starts_and_runs_its_expressions_on_the_right_day() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = new_pilot(&data);
    let year = session.date().year();
    session.set_control_bit(bit(42), false);
    session.set_date(GameDate::new(year, 8, 30).expect("a date"));
    session.let_days_pass(1, &mut NeverFires);
    assert_eq!(session.date(), GameDate::new(year, 8, 31).expect("a date"));
    assert!(!session.control_bit(bit(42)), "31 Aug is before its window");
    session.let_days_pass(1, &mut NeverFires);
    let first = GameDate::new(year, 9, 1).expect("a date");
    assert_eq!(session.date(), first);
    assert!(session.control_bit(bit(42)), "crön 156's OnStart on 1 Sep");
    let last = (0..105).fold(first, |date, _| date.next_day());
    while session.date().next_day() < last {
        session.let_days_pass(1, &mut NeverFires);
        assert!(
            session.control_bit(bit(42)),
            "running on {:?}",
            session.date()
        );
    }
    session.let_days_pass(1, &mut NeverFires);
    assert_eq!(session.date(), last);
    assert_eq!(last, GameDate::new(year, 12, 15).expect("a date"));
    assert!(
        !session.control_bit(bit(42)),
        "its OnEnd as its 105 days end"
    );
}

#[test]
fn a_stock_cron_with_no_dates_starts_on_its_enable_on() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = new_pilot(&data);
    session.set_control_bit(bit(8888), true);
    session.set_control_bit(bit(8889), false);
    session.let_days_pass(1, &mut NeverFires);
    assert!(session.control_bit(bit(8889)), "crön 222's OnStart");
}

/// Crön 184's counters in `session`, as `(days, holdoff)`.
fn border_change(session: &Session) -> Option<(i16, i16)> {
    session
        .pilot()
        .crons()
        .get(&CronId(184))
        .map(|state| (state.days, state.holdoff))
}

/// Whether crön 184's news shows in `session`.
fn border_news(session: &Session) -> bool {
    session
        .pilot()
        .crons()
        .get(&CronId(184))
        .copied()
        .is_some_and(news::running)
}

/// A new stock pilot reading `rules`, crön 184 run to its end: its
/// `OnStart` on the first day and its `OnEnd` 14 days later.
fn border_change_ended(data: &GameData, rules: Rulebook) -> Session {
    let mut session = new_pilot(data).with_rules(rules);
    session.set_control_bit(bit(6031), false);
    session.set_control_bit(bit(333), false);
    session.let_days_pass(1, &mut NeverFires);
    assert!(session.control_bit(bit(6021)), "crön 184's OnStart");
    assert_eq!(border_change(&session), Some((14, 0)));
    session.let_days_pass(14, &mut NeverFires);
    assert!(!session.control_bit(bit(6021)), "its OnEnd");
    session
}

#[test]
fn a_rare_border_change_rearms_after_its_post_holdoff_and_by_the_engine_never_ends() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let record = data
        .crons()
        .into_iter()
        .find(|record| record.id == CronId(184))
        .expect("crön 184");
    assert_eq!(
        news::slots(&record).collect::<Vec<_>>(),
        [(GovtId(128), 15027), (GovtId(129), 15053)]
    );

    let mut session = border_change_ended(&data, Rulebook::default());
    assert_eq!(border_change(&session), Some((-1, 550)));
    assert!(!border_news(&session), "no news in its post-holdoff");
    session.let_days_pass(549, &mut NeverFires);
    assert_eq!(border_change(&session), Some((-1, 1)));
    session.let_days_pass(1, &mut NeverFires);
    assert_eq!(border_change(&session), None);
    session.let_days_pass(1, &mut NeverFires);
    assert!(session.control_bit(bit(6021)), "it fired again");

    let engine = Rulebook::default().with_override(RuleKey::CronPostHoldoff, RuleSource::Engine);
    let mut session = border_change_ended(&data, engine);
    assert_eq!(border_change(&session), Some((0, 0)));
    assert!(border_news(&session));
    for day in 1..=600 {
        session.set_control_bit(bit(6021), true);
        session.let_days_pass(1, &mut NeverFires);
        assert!(!session.control_bit(bit(6021)), "its OnEnd on day {day}");
        let state = session.pilot().crons().get(&CronId(184)).copied();
        assert!(
            matches!(state, Some(CronState { holdoff: 0, .. })),
            "day {day}: {state:?}"
        );
        assert!(border_news(&session), "its news on day {day}");
    }
}
