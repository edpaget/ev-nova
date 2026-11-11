//! The new game's port loop played end to end over the checked-in
//! fixture (`nova_data::new_game::fixture::folder`), through the app as
//! `main` builds it (`crate::new_game_port`'s app): no interface file, the
//! placeholder dialogs `nova::interface::wire` gives the new game, the
//! start-up appear run, the new game's rulebook and the Ink dialog loader.
//! Each frame asserts that nothing failed but a missing picture and that
//! nothing was warned of (`Harness::frame`).
//!
//! One pilot, Ada, plays it through the app's own inputs:
//!
//! 1. New Pilot, flying in Hearth in the start's hauler.
//! 2. Hearth Dock, every service laid out.
//! 3. The dock job's offer, deferred as the landing opens it, then taken
//!    from the bar: the hull plate aboard and a Dock pass lent.
//! 4. Grain bought and sold back at the Trade Center.
//! 5. The racked hauler bought at the shipyard, the hauler traded in and
//!    the plate kept aboard, its tank full; the turret ring kit fitted to
//!    it at the outfitter and a second mass driver bought onto its ring;
//!    grain bought to hold.
//! 6. Take-off, the stay's days passing, then the course to the Verge,
//!    clicked on the course map and flown on the gate autopilot with J
//!    through the side road, the passage costing the hull's gate fuel.
//! 7. Verge Camp: the delivery's completion, then the camp's greeting
//!    and the news the success releases.
//! 8. Take-off from the camp and a landing at Verge Workshop, the news
//!    heard there closed, where Recharge refills what the passage spent.
//! 9. The window closed, which saves the pilot, and the pilot opened
//!    again in a new app over the same store.
//!
//! The pilot saved after the Recharge is the pilot just before saving,
//! field by field. The pilot loaded from it is too, but for the course's
//! start: the side road's exit leaves it at the Verge, as the original's
//! hypergate exit does, and the load clears it by `RuleKey::LoadedCourse`'s
//! engine reading, as the original's open does.
//!
//! The passage's fare is `RuleKey::GateFuel`'s other reading, which the
//! new game's profile pins (`nova::ruleset`): the racked hauler's gate
//! fuel, read from the opened data, never retyped, so a fare the store
//! dropped fails the loop.
//!
//! What it pins is the seams between the phases that each pin their own
//! piece in `new_game_port`, `new_game_trade`, `new_game_outfits`,
//! `new_game_shipyard`, `new_game_missions` and `new_game_routes`. Expected
//! cash is worked from the start's, the listed prices and the records'
//! costs, so a price in the fixture moves them together.
//!
//! One state is put in rather than played, as every app test does:
//! approaching a stellar. The ship is put over Hearth Dock, Verge Camp and
//! Verge Workshop (`nova_sim::fixture::put_player_at`) before L, as the
//! engine has no autopilot to a stellar, and the original none. The
//! course between systems is flown, and the fuel Recharge fills is the
//! fuel the flight spent.
//!
//! The lent Dock pass goes with the old hull at the purchase, as the
//! original's shipyard sends every outfit not flagged persistent: the new
//! game's mapping writes no such flag, and whether a lent outfit should
//! survive a hull change is nova-like-game's question
//! (`lent-outfit-across-hull-change`). The completion's removal then
//! finds none to take back.

use std::collections::BTreeMap;

use nova::app::{Control, Showing, WindowEvent};
use nova_data::MissionObjective;
use nova_data::new_game::fixture::folder;
use nova_data::records::outfit::Outfit;
use nova_data::records::ship::Ship;
use nova_sim::fixture::MemoryPilots;
use nova_sim::flight_frame::FlightFrame;
use nova_sim::market::CLICK_TONS;
use nova_sim::navigation::NavMode;
use nova_sim::story::day::Stay;
use nova_sim::storylines::state::MissionState;
use nova_sim::{
    FUEL_PRICE_PER_UNIT, Gauge, OutfitId, Pilot, Service, Session, ShipId, StellarId, SystemId,
    Vec2,
};
use nova_view::galaxy::dialog::map_area;
use nova_view::spaceport::bar::{LEAVE_ITEM as BAR_LEAVE_ITEM, STORY_ITEMS};
use nova_view::spaceport::layout::{LEAVE_ITEM, RECHARGE_ITEM, service_item};
use nova_view::spaceport::outfitter::{BUY_ITEM as OUTFIT_BUY_ITEM, DONE_ITEM as OUTFIT_DONE_ITEM};
use nova_view::spaceport::shipyard::BUY_ITEM as SHIPYARD_BUY_ITEM;
use nova_view::spaceport::trade::{
    BUY_ITEM as TRADE_BUY_ITEM, DONE_ITEM as TRADE_DONE_ITEM, SELL_ITEM as TRADE_SELL_ITEM,
};
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{
    drawn, land_at, new_pilot_flying, opened_from, position_of, resumed_in,
};
use crate::new_game_port::{
    app_over, centre_of, click_bar_item, click_port_item, open_the_map, pilot, spaceport,
};
use crate::new_game_trade::{good, market, open_exchange, row_of, trade};
use crate::new_game_travel::{fixture, gates, nova_of, side_road_closing_day};
use crate::pilots::{Harness, saved};

/// The dock job's offer, its first line.
const OFFER_OPENS: &str = "The dockmaster looks up from a slate.";

/// The tons of hull plate the dock job delivers (`missions/dock-job.md`).
fn plate_tons() -> u32 {
    match opened_from(&folder()).storylines().missions["dock-job"].objective {
        Some(MissionObjective::Deliver { tons, .. }) => tons,
        ref other => panic!("a delivery: {other:?}"),
    }
}

/// The fixture's outfit `id`, loose or a kit.
fn outfit(id: &str) -> OutfitId {
    let ids = &fixture().ids;
    let table = if ids.kits.keys().any(|kit| kit.as_str() == id) {
        &ids.kits
    } else {
        &ids.outfits
    };
    OutfitId(nova_of(table, id))
}

/// The fuel a gate passage costs the racked hauler, as the opened data
/// gives it (see the module docs).
fn racked_fare() -> f32 {
    let fare = opened_from(&folder())
        .gate_fuel(ship("racked-hauler"))
        .expect("the racked hauler's gate fuel");
    f32::from(fare)
}

#[test]
fn a_new_pilot_plays_the_port_loop_and_the_pilot_saved_after_it_loads_equal() {
    let store = MemoryPilots::new();
    // 1. New Pilot, flying in Hearth in the hauler.
    let mut game = new_pilot_flying(app_over(opened_from(&folder()), &store));
    let start = pilot(&game);
    land_at_hearth_dock(&mut game);
    take_the_dock_job(&mut game);
    buy_and_sell_back_grain(&mut game);
    buy_the_racked_hauler(&mut game);
    fit_the_turret_ring_and_a_mass_driver(&mut game);
    buy_grain_to_hold(&mut game);
    take_off(&mut game, &start);
    fly_to_the_verge(&mut game);
    deliver_the_plate(&mut game);
    recharge_at_verge_workshop(&mut game);
    save_and_load(game, &store);
}

/// Step 2: Hearth Dock, every service laid out.
fn land_at_hearth_dock(game: &mut Harness) {
    land_at(game, "hearth-dock");
    let port = spaceport(game);
    assert_eq!(port.problem(), None, "laid out");
    for service in [
        Service::Bar,
        Service::TradeCenter,
        Service::Outfitter,
        Service::Shipyard,
    ] {
        assert!(port.offered().contains(&service), "{service:?}");
        assert!(
            port.dialog()
                .expect("laid out")
                .item_shown(service_item(service)),
            "{service:?}"
        );
    }
}

/// Step 3: the dock job: its offer opens by itself; deferred, it is taken
/// from the bar, putting the plate aboard and lending a Dock pass.
fn take_the_dock_job(game: &mut Harness) {
    let before = pilot(game);
    let page = drawn(game);
    assert!(page.contains(&OFFER_OPENS.to_owned()), "{page:?}");
    game.press(KeyCode::Digit3);
    game.press(KeyCode::Enter);
    assert_eq!(
        pilot(game).story.mission("dock-job"),
        Some(MissionState::Offered)
    );
    click_port_item(game, service_item(Service::Bar));
    game.frame();
    click_bar_item(game, STORY_ITEMS[0]);
    let page = drawn(game);
    assert!(page.contains(&OFFER_OPENS.to_owned()), "{page:?}");
    game.press(KeyCode::Digit1);
    game.press(KeyCode::Enter);
    game.frame();
    let leave = centre_of(
        spaceport(game).open_bar().expect("the bar").dialog(),
        BAR_LEAVE_ITEM,
    );
    game.click(leave);
    game.frame();
    assert!(spaceport(game).open_bar().is_none(), "the bar closed");
    let taken = pilot(game);
    let plate = plate_tons();
    assert_eq!(
        taken.story.mission("dock-job"),
        Some(MissionState::Active {
            cargo: plate,
            met: false
        })
    );
    assert_eq!(taken.total_cargo(), i64::from(plate), "the plate");
    assert_eq!(taken.owned(outfit("dock-pass")), 1, "the Dock pass");
    assert_eq!(taken.cash, before.cash);
}

/// Grain's price at the open exchange.
fn grain_price(game: &Harness) -> i64 {
    market(game).rows[row_of(game, good("grain"))].price
}

/// Step 4: Grain bought and sold back at the Trade Center.
fn buy_and_sell_back_grain(game: &mut Harness) {
    let grain = good("grain");
    let before = pilot(game);
    open_exchange(game);
    let price = grain_price(game);
    trade(game, grain, TRADE_BUY_ITEM);
    let bought = pilot(game);
    assert_eq!(bought.cargo.get(&grain), Some(&CLICK_TONS));
    assert_eq!(bought.cash, before.cash - i64::from(CLICK_TONS) * price);
    trade(game, grain, TRADE_SELL_ITEM);
    let sold = pilot(game);
    assert_eq!(sold.cargo.get(&grain), None, "sold back");
    assert_eq!(sold.cash, before.cash, "at the price it was bought at");
    close_exchange(game);
}

/// Step 5a: the racked hauler, bought from the shipyard, the hauler traded
/// in: the plate stays aboard, and the lent pass goes (see the module
/// docs).
fn buy_the_racked_hauler(game: &mut Harness) {
    let before = pilot(game);
    let racked = ship("racked-hauler");
    let trade_in = buy_ship(game, racked);
    let rehulled = pilot(game);
    assert_eq!(rehulled.ship, racked);
    assert_eq!(
        rehulled.cash,
        before.cash - hull_price(racked) + trade_in,
        "less the trade-in"
    );
    assert_eq!(rehulled.owned(outfit("cargo-racks")), 1, "its kit");
    assert_eq!(rehulled.owned(outfit("mass-driver")), 1, "its nose gun");
    assert_eq!(
        rehulled.total_cargo(),
        i64::from(plate_tons()),
        "the plate aboard the new hull"
    );
    assert_eq!(
        rehulled.owned(outfit("dock-pass")),
        0,
        "not persistent, the lent pass goes with the old hull"
    );
    let fuel = rehulled.reserves.fuel;
    assert_eq!(fuel, Gauge::full(fuel.max), "the bought hull's tank full");
}

/// Step 5b and 5c: a kit fitted to the bought hull, the turret ring, adding
/// a turret hardpoint beside the nose's; then an outfit bought onto it, a
/// second mass driver.
fn fit_the_turret_ring_and_a_mass_driver(game: &mut Harness) {
    let before = pilot(game);
    let ring = outfit("turret-ring");
    open_outfitter(game);
    assert_eq!(turrets(game), (1, 1), "the nose turret, carrying its gun");
    buy_outfit(game, ring);
    let ringed = pilot(game);
    assert_eq!(ringed.owned(ring), 1);
    assert_eq!(ringed.cash, before.cash - outfit_price(ring));
    assert_eq!(turrets(game), (1, 2), "a second hardpoint, empty");

    let driver = outfit("mass-driver");
    buy_outfit(game, driver);
    let armed = pilot(game);
    assert_eq!(armed.owned(driver), 2);
    assert_eq!(armed.cash, ringed.cash - outfit_price(driver));
    assert_eq!(turrets(game), (2, 2), "the ring carrying it");
    close_outfitter(game);
}

/// Step 5d: Grain bought to hold.
fn buy_grain_to_hold(game: &mut Harness) {
    let grain = good("grain");
    let before = pilot(game);
    open_exchange(game);
    let price = grain_price(game);
    trade(game, grain, TRADE_BUY_ITEM);
    let stocked = pilot(game);
    assert_eq!(stocked.cargo.get(&grain), Some(&CLICK_TONS));
    assert_eq!(stocked.cash, before.cash - i64::from(CLICK_TONS) * price);
    close_exchange(game);
}

/// Step 6a: take-off: the stay's days pass, a ship bought and outfits
/// traded, and the side road is still open.
fn take_off(game: &mut Harness, start: &Pilot) {
    click_port_item(game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "taken off");
    let stay = Stay {
        outfit_traded: true,
        ship_bought: true,
        escorts_changed: 0,
    };
    let day = pilot(game).story.day;
    assert_eq!(day, start.story.day + stay.leaving_days());
    assert!(
        day < side_road_closing_day(),
        "the side road is still open on day {day}"
    );
}

/// Step 6b: the course to the Verge, clicked on the map and flown on the
/// gate autopilot: it comes out at the side road's far gate, on the same
/// day, its fuel down by the racked hauler's fare.
fn fly_to_the_verge(game: &mut Harness) {
    let before = pilot(game);
    let verge = system("verge");
    plot_course_to(game, verge);
    game.press(KeyCode::KeyJ);
    fly_the_autopilot(game);
    let arrived = pilot(game);
    assert_eq!(arrived.system, verge, "came out in the Verge");
    let exit = flight_session(game).player().position;
    assert_eq!(exit, side_road_exit(), "at the side road's far gate");
    assert_eq!(arrived.story.day, before.story.day, "a gate passes no day");
    assert_eq!(
        arrived.reserves.fuel,
        Gauge {
            now: before.reserves.fuel.now - racked_fare(),
            ..before.reserves.fuel
        },
        "the passage's fare spent"
    );
}

/// Step 7: Verge Camp: the delivery's completion, first; then the camp's
/// greeting and the news its success releases.
fn deliver_the_plate(game: &mut Harness) {
    let before = pilot(game);
    land_at_verge_camp(game);
    for (line, what) in [
        (
            "The camp boss signs for the plate.",
            "the completion, first",
        ),
        ("Wind snaps at the tents.", "the camp's greeting"),
        ("Hull plate reaches Verge Camp", "the news pop"),
    ] {
        let page = drawn(game);
        assert!(page.contains(&line.to_owned()), "{what}: {page:?}");
        game.press(KeyCode::Enter);
    }
    let delivered = pilot(game);
    assert_eq!(
        delivered.story.mission("dock-job"),
        Some(MissionState::Done)
    );
    assert!(delivered.flags.is_set("dock_regular"));
    assert_eq!(
        delivered.cargo,
        BTreeMap::from([(good("grain"), CLICK_TONS)]),
        "the plate dropped off, the grain kept"
    );
    assert_eq!(delivered.cash, before.cash, "the job pays nothing");
}

/// Step 8: off the camp and down at Verge Workshop, which sells fuel:
/// Recharge refills what the side road's passage spent, at its price a
/// unit.
fn recharge_at_verge_workshop(game: &mut Harness) {
    click_port_item(game, LEAVE_ITEM);
    assert_eq!(game.showing(), Showing::Flight, "off the camp");
    land_at(game, "verge-workshop");
    let landed = pilot(game);
    let fuel = landed.reserves.fuel;
    assert_eq!(
        fuel,
        Gauge {
            now: fuel.max - racked_fare(),
            ..fuel
        },
        "short by the fare"
    );
    // The news the delivery released is heard here too, over the port.
    let page = drawn(game);
    assert!(
        page.contains(&"Hull plate reaches Verge Camp".to_owned()),
        "the news pop: {page:?}"
    );
    game.press(KeyCode::Enter);
    click_port_item(game, RECHARGE_ITEM);
    let fuelled = pilot(game);
    assert_eq!(fuelled.reserves.fuel, Gauge::full(fuel.max), "filled");
    // A whole number of units: the fare is a store `i16`.
    #[allow(clippy::cast_possible_truncation)]
    let units = racked_fare() as i64;
    assert_eq!(fuelled.cash, landed.cash - units * FUEL_PRICE_PER_UNIT);
}

/// Step 9: saved as the window closes, the pilot is the pilot before
/// saving, field by field, and loads so in a new app over the same store
/// but for the course's start the load clears, holding the bought hull,
/// its kits and outfits, the grain, its cash and the completed job.
fn save_and_load(mut game: Harness, store: &MemoryPilots) {
    let before = pilot(&game);
    assert_eq!(before.ship, ship("racked-hauler"), "the bought hull");
    for (id, count) in [("cargo-racks", 1), ("turret-ring", 1), ("mass-driver", 2)] {
        assert_eq!(before.owned(outfit(id)), count, "{id}");
    }
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_same(&saved(store, "Ada"), &before, "saved");
    let mut resumed = resumed_in(app_over(opened_from(&folder()), store));
    resumed.frame();
    let loaded = pilot(&resumed);
    // The side road's exit restarts the course at the Verge, as the
    // original's hypergate exit leaves slot 0 at the system entered and
    // the hops at -1 (@0x63e0e-0x63e3c); nothing before the save clears
    // it. The load then clears it by `RuleKey::LoadedCourse`'s engine
    // reading, as `_OpenPilotFile`'s `_ResetPlayer` sets all 32 slots to
    // -1 (@0x1d6bb-0x1d6cd).
    assert_eq!(
        (before.course.as_slice(), before.course_start),
        ([].as_slice(), Some(system("verge"))),
        "the gate's exit restarted the course at the Verge"
    );
    let cleared = Pilot {
        course_start: None,
        ..before
    };
    assert_same(&loaded, &cleared, "loaded");
}

/// Asserts that `got` is `want`, field by field, then whole: each field
/// the loop changed with its own message, then every field.
fn assert_same(got: &Pilot, want: &Pilot, how: &str) {
    assert_eq!(got.name, want.name, "{how}: name");
    assert_eq!(
        (got.ship, &got.ship_name),
        (want.ship, &want.ship_name),
        "{how}: ship"
    );
    assert_eq!(got.outfits, want.outfits, "{how}: outfits");
    assert_eq!(got.cargo, want.cargo, "{how}: cargo");
    assert_eq!(got.cash, want.cash, "{how}: cash");
    assert_eq!(got.reserves, want.reserves, "{how}: reserves");
    assert_eq!(
        (got.system, got.stellar),
        (want.system, want.stellar),
        "{how}: where"
    );
    assert_eq!(
        got.story.mission("dock-job"),
        want.story.mission("dock-job"),
        "{how}: the dock job"
    );
    assert_eq!(got.story.day, want.story.day, "{how}: day");
    assert_eq!(got.flags, want.flags, "{how}: flags");
    assert_eq!(got.dialogs, want.dialogs, "{how}: dialogs");
    assert_eq!(got, want, "{how}: the whole pilot");
}

/// The session in flight.
fn flight_session(game: &Harness) -> &Session {
    game.app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session")
}

/// Where the side road comes out in the Verge: its far gate,
/// `verge/side-road`'s.
fn side_road_exit() -> Vec2 {
    let game = fixture();
    let road = StellarId(nova_of(&game.ids.gates, "hearth/side-road"));
    gates(&game)
        .into_iter()
        .find(|gate| gate.stellar == road)
        .expect("the side road's gate")
        .partner
        .2
}

/// Puts the ship over Verge Camp, an ungoverned outpost, and lands with
/// one L.
fn land_at_verge_camp(game: &mut Harness) {
    let camp = position_of("verge-camp");
    game.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            nova_sim::fixture::put_player_at(sim.session_mut().expect("a session"), camp);
        });
    game.press(KeyCode::KeyL);
    assert_eq!(game.showing(), Showing::Spaceport, "landed at Verge Camp");
}

/// The fixture's system `id`.
fn system(id: &str) -> SystemId {
    SystemId(nova_of(&fixture().ids.systems, id))
}

/// Opens the course map, clicks `to` on it, scrolled into view with the
/// arrows first, and closes it: one gate away, `to` is the next hop,
/// selected in mode 3 with no further course.
fn plot_course_to(game: &mut Harness, to: SystemId) {
    open_the_map(game);
    let shown = map_area().grown(-12.0);
    let at = (0..200)
        .find_map(|_| {
            let map = game
                .app
                .screen()
                .flight_view()
                .expect("flying")
                .course_map();
            let system = map.model().system(to).expect("on the map");
            let at = map.transform().world_to_screen(system.position());
            let key = if at.x < shown.min.x {
                KeyCode::ArrowLeft
            } else if at.x > shown.max.x {
                KeyCode::ArrowRight
            } else if at.y < shown.min.y {
                KeyCode::ArrowUp
            } else if at.y > shown.max.y {
                KeyCode::ArrowDown
            } else {
                return Some(at);
            };
            game.press(key);
            None
        })
        .expect("scrolled into view");
    game.click(at);
    game.frame();
    let map = game
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .course_map();
    assert_eq!(map.selected(), Some(to), "selected");
    let session = game
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session");
    assert_eq!(
        session.nav_mode(),
        NavMode::Hyperspace(Some(to)),
        "the next hop, one gate away"
    );
    game.press(KeyCode::KeyM);
    game.frame();
    assert!(
        !game.app.screen().flight_view().expect("flying").map_open(),
        "closed"
    );
}

/// The most frames the gate autopilot may take to fly its course.
const AUTOPILOT_FRAMES: u32 = 8_000;

/// Draws frames while the gate autopilot flies, until it lets go.
fn fly_the_autopilot(game: &mut Harness) {
    let flying = |game: &Harness| {
        let session = game.app.screen().flight_view().expect("flying").session();
        let mut frame = FlightFrame::default();
        session.expect("a session").fill_frame(&mut frame);
        frame.gate_pilot.is_some()
    };
    assert!(flying(game), "J engages the gate autopilot");
    for _ in 0..AUTOPILOT_FRAMES {
        game.frame();
        if !flying(game) {
            return;
        }
    }
    panic!("the autopilot still flies after {AUTOPILOT_FRAMES} frames");
}

/// The turrets the bought hull carries and the most it can, as the open
/// outfitter shows them.
fn turrets(game: &Harness) -> (i32, i32) {
    let armed = spaceport(game)
        .open_outfitter()
        .map(|outfitter| outfitter.outfitter().armed)
        .expect("the outfitter");
    (armed.turrets, armed.turret_limit)
}

/// Opens the outfitter from the spaceport.
fn open_outfitter(game: &mut Harness) {
    click_port_item(game, service_item(Service::Outfitter));
    game.frame();
}

/// Buys one `outfit` at the open outfitter: its cell, then Buy.
fn buy_outfit(game: &mut Harness, outfit: OutfitId) {
    let outfitter = spaceport(game).open_outfitter().expect("the outfitter");
    assert_eq!(outfitter.problem(), None, "laid out");
    assert_eq!(outfitter.top_row(), 0, "scrolled to its top");
    let index = (outfitter.outfitter().rows.iter())
        .position(|row| row.id == outfit)
        .unwrap_or_else(|| panic!("{outfit:?} listed"));
    let cell = outfitter.cell_bounds(index).expect("a cell").center();
    game.click(cell);
    game.frame();
    let buy = centre_of(
        spaceport(game).open_outfitter().expect("open").dialog(),
        OUTFIT_BUY_ITEM,
    );
    game.click(buy);
    game.frame();
    let outfitter = spaceport(game).open_outfitter().expect("still open");
    assert!(outfitter.quantity().is_none(), "no count asked");
}

/// Closes the outfitter with its Done.
fn close_outfitter(game: &mut Harness) {
    let done = centre_of(
        spaceport(game).open_outfitter().expect("open").dialog(),
        OUTFIT_DONE_ITEM,
    );
    game.click(done);
    game.frame();
    assert!(spaceport(game).open_outfitter().is_none(), "closed");
}

/// The price `outfit`'s record asks.
fn outfit_price(outfit: OutfitId) -> i64 {
    let data = opened_from(&folder());
    let entry = data
        .get::<Outfit>(outfit.0)
        .expect("present")
        .expect("decoded");
    i64::from(entry.record.cost)
}

/// Closes the Trade Center with its Done.
fn close_exchange(game: &mut Harness) {
    let done = centre_of(
        spaceport(game).open_trade().expect("the exchange").dialog(),
        TRADE_DONE_ITEM,
    );
    game.click(done);
    game.frame();
    assert!(spaceport(game).open_trade().is_none(), "closed");
}

/// The fixture's hull `id`.
fn ship(id: &str) -> ShipId {
    ShipId(nova_of(&fixture().ids.ships, id))
}

/// The price `ship`'s record asks.
fn hull_price(ship: ShipId) -> i64 {
    let data = opened_from(&folder());
    let entry = data.get::<Ship>(ship.0).expect("present").expect("decoded");
    i64::from(entry.record.cost)
}

/// Buys `ship` at the shipyard: its cell, Buy Ship, and Return on the
/// name prompt's default. Gives the trade-in the shipyard offered.
fn buy_ship(game: &mut Harness, ship: ShipId) -> i64 {
    click_port_item(game, service_item(Service::Shipyard));
    game.frame();
    let shipyard = spaceport(game).open_shipyard().expect("the shipyard");
    assert_eq!(shipyard.problem(), None, "laid out");
    let trade_in = shipyard.shipyard().trade_in;
    assert_eq!(shipyard.top_row(), 0, "scrolled to its top");
    let index = (shipyard.shipyard().rows.iter())
        .position(|row| row.id == ship)
        .expect("listed");
    let cell = shipyard.cell_bounds(index).expect("a cell").center();
    game.click(cell);
    game.frame();
    let buy = centre_of(
        spaceport(game).open_shipyard().expect("open").dialog(),
        SHIPYARD_BUY_ITEM,
    );
    game.click(buy);
    game.frame();
    assert!(
        spaceport(game)
            .open_shipyard()
            .expect("open")
            .naming()
            .is_some(),
        "the name prompt"
    );
    game.press(KeyCode::Enter);
    game.frame();
    assert!(
        spaceport(game).open_shipyard().is_none(),
        "the purchase closes the shipyard"
    );
    trade_in
}
