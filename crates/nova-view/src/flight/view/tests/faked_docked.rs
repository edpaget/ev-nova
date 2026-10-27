//! The docked bar's component tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-11-spaceport-on-port): the nav area,
//! the gauges and the status bar's graphic while docked, the texts a
//! landing's settling stages, the bar's map in port, and the take-off's
//! held nav, each over a scripted frame, answers and the loads and texts
//! the flight drains. What the scripts write and queue is nova-sim's.
//! Kept apart from `tests/mod.rs`, which is large.

use super::faked_shops::{
    EARTH, PROXIMA, UNDER_129, WHOLE_BAR, at_earth, brief, docked, docked_on, docked_upgrading,
    fuelled, full, leaves, load, marked_by_the_shipyard, upgrading, writes_nav,
};
use super::faked_world::{in_sol_with, the_player_at};
use super::*;
use crate::system::SystemScene;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::navigation::NavMode;
use nova_sim::{Gauge, LandOutcome, LandPress, MapClicked, SavedNav, SessionEvent, Settled};

/// Barnard (132), with no stellar: what a landed `N131` or `M132` leaves
/// in mode 2.
const NOWHERE: NavMode = NavMode::Stellar(Some(StellarId(132)));

// The nav area while docked.

#[test]
fn by_the_bible_the_port_names_a_landed_ms_stellar_outside_the_scene_at_once() {
    let mut view = docked_on(catalog(), at_earth(), RuleSource::Bible);
    writes_nav(&mut view, PROXIMA);
    assert_eq!(port_nav(&view), naming("Proxima"), "the flight's name");
    writes_nav(&mut view, NOWHERE);
    assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
}

#[test]
fn by_the_engine_a_landed_ms_stellar_shows_at_the_ports_next_nav_redraw() {
    type Step = fn(&mut Faked);
    let redraws: [(&str, Step); 6] = [
        ("a shop closing", FlightView::refresh_status),
        ("a name prompt's Cancel", |view| {
            view.decline_ship(ShipId(129));
        }),
        ("a refused purchase, after its prompt's close", |view| {
            view.sim
                .queue_answer(Answer::ShipBought(Err(nova_sim::ShipRefusal::NotListed)));
            assert!(view.buy_ship(ShipId(999), "Kestrel").is_err());
        }),
        ("a mission text closing", |view| {
            view.sim.queue_event(brief(1, PROXIMA, full()));
            assert_eq!(view.take_mission_texts().len(), 1);
            assert_eq!(port_nav(view), naming("Earth"), "shown over the bar");
            view.close_mission_text();
        }),
        ("a ship purchase", |view| {
            view.sim
                .queue_answer(Answer::ShipBought(Ok(super::faked_shops::purchase())));
            view.buy_ship(ShipId(129), "Kestrel").expect("bought");
        }),
        ("an outfitter order changing the ïntf", |view| {
            view.sim.queue_event(load(UNDER_129, PROXIMA, full(), 0));
            view.sim.queue_answer(Answer::Outfitted(passed_run()));
            assert_eq!(view.outfit(BUY_TANK), Ok(()));
        }),
    ];
    let held: [(&str, Step); 3] = [
        ("a trade", |view| {
            view.sim.queue_answer(Answer::Traded(Ok(10)));
            assert_eq!(view.trade(BUY_FOOD), Ok(10));
        }),
        ("a recharge", |view| {
            view.sim.queue_answer(Answer::Recharged(Ok(150)));
            assert!(view.recharge().is_ok());
        }),
        ("an outfitter order keeping the ïntf", |view| {
            view.sim.queue_answer(Answer::Outfitted(passed_run()));
            assert_eq!(view.outfit(BUY_TANK), Ok(()));
        }),
    ];
    for (steps, shown) in [(&redraws[..], "Proxima"), (&held[..], "Earth")] {
        for (what, step) in steps {
            let mut view = docked_upgrading();
            writes_nav(&mut view, PROXIMA);
            assert_eq!(port_nav(&view), naming("Earth"), "{what}: held");
            step(&mut view);
            assert_eq!(port_nav(&view), naming(shown), "{what}");
        }
    }
}

/// An outfitter run of one pass, its button passed.
fn passed_run() -> nova_sim::OutfitRun {
    nova_sim::OutfitRun {
        run: nova_sim::order_loop::OrderRun {
            done: 1,
            stopped: None,
        },
        alert: None,
        after: nova_sim::Reselection::Kept,
        button_passed: true,
    }
}

#[test]
fn by_the_engine_a_landed_n_shows_no_destination_at_the_next_redraw() {
    let mut view = docked();
    writes_nav(&mut view, NOWHERE);
    assert_eq!(port_nav(&view), naming("Earth"), "held");
    view.refresh_status();
    assert_eq!(port_nav(&view), naming(hud::NAV_NO_DESTINATION));
}

/// [`docked`], `dialog` opened, a landed `M131` run inside it, and the
/// dialog closed, its restore putting Earth back.
fn m_inside(dialog: PortDialog) -> Faked {
    let mut view = docked();
    view.open_port_dialog();
    writes_nav(&mut view, PROXIMA);
    let restored = leaves(&view, |frame| frame.nav_mode = EARTH);
    view.sim.queue_answer_showing(Answer::Done, restored);
    view.close_port_dialog(dialog);
    view
}

#[test]
fn a_landed_m_in_the_outfitter_exchange_or_computer_is_undone_and_shown_at_once() {
    for dialog in [
        PortDialog::Outfitter,
        PortDialog::Exchange,
        PortDialog::MissionComputer,
    ] {
        let view = m_inside(dialog);
        assert_eq!(
            sent(&view),
            [FlightCommand::SavePortNav, FlightCommand::RestorePortNav],
            "{dialog:?}"
        );
        assert_eq!(port_nav(&view), naming("Earth"), "{dialog:?}");
        assert!(!view.nav_marked, "{dialog:?}");
    }
}

#[test]
fn an_empty_outfitter_draws_a_nav_the_shipyard_left_marked() {
    let mut view = marked_by_the_shipyard();
    view.empty_outfitter_returned();
    assert_eq!(port_nav(&view), naming("Earth"), "the restore drawn");
    writes_nav(&mut view, PROXIMA);
    view.empty_outfitter_returned();
    assert_eq!(port_nav(&view), naming("Earth"), "nothing marked");
}

#[test]
fn the_mission_computers_close_redraws_the_whole_bar() {
    let mut view = docked();
    writes_nav(&mut view, PROXIMA);
    view.sim.frame_mut().hud.reserves = fuelled(100.0);
    view.refresh_frame();
    assert_eq!(port_nav(&view), naming("Earth"), "held");
    assert_ne!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "held"
    );
    view.open_port_dialog();
    view.sim.queue_answer(Answer::Done);
    view.close_port_dialog(PortDialog::MissionComputer);
    assert_eq!(
        port_nav(&view),
        naming("Proxima"),
        "redrawn, though the restore changed nothing"
    );
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "redrawn"
    );
}

// A landing's texts and the bar under them.

/// [`docked_upgrading`] as a landing leaves it, its settling about to run
/// with its texts and loads queued as `queue` queues them, and leaving
/// `after`'s changes.
fn landing_settles(queue: impl FnOnce(&mut Faked), after: impl FnOnce(&mut FlightFrame)) -> Faked {
    let mut view = docked_upgrading();
    queue(&mut view);
    let settled = leaves(&view, after);
    view.sim
        .queue_answer_showing(Answer::Settled(Settled::default()), settled);
    view.settle_script();
    view
}

#[test]
fn a_comp_text_shows_the_landed_stellar_and_the_next_redraw_its_on_successs_move() {
    // `CompText`, then `OnSuccess`'s `M131`.
    let mut view = landing_settles(
        |view| view.sim.queue_event(brief(1, EARTH, full())),
        |frame| {
            frame.nav_mode = PROXIMA;
            frame.texts_queued = 1;
        },
    );
    assert_eq!(view.take_mission_texts().len(), 1);
    assert_eq!(port_nav(&view), naming("Earth"), "as the text noted it");
    view.refresh_status();
    assert_eq!(port_nav(&view), naming("Proxima"));
}

#[test]
fn a_comp_text_shows_the_gauges_from_before_on_success() {
    // `OnSuccess` of `G201 M131` or `H130 G201 M131`, after the text.
    for at_h in [false, true] {
        let tanked = Reserves {
            fuel: Gauge::full(350.0),
            ..full()
        };
        let mut view = landing_settles(
            |view| {
                view.sim.queue_event(brief(1, EARTH, full()));
                if at_h {
                    view.sim
                        .queue_event(load(UNDER_129, EARTH, fuelled(200.0), 1));
                }
            },
            |frame| {
                frame.nav_mode = PROXIMA;
                frame.hud.reserves = tanked;
                frame.texts_queued = 1;
                if at_h {
                    frame.hud.interface_government = UNDER_129;
                }
            },
        );
        assert_eq!(view.take_mission_texts().len(), 1, "at the H: {at_h}");
        assert_eq!(
            view.reserves_shown(&view.frame_now()),
            full(),
            "at the H: {at_h}: the landing's"
        );
        assert_eq!(port_nav(&view), naming("Earth"), "at the H: {at_h}");
        view.close_mission_text();
        let shown = view.reserves_shown(&view.frame_now());
        if at_h {
            assert_eq!(shown, fuelled(200.0), "as at the H");
        } else {
            assert_eq!(shown, full(), "as at the text");
        }
        assert_eq!(
            port_nav(&view),
            naming("Earth"),
            "at the H: {at_h}: before the M"
        );
        view.refresh_status();
        assert_eq!(
            retaken(&view),
            WHOLE_BAR,
            "at the H: {at_h}: the next redraw"
        );
        assert_eq!(port_nav(&view), naming("Proxima"), "at the H: {at_h}");
    }
}

#[test]
fn a_fail_text_shows_over_the_bar_as_on_failure_left_it() {
    // `OnFailure` of `H130 G201 M131` or `H129 G201 M131`, before its
    // `FailText`.
    for changes in [true, false] {
        let at_h = fuelled(200.0);
        let tanked = Reserves {
            fuel: Gauge::full(350.0),
            ..full()
        };
        let government = if changes { UNDER_129 } else { None };
        let mut view = landing_settles(
            |view| {
                view.sim.queue_event(load(government, EARTH, at_h, 0));
                view.sim.queue_event(brief(1, PROXIMA, tanked));
            },
            |frame| {
                frame.nav_mode = PROXIMA;
                frame.hud.reserves = tanked;
                frame.hud.interface_government = government;
                frame.texts_queued = 1;
            },
        );
        assert_eq!(view.take_mission_texts().len(), 1, "changes {changes}");
        let shown = view.reserves_shown(&view.frame_now());
        if changes {
            assert_eq!(shown, at_h, "as at the H");
        } else {
            assert_eq!(shown, full(), "the landing's");
        }
        assert_eq!(port_nav(&view), naming("Earth"), "changes {changes}");
        view.close_mission_text();
        assert_eq!(
            retaken(&view),
            WHOLE_BAR,
            "changes {changes}: as the text noted it"
        );
        assert_eq!(port_nav(&view), naming("Proxima"), "changes {changes}");
    }
}

#[test]
fn a_landings_two_texts_each_close_to_their_own_bar_and_the_loads_before_the_next() {
    // A's `CompText`, A's `OnSuccess` `H130 G201`, B's `CompText` after
    // it, then B's `OnSuccess` `M131 H129`.
    let at_a = Reserves {
        fuel: Gauge::full(350.0),
        ..full()
    };
    let at_b = Reserves {
        fuel: Gauge::full(450.0),
        ..full()
    };
    let mut view = landing_settles(
        |view| {
            view.sim.queue_event(brief(1, EARTH, full()));
            view.sim.queue_event(load(UNDER_129, EARTH, at_a, 1));
            view.sim.queue_event(brief(2, EARTH, at_b));
            view.sim.queue_event(load(None, PROXIMA, at_b, 2));
        },
        |frame| {
            frame.nav_mode = PROXIMA;
            frame.hud.reserves = at_b;
            frame.texts_queued = 2;
        },
    );
    assert_eq!(
        view.take_mission_texts().len(),
        2,
        "A's CompText, then B's after A's OnSuccess"
    );
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        full(),
        "the landing's"
    );
    assert_eq!(port_nav(&view), naming("Earth"));
    view.close_mission_text();
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        at_a,
        "A's text as it noted the bar, then A's H130 as at its operator"
    );
    assert_eq!(
        port_nav(&view),
        naming("Earth"),
        "B's M and H come after B's text, and no dialog redraws"
    );
    view.close_mission_text();
    assert_eq!(
        retaken(&view),
        WHOLE_BAR,
        "B's text, then B's H129 after its M"
    );
    assert_eq!(port_nav(&view), naming("Proxima"));
    assert!(view.bar_steps.is_empty());
}

/// A text queued before an `H` onto ship 130 (under `ïntf` 129, whose
/// background is `PICT` 705): a landing's `CompText` before its
/// `OnSuccess` (@0xa048c → @0xa04f8), and a refusal's `RefuseText`
/// before its `OnRefuse` (@0xa2f04 → @0xa2f41). Each gives the view,
/// docked with the text taken and a frame's catch-up made.
fn texts_before_an_h(cadence: RuleSource) -> Vec<(&'static str, Faked)> {
    let queue = |view: &mut Faked| {
        view.sim.queue_event(brief(1, EARTH, full()));
        view.sim.queue_event(load(UNDER_129, EARTH, full(), 1));
    };
    let after = |view: &Faked| {
        leaves(view, |frame| {
            frame.hud.interface_government = UNDER_129;
            frame.texts_queued = 1;
        })
    };
    let mut landing = docked_on(upgrading(), at_earth(), cadence);
    assert_eq!(bar_shown(&landing), Some(700), "ship 128's");
    queue(&mut landing);
    let settled = after(&landing);
    landing
        .sim
        .queue_answer_showing(Answer::Settled(Settled::default()), settled);
    landing.settle_script();
    let mut refusal = docked_on(upgrading(), at_earth(), cadence);
    queue(&mut refusal);
    let refused = after(&refusal);
    refusal.sim.queue_answer_showing(
        Answer::RefuseShown(Ok(nova_sim::RefuseShown { offer_hidden: true })),
        refused,
    );
    assert!(refusal.refuse_mission(MissionId(130)).is_ok());
    vec![("a CompText", landing), ("a RefuseText", refusal)]
        .into_iter()
        .map(|(what, mut view)| {
            assert_eq!(view.take_mission_texts().len(), 1, "{what}");
            view.catch_up();
            (what, view)
        })
        .collect()
}

#[test]
fn a_text_queued_before_an_h_shows_over_the_old_graphic_until_it_closes() {
    for (what, mut view) in texts_before_an_h(RuleSource::Engine) {
        assert_eq!(bar_shown(&view), Some(700), "{what}: the old graphic");
        view.catch_up();
        assert_eq!(bar_shown(&view), Some(700), "{what}: a frame on");
        view.close_mission_text();
        assert_eq!(bar_shown(&view), Some(705), "{what}: at the H, once closed");
        view.catch_up();
        assert_eq!(bar_shown(&view), Some(705), "{what}: kept");
    }
}

#[test]
fn under_the_bible_cadence_the_graphic_follows_at_once() {
    for (what, mut view) in texts_before_an_h(RuleSource::Bible) {
        assert_eq!(bar_shown(&view), Some(705), "{what}: at once");
        view.close_mission_text();
        assert_eq!(bar_shown(&view), Some(705), "{what}");
    }
}

#[test]
fn leaving_the_dock_drops_the_texts_pending_redraws() {
    let mut view = docked();
    view.sim.queue_event(brief(1, PROXIMA, full()));
    view.sim.queue_answer(Answer::Accepted(Ok(0)));
    assert_eq!(view.accept_mission(MissionId(131)), Ok(0));
    assert_eq!(view.take_mission_texts().len(), 1);
    assert_eq!(view.bar_steps.len(), 1, "the brief's close");
    let flying = leaves(&view, |frame| frame.landed = None);
    view.sim.queue_answer_showing(
        Answer::Departed(nova_sim::Departure {
            stellar: StellarId(128),
            settled: Settled::default(),
        }),
        flying,
    );
    view.take_off().expect("takes off");
    assert!(view.bar_steps.is_empty(), "the take-off's redraw");
}

// The gauges while docked.

/// The fuel gauge's full-colour fill `view`'s port bar draws, if any.
fn fuel_drawn(view: &Faked) -> Option<DrawCommand> {
    let mut bar = DrawList::new();
    view.draw_status_bar(&mut bar);
    bar.iter()
        .find(|c| matches!(c, DrawCommand::Line { color, .. } if *color == layout().fuel_full))
        .cloned()
}

#[test]
fn by_the_engine_the_gauges_hold_while_docked_until_a_shop_closes() {
    let mut view = docked();
    let landed = fuel_drawn(&view);
    assert!(landed.is_some());
    view.sim.frame_mut().hud.reserves = fuelled(100.0);
    view.refresh_frame();
    let tanked = leaves(&view, |frame| frame.hud.reserves.fuel.max = 350.0);
    view.sim
        .queue_answer_showing(Answer::Outfitted(passed_run()), tanked);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(view.reserves_shown(&view.frame_now()), full(), "landing's");
    assert_eq!(fuel_drawn(&view), landed, "drawn held");
    view.refresh_status();
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "the shop closed"
    );
    assert_ne!(fuel_drawn(&view), landed);
}

#[test]
fn by_the_other_reading_the_gauges_follow_at_once_while_docked() {
    let mut view = docked_on(catalog(), at_earth(), RuleSource::Bible);
    let landed = fuel_drawn(&view);
    let tanked = leaves(&view, |frame| {
        frame.hud.reserves.fuel = Gauge {
            now: 100.0,
            max: 350.0,
        };
    });
    view.sim
        .queue_answer_showing(Answer::Outfitted(passed_run()), tanked);
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "at once"
    );
    assert_ne!(fuel_drawn(&view), landed);
}

#[test]
fn landing_redraws_the_held_gauges() {
    let flying = FlightFrame {
        landed: None,
        ..at_earth()
    };
    let mut view = docked_on(catalog(), flying, RuleSource::Engine);
    view.reserves_scene = Some(Reserves::default());
    let landed = leaves(&view, |frame| {
        frame.landed = Some(StellarId(128));
        frame.hud.reserves = fuelled(100.0);
    });
    view.sim.queue_answer_showing(
        Answer::LandPress(LandPress::Outcome(LandOutcome::Landed(StellarId(128)))),
        landed,
    );
    view.land();
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert_eq!(
        view.reserves_shown(&view.frame_now()),
        view.reserves(),
        "landing's"
    );
    assert_eq!(view.reserves_shown(&view.frame_now()).fuel.now, 100.0);
}

#[test]
fn in_flight_the_gauges_stay_live_by_the_engine() {
    let flying = FlightFrame {
        landed: None,
        ..at_earth()
    };
    let mut view = docked_on(catalog(), flying, RuleSource::Engine);
    assert!(view.hud_waits());
    view.redraw_status();
    let before = fuel_drawn(&view);
    view.sim.frame_mut().hud.reserves = fuelled(100.0);
    view.refresh_frame();
    assert_eq!(
        view.reserves_shown(&view.frame_now()).fuel.now,
        100.0,
        "at once"
    );
    assert_ne!(fuel_drawn(&view), before);
}

// The take-off's redraw before its deselect.

/// The nav area naming `name` in mode 2, as flight shows it.
fn flashing(name: &str) -> NavDisplay {
    NavDisplay::Stellar(Some(name.to_owned()))
}

/// One frame of `dt` on `view`'s clock, as a frame's tick moves it: the
/// stellars stepped, then the HUD advanced.
fn frame_of(view: &mut Faked, dt: Duration) {
    view.step_stellars(dt);
    view.advance_hud();
}

/// Takes off from `view`'s Earth by `leave` (the port's Leave, or a held
/// `Q`'s), the take-off's nav `held`, the flight leaving `after` in
/// mode 2 with nothing selected, or as `after` says.
fn take_off_from(
    view: &mut Faked,
    held: NavMode,
    after: NavMode,
    leave: fn(&mut Faked) -> Option<StellarId>,
) {
    view.sim.queue_event(FlightEvent::TakeoffNav(held));
    let flying = leaves(view, |frame| {
        frame.landed = None;
        frame.nav_mode = after;
    });
    view.sim.queue_answer_showing(
        Answer::Departed(nova_sim::Departure {
            stellar: StellarId(128),
            settled: Settled::default(),
        }),
        flying,
    );
    assert_eq!(leave(view), Some(StellarId(128)));
}

/// The port's Leave and a held `Q`'s.
type Leave = fn(&mut Faked) -> Option<StellarId>;
const LEAVES: [(&str, Leave); 2] = [
    ("the take-off", |view| view.take_off()),
    ("a Q", |view| view.leave_if_due()),
];

/// Mode 2 with nothing selected: what the take-off's deselect leaves.
const LET_GO: NavMode = NavMode::Stellar(None);

#[test]
fn the_take_off_after_a_landed_m_names_its_stellar_for_one_frame() {
    for (what, leave) in LEAVES {
        let mut view = docked();
        writes_nav(&mut view, PROXIMA);
        take_off_from(&mut view, PROXIMA, LET_GO, leave);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            flashing("Proxima"),
            "{what}: held"
        );
        frame_of(&mut view, Duration::from_millis(1));
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            flashing("Proxima"),
            "{what}: a frame within the tick"
        );
        frame_of(&mut view, TICK);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Stellar(None),
            "{what}: a tick on"
        );
        let mut view = docked_on(catalog(), at_earth(), RuleSource::Bible);
        writes_nav(&mut view, PROXIMA);
        take_off_from(&mut view, PROXIMA, LET_GO, leave);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Stellar(None),
            "{what}: by the bible"
        );
    }
}

#[test]
fn the_take_offs_held_nav_shows_in_the_first_frame_drawn_after_it_whatever_the_rate() {
    for fps in [120, 60, 30, 20] {
        let frame = Duration::from_secs(1) / fps;
        let mut view = docked();
        writes_nav(&mut view, PROXIMA);
        view.elapsed = Duration::from_nanos(333_333_329); // ten ticks less 1 ns
        assert_eq!(
            crate::time::ticks(view.elapsed + Duration::from_nanos(1)),
            10
        );
        take_off_from(&mut view, PROXIMA, LET_GO, LEAVES[0].1);
        frame_of(&mut view, frame);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            flashing("Proxima"),
            "{fps} fps: the first frame drawn, though it crosses a tick"
        );
        frame_of(&mut view, TICK);
        assert_eq!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Stellar(None),
            "{fps} fps: a tick on"
        );
    }
}

#[test]
fn a_take_off_with_nothing_let_go_holds_nothing() {
    let mut view = docked();
    writes_nav(&mut view, LET_GO);
    take_off_from(&mut view, LET_GO, LET_GO, LEAVES[0].1);
    assert_eq!(view.nav_shown(&view.frame_now()), NavDisplay::Stellar(None));
    assert_eq!(view.nav_flash, None, "no flash to end");
}

#[test]
fn a_plain_take_off_names_the_landed_stellar_for_one_frame() {
    let mut view = docked();
    take_off_from(&mut view, EARTH, LET_GO, LEAVES[0].1);
    assert_eq!(view.nav_shown(&view.frame_now()), flashing("Earth"), "held");
    frame_of(&mut view, TICK);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Earth"),
        "the first frame"
    );
    frame_of(&mut view, TICK);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        NavDisplay::Stellar(None),
        "a tick on"
    );
}

#[test]
fn a_take_off_with_a_course_names_the_landed_stellar_over_the_cue_for_one_frame() {
    let mut view = docked();
    let cued = NavMode::Hyperspace(Some(SystemId(131)));
    take_off_from(&mut view, EARTH, cued, LEAVES[0].1);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Earth"),
        "held over the cue"
    );
    frame_of(&mut view, TICK);
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Earth"),
        "the first frame"
    );
    frame_of(&mut view, TICK);
    assert!(
        matches!(
            view.nav_shown(&view.frame_now()),
            NavDisplay::Hyperspace { .. }
        ),
        "a tick on: {:?}",
        view.nav_shown(&view.frame_now())
    );
}

#[test]
fn a_text_queued_in_the_take_offs_days_shows_over_its_held_nav() {
    let mut view = docked();
    writes_nav(&mut view, PROXIMA);
    // A crön's `S131` in the stay's days, its text noting the nav before
    // the deselect.
    view.sim.queue_event(brief(1, PROXIMA, full()));
    view.sim.queue_event(FlightEvent::TakeoffNav(PROXIMA));
    let flying = leaves(&view, |frame| {
        frame.landed = None;
        frame.nav_mode = LET_GO;
        frame.texts_queued = 1;
    });
    view.sim.queue_answer_showing(
        Answer::Departed(nova_sim::Departure {
            stellar: StellarId(128),
            settled: Settled::default(),
        }),
        flying,
    );
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.take_mission_texts().len(), 1, "the crön's mission");
    assert_eq!(
        view.nav_shown(&view.frame_now()),
        flashing("Proxima"),
        "under its text"
    );
    frame_of(&mut view, TICK);
    assert_eq!(view.nav_shown(&view.frame_now()), NavDisplay::Stellar(None));
}

/// A text queued before the nav area's last redraw is older than it: its
/// nav doesn't replace the nav area as the texts are taken in flight.
#[test]
fn a_text_queued_before_the_last_nav_redraw_leaves_the_nav_area_as_redrawn() {
    let mut view = faked(FlightFrame {
        texts_queued: 1,
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    });
    view.sim.queue_event(brief(1, NavMode::Off, full()));
    view.redraw_status();
    view.sim.frame_mut().nav_mode = EARTH;
    view.refresh_frame();
    assert_eq!(view.take_mission_texts().len(), 1, "the brief");
    assert_eq!(
        view.nav_scene.as_ref().map(|(mode, _)| *mode),
        Some(EARTH),
        "as the take's redraw drew it, not the text's nav off"
    );
}

// The bar's map in port.

/// [`docked`], its flight drawing the course map as [`sol_drawing`] and
/// preselecting Sol, the nav saved as the map opens Earth's.
fn mapped() -> Faked {
    let mut view = docked();
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.map_preselection = Some(SystemId(130));
    map.map_nav = Some(SavedNav::of(EARTH));
    view
}

/// Clicks system `id` on the map open in port, as the port's map input
/// gives the press and the release.
fn faked_port_click(view: &mut Faked, id: i16, shift: bool) {
    let at = faked_on_map(view, id);
    if shift {
        view.port_map_input(&key(Key::Shift, true));
    }
    for pressed in [true, false] {
        view.port_map_input(&Input::PointerButton {
            button: crate::MouseButton::Left,
            pressed,
            at,
        });
    }
    if shift {
        view.port_map_input(&key(Key::Shift, false));
    }
}

/// The answer to a port click selecting Alpha Centauri (131), its course
/// changed when `plotted`.
fn clicked_alpha(plotted: bool) -> Answer {
    Answer::Clicked(MapClicked {
        selected: Some(SystemId(131)),
        gate_pick: None,
        clear_route: plotted.then_some(true),
        course_changed: plotted,
        nav_redraw: false,
    })
}

#[test]
fn the_bars_map_closing_shows_the_maps_selection_and_puts_back_the_bars_nav() {
    let mut view = mapped();
    view.open_bar_map();
    assert!(view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    // The planner's reading writes the nav in port, which the close shows.
    let cued = NavMode::Hyperspace(Some(SystemId(131)));
    let clicked = leaves(&view, |frame| frame.nav_mode = cued);
    view.sim.queue_answer_showing(clicked_alpha(false), clicked);
    faked_port_click(&mut view, 131, false);
    assert!(view.map_open(), "a click doesn't close it");
    view.sim.queue_answer(Answer::Done);
    let put_back = leaves(&view, |frame| frame.nav_mode = EARTH);
    view.sim.queue_answer_showing(Answer::Done, put_back);
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open(), "M closes it");
    assert_eq!(port_nav(&view), cueing_alpha(), "the map's selection drawn");
    assert_eq!(
        sent(&view)[1..],
        [
            FlightCommand::RestoreMapSelection(SavedNav::of(EARTH)),
            FlightCommand::RestoreBarNav,
        ],
        "the selection, then the bar's nav, put back"
    );
    view.refresh_status();
    assert_eq!(port_nav(&view), naming("Earth"), "the bar's close");
}

#[test]
fn a_map_in_port_the_bar_did_not_open_leaves_the_bars_nav_saved() {
    let mut view = mapped();
    view.open_bar_map();
    view.port_map_input(&key(MAP, true));
    assert!(
        sent(&view).contains(&FlightCommand::RestoreBarNav),
        "the bar's map"
    );
    let before = sent(&view).len();
    view.open_map();
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert!(
        !sent(&view)[before..].contains(&FlightCommand::RestoreBarNav),
        "another map in port leaves the bar's nav saved"
    );
}

#[test]
fn the_bars_map_key_repeat_or_release_leaves_it_open() {
    let mut view = mapped();
    view.open_bar_map();
    view.port_map_input(&Input::Key {
        key: MAP,
        pressed: true,
        repeat: true,
    });
    view.port_map_input(&key(MAP, false));
    assert!(view.map_open());
}

/// The map's beeps are `_HandlePlayer`'s, around the map M opens in
/// flight (@0x6a809, @0x6a85a): the map opened in port makes neither.
#[test]
fn the_map_in_port_beeps_neither_as_it_opens_nor_as_it_closes() {
    let mut view = mapped();
    view.open_bar_map();
    assert_eq!(view.ui_sounds, [], "the bar's map opened");
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert_eq!(view.ui_sounds, [], "the bar's map closed");
    view.open_map();
    view.close_map();
    assert_eq!(view.ui_sounds, [], "another map in port");
}

#[test]
fn in_port_a_plain_click_on_the_map_selects_only() {
    let mut view = mapped();
    view.open_bar_map();
    view.sim.queue_answer(clicked_alpha(false));
    faked_port_click(&mut view, 131, false);
    assert_eq!(view.course_map().selected(), Some(SystemId(131)));
    let FlightCommand::MapClick(click) = &sent(&view)[0] else {
        panic!("a click: {:?}", sent(&view));
    };
    assert!(!click.shift, "a plain click");
    assert_eq!(
        port_nav(&view),
        naming("Earth"),
        "no nav written docked (@0x136ec)"
    );
}

#[test]
fn in_port_a_shift_click_builds_the_course_and_the_close_cues_nothing() {
    let mut view = mapped();
    view.open_bar_map();
    let plotted = leaves(&view, |frame| frame.course_shown = vec![SystemId(131)]);
    view.sim.queue_answer_showing(clicked_alpha(true), plotted);
    faked_port_click(&mut view, 131, true);
    let FlightCommand::MapClick(click) = &sent(&view)[0] else {
        panic!("a click: {:?}", sent(&view));
    };
    assert!(click.shift, "the Shift-click never tests _onPlanet");
    assert!(view.course_map().clear_route_enabled());
    view.port_map_input(&key(MAP, true));
    assert!(!view.map_open());
    assert!(
        !sent(&view).contains(&FlightCommand::CueCourse),
        "no cue in port"
    );
    assert_eq!(port_nav(&view), naming("Earth"), "no cue drawn");
}

// The settling's catch-up.

/// A frame in system `system`, the player alone at the centre in ship
/// `class`, `landed` on Earth or not.
fn in_system(system: i16, class: i16, landed: bool) -> FlightFrame {
    FlightFrame {
        system: SystemId(system),
        landed: landed.then_some(StellarId(128)),
        ships: vec![record::player(class, Vec2::ZERO, 0.0)],
        ..in_sol()
    }
}

/// Settles `view`'s flight, which tells what `events` say and leaves
/// `after`.
fn settles_to(view: &mut Faked, events: &[SessionEvent], after: FlightFrame) {
    for event in events {
        view.sim.queue_event(FlightEvent::Session(*event));
    }
    view.sim
        .queue_answer_showing(Answer::Settled(Settled::default()), after);
    view.settle_script();
}

#[test]
fn a_move_in_flight_lays_out_the_system_moved_to_once_settled() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, false));
    view.catalog().systems_read.borrow_mut().clear();
    settles_to(
        &mut view,
        &[SessionEvent::SystemChanged],
        in_system(131, 128, false),
    );
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(131)]);
    assert_eq!(view.course_map().current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "on Proxima");
    assert_eq!(view.message(), None);
    settles_to(&mut view, &[], in_system(131, 128, false));
    assert_eq!(
        *view.catalog().systems_read.borrow(),
        [SystemId(131)],
        "nothing more to settle"
    );
}

#[test]
fn a_move_within_the_system_lays_it_out_afresh() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, false));
    view.catalog().systems_read.borrow_mut().clear();
    settles_to(
        &mut view,
        &[SessionEvent::SystemChanged],
        in_system(130, 128, false),
    );
    assert_eq!(*view.catalog().systems_read.borrow(), [SystemId(130)]);
}

#[test]
fn a_move_while_landed_is_laid_out_once_the_ship_takes_off() {
    let mut view = super::faked_world::world(catalog(), in_system(130, 128, true));
    view.catalog().systems_read.borrow_mut().clear();
    // The session tells the move only as the ship takes off.
    settles_to(&mut view, &[], in_system(131, 128, true));
    assert_eq!(*view.catalog().systems_read.borrow(), []);
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim.queue_answer_showing(
        Answer::Departed(nova_sim::Departure {
            stellar: StellarId(128),
            settled: Settled::default(),
        }),
        in_system(131, 128, false),
    );
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0));
}

#[test]
fn an_h_from_a_set_expression_redraws_the_new_hull_once_settled() {
    let mut view = super::faked_world::world(shipbuying(), in_system(130, 128, false));
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
    settles_to(
        &mut view,
        &[SessionEvent::ShipChanged],
        in_system(130, 129, false),
    );
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)]
    );
    let ids: Vec<_> = super::faked_world::faked_ship_sprites(&view)
        .iter()
        .map(|(image, _)| image.id)
        .collect();
    assert_eq!(ids, [2001], "ship 129's sheet");
    assert_eq!(view.frame(), Some(0), "of 72 rotations");
}

#[test]
fn settling_reads_no_sheet_when_the_class_is_unchanged() {
    let mut view = super::faked_world::world(shipbuying(), in_system(130, 128, false));
    settles_to(&mut view, &[], in_system(130, 128, false));
    assert_eq!(*view.catalog().sheets_asked.borrow(), [ShipId(128)]);
}

#[test]
fn an_outfits_on_purchase_h_redraws_the_new_hull_once_settled_and_at_the_take_off() {
    let mut view = super::faked_world::world(shipbuying(), in_system(130, 128, true));
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::ShipChanged));
    view.sim
        .queue_answer_showing(Answer::Outfitted(passed_run()), in_system(130, 129, true));
    assert_eq!(view.outfit(BUY_TANK), Ok(()));
    settles_to(&mut view, &[], in_system(130, 129, true));
    settles_to(&mut view, &[], in_system(130, 129, true));
    assert_eq!(
        *view.catalog().sheets_asked.borrow(),
        [ShipId(128), ShipId(129)],
        "once"
    );
    view.sim.queue_answer_showing(
        Answer::Departed(nova_sim::Departure {
            stellar: StellarId(128),
            settled: Settled::default(),
        }),
        in_system(130, 129, false),
    );
    view.take_off().expect("took off");
    let ids: Vec<_> = super::faked_world::faked_ship_sprites(&view)
        .iter()
        .map(|(image, _)| image.id)
        .collect();
    assert_eq!(ids, [2001], "the new hull");
}
