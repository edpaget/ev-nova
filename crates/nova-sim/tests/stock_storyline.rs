//! The storyline walker over the stock data: a new pilot walks United
//! Shipping's chain, `mïsn` 504 "United Shipping Intro" then 505 "Un.
//! Shipping Delivery", from its first take to its last delivery. Skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! Why this chain:
//! - a new pilot can take 504: its `AvailBits` holds with no bit set, and
//!   its `AvailStel` (10000) takes any Federation stellar;
//! - 504's `OnSuccess`, `b9 K145`, sets b9, the bit 505's `AvailBits`
//!   needs, and nothing else in the chain tests a bit, a rank, a rating or
//!   a legal record;
//! - neither mission has ships, crons or a rank requirement, so landings
//!   alone complete both; and each picks its cargo up at its destination
//!   (`PickupMode` 1), two tons the Shuttle's holds take;
//! - completed at its return, a Federation stellar, 504 re-offers there
//!   with the table kept, so 505 (at the mission computer) is offered
//!   with no new landing; and 505's 20-day deadline is never at risk, as
//!   no day passes.
//!
//! `K145` grants a rank, which the walk does not need. It is the only
//! two-mission bit-linked chain without ships a new pilot reaches at the
//! mission computer and the bar; the other venues come later.

mod common;

use nova_data::GameData;
use nova_sim::fixture::{Done, Step, StoryWalker};
use nova_sim::{Bit, MissionId, NeverFires, Pilot, PilotCatalog, RankId, Session, is_landable};

/// A new stock pilot docked at the first landable stellar of its system,
/// through a save that says so, flown and settled with every roll 1.
fn docked_new_pilot(data: &GameData) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let mut session = Session::fly(data, docked).expect("flies");
    assert_eq!(session.landed(), Some(site.id), "docked");
    let _ = session.settle_script(data, &mut NeverFires);
    session
}

const INTRO: MissionId = MissionId(504);
const DELIVERY: MissionId = MissionId(505);

#[test]
fn a_new_pilot_walks_united_shippings_chain_to_its_end() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_new_pilot(&data);
    let mut walker = StoryWalker::new(&data);
    let intro = walker
        .walk(
            &mut session,
            &[
                Step::Take(INTRO),
                Step::Deliver(INTRO),
                Step::HasRank(RankId(145)),
            ],
        )
        .unwrap_or_else(|failure| panic!("{failure}"));
    assert!(
        session.pilot().control_bit(Bit::new(9).expect("a bit")),
        "504 sets b9"
    );
    let delivery = walker
        .walk(
            &mut session,
            &[Step::Take(DELIVERY), Step::Deliver(DELIVERY)],
        )
        .unwrap_or_else(|failure| panic!("{failure}"));
    let (Some(Done::Delivered { stellar: back, .. }), Some(Done::Took { stellar: took, .. })) =
        (intro.done.get(1), delivery.done.first())
    else {
        panic!("{intro:?} {delivery:?}");
    };
    assert_eq!(took, back, "505 offered where 504 was completed");
    assert_eq!(session.pilot().missions().missions(), [], "no mission held");
}
