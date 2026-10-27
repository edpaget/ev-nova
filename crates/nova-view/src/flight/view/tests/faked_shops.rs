//! The spaceport shops' component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-11-spaceport-on-port): the commands the
//! exchange, the recharge, the outfitter, the shipyard, the bar, the
//! jettison, the assignment and the intro send, what the view hands back
//! from the flight's answers, and the bar's redraws around them. What the
//! shops trade, cost and roll is nova-sim's (`session::apply::port` and
//! the session's own tests). Kept apart from `tests/mod.rs`, which is
//! large.

use super::faked_hud::barred_by;
use super::faked_travel::{faked_lasts, faked_words, sent_but_lines};
use super::faked_world::{in_sol_with, the_player_at};
use super::*;
use crate::flight::radar::RadarScene;
use crate::flight::target::TargetPanel;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::navigation::NavMode;
use nova_sim::order_loop::{OrderRun, Stopped};
use nova_sim::status_cargo::CommodityRow;
use nova_sim::{
    Gauge, InterfaceLoad, MissionText, MissionTextKind, OutfitRefusal, OutfitRun, RechargeRefusal,
    Reselection, SessionEvent, ShipNaming, ShipPurchase, ShipRefusal, SimMessage, TextBar,
    TradeRefusal,
};

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

// The bar's redraws around a shop's order.

/// Gövt 150, whose `Interface` is `ïntf` 129 in [`upgrading`]: the bar
/// over `PICT` 705.
pub(super) const UNDER_129: Option<GovtId> = Some(GovtId(150));

/// [`catalog`] where gövt 150's `Interface` is `ïntf` 129 ([`governed`]).
pub(super) fn upgrading() -> FakeCatalog {
    governed(catalog(), 130, 129)
}

/// [`docked_on`] [`upgrading`] at Earth by the engine's cadence.
pub(super) fn docked_upgrading() -> Faked {
    docked_on(upgrading(), at_earth(), RuleSource::Engine)
}

/// A `C`, `E` or `H` run docked after `texts` texts, under `government`,
/// the nav `nav` and the gauges `reserves` at its operator.
pub(super) fn load(
    government: Option<GovtId>,
    nav: NavMode,
    reserves: Reserves,
    texts: u64,
) -> FlightEvent {
    FlightEvent::InterfaceLoad(InterfaceLoad {
        government,
        nav,
        reserves,
        texts,
    })
}

/// Mission 131's `BriefText`, queued as the `texts`th, the bar noted with
/// the nav `nav` and the gauges `reserves`.
pub(super) fn brief(texts: u64, nav: NavMode, reserves: Reserves) -> FlightEvent {
    FlightEvent::MissionShown {
        shown: MissionShown::Text(MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(131),
            text: "Go.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }),
        bar: Some(TextBar {
            texts,
            nav,
            reserves,
        }),
    }
}

/// Earth (128) in mode 2.
pub(super) const EARTH: NavMode = NavMode::Stellar(Some(StellarId(128)));

/// Holds a stale snapshot of each of the status bar's parts: no gauges,
/// nav naming "Stale", a target panel for "Stale", and a radar with the
/// player off its centre.
pub(super) fn stale(view: &mut Faked) {
    view.reserves_scene = Some(Reserves::default());
    view.nav_scene = Some((
        view.nav_mode(),
        NavDisplay::Stellar(Some("Stale".to_owned())),
    ));
    view.target_scene = Some((
        None,
        Some(TargetPanel {
            name: "Stale".to_owned(),
            subtitle: String::new(),
            picture: None,
            status: nova_sim::TargetStatus::Hidden,
            tag: None,
        }),
    ));
    view.radar_scene = Some(RadarScene {
        player: at(-1.0, -1.0),
        ..RadarScene::default()
    });
    assert_eq!(retaken(view), [false; 4], "stale");
}

/// The whole bar redrawn, or the radar alone.
pub(super) const WHOLE_BAR: [bool; 4] = [true; 4];
pub(super) const RADAR_ALONE: [bool; 4] = [false, false, false, true];

/// An outfitter run of `done` passes, its button passed or not, stopped
/// by `stopped` if it was.
fn run(done: u32, passed: bool, stopped: Option<OutfitRefusal>) -> OutfitRun {
    OutfitRun {
        run: OrderRun {
            done,
            stopped: stopped.map(|refusal| Stopped {
                pass: done,
                refusal,
            }),
        },
        alert: None,
        after: Reselection::Kept,
        button_passed: passed,
    }
}

/// What a purchase bought: a ship for nothing.
pub(super) fn purchase() -> ShipPurchase {
    ShipPurchase {
        price: 0,
        trade_in: 0,
        sold_back: std::collections::BTreeMap::new(),
        refund: 0,
        left_behind: std::collections::BTreeMap::new(),
    }
}

/// The run a flight that never started gives: no outfitter, refused at
/// the first pass, its button failing.
fn refused_run() -> OutfitRun {
    run(0, false, Some(OutfitRefusal::NoOutfitter))
}

/// The outfitter's three orders of [`BUY_TANK`], each a pass that went
/// through, answered with `answer`: whether the view said it went
/// through.
type Ordered = fn(&mut Faked) -> bool;
const ORDERS: [(&str, Ordered); 3] = [
    ("outfit", |view| view.outfit(BUY_TANK).is_ok()),
    ("outfit_counted", |view| {
        view.outfit_counted(BUY_TANK, 1) == Ok(1)
    }),
    ("outfit_order", |view| {
        view.outfit_order(BUY_TANK, 1).run.done == 1
    }),
];

#[test]
fn the_outfitter_and_the_shipyard_are_the_flights() {
    let mut view = docked();
    assert_eq!(view.outfitter(), None);
    assert_eq!(view.shipyard(), None);
    assert_eq!(view.ship_naming(ShipId(129)), Err(ShipRefusal::NoShipyard));
    let naming = ShipNaming {
        ship: ShipId(129),
        prompt: "Name her.".to_owned(),
        default: "Kestrel 123".to_owned(),
    };
    let port = view.sim.port_mut();
    port.outfitter = Some(nova_sim::Outfitter::default());
    port.ship_naming = Some(Ok(naming.clone()));
    assert_eq!(view.outfitter(), Some(nova_sim::Outfitter::default()));
    assert_eq!(view.ship_naming(ShipId(129)), Ok(naming));
}

#[test]
fn opening_and_closing_the_outfitter_send_their_commands() {
    let mut view = docked();
    view.open_outfitter();
    view.close_outfitter();
    assert_eq!(
        sent(&view),
        [FlightCommand::OpenOutfitter, FlightCommand::CloseOutfitter]
    );
}

#[test]
fn an_outfitter_order_sends_its_count_and_hands_back_the_flights_run() {
    let mut view = docked();
    view.sim.queue_answer(Answer::Outfitted(run(3, true, None)));
    assert_eq!(view.outfit_order(BUY_TANK, 3), run(3, true, None));
    view.sim.queue_answer(Answer::Outfitted(run(2, true, None)));
    assert_eq!(view.outfit_counted(BUY_TANK, 2), Ok(2));
    view.sim.queue_answer(Answer::Outfitted(run(1, true, None)));
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(
        sent(&view),
        [
            FlightCommand::Outfit {
                order: BUY_TANK,
                count: 3
            },
            FlightCommand::OutfitCounted {
                order: BUY_TANK,
                count: 2
            },
            FlightCommand::OutfitCounted {
                order: BUY_TANK,
                count: 1
            },
        ]
    );
    assert_eq!(view.outfit_order(BUY_TANK, 1), refused_run(), "refused");
    assert_eq!(
        view.outfit_counted(BUY_TANK, 1),
        Err(OutfitRefusal::NoOutfitter)
    );
    assert_eq!(view.outfit(BUY_TANK), Err(OutfitRefusal::NoOutfitter));
}

#[test]
fn every_outfitter_order_redraws_the_whole_bar_on_an_intf_change_and_otherwise_the_radar() {
    for (name, order) in ORDERS {
        let mut view = docked_upgrading();
        stale(&mut view);
        view.sim.queue_event(load(UNDER_129, EARTH, full(), 0));
        view.sim.queue_answer(Answer::Outfitted(run(1, true, None)));
        assert!(order(&mut view), "{name} went through");
        assert_eq!(retaken(&view), WHOLE_BAR, "{name}: ïntf 129");
        let mut view = docked_upgrading();
        stale(&mut view);
        view.sim.queue_event(load(None, EARTH, full(), 0));
        view.sim.queue_answer(Answer::Outfitted(run(1, true, None)));
        assert!(order(&mut view), "{name} went through");
        assert_eq!(retaken(&view), RADAR_ALONE, "{name}: the ïntf kept");
        let mut view = docked_upgrading();
        stale(&mut view);
        view.sim.queue_answer(Answer::Outfitted(run(
            0,
            false,
            Some(OutfitRefusal::CannotAfford),
        )));
        assert!(!order(&mut view), "{name} refused");
        assert_eq!(retaken(&view), RADAR_ALONE, "{name} refused");
    }
}

#[test]
fn an_outfit_orders_nav_shows_as_at_its_last_intf_change() {
    for (script, at_h, shown) in [
        ("H130 M131", EARTH, "Earth"),
        ("M131 H130", PROXIMA, "Proxima"),
    ] {
        let mut view = docked_upgrading();
        view.sim.queue_event(load(UNDER_129, at_h, full(), 0));
        let written = leaves(&view, |frame| {
            frame.nav_mode = PROXIMA;
            frame.hud.interface_government = UNDER_129;
        });
        view.sim
            .queue_answer_showing(Answer::Outfitted(run(1, true, None)), written);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(port_nav(&view), naming(shown), "{script}");
        view.refresh_status();
        assert_eq!(
            port_nav(&view),
            naming("Proxima"),
            "{script}: the next nav redraw"
        );
    }
}

#[test]
fn an_outfit_order_changing_the_intf_and_back_redraws_the_bar_as_at_the_last_change() {
    let mut view = docked_upgrading();
    let at_last = fuelled(200.0);
    view.sim.queue_event(load(UNDER_129, PROXIMA, full(), 0));
    view.sim.queue_event(load(None, PROXIMA, at_last, 0));
    let tanked = leaves(&view, |frame| {
        frame.nav_mode = PROXIMA;
        frame.hud.reserves.fuel = Gauge::full(350.0);
    });
    view.sim
        .queue_answer_showing(Answer::Outfitted(run(1, true, None)), tanked);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(port_nav(&view), naming("Proxima"), "redrawn at the H");
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        at_last,
        "the gauges as at the last H, before the tank"
    );
}

#[test]
fn an_outfit_hooks_text_shows_over_the_bar_as_the_hook_left_it() {
    // `H130 G201 S131 H129 M131`: the H130's load, G201's tank, S131's
    // brief noting the bar, the H129's load after it, and the M.
    let mut view = docked_upgrading();
    let tanked = Reserves {
        fuel: Gauge::full(350.0),
        ..full()
    };
    view.sim.queue_event(load(UNDER_129, EARTH, full(), 0));
    view.sim.queue_event(brief(1, EARTH, tanked));
    view.sim.queue_event(load(None, EARTH, tanked, 1));
    let after = leaves(&view, |frame| {
        frame.nav_mode = PROXIMA;
        frame.hud.reserves = tanked;
        frame.texts_queued = 1;
    });
    view.sim
        .queue_answer_showing(Answer::Outfitted(run(1, true, None)), after);
    stale(&mut view);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.take_mission_texts().len(), 1, "S131's brief");
    view.catch_up();
    assert_eq!(bar_shown(&view), Some(705), "at the H130");
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        full(),
        "as at the H130, before G201's tank"
    );
    assert_eq!(port_nav(&view), naming("Earth"), "at the H130");
    view.close_mission_text();
    assert_eq!(bar_shown(&view), Some(700), "the H129, once closed");
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        tanked,
        "at the H129"
    );
    assert_eq!(
        port_nav(&view),
        naming("Earth"),
        "at the H129, before the M"
    );
}

/// The order's own draw (@0x5c5a1, @0x5d328) comes after its hook's
/// texts, not under them. The text's close redraws the whole bar first
/// (`_RefreshSAT` @0xa1f13), which draws the nav and so lets go of the
/// shipyard's mark, so the order's draw then draws no nav.
#[test]
fn an_outfit_orders_own_draw_waits_for_its_hooks_texts() {
    for marked in [false, true] {
        let mut view = if marked {
            marked_by_the_shipyard()
        } else {
            docked()
        };
        view.sim.queue_event(brief(1, EARTH, full()));
        let after = leaves(&view, |frame| {
            frame.nav_mode = PROXIMA;
            frame.texts_queued = 1;
        });
        view.sim
            .queue_answer_showing(Answer::Outfitted(run(1, true, None)), after);
        stale(&mut view);
        assert_eq!(view.outfit(BUY_TANK), Ok(()));
        assert_eq!(view.take_mission_texts().len(), 1, "S131's brief");
        view.catch_up();
        assert_eq!(
            retaken(&view),
            [false; 4],
            "marked {marked}: held under the text"
        );
        assert_eq!(port_nav(&view), naming("Stale"), "marked {marked}");
        assert_eq!(view.nav_marked, marked, "the mark kept");
        view.close_mission_text();
        assert_eq!(
            port_nav(&view),
            naming("Earth"),
            "marked {marked}: the text's bar, before the M"
        );
        assert!(!view.nav_marked, "marked {marked}: its redraw drew the nav");
        assert_eq!(
            view.radar_scene,
            Some(view.radar_now(&view.frame_now())),
            "marked {marked}"
        );
    }
}

#[test]
fn an_outfitter_order_redraws_the_radar_at_once_while_docked() {
    for (name, order) in ORDERS {
        let mut view = docked();
        assert!(view.hud_waits(), "the engine's reading");
        assert!(
            !view.radar_shown(&view.frame_now()).iff,
            "{name}: landing's"
        );
        let bought = leaves(&view, |frame| frame.hud.radar_gear.iff = true);
        view.sim
            .queue_answer_showing(Answer::Outfitted(run(1, true, None)), bought);
        assert!(order(&mut view), "{name}");
        assert!(view.radar_shown(&view.frame_now()).iff, "{name}: bought");
        let sold = leaves(&view, |frame| frame.hud.radar_gear.iff = false);
        view.sim
            .queue_answer_showing(Answer::Outfitted(run(1, true, None)), sold);
        assert!(order(&mut view), "{name}");
        assert!(!view.radar_shown(&view.frame_now()).iff, "{name}: sold");
    }
}

/// [`docked`], the shipyard (or the hire screen, `hire`) opened, a landed
/// `M131` run inside it and its close restoring Earth: the close shows
/// Proxima, the nav marked.
pub(super) fn marked_in(hire: bool) -> Faked {
    let mut view = docked();
    view.open_port_dialog();
    writes_nav(&mut view, PROXIMA);
    let restored = leaves(&view, |frame| frame.nav_mode = EARTH);
    view.sim.queue_answer_showing(Answer::Done, restored);
    let dialog = if hire {
        PortDialog::Hire
    } else {
        PortDialog::Shipyard
    };
    view.close_port_dialog(dialog);
    assert_eq!(
        port_nav(&view),
        naming("Proxima"),
        "{dialog:?}: the close's"
    );
    assert!(view.nav_marked);
    view
}

/// [`marked_in`] the shipyard.
pub(super) fn marked_by_the_shipyard() -> Faked {
    marked_in(false)
}

/// A port order, run on the view after its answer is queued.
type PortStep = fn(&mut Faked);

/// The orders the port's `_DrawStatus` follows, each with the answer it
/// goes through on: each draws the nav the shipyard's restore marked.
fn drawing_orders() -> [(&'static str, Answer, PortStep); 11] {
    [
        ("a trade", Answer::Traded(Ok(10)), |view| {
            assert!(view.trade(BUY_FOOD).is_ok());
        }),
        ("a recharge", Answer::Recharged(Ok(150)), |view| {
            assert!(view.recharge().is_ok());
        }),
        // `_DoPortDialog` marks and draws the status whatever the fill
        // (@0x5fdf1-0x5fe05).
        ("a full tank's recharge", Answer::Recharged(Ok(0)), |view| {
            assert_eq!(view.recharge(), Ok(0));
        }),
        (
            "an outfitter order",
            Answer::Outfitted(run(1, true, None)),
            |view| {
                assert!(view.outfit(BUY_TANK).is_ok());
            },
        ),
        (
            "a counted outfitter order",
            Answer::Outfitted(run(1, true, None)),
            |view| {
                assert_eq!(view.outfit_counted(BUY_TANK, 1), Ok(1));
            },
        ),
        (
            "an outfitter run",
            Answer::Outfitted(run(1, true, None)),
            |view| {
                view.outfit_order(BUY_TANK, 1);
            },
        ),
        (
            "an outfitter run stopped after its first pass",
            Answer::Outfitted(run(1, true, Some(OutfitRefusal::CannotAfford))),
            |view| {
                view.outfit_order(BUY_TANK, 10_000);
            },
        ),
        (
            "a counted order's Cancel",
            Answer::Outfitted(run(0, true, None)),
            |view| {
                assert_eq!(view.outfit_counted(BUY_TANK, 0), Ok(0));
            },
        ),
        (
            "an outfitter run's Cancel",
            Answer::Outfitted(run(0, true, None)),
            |view| {
                view.outfit_order(BUY_TANK, 0);
            },
        ),
        (
            "a ship purchase",
            Answer::ShipBought(Ok(purchase())),
            |view| {
                view.buy_ship(ShipId(129), "Kestrel").expect("bought");
            },
        ),
        ("a shop closing", Answer::Done, FlightView::refresh_status),
    ]
}

/// The orders refused, each with its answer: none draws the nav the
/// shipyard's restore marked.
fn refused_orders() -> [(&'static str, Answer, PortStep); 5] {
    [
        (
            "a refused trade",
            Answer::Traded(Err(TradeRefusal::OutOfRange)),
            |view| {
                assert!(view.trade(BUY_FOOD).is_err());
            },
        ),
        (
            "a refused outfitter order",
            Answer::Outfitted(refused_run()),
            |view| {
                assert!(view.outfit(BUY_TANK).is_err());
            },
        ),
        (
            "a refused counted order",
            Answer::Outfitted(refused_run()),
            |view| {
                assert!(view.outfit_counted(BUY_TANK, 1).is_err());
            },
        ),
        (
            "a refused outfitter run",
            Answer::Outfitted(refused_run()),
            |view| {
                view.outfit_order(BUY_TANK, 1);
            },
        ),
        (
            "a Cancel whose button fails its check",
            Answer::Outfitted(run(0, false, None)),
            |view| {
                view.outfit_order(BUY_TANK, 0);
            },
        ),
    ]
}

#[test]
fn a_landed_m_in_the_shipyard_is_undone_but_its_close_shows_it_until_the_next_draw_status() {
    let (draws, refused) = (drawing_orders(), refused_orders());
    for hire in [false, true] {
        for (steps, shown) in [(&draws[..], "Earth"), (&refused[..], "Proxima")] {
            for (what, answer, step) in steps {
                let mut view = marked_in(hire);
                view.sim.queue_answer(answer.clone());
                step(&mut view);
                assert_eq!(port_nav(&view), naming(shown), "hire {hire}, {what}");
            }
        }
    }
}

// The shipyard.

#[test]
fn the_name_prompts_cancel_redraws_the_whole_bar_and_sends_the_decline() {
    let mut view = docked();
    stale(&mut view);
    view.decline_ship(ShipId(129));
    assert_eq!(retaken(&view), WHOLE_BAR, "`_RefreshSAT` @0x5eb73");
    assert_eq!(sent(&view), [FlightCommand::DeclineShip(ShipId(129))]);
}

#[test]
fn a_purchase_sends_its_class_and_name_and_reads_the_new_hulls_sheet_once() {
    let mut view = docked_on(shipbuying(), at_earth(), RuleSource::Engine);
    let bought = purchase();
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::ShipChanged));
    let new_hull = leaves(&view, |frame| {
        frame.ships = vec![record::player(129, Vec2::ZERO, 0.0)];
    });
    view.sim
        .queue_answer_showing(Answer::ShipBought(Ok(bought.clone())), new_hull);
    assert_eq!(view.buy_ship(ShipId(129), "Kestrel"), Ok(bought));
    assert_eq!(
        sent(&view),
        [FlightCommand::BuyShip {
            ship: ShipId(129),
            name: "Kestrel".to_owned()
        }]
    );
    assert_eq!(view.sheet_ship, Some(ShipId(129)), "the new hull's sheet");
    view.sim
        .queue_answer(Answer::ShipBought(Err(ShipRefusal::CannotAfford)));
    assert_eq!(
        view.buy_ship(ShipId(129), "Kestrel"),
        Err(ShipRefusal::CannotAfford)
    );
    assert_eq!(
        view.buy_ship(ShipId(129), "Kestrel"),
        Err(ShipRefusal::NoShipyard),
        "refused: a flight that never started has no shipyard"
    );
}

#[test]
fn a_purchase_redraws_the_whole_bar_before_it_buys() {
    let mut view = docked();
    view.sim.frame_mut().hud.reserves = fuelled(100.0);
    view.refresh_frame();
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        250.0,
        "held"
    );
    let bought = leaves(&view, |frame| frame.hud.reserves.fuel = Gauge::full(350.0));
    view.sim
        .queue_answer_showing(Answer::ShipBought(Ok(purchase())), bought);
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    let shown = view.reserves_shown(&view.frame_now()).fuel;
    assert_eq!(
        (shown.now, shown.max),
        (100.0, 250.0),
        "the prompt's close, before the purchase"
    );
}

#[test]
fn a_ship_bought_under_the_same_bar_keeps_the_gauges_until_the_shipyard_closes() {
    let mut view = docked();
    let bought = leaves(&view, |frame| frame.hud.reserves.fuel = Gauge::full(350.0));
    view.sim
        .queue_answer_showing(Answer::ShipBought(Ok(purchase())), bought);
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.max,
        250.0,
        "the old ship's"
    );
    view.refresh_status();
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.max,
        350.0,
        "the shipyard closed"
    );
}

#[test]
fn a_ship_bought_under_another_bar_shows_it_and_its_gauges_at_once() {
    let mut view = docked_upgrading();
    let bought = leaves(&view, |frame| {
        frame.hud.reserves.fuel = Gauge::full(350.0);
        frame.hud.interface_government = UNDER_129;
    });
    view.sim
        .queue_answer_showing(Answer::ShipBought(Ok(purchase())), bought);
    view.buy_ship(ShipId(129), "Kestrel").expect("bought");
    assert_eq!(bar_shown(&view), Some(705), "ïntf 129");
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.max,
        350.0,
        "at once"
    );
}

#[test]
fn a_ship_bought_redraws_the_whole_bar_on_an_intf_change_and_all_but_the_gauges_otherwise() {
    for (government, redrawn) in [(UNDER_129, WHOLE_BAR), (None, [false, true, true, true])] {
        let mut view = docked_upgrading();
        stale(&mut view);
        let bought = leaves(&view, |frame| {
            frame.hud.interface_government = government;
            frame.hud.reserves.fuel = Gauge::full(350.0);
        });
        view.sim
            .queue_answer_showing(Answer::ShipBought(Ok(purchase())), bought);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(retaken(&view), redrawn, "{government:?}");
    }
}

#[test]
fn a_ship_purchases_on_purchase_intf_change_shows_the_gauges_before_the_fill() {
    for (script, nav, shown) in [("H130", EARTH, "Earth"), ("H130 M131", PROXIMA, "Proxima")] {
        let mut view = docked_upgrading();
        let unfilled = Reserves {
            fuel: Gauge {
                now: 100.0,
                max: 350.0,
            },
            ..full()
        };
        view.sim.queue_event(load(UNDER_129, EARTH, unfilled, 0));
        let filled = leaves(&view, |frame| {
            frame.nav_mode = nav;
            frame.hud.interface_government = UNDER_129;
            frame.hud.reserves.fuel = Gauge::full(350.0);
        });
        view.sim
            .queue_answer_showing(Answer::ShipBought(Ok(purchase())), filled);
        view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            unfilled,
            "{script}: ship 130's, under its ïntf, not yet filled"
        );
        assert_eq!(
            port_nav(&view),
            naming(shown),
            "{script}: the purchase's own nav draw (@0x5f0b1)"
        );
    }
}

#[test]
fn a_ship_hooks_text_shows_before_the_purchases_own_redraw() {
    let mut view = docked_upgrading();
    let before = fuelled(1.0);
    view.sim.frame_mut().hud.reserves = before;
    view.refresh_status();
    view.sim.queue_event(brief(1, EARTH, before));
    let filled = leaves(&view, |frame| {
        frame.hud.interface_government = UNDER_129;
        frame.hud.reserves.fuel = Gauge::full(350.0);
        frame.texts_queued = 1;
    });
    view.sim
        .queue_answer_showing(Answer::ShipBought(Ok(purchase())), filled);
    view.buy_ship(ShipId(130), "Kestrel").expect("bought");
    assert_eq!(view.take_mission_texts().len(), 1, "S131's brief");
    view.catch_up();
    assert_eq!(bar_shown(&view), Some(700), "the old graphic");
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        before,
        "before the purchase"
    );
    view.close_mission_text();
    assert_eq!(bar_shown(&view), Some(705), "the new ship's");
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "filled, once closed"
    );
}

#[test]
fn a_redraw_clears_the_shipyards_nav_mark() {
    type Step = fn(&mut Faked);
    let redraws: [(&str, Step); 2] = [
        ("a whole-bar redraw", FlightView::refresh_status),
        ("a ship purchase", |view| {
            view.sim.queue_answer(Answer::ShipBought(Ok(purchase())));
            view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        }),
    ];
    let draws: [(&str, Answer, Step); 3] = [
        ("a trade", Answer::Traded(Ok(10)), |view| {
            assert!(view.trade(BUY_FOOD).is_ok());
        }),
        ("a recharge", Answer::Recharged(Ok(150)), |view| {
            assert!(view.recharge().is_ok());
        }),
        (
            "an outfitter order",
            Answer::Outfitted(run(1, true, None)),
            |view| {
                assert_eq!(view.outfit(BUY_TANK), Ok(()));
            },
        ),
    ];
    for (redrawn, redraw) in &redraws {
        for (drawn, answer, draw) in &draws {
            let mut view = marked_by_the_shipyard();
            redraw(&mut view);
            assert_eq!(port_nav(&view), naming("Earth"), "{redrawn}");
            writes_nav(&mut view, PROXIMA);
            assert_eq!(port_nav(&view), naming("Earth"), "{redrawn}: held");
            view.sim.queue_answer(answer.clone());
            draw(&mut view);
            assert_eq!(
                port_nav(&view),
                naming("Earth"),
                "{redrawn}, then {drawn}: the mark was cleared"
            );
        }
    }
}

// The bar.

#[test]
fn opening_the_bar_sends_its_command_and_says_whether_it_opened() {
    let mut view = docked();
    view.sim.queue_answer(Answer::Done);
    assert!(view.open_bar());
    assert!(!view.open_bar(), "refused: no bar");
    assert_eq!(
        sent(&view),
        [FlightCommand::OpenBar, FlightCommand::OpenBar]
    );
}

#[test]
fn the_bars_news_its_picture_and_its_hire_list_are_the_flights() {
    let mut view = docked();
    assert_eq!(view.news(), None);
    assert_eq!(view.news_picture(), None);
    assert_eq!(view.escorts_for_hire(), None);
    let news = nova_sim::News {
        commercial: "C2".to_owned(),
        story: "G2".to_owned(),
    };
    let list = nova_sim::HireList {
        rows: Vec::new(),
        cash: 1000,
        room: true,
        rules: Rulebook::default(),
    };
    let port = view.sim.port_mut();
    port.news = Some(news.clone());
    port.news_picture = Some(9000);
    port.hire_list = Some(list.clone());
    assert_eq!(view.news(), Some(&news));
    assert_eq!(view.news_picture(), Some(9000));
    assert_eq!(view.escorts_for_hire(), Some(list));
}

#[test]
fn a_hire_sends_its_class_and_hands_back_the_flights_answer() {
    let mut view = docked();
    let hired = nova_sim::Hired {
        ship: ShipId(129),
        fee: 90,
        wage: 9,
        joined: true,
    };
    view.sim.queue_answer(Answer::Hired(Ok(hired)));
    assert_eq!(view.hire(ShipId(129)), Ok(hired));
    view.sim
        .queue_answer(Answer::Hired(Err(nova_sim::HireRefusal::NotForHire)));
    assert_eq!(
        view.hire(ShipId(129)),
        Err(nova_sim::HireRefusal::NotForHire)
    );
    assert_eq!(
        view.hire(ShipId(129)),
        Err(nova_sim::HireRefusal::NoBar),
        "refused: a flight that never started has no bar"
    );
    assert_eq!(sent(&view), vec![FlightCommand::Hire(ShipId(129)); 3]);
}

#[test]
fn closing_the_hire_screen_or_the_outfitter_sends_its_close() {
    let mut view = docked();
    view.close_hire();
    view.close_outfitter();
    assert_eq!(
        sent(&view),
        [FlightCommand::CloseHire, FlightCommand::CloseOutfitter]
    );
}

// The jettison.

/// Holds Command (and Option, when `option`) as the jettison key is
/// pressed, then lets go of them.
fn jettison_key(view: &mut Faked, option: bool) {
    view.held.insert(Key::Command);
    if option {
        view.held.insert(Key::Alt);
    }
    view.jettison(faked_words());
    view.held.clear();
}

#[test]
fn command_k_jettisons_everything_and_says_what_the_flight_said() {
    let mut view = faked(in_sol());
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::MissionFailed));
    view.sim.queue_answer(Answer::Done);
    jettison_key(&mut view, false);
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Jettison { all: true }]
    );
    faked_lasts(
        &mut view,
        &format!("{:?}", SimMessage::MissionFailed),
        SimMessage::MissionFailed.ticks(),
    );
    assert_eq!(JETTISON_KEY, Key::Char('k'));
    assert_eq!(MISSION_FAILED, "Mission failed.");
    assert!(HELP.contains("Cmd-K: jettison"), "{HELP}");
}

#[test]
fn command_option_k_keeps_the_mission_cargo() {
    let mut view = faked(in_sol());
    view.sim.queue_answer(Answer::Done);
    jettison_key(&mut view, true);
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Jettison { all: false }]
    );
    assert_eq!(view.message(), None, "nothing said");
    assert_eq!(
        NON_MISSION_CARGO_JETTISONED,
        "Non-mission cargo jettisoned."
    );
}

#[test]
fn k_without_command_jettisons_nothing() {
    let mut view = faked(in_sol());
    view.jettison(faked_words());
    view.held.insert(Key::Alt);
    view.jettison(faked_words());
    assert_eq!(sent(&view), [], "K alone, then Option-K");
}

#[test]
fn the_player_infos_jettison_sends_everything_and_says_what_the_flight_said() {
    let mut view = faked(in_sol());
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::CargoJettisoned));
    view.sim.queue_answer(Answer::Done);
    view.jettison_all_with(faked_words());
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Jettison { all: true }]
    );
    assert_eq!(
        view.message(),
        Some(format!("{:?}", SimMessage::CargoJettisoned).as_str())
    );
    assert_eq!(CARGO_JETTISONED, "Cargo jettisoned.");
}

#[test]
fn a_jettisons_boxes_are_drawn_back_along_their_drift_under_the_ship_and_it_sounds() {
    let drifting = |at: Vec2| nova_sim::flight_frame::FrameBox {
        velocity: Vec2::new(2.0, -3.0),
        ..super::faked_world::box_at(at)
    };
    let mut view =
        super::faked_world::world(trading(), in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]));
    view.sim
        .queue_event(FlightEvent::Sound(nova_sim::SimSound::CargoJettisoned));
    let dropped = leaves(&view, |frame| {
        frame.cargo_boxes = vec![
            drifting(Vec2::new(-20.0, 20.0)),
            drifting(Vec2::new(-25.0, 18.0)),
        ];
    });
    view.sim.queue_answer_showing(Answer::Done, dropped);
    view.held.insert(Key::Command);
    view.jettison(faked_words());
    assert_eq!(
        view.drained_sounds(),
        [Sound::Sim(nova_sim::SimSound::CargoJettisoned)],
        "snd 154"
    );
    // A quarter of the way from the last step to this one.
    view.alpha = 0.25;
    let camera = view.camera();
    let expected: Vec<_> = view
        .frame
        .cargo_boxes
        .iter()
        .map(|made| {
            (
                ImageKey::sprite(500, made.frame),
                camera.world_to_screen(Point::new(
                    made.velocity.x.mul_add(-0.75, made.position.x),
                    made.velocity.y.mul_add(-0.75, made.position.y),
                )),
            )
        })
        .collect();
    let list = super::faked_world::world_list(&view);
    let boxes: Vec<_> = sprites(&list)
        .into_iter()
        .filter(|(image, _)| image.id == 500)
        .collect();
    assert_eq!(boxes, expected);
    let first_box = first(&list, sprite_of(500));
    let ship = first(&list, sprite_of(2000));
    assert!(first_box < ship, "under the ship");
}

#[test]
fn an_asteroid_is_drawn_under_the_cargo_boxes() {
    let frame = FlightFrame {
        asteroids: vec![super::faked_world::asteroid_at(Vec2::new(40.0, 40.0))],
        cargo_boxes: vec![super::faked_world::box_at(Vec2::new(-20.0, 20.0))],
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    };
    let view = super::faked_world::world(super::missions::rocky(trading()), frame);
    let list = super::faked_world::world_list(&view);
    let asteroid = first(&list, sprite_of(1800));
    let cargo_box = first(&list, sprite_of(500));
    assert!(asteroid < cargo_box, "{asteroid} {cargo_box}");
}

// The capture's assignment.

#[test]
fn use_as_escort_sends_its_choice_and_says_so_the_old_ship_flying_on() {
    let mut view = super::faked_world::world(
        boardable(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    view.alpha = 0.5;
    view.sim.queue_answer(Answer::Assigned(Assigned::Escort));
    assert_eq!(view.assign(Assignment::Escort), Some(Assigned::Escort));
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Assign(Assignment::Escort)]
    );
    assert_eq!(view.alpha(), 0.5, "the old ship flies on");
    faked_lasts(&mut view, ASSIGNED_ESCORT, 240);
}

#[test]
fn use_as_my_ship_draws_the_captured_ship_where_it_is_reading_its_sheet_once() {
    let mut view = super::faked_world::world(
        boardable(),
        in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]),
    );
    view.alpha = 0.5;
    // Where the old ship was drawn from, a step ago.
    view.previous = nova_sim::flight_frame::ShipPlace {
        position: Vec2::new(-40.0, 30.0),
        heading: 90.0,
    };
    assert_eq!(
        view.frame.player().class,
        ShipId(128),
        "the old ship's frame"
    );
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::ShipChanged));
    let taken = leaves(&view, |frame| {
        frame.ships[0] = record::player(129, Vec2::new(60.0, -10.0), 45.0);
    });
    view.sim
        .queue_answer_showing(Answer::Assigned(Assigned::MyShip), taken);
    assert_eq!(view.assign(Assignment::MyShip), Some(Assigned::MyShip));
    assert_eq!(
        view.previous,
        nova_sim::flight_frame::ShipPlace {
            position: Vec2::new(60.0, -10.0),
            heading: 45.0,
        },
        "not from the old place"
    );
    assert_eq!(view.alpha(), 0.0, "drawn where it is");
    assert_eq!(view.shown_position(), at(60.0, -10.0));
    assert_eq!(
        view.frame.player().class,
        ShipId(129),
        "the new ship's frame"
    );
    assert_eq!(view.sheet_ship, Some(ShipId(129)), "its sheet read");
    assert_eq!(
        view.assign(Assignment::MyShip),
        None,
        "refused: nothing awaits"
    );
    faked_lasts(&mut view, RETAINED_OLD_SHIP, OLD_SHIP_TICKS);
}

// The intro.

#[test]
fn the_intro_seen_is_the_flights_and_its_mark_is_sent() {
    let mut view = faked(in_sol());
    assert!(!view.seen_intro());
    view.sim.port_mut().seen_intro = true;
    assert!(view.seen_intro());
    view.mark_intro_seen();
    assert_eq!(sent(&view), [FlightCommand::MarkIntroSeen]);
}
