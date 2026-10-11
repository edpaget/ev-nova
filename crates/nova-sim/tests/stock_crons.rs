//! The daily cron pass over the stock data: a new pilot's days start
//! stock cröns on the right day and run their expressions. Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - Crön 156 "Auroran Drop Bear Mating Season" runs yearly from 1 Sep
//!   to 30 Dec (`F 1/9/-1`, `L 30/12/-1`), `Random` 100, `Duration` 105,
//!   `EnableOn !b42`, `OnStart b42` and `OnEnd !b42`.
//! - Crön 222 "Advanced Pirate news" has no dates, `EnableOn b8888 &
//!   !b8889` and `OnStart b8889`.
//!
//! Every roll is 0 ([`NeverFires`]), which passes every `Random` of 0 or
//! more by the engine's reading, so the other stock cröns start too:
//! only the bits these two own are asserted.

mod common;

use nova_data::GameData;
use nova_sim::{Bit, GameDate, NeverFires, Session};

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
