//! The storyline walker over the stock data, two walks. Each skips,
//! passing, when `NOVA_DATA` is unset.
//!
//! - United Shipping's chain, `mïsn` 504 "United Shipping Intro" then 505
//!   "Un. Shipping Delivery", from a new pilot's first take to its last
//!   delivery.
//! - The Federation string, Fed1 to Fed43 (`mïsn` 428 to 474), the goal
//!   the engine builds toward. It starts from a pilot who already meets
//!   the string's combat rating, legal record and cargo needs: rating
//!   comes only from kills in flight, which the walker skips, and flying
//!   every fight is not the walk's job. On an engine gap it stops and
//!   prints the walker's report.
//!
//! Why United Shipping's chain:
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
use nova_sim::{
    Bit, GovtId, MissionId, NeverFires, Pilot, PilotCatalog, RankId, Session, ShipId, is_landable,
};

/// A new stock pilot docked at the first landable stellar of its system,
/// through a save that says so, flown and settled with every roll 1.
fn docked_new_pilot(data: &GameData) -> Session {
    docked_pilot(data, |_| {})
}

/// The Federation's government, whose stellars offer Fed1 (428's
/// `AvailStel` 10000).
const FEDERATION_GOVT: GovtId = GovtId(128);

/// The Terrapin, `shïp` 136: 125 tons of cargo, priced 150000, tech
/// level 4, `BuyRandom` 90 and an empty `Require`, so a player can buy
/// it. The starting Shuttle (128) holds 10.
const TERRAPIN: ShipId = ShipId(136);

/// The most cargo any Federation mission carries: Fed1-6 each carry 20
/// tons (`CargoQty` 20, with `Flags2` bit 1 asking for room); later
/// missions carry 15 or fewer.
const FEDERATION_CARGO: u32 = 20;

/// A new stock pilot, as [`docked_new_pilot`] builds it, but already
/// qualified for every mission in [`FEDERATION`], through the save.
fn docked_qualified_pilot(data: &GameData) -> Session {
    let session = docked_pilot(data, |save| {
        // The highest `AvailRating` in the string: 428 = 150, 457 = 50,
        // 458-461 = 100, 462-463 = 150, 464-474 = 200. `avail_rating`
        // passes when the rating is at least that, with no upper bound.
        save["combat_rating"] = serde_json::json!(200);
        // 428's `AvailRecord` is 2, the most Fed1 needs, and no more on
        // purpose: the chain should raise the record for 435-438 itself.
        save["legal"] = serde_json::json!([{ "govt": FEDERATION_GOVT.0, "record": 2 }]);
        // `null` derives every system's record from `legal` on `fly`
        // (`legal::systems_from_govts`), so each Federation system holds
        // 2, the record `avail_record` reads under the default rules.
        save["system_legal"] = serde_json::Value::Null;
        save["ship"] = serde_json::json!(TERRAPIN.0);
        // The Terrapin's own default items and stock weapons, read on
        // `fly`, not the Shuttle's.
        save["outfits"] = serde_json::Value::Null;
        save["stock_weapons_fitted"] = serde_json::json!(false);
        // `fly` refits to the Terrapin's stats and holds each reserve to
        // its maximum, so these fill it.
        let full = serde_json::json!({ "now": f32::MAX, "max": f32::MAX });
        save["reserves"] = serde_json::json!({ "shield": full, "armor": full, "fuel": full });
    });
    let pilot = session.pilot();
    assert_eq!(pilot.combat_rating(), 200, "rating");
    assert_eq!(pilot.legal_record(FEDERATION_GOVT), 2, "Federation record");
    assert_eq!(pilot.system_record(session.system()), 2, "record here");
    assert_eq!(session.ship(), TERRAPIN, "ship");
    assert!(session.stats().capacity >= FEDERATION_CARGO, "hold space");
    assert!(session.reserves().fuel.now >= session.stats().fuel, "fuel");
    session
}

/// A new stock pilot, its save changed by `edit`, docked at the first
/// landable stellar of its system, flown and settled with every roll 1.
fn docked_pilot(data: &GameData, edit: impl FnOnce(&mut serde_json::Value)) -> Session {
    let pilot = Pilot::new(data, "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    edit(&mut save);
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

/// The Federation string in play order, Fed1 (428) to Fed43 "LAST" (474).
///
/// The order follows the bit chain: Fed N's `OnSuccess` sets the bit Fed
/// N+1's `AvailBits` needs, b50 (set by 428) up to b89 (set by 474). The
/// names, which end `;FedN`, only confirm it. Where the string branches,
/// this is the path a new pilot with no other storyline's bits takes:
/// - Fed13 is 443 "from Fed": 440 needs the bounty-hunter bits (`b8 |
///   b500`) and 611 the Rebel bit b126;
/// - Fed17 is 447 "from Fed": 448 needs `b8 | b500` and leads into the
///   forced branch (575-596);
/// - Fed18-20 are left out: 449 needs `b8 | b500`, and 450 and 451 are
///   an optional mission-computer loop (b123 on, then off), while 452
///   (Fed21) needs only b66;
/// - the repeatable runs 442 and 445 (Fed14a/15a) and the side missions
///   597 (Gli-tech7), 851 and 852 (the Action Man runs) are left out;
/// - Fed30 is 461 (registered), not 824 (the unregistered cutoff);
/// - Fed42 is 473, not 613, which needs b1000 from the Gli-tech side
///   mission.
///
/// Fed25 (456), Fed26 (457) and Fed38 (469) set b6666, which the next
/// mission refuses; crön 221 clears it. Fed12's `OnSuccess`, Fed35's and
/// Fed37's `OnAccept` and Fed43's `OnSuccess` grant ranks 128, 150, 129
/// and 149, each checked after the step that grants it. Fed38 (goal 4,
/// observe) and Fed42 (goal 0, destroy) have ship goals; Fed7 (434) and
/// Fed24 (455) name goal 1 but `ShipCount` -1, so no ships, and are
/// scripted as a plain take and delivery.
const FEDERATION: &[Step] = &[
    // Fed1 Federation Resupply
    Step::Take(MissionId(428)),
    Step::Deliver(MissionId(428)),
    // Fed2 Federation Resupply
    Step::Take(MissionId(429)),
    Step::Deliver(MissionId(429)),
    // Fed3 Federation Resupply
    Step::Take(MissionId(430)),
    Step::Deliver(MissionId(430)),
    // Fed4 Federation Resupply
    Step::Take(MissionId(431)),
    Step::Deliver(MissionId(431)),
    // Fed5 Federation Resupply
    Step::Take(MissionId(432)),
    Step::Deliver(MissionId(432)),
    // Fed6 Federation Resupply
    Step::Take(MissionId(433)),
    Step::Deliver(MissionId(433)),
    // Fed7 Bring in Terrorist
    Step::Take(MissionId(434)),
    Step::Deliver(MissionId(434)),
    // Fed8 Bring in Spy
    Step::Take(MissionId(435)),
    Step::Deliver(MissionId(435)),
    // Fed9 Bring in Dissident
    Step::Take(MissionId(436)),
    Step::Deliver(MissionId(436)),
    // Fed10 Bring in Political Dissident
    Step::Take(MissionId(437)),
    Step::Deliver(MissionId(437)),
    // Fed11 Bring in Political Dissident (Rebel link)
    Step::Take(MissionId(438)),
    Step::Deliver(MissionId(438)),
    // Fed12 Bring in Fed Councilor
    Step::Take(MissionId(439)),
    Step::Deliver(MissionId(439)),
    Step::HasRank(RankId(128)),
    // Fed13 Infiltrate the Rebels (from Fed)
    Step::Take(MissionId(443)),
    Step::Deliver(MissionId(443)),
    // Fed14 Rebel Food Drop
    Step::Take(MissionId(441)),
    Step::Deliver(MissionId(441)),
    // Fed15 Rebel Equipment Drop
    Step::Take(MissionId(444)),
    Step::Deliver(MissionId(444)),
    // Fed16 Rescue Agent
    Step::Take(MissionId(446)),
    Step::Deliver(MissionId(446)),
    // Fed17 Receive Instructions from Krane (from Fed)
    Step::Take(MissionId(447)),
    Step::Deliver(MissionId(447)),
    // Fed21 Rebel Insertion
    Step::Take(MissionId(452)),
    Step::Deliver(MissionId(452)),
    // Fed22 Scout Moash Space
    Step::Take(MissionId(453)),
    Step::Deliver(MissionId(453)),
    // Fed23 Distract Moash House
    Step::Take(MissionId(454)),
    Step::Deliver(MissionId(454)),
    // Fed24 Talk to Moash Elders
    Step::Take(MissionId(455)),
    Step::Deliver(MissionId(455)),
    // Fed25 Return With Response
    Step::Take(MissionId(456)),
    Step::Deliver(MissionId(456)),
    // Crön 221 clears b6666: `EnableOn b6666`, `PreHoldoff` 1, `Duration`
    // 0, `OnEnd !b6666`. Taking off passes a day (`StayDays`, the
    // engine's by default), whose crön pass starts its 1-day pre-holdoff;
    // the next day's pass ends the holdoff, runs `OnStart`, and, as its
    // `Duration` is 0, `OnEnd` too (`CronZeroDurationHoldoff`, the
    // intended end by default), leaving it inactive. So one day on top of
    // the take-off's. `WaitForCron` cannot see it: it leaves no state.
    Step::PassDays(1),
    // Fed26 Deliver Instructions
    Step::Take(MissionId(457)),
    Step::Deliver(MissionId(457)),
    Step::PassDays(1), // b6666, as after Fed25
    // Fed27 Deliver Further Instructions
    Step::Take(MissionId(458)),
    Step::Deliver(MissionId(458)),
    // Fed28 Re-Infiltrate Rebels
    Step::Take(MissionId(459)),
    Step::Deliver(MissionId(459)),
    // Fed29 Carry Rebel Message
    Step::Take(MissionId(460)),
    Step::Deliver(MissionId(460)),
    // Fed30 Carry Reply
    Step::Take(MissionId(461)),
    Step::Deliver(MissionId(461)),
    // Fed31 Organize Meeting
    Step::Take(MissionId(462)),
    Step::Deliver(MissionId(462)),
    // Fed32 Lead Attack on Rebels
    Step::Take(MissionId(463)),
    Step::Deliver(MissionId(463)),
    // Fed33 Deliver Final Instructions
    Step::Take(MissionId(464)),
    Step::Deliver(MissionId(464)),
    // Fed34 Chat With Federation President
    Step::Take(MissionId(465)),
    Step::Deliver(MissionId(465)),
    // Fed35 Polaris Diplomatic Mission
    Step::Take(MissionId(466)),
    Step::HasRank(RankId(150)),
    Step::Deliver(MissionId(466)),
    // Fed36 Return With Polaris Proposal
    Step::Take(MissionId(467)),
    Step::Deliver(MissionId(467)),
    // Fed37 Scout Polaris Space
    Step::Take(MissionId(468)),
    Step::HasRank(RankId(129)),
    Step::Deliver(MissionId(468)),
    // Fed38 Scout Polaris Space
    Step::Take(MissionId(469)),
    Step::MeetGoal(MissionId(469)),
    Step::Deliver(MissionId(469)),
    Step::PassDays(1), // b6666, as after Fed25
    // Fed39 Fetch Polaris Leader
    Step::Take(MissionId(470)),
    Step::Deliver(MissionId(470)),
    // Fed40 Meet Auroran Forces
    Step::Take(MissionId(471)),
    Step::Deliver(MissionId(471)),
    // Fed41 Lead Auroran Forces
    Step::Take(MissionId(472)),
    Step::Deliver(MissionId(472)),
    // Fed42 Destroy Polaris Raven
    Step::Take(MissionId(473)),
    Step::MeetGoal(MissionId(473)),
    Step::Deliver(MissionId(473)),
    // Fed43 Take Krane to Earth (LAST)
    Step::Take(MissionId(474)),
    Step::Deliver(MissionId(474)),
    Step::HasRank(RankId(149)),
];

/// A qualified pilot walks [`FEDERATION`] to its last delivery, b89,
/// and rank 149. The stock data has one chär, 128 ".Trader", whose
/// `OnStart` is empty, so [`docked_qualified_pilot`] starts from the
/// pilot a new game starts with, given the rating, record and ship the
/// string needs.
#[test]
fn a_qualified_pilot_walks_the_federation_string_to_its_end() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let mut session = docked_qualified_pilot(&data);
    StoryWalker::new(&data)
        .walk(&mut session, FEDERATION)
        .unwrap_or_else(|failure| panic!("{failure}"));
}
