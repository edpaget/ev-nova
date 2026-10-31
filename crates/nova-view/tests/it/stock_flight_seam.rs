//! The flight's seam over the stock data: a stock new pilot's flight,
//! opened at the edge, through `SessionFlight`. Skips, passing, when
//! `NOVA_DATA` is unset.

use crate::common;

use std::rc::Rc;

use nova_data::GameData;
use nova_sim::control::Bit;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::{HyperlinkRule, Pilot, RuleKey, RuleSource, Rulebook, SessionSetup, SystemId};
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
    assert!(view.failure().is_none(), "the stock pilot starts");
    assert_eq!(
        check_contract(&mut view.sim().clone(), &view.sim().opening()),
        Ok(())
    );
    let mut frame = FlightFrame::default();
    view.sim().frame(&mut frame);
    let at = frame.player().position;
    assert_eq!(view.shown_position(), Point::new(at.x, at.y));
}

/// Aldebaran's two versions, 141 and 532, both visible between Polaris
/// missions 183 and 887, opened at the edge with no setup of the test's
/// own and no tick. 173's Con slot names 532, so a pilot who explored 173
/// knows it by the other `colocated_resolution` reading alone: the load
/// works the known set out by the rule it is given. The two list the same
/// slots, so their links agree here; `edge_checks`' fixture is where links
/// differ.
#[test]
fn a_stock_pilot_opens_on_aldebaran_by_the_colocated_rule_it_is_set_up_with() {
    let Some(data) = open() else {
        return;
    };
    let data = Rc::new(data);
    let mut pilot = Pilot::new(&data, "Ada").expect("a stock pilot");
    pilot.bits_mut().set(Bit::new(305).expect("a bit"));
    pilot.explore(SystemId(173));
    assert!(!pilot.has_explored(SystemId(532)));
    for source in RuleSource::ALL {
        let rules = Rulebook::default().with_override(RuleKey::ColocatedResolution, source);
        let view = open_flight(
            Rc::clone(&data),
            FlightStart::Loaded(pilot.clone()),
            SessionSetup {
                rules,
                ..SessionSetup::default()
            },
            Collaborators::default(),
        );
        let session = view.session().expect("flying");
        let (known, jumps) = match source {
            RuleSource::Engine => (false, vec![SystemId(141)]),
            RuleSource::Bible => (true, vec![SystemId(141), SystemId(532)]),
        };
        assert_eq!(session.is_known(SystemId(532)), known, "{source:?}");
        assert_eq!(
            session
                .star_map()
                .jumps(SystemId(173), HyperlinkRule::Engine),
            jumps,
            "{source:?}"
        );
        assert_eq!(
            view.course_map().model().links(),
            session.star_map().links(),
            "{source:?}"
        );
    }
}
