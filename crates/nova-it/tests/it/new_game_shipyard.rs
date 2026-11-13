//! The new-game fixture's shipyard played headless through the session's
//! own commands: Hearth Dock, the fixture's one shipyard, sells the
//! cutter, the racked hauler and the tender, which their `sold_at` names;
//! the hauler, priced but sold nowhere, the wardens' picket and the
//! tender's drone aren't listed. A
//! fresh start, on its own cash in the hauler, buys the racked hauler:
//! the hauler is traded in, the difference charged, and the new hull
//! flies with its default kit and its own turret hardpoint's stock gun.
//! It can still, having first bought goods and fitted the racks to the
//! hauler, as the port loop does, and the hauler's racks, unmarked, go
//! with it. The Dock pass the dock job lends, whose file says it stays
//! with the pilot, is still held after the purchase.
//!
//! This pins only that the fixture's data, the new game's mapping
//! (`nova_data::new_game`) and the shipyard meet. The rules are pinned in
//! their crates: the price through the `base` chain in
//! nova-galaxy-format, the tech numbers and `SpecialTech` slots in
//! nova-data, the listing, flux, trade-in and purchase in nova-sim-port,
//! and the bought hull's fittings in nova-sim.

use nova::chance::SplitMix;
use nova_data::new_game::fixture::folder;
use nova_data::records::character::Character;
use nova_sim::outfitter::OutfitOrder;
use nova_sim::{Direction, Good, Lot, Order, OutfitId, ShipId};

use crate::new_game_play::Play;
use crate::new_game_travel::nova_of;

/// The fixture's start, as it begins, flying in Hearth.
fn play() -> Play {
    Play::new(&folder(), "hearth", |_| {})
}

fn ship(play: &Play, id: &str) -> ShipId {
    ShipId(nova_of(&play.game.ids.ships, id))
}

/// The shipyard's rows where the ship is docked, as `(class, price)`, and
/// its trade-in.
fn listed(play: &mut Play) -> (Vec<(ShipId, i64)>, i64) {
    let shipyard = play.session.shipyard(&mut play.chance).expect("a shipyard");
    let rows = shipyard
        .rows
        .iter()
        .map(|row| (row.id, row.price))
        .collect();
    (rows, shipyard.trade_in)
}

/// The start's own cash, as its `chär` holds it.
fn start_cash(play: &Play) -> i64 {
    let start = nova_of(&play.game.ids.starts, "dock-hand");
    let entry = play
        .data
        .get::<Character>(start)
        .expect("present")
        .expect("decoded");
    i64::from(entry.record.cash)
}

#[test]
fn hearth_dock_s_shipyard_lists_the_hulls_the_data_sells_there_at_their_prices() {
    let mut play = play();
    play.land("hearth-dock");
    let (rows, trade_in) = listed(&mut play);
    let cutter = ship(&play, "cutter");
    let racked = ship(&play, "racked-hauler");
    let tender = ship(&play, "tender");
    let tender_cost = play
        .data
        .get::<nova_data::records::ship::Ship>(tender.0)
        .expect("present")
        .expect("decoded")
        .record
        .cost;
    assert_eq!(
        rows,
        [
            (cutter, 50_000),
            (racked, 6000),
            (tender, i64::from(tender_cost))
        ]
    );
    assert_eq!(
        trade_in, 1000,
        "a quarter of the hauler's 4000, whose tech level takes no discount"
    );
}

#[test]
fn no_other_stellar_sells_a_hull() {
    let mut play = play();
    play.go_through("hearth/side-road");
    for stellar in ["verge-workshop", "verge-camp"] {
        play.land(stellar);
        assert!(
            play.session.shipyard(&mut SplitMix::new(1)).is_none(),
            "{stellar}"
        );
        play.take_off();
    }
}

#[test]
fn buying_the_racked_hauler_trades_in_the_hauler_charges_the_difference_and_fits_its_kit_and_hardpoints()
 {
    let mut play = play();
    let cash = start_cash(&play);
    assert_eq!(cash, 10_000, "starts/dock-hand.md");
    assert_eq!(play.session.pilot().cash(), cash);
    let hauler = ship(&play, "hauler");
    let racked = ship(&play, "racked-hauler");
    let racks = OutfitId(nova_of(&play.game.ids.kits, "cargo-racks"));
    let driver = OutfitId(nova_of(&play.game.ids.outfits, "mass-driver"));
    assert_eq!(play.session.pilot().ship(), hauler);

    play.land("hearth-dock");
    let bare = play.session.stats().capacity;
    play.session
        .buy_ship(racked, "Rack", &mut play.chance)
        .expect("bought");
    assert_eq!(play.session.pilot().cash(), cash - 6000 + 1000);
    assert_eq!(play.session.pilot().ship(), racked);
    assert_eq!(play.session.pilot().owned(racks), 1, "its default kit");
    assert_eq!(
        play.session.stats().capacity,
        bare + 10,
        "the racks' 10 tons"
    );

    let outfitter = play
        .session
        .outfitter(&mut play.chance)
        .expect("an outfitter");
    assert_eq!(
        (outfitter.armed.turrets, outfitter.armed.turret_limit),
        (1, 1),
        "its own nose turret, carrying its mass driver"
    );
    assert_eq!(play.session.pilot().owned(driver), 1, "folded");
    assert!(
        outfitter.rows.iter().any(|row| row.id == driver),
        "a turret hull is offered the mass driver loose"
    );
}

#[test]
fn the_dock_job_s_lent_pass_stays_with_the_pilot_across_the_racked_hauler_s_purchase() {
    let mut play = play();
    let racked = ship(&play, "racked-hauler");
    let pass = OutfitId(nova_of(&play.game.ids.outfits, "dock-pass"));
    play.land("hearth-dock");
    play.open();
    play.choose(0);
    play.close();
    assert_eq!(play.session.pilot().owned(pass), 1, "lent at the accept");

    play.session
        .buy_ship(racked, "Rack", &mut play.chance)
        .expect("bought");
    assert_eq!(play.session.pilot().ship(), racked);
    assert_eq!(
        play.session.pilot().owned(pass),
        1,
        "marked to stay with the pilot, kept at the trade-in"
    );
}

#[test]
fn a_fresh_start_that_bought_goods_and_an_outfit_can_still_buy_the_racked_hauler() {
    let mut play = play();
    let cash = start_cash(&play);
    let racked = ship(&play, "racked-hauler");
    let racks = OutfitId(nova_of(&play.game.ids.kits, "cargo-racks"));
    let grain = Good::Commodity(
        u8::try_from(nova_of(&play.game.ids.commodities, "grain")).expect("a slot"),
    );
    play.land("hearth-dock");

    let market = play.session.market().expect("an exchange");
    let row = (market.rows.iter())
        .position(|row| row.good == grain)
        .expect("grain listed");
    let order = Order {
        row,
        good: grain,
        direction: Direction::Buy,
        lot: Lot::Count(20),
    };
    let tons = market.tons(order).expect("affordable");
    assert_eq!(tons, 20);
    let price = market.rows[row].price;
    assert!(play.session.transact(|pilot| {
        nova_sim::market::settle(pilot, order, tons, price);
    }));
    let racked_out = OutfitOrder {
        outfit: racks,
        direction: Direction::Buy,
    };
    play.session
        .outfit(racked_out, &mut play.chance)
        .expect("the racks fitted");
    let spent = i64::from(tons) * price + 2000;
    assert_eq!(play.session.pilot().cash(), cash - spent);

    let (_, trade_in) = listed(&mut play);
    assert_eq!(
        trade_in,
        1000 + 1000,
        "the hauler's quarter and the racks' half"
    );
    play.session
        .buy_ship(racked, "Rack", &mut play.chance)
        .expect("affordable");
    assert_eq!(play.session.pilot().cash(), cash - spent - 6000 + trade_in);
    assert_eq!(
        play.session.pilot().owned(racks),
        1,
        "the hauler's racks, unmarked, went with it: these are the new hull's default kit"
    );
}
