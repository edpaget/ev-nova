//! The flight's seam over the stock data: a stock new pilot's flight,
//! opened at the edge, through `SessionFlight`. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::SessionSetup;
use nova_sim::flight_frame::FlightFrame;
use nova_view::Point;
use nova_view::flight::sim_port::fixture::check_contract;
use nova_view::flight::{Collaborators, FlightSim, FlightStart, open_flight};

fn open() -> Option<GameData> {
    let dir = common::nova_data()?;
    Some(GameData::open(&dir, None).expect("the stock data opens"))
}

/// The contract on `SessionFlight` over the stock data, and the first
/// frame (`FlightSim::frame`): a stock new pilot opened at the edge
/// (`open_flight`) meets the port's contract, and the view draws its ship
/// where its first frame's player record puts it.
#[test]
fn a_stock_new_pilot_meets_the_contract_and_is_drawn_where_its_first_frame_puts_it() {
    let Some(data) = open() else {
        return;
    };
    let view = open_flight(
        Rc::new(data),
        FlightStart::New,
        SessionSetup::default(),
        Collaborators::default(),
    );
    assert!(view.sim().failure().is_none(), "the stock pilot starts");
    assert_eq!(check_contract(&mut view.sim().clone()), Ok(()));
    let mut frame = FlightFrame::default();
    view.sim().frame(&mut frame);
    let at = frame.player().position;
    assert_eq!(view.shown_position(), Point::new(at.x, at.y));
}
