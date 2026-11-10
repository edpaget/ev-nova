//! Acceptance: the new game's exchange over the checked-in fixture
//! (`nova_data::new_game::fixture::folder`), wired as `main` wires it
//! (`crate::new_game_port`'s app), with the new game's rulebook.
//!
//! The fixture's seven goods are six standard ones, in slots 0 to 5, and
//! Vintage wine, `jünk` 128. Hearth Dock prices all seven and Verge Camp
//! three ([`HEARTH_DOCK`], [`VERGE_CAMP`]).
//!
//! - **Listing.** Each exchange lists the standard goods it prices, in
//!   slot order, at their levels' prices, then Vintage wine at its port's
//!   price: Hearth Dock's high, as it buys it there, Verge Camp's low.
//! - **Buying and selling.** At Hearth Dock the pilot buys Grain and
//!   Vintage wine; the hold gains them and the pilot is charged, both
//!   saved. That saved pilot, dock job and all, moved into the Verge and
//!   landed at Verge Camp, sells them there at Verge Camp's prices.
//! - **Every special good.** A port that prices two special goods at one
//!   level lists both under the new game's rulebook, and only the
//!   highest-ID one by the original's reading of
//!   [`RuleKey::JunkListing`].
//!
//! Expected prices are worked from the fixture's base prices by
//! `nova_sim::market::band_price` at the standard markup, and a click's
//! lot from `nova_sim::market::CLICK_TONS`: what a level costs and how
//! much a click moves are nova-sim's, pinned there; this pins that the
//! new game's data reaches them.

use std::rc::Rc;

use nova::rulebook::ruleset_rulebook;
use nova::ruleset::Ruleset;
use nova::startup::{DataSource, DiskData, NewGameOpened};
use nova_data::new_game::fixture::folder;
use nova_data::records::junk::Junk;
use nova_data::{AnyRecord, Entry, GameData, StellarId as DataStellarId};
use nova_sim::fixture::MemoryPilots;
use nova_sim::market::{CLICK_TONS, Markup, PriceLevel, band_price};
use nova_sim::{Good, JunkId, Market, Pilot, PilotStore, RuleKey, RuleSource, Service};
use nova_view::spaceport::layout::service_item;
use nova_view::spaceport::trade::{BUY_ITEM, FIRST_ROW_ITEM, SELL_ITEM};
use winit::keyboard::KeyCode;

use crate::new_game_conversation::{
    land_at, new_pilot_flying, opened_from, position_of, resumed_in,
};
use crate::new_game_port::{
    app_over, centre_of, click_port_item, on_the_dock_job, pilot, resumed_after, spaceport,
};
use crate::new_game_travel::{fixture, nova_of};
use crate::pilots::{Harness, saved};

use PriceLevel::{High, Low, Medium};

/// What Hearth Dock prices, as its file has it: each good's name, its id,
/// its base price (`goods/<id>.md`) and its level there.
const HEARTH_DOCK: [(&str, &str, i64, PriceLevel); 7] = [
    ("Alloys", "alloys", 400, Medium),
    ("Fuel cells", "fuel-cells", 150, Low),
    ("Grain", "grain", 100, Low),
    ("Machinery", "machinery", 600, High),
    ("Medicine", "medicine", 800, Medium),
    ("Textiles", "textiles", 250, High),
    ("Vintage wine", "vintage-wine", 1200, High),
];

/// What Verge Camp prices, as its file has it.
const VERGE_CAMP: [(&str, &str, i64, PriceLevel); 3] = [
    ("Grain", "grain", 100, High),
    ("Medicine", "medicine", 800, High),
    ("Vintage wine", "vintage-wine", 1200, Low),
];

/// The good the fixture's `id` maps to: its standard slot, or its
/// `jünk`.
pub(crate) fn good(id: &str) -> Good {
    let ids = &fixture().ids;
    if ids.commodities.keys().any(|good| good.as_str() == id) {
        Good::Commodity(u8::try_from(nova_of(&ids.commodities, id)).expect("a slot"))
    } else {
        Good::Junk(JunkId(nova_of(&ids.junk, id)))
    }
}

/// The rows `prices` should list: each good, its name and its price at
/// its level.
fn expected(prices: &[(&str, &str, i64, PriceLevel)]) -> Vec<(Good, String, i64)> {
    prices
        .iter()
        .map(|&(name, id, base, level)| {
            (
                good(id),
                name.to_owned(),
                band_price(base, level, Markup::Standard),
            )
        })
        .collect()
}

/// The exchange open over the spaceport.
pub(crate) fn market(game: &Harness) -> Market {
    spaceport(game)
        .open_trade()
        .expect("the exchange is open")
        .market()
        .clone()
}

/// `market`'s rows as good, name and price.
fn listed(market: &Market) -> Vec<(Good, String, i64)> {
    market
        .rows
        .iter()
        .map(|row| (row.good, row.name.clone(), row.price))
        .collect()
}

/// Opens the Trade Center from the spaceport.
pub(crate) fn open_exchange(game: &mut Harness) {
    click_port_item(game, service_item(Service::TradeCenter));
    game.frame();
}

/// The row `good` is listed on.
pub(crate) fn row_of(game: &Harness, good: Good) -> usize {
    market(game)
        .rows
        .iter()
        .position(|row| row.good == good)
        .expect("listed")
}

/// Selects `good`'s row and clicks `button`, Buy or Sell. The list shows
/// each row in its list slot (`MarketRow::slot`), which is the row's
/// place while each slot holds one row, as in the fixture.
pub(crate) fn trade(game: &mut Harness, good: Good, button: usize) {
    let slot = market(game).rows[row_of(game, good)].slot;
    let dialog = spaceport(game).open_trade().expect("open").dialog();
    let at = centre_of(dialog, FIRST_ROW_ITEM + usize::from(slot));
    game.click(at);
    game.frame();
    let dialog = spaceport(game).open_trade().expect("open").dialog();
    let at = centre_of(dialog, button);
    game.click(at);
    game.frame();
}

/// A pilot flying in the Verge, with nothing else changed.
fn in_the_verge(pilot: &mut Pilot) {
    pilot.system = nova_sim::SystemId(nova_of(&fixture().ids.systems, "verge"));
    pilot.stellar = None;
}

/// Puts the ship over Verge Camp and lands with one L, as at an
/// ungoverned outpost.
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
    assert_eq!(
        game.showing(),
        nova::app::Showing::Spaceport,
        "landed at Verge Camp"
    );
}

#[test]
fn hearth_dock_lists_its_standard_goods_at_their_levels_then_vintage_wine_at_its_high_price() {
    let (mut game, _store) = resumed_after(on_the_dock_job);
    land_at(&mut game, "hearth-dock");
    open_exchange(&mut game);
    assert_eq!(listed(&market(&game)), expected(&HEARTH_DOCK));
}

#[test]
fn verge_camp_lists_grain_and_medicine_high_and_vintage_wine_low() {
    let (mut game, _store) = resumed_after(in_the_verge);
    land_at_verge_camp(&mut game);
    // The camp's own greeting opens at landing.
    game.press(KeyCode::Enter);
    open_exchange(&mut game);
    assert_eq!(listed(&market(&game)), expected(&VERGE_CAMP));
}

/// (c) then (d): the pilot buys at Hearth Dock, and the pilot saved
/// there, dock job and all, sells at Verge Camp.
///
/// The Vintage wine is bought at Hearth Dock too: Hearth Dock buys it
/// (`BoughtAt`), and by the engine's reading of [`RuleKey::JunkTrade`]
/// such a row trades both ways, at its high price. The Verge Camp leg
/// reuses the pilot saved at Hearth Dock, its dock job included: moved
/// into the Verge and resumed in a new app, it lands at Verge Camp, where
/// the dock job's completion, the camp's greeting and the news its
/// success releases open first.
#[test]
fn goods_bought_at_hearth_dock_fill_the_hold_and_sell_at_verge_camp_s_prices() {
    let (mut game, store) = resumed_after(on_the_dock_job);
    land_at(&mut game, "hearth-dock");
    open_exchange(&mut game);
    let (grain, wine) = (good("grain"), good("vintage-wine"));
    let price_at = |game: &Harness, good: Good| market(game).rows[row_of(game, good)].price;

    let before = pilot(&game);
    let grain_price = price_at(&game, grain);
    trade(&mut game, grain, BUY_ITEM);
    let after_grain = pilot(&game);
    assert_eq!(after_grain.cargo.get(&grain), Some(&CLICK_TONS));
    assert_eq!(
        after_grain.cash,
        before.cash - i64::from(CLICK_TONS) * grain_price
    );

    let wine_price = price_at(&game, wine);
    trade(&mut game, wine, BUY_ITEM);
    let bought = pilot(&game);
    let wine_tons = bought.cargo.get(&wine).copied().unwrap_or(0);
    assert!(wine_tons > 0, "some wine is bought: {:?}", bought.cargo);
    assert_eq!(
        bought.cash,
        after_grain.cash - i64::from(wine_tons) * wine_price
    );
    let kept = saved(&store, "Ada");
    assert_eq!(
        (kept.cash, &kept.cargo),
        (bought.cash, &bought.cargo),
        "saved"
    );

    // The saved pilot, moved into the Verge, in a new app.
    let mut moved = kept;
    in_the_verge(&mut moved);
    store
        .write("Ada", &nova_sim::save::encode(&moved))
        .expect("writes");
    let mut game = resumed_in(app_over(opened_from(&folder()), &store));
    land_at_verge_camp(&mut game);
    // The dock job's completion, the camp's greeting and the news pop.
    for _ in 0..3 {
        game.press(KeyCode::Enter);
    }
    open_exchange(&mut game);
    let camp = expected(&VERGE_CAMP);
    let camp_price = |good: Good| {
        camp.iter()
            .find(|(listed, _, _)| *listed == good)
            .map(|&(_, _, price)| price)
            .expect("Verge Camp prices it")
    };
    assert_ne!(camp_price(grain), grain_price, "another port's price");
    assert_ne!(camp_price(wine), wine_price, "another port's price");

    for (good, tons) in [(grain, CLICK_TONS), (wine, wine_tons.min(CLICK_TONS))] {
        let before = pilot(&game);
        trade(&mut game, good, SELL_ITEM);
        let after = pilot(&game);
        assert_eq!(
            after.cash,
            before.cash + i64::from(tons) * camp_price(good),
            "{good:?}"
        );
        let held = |pilot: &Pilot| pilot.cargo.get(&good).copied().unwrap_or(0);
        assert_eq!(held(&after), held(&before) - tons, "{good:?}");
    }
}

/// The fixture with a second special good, Old brandy (`jünk` 129),
/// bought at Hearth Dock beside the Vintage wine.
fn with_old_brandy() -> Rc<GameData> {
    let NewGameOpened::Loaded(loaded) = DiskData.open_new_game(&folder()) else {
        panic!("the fixture opens");
    };
    let dock = DataStellarId(nova_of(&fixture().ids.stellars, "hearth-dock"));
    let wine = (loaded.data.get::<Junk>(128))
        .expect("present")
        .expect("decoded")
        .record
        .clone();
    let mut bought_at = [None; 8];
    bought_at[0] = Some(dock);
    let brandy = Junk {
        bought_at,
        sold_at: [None; 8],
        base_price: 900,
        lc_name: "old brandy".into(),
        abbrev: "Old brandy".into(),
        ..wine
    };
    Rc::new(loaded.data.with_decoded(Entry {
        id: 129,
        name: Some("Old brandy".to_owned()),
        record: AnyRecord::Junk(Box::new(brandy)),
    }))
}

/// What Hearth Dock's exchange lists of the special goods, by name, over
/// [`with_old_brandy`] under the new game's rulebook with `junk_listing`.
fn special_goods_listed(junk_listing: RuleSource) -> Vec<String> {
    let rules = ruleset_rulebook(None, Ruleset::NewGame.profile().readings)
        .0
        .with_override(RuleKey::JunkListing, junk_listing);
    let store = MemoryPilots::new();
    let data = with_old_brandy();
    let app = Harness::wired_without_interface(Rc::clone(&data), &store, |screen| {
        screen
            .with_rulebook(&rules)
            .with_dialog_loader(nova::dialogs::loader(1))
    });
    drop(new_pilot_flying(app));
    let mut pilot = saved(&store, "Ada");
    on_the_dock_job(&mut pilot);
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");
    let app = Harness::wired_without_interface(data, &store, |screen| {
        screen
            .with_rulebook(&rules)
            .with_dialog_loader(nova::dialogs::loader(1))
    });
    let mut game = resumed_in(app);
    land_at(&mut game, "hearth-dock");
    open_exchange(&mut game);
    market(&game)
        .rows
        .iter()
        .filter(|row| matches!(row.good, Good::Junk(_)))
        .map(|row| row.name.clone())
        .collect()
}

#[test]
fn a_port_pricing_two_special_goods_high_lists_both_under_the_new_game_s_rulebook() {
    assert_eq!(
        special_goods_listed(RuleSource::Bible),
        ["Vintage wine", "Old brandy"],
        "the new game's reading"
    );
    assert_eq!(
        special_goods_listed(RuleSource::Engine),
        ["Old brandy"],
        "the original's: only the highest ID"
    );
}
