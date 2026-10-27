//! The spaceport shops' component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-11-spaceport-on-port): the commands the
//! exchange, the recharge, the outfitter, the shipyard, the bar, the
//! jettison, the assignment and the intro send, what the view hands back
//! from the flight's answers, and the bar's redraws around them. What the
//! shops trade, cost and roll is nova-sim's (`session::apply::port` and
//! the session's own tests). Kept apart from `tests/mod.rs`, which is
//! large.

use super::faked_hud::barred_by;
use super::faked_world::{in_sol_with, the_player_at};
use super::*;
use nova_sim::navigation::NavMode;
use nova_sim::status_cargo::CommodityRow;
use nova_sim::{Gauge, RechargeRefusal, TradeRefusal};

/// Ship 128's full gauges: a 40 shield, a 30 armour, 250 fuel.
pub(super) fn full() -> Reserves {
    Reserves {
        shield: Gauge::full(40.0),
        armor: Gauge::full(30.0),
        fuel: Gauge::full(250.0),
    }
}

/// [`full`] with `fuel` of fuel.
pub(super) fn fuelled(fuel: f32) -> Reserves {
    let mut reserves = full();
    reserves.fuel.now = fuel;
    reserves
}

/// The frame of a flight docked at Earth (128) in Sol, mode 2 on it,
/// the player at the centre with [`full`] gauges.
pub(super) fn at_earth() -> FlightFrame {
    let mut frame = FlightFrame {
        landed: Some(StellarId(128)),
        nav_mode: NavMode::Stellar(Some(StellarId(128))),
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    };
    frame.hud.reserves = full();
    frame
}

/// A view of `catalog` over a flight showing `frame`, the HUD's cadence
/// read as `cadence` says, with the stock status bar, and the flight
/// naming Earth (128) and Proxima (140), the second outside the scene.
pub(super) fn docked_on(catalog: FakeCatalog, frame: FlightFrame, cadence: RuleSource) -> Faked {
    let rules = Rulebook::default().with_override(RuleKey::HudCadence, cadence);
    let mut view = barred_by(catalog, frame, &rules);
    view.sim.map_mut().stellar_names = vec![
        (StellarId(128), "Earth".to_owned()),
        (StellarId(140), "Proxima".to_owned()),
    ];
    view.redraw_status();
    view
}

/// [`docked_on`] at Earth ([`at_earth`]) by the engine's cadence.
pub(super) fn docked() -> Faked {
    docked_on(catalog(), at_earth(), RuleSource::Engine)
}

/// Writes `mode` behind the port, as a landed `M` or `N` writes the nav,
/// the kept frame following it.
pub(super) fn writes_nav(view: &mut Faked, mode: NavMode) {
    view.sim.frame_mut().nav_mode = mode;
    view.refresh_frame();
}

/// Proxima (140), Alpha Centauri's first stellar, in mode 2: what a
/// landed `M131` writes.
pub(super) const PROXIMA: NavMode = NavMode::Stellar(Some(StellarId(140)));

/// The frame `view`'s flight shows now, changed by `change`: what a
/// command leaves.
pub(super) fn leaves(view: &Faked, change: impl FnOnce(&mut FlightFrame)) -> FlightFrame {
    let mut frame = view.frame_now();
    change(&mut frame);
    frame
}

// The exchange.

#[test]
fn the_exchange_is_the_flights() {
    let mut view = docked();
    assert_eq!(view.market(), None);
    view.sim.port_mut().market = Some(nova_sim::Market::default());
    assert_eq!(view.market(), Some(nova_sim::Market::default()));
}

#[test]
fn a_trade_sends_its_order_and_hands_back_the_flights_answer() {
    let mut view = docked();
    view.sim.queue_answer(Answer::Traded(Ok(10)));
    assert_eq!(view.trade(BUY_FOOD), Ok(10));
    view.sim
        .queue_answer(Answer::Traded(Err(TradeRefusal::NoSpace)));
    assert_eq!(view.trade(BUY_FOOD), Err(TradeRefusal::NoSpace));
    assert_eq!(
        view.trade(BUY_FOOD),
        Err(TradeRefusal::NoMarket),
        "refused: a flight that never started has no exchange"
    );
    assert_eq!(sent(&view), vec![FlightCommand::Trade(BUY_FOOD); 3]);
}

#[test]
fn the_port_bar_shows_the_hold_a_trade_changed() {
    let mut view = docked();
    let bar_texts = |view: &Faked| {
        let mut bar = DrawList::new();
        view.draw_status_bar(&mut bar);
        texts(&bar)
    };
    assert!(
        !bar_texts(&view).contains(&"Food:".to_owned()),
        "an empty hold"
    );
    let traded = leaves(&view, |frame| {
        frame.hud.cargo.rows[0] = Some(CommodityRow {
            label: "Food:".to_owned(),
            tons: 10,
        });
    });
    view.sim
        .queue_answer_showing(Answer::Traded(Ok(10)), traded);
    assert_eq!(view.trade(BUY_FOOD), Ok(10));
    assert!(bar_texts(&view).contains(&"Food:".to_owned()));
    view.check_frame_kept();
}

// The recharge.

#[test]
fn a_recharge_sends_its_command_and_hands_back_the_flights_answer() {
    let mut view = docked();
    view.sim.queue_answer(Answer::Recharged(Ok(150)));
    assert_eq!(view.recharge(), Ok(150));
    view.sim
        .queue_answer(Answer::Recharged(Err(RechargeRefusal::NoFuel)));
    assert_eq!(view.recharge(), Err(RechargeRefusal::NoFuel));
    assert_eq!(
        view.recharge(),
        Err(RechargeRefusal::NoFuel),
        "refused: a flight that never started sells no fuel"
    );
    assert_eq!(sent(&view), vec![FlightCommand::Recharge; 3]);
}

#[test]
fn recharge_is_enabled_as_the_flight_says() {
    let mut view = docked();
    assert!(!view.recharge_enabled());
    view.sim.port_mut().recharge_enabled = true;
    assert!(view.recharge_enabled());
}

#[test]
fn a_recharge_redraws_the_fuel_gauge_alone() {
    let mut view = docked_on(catalog(), at_earth(), RuleSource::Engine);
    view.sim.frame_mut().hud.reserves = fuelled(100.0);
    view.refresh_status();
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "held at 100"
    );
    view.sim.frame_mut().hud.reserves.shield.now = 10.0;
    view.refresh_frame();
    assert_eq!(
        view.reserves_shown(&view.frame_now()).shield.now,
        40.0,
        "held full"
    );
    let filled = leaves(&view, |frame| frame.hud.reserves.fuel.now = 250.0);
    view.sim
        .queue_answer_showing(Answer::Recharged(Ok(150)), filled);
    assert!(view.recharge().is_ok());
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        250.0,
        "redrawn at once"
    );
    assert_eq!(
        view.reserves_shown(&view.frame_now()).shield.now,
        40.0,
        "still held"
    );
    let unpaid = leaves(&view, |frame| frame.hud.reserves.fuel.now = 100.0);
    view.sim.set_frame(unpaid);
    view.refresh_frame();
    view.sim.queue_answer(Answer::Recharged(Ok(0)));
    assert_eq!(view.recharge(), Ok(0), "nothing to pay with");
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "redrawn, as `_DoPortDialog` draws whatever the fill"
    );
    view.sim.frame_mut().hud.reserves.fuel.now = 50.0;
    view.refresh_frame();
    view.sim
        .queue_answer(Answer::Recharged(Err(RechargeRefusal::NoFuel)));
    assert!(view.recharge().is_err());
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "a refused recharge draws nothing"
    );
}

/// A landed `M131` run in the shipyard, undone as it closes, its close
/// leaving the nav marked and showing Proxima, then each order: the
/// shipyard's mark drawn by an order that went through, and left by one
/// refused.
#[test]
fn the_shipyards_marked_nav_is_drawn_by_a_trade_or_recharge_that_went_through() {
    type Step = fn(&mut Faked) -> bool;
    let orders: [(&str, Answer, Step); 6] = [
        ("a trade", Answer::Traded(Ok(10)), |view| {
            view.trade(BUY_FOOD).is_ok()
        }),
        ("a recharge", Answer::Recharged(Ok(150)), |view| {
            view.recharge().is_ok()
        }),
        // `_DoPortDialog` marks and draws the status whatever the fill
        // (@0x5fdf1-0x5fe05).
        ("a full tank's recharge", Answer::Recharged(Ok(0)), |view| {
            view.recharge().is_ok()
        }),
        (
            "a refused trade",
            Answer::Traded(Err(TradeRefusal::NoSpace)),
            |view| view.trade(BUY_FOOD).is_ok(),
        ),
        (
            "a refused recharge",
            Answer::Recharged(Err(RechargeRefusal::NoFuel)),
            |view| view.recharge().is_ok(),
        ),
        ("an order the flight refused", Answer::Refused, |view| {
            view.trade(BUY_FOOD).is_ok()
        }),
    ];
    for (what, answer, order) in orders {
        let mut view = marked_by_the_shipyard();
        view.sim.queue_answer(answer);
        let through = order(&mut view);
        let shown = if through { "Earth" } else { "Proxima" };
        assert_eq!(port_nav(&view), naming(shown), "{what}");
        assert_eq!(view.nav_marked, !through, "{what}");
    }
}

/// [`docked`], the shipyard opened, a landed `M131` run inside it and
/// its close restoring Earth: the close shows Proxima, the nav marked.
pub(super) fn marked_by_the_shipyard() -> Faked {
    let mut view = docked();
    view.open_port_dialog();
    writes_nav(&mut view, PROXIMA);
    let restored = leaves(&view, |frame| {
        frame.nav_mode = NavMode::Stellar(Some(StellarId(128)));
    });
    view.sim.queue_answer_showing(Answer::Done, restored);
    view.close_port_dialog(PortDialog::Shipyard);
    assert_eq!(port_nav(&view), naming("Proxima"), "the close's");
    assert!(view.nav_marked);
    view
}
