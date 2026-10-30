//! The land key's, the gates' and the course map's close's component
//! tests on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-9-travel-on-port): the commands a
//! press and a close send, what they say from the flight's answers, and
//! what the view lets go of. Kept apart from `tests/mod.rs`, which is
//! large.

use super::faked_travel::{faked_lasts, sent_but_lines, shown_to_flight};
use super::faked_world::{raise, world};
use super::*;
use crate::flight::flash::ShieldFlash;
use crate::flight::jump::{ARRIVAL_FLASH_FOR, FADE_IN_FOR, JumpPhase};
use nova_sim::flight_frame::FlightEvent;
use nova_sim::landing::LandOutcome;
use nova_sim::{GateKind, GateRefusal, LandPress, LandingRefusal, SessionEvent, SimSound};

/// The answer to a press that requested clearance at `stellar`.
fn requested(stellar: i16) -> Answer {
    Answer::LandPress(LandPress::Outcome(LandOutcome::Requested {
        stellar: StellarId(stellar),
        station: false,
    }))
}

/// The answer to a press that landed on `stellar`.
fn landed(stellar: i16) -> Answer {
    Answer::LandPress(LandPress::Outcome(LandOutcome::Landed(StellarId(stellar))))
}

/// The answer to a press over the cleared gate 300 of `kind`.
pub(super) fn at_gate(kind: GateKind) -> Answer {
    Answer::LandPress(LandPress::AtGate {
        stellar: StellarId(300),
        kind,
    })
}

/// The answer to a refused press, its stellar's gate `gate`.
fn refused(refusal: LandingRefusal, gate: Option<GateKind>) -> Answer {
    Answer::LandRefused { refusal, gate }
}

/// Presses L on `view`'s flight, which answers `answer`.
fn press_l(view: &mut Faked, answer: Answer) {
    view.sim.queue_answer(answer);
    view.land();
}

/// The frame of the flight come out in Alpha Centauri (131), and the
/// session's events of the system entered and its arrival's `message`
/// drained next.
pub(super) fn came_out_in_alpha(view: &mut Faked, message: SimMessage) {
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        stellars: Vec::new(),
        ..sol()
    });
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
    view.sim.queue_event(FlightEvent::Message(message));
}

/// The answer to a gate's entry that came out in Alpha Centauri.
pub(super) fn arrived() -> Answer {
    Answer::Arrived {
        system: SystemId(131),
        combat: Vec::new(),
    }
}

// The land key.

#[test]
fn l_requests_says_what_the_flight_said_refuses_with_its_beep_and_lands_for_the_router() {
    let mut view = faked(sol());
    press_l(&mut view, requested(129));
    assert_eq!(sent_but_lines(&view), [FlightCommand::Land]);
    assert_eq!(view.take_landing(), None, "requested, not landed");
    assert_eq!(view.message(), None, "the flight said nothing: silent");
    view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
    press_l(
        &mut view,
        refused(
            LandingRefusal::TooFar {
                stellar: StellarId(129),
                station: false,
            },
            None,
        ),
    );
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.drained_sounds(), [Sound::Sim(SimSound::Refused)]);
    assert_eq!(view.message(), Some(TOO_FAR_PLANET));
    press_l(&mut view, landed(129));
    assert_eq!(view.take_landing(), Some(StellarId(129)));
    assert_eq!(view.take_landing(), None, "taken");
    assert_eq!(view.message(), None, "the landing empties the line");
}

#[test]
fn the_presss_messages_show_first_and_a_refusal_replaces_them() {
    let mut view = faked(sol());
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::NoResponse));
    press_l(
        &mut view,
        refused(
            LandingRefusal::TooFar {
                stellar: StellarId(128),
                station: false,
            },
            None,
        ),
    );
    assert_eq!(
        shown_to_flight(&view),
        [SimMessage::NoResponse.ticks(), TOO_FAR_TICKS],
        "the press's line, then the refusal over it"
    );
    faked_lasts(&mut view, TOO_FAR_PLANET, TOO_FAR_TICKS);
}

/// Earth (128), as a refusal names it.
const EARTH: StellarId = StellarId(128);

/// Out of reach of Earth, a station or not.
fn too_far(station: bool) -> LandingRefusal {
    LandingRefusal::TooFar {
        stellar: EARTH,
        station,
    }
}

/// Too fast for Earth, a station or not.
fn too_fast(station: bool) -> LandingRefusal {
    LandingRefusal::TooFast {
        stellar: EARTH,
        station,
        speed: 15.0,
    }
}

/// Denied at `stellar`, a station or not.
fn denied(stellar: StellarId, station: bool) -> LandingRefusal {
    LandingRefusal::Denied {
        stellar,
        station,
        min_status: 1,
    }
}

/// `stellar` not landable, a station or not.
fn not_landable(stellar: StellarId, station: bool) -> LandingRefusal {
    LandingRefusal::NotLandable { stellar, station }
}

#[test]
fn each_refusal_says_why_in_the_originals_words_for_its_ticks() {
    let (hypergate, wormhole) = (Some(GateKind::Hypergate), Some(GateKind::Wormhole));
    let hole = StellarId(150);
    let cases = [
        (too_far(false), None, TOO_FAR_PLANET, TOO_FAR_TICKS),
        (too_far(true), None, TOO_FAR_STATION, TOO_FAR_TICKS),
        (too_far(false), hypergate, TOO_FAR_HYPERGATE, TOO_FAR_TICKS),
        (too_far(false), wormhole, TOO_FAR_WORMHOLE, TOO_FAR_TICKS),
        (too_fast(false), None, TOO_FAST_PLANET, TOO_FAR_TICKS),
        (
            too_fast(true),
            None,
            "You're moving too fast to dock at this station.",
            TOO_FAR_TICKS,
        ),
        (
            too_fast(false),
            hypergate,
            TOO_FAST_HYPERGATE,
            TOO_FAR_TICKS,
        ),
        (too_fast(false), wormhole, TOO_FAST_WORMHOLE, TOO_FAR_TICKS),
        (
            denied(EARTH, false),
            None,
            "Landing request denied.",
            LANDING_TICKS,
        ),
        (
            denied(EARTH, true),
            None,
            "Docking request denied.",
            LANDING_TICKS,
        ),
        (
            denied(StellarId(300), false),
            hypergate,
            HYPERGATE_DENIED,
            LANDING_TICKS,
        ),
        (
            not_landable(EARTH, true),
            None,
            "Your ship is unable to dock at  Earth. The station's hull integrity is too unstable.",
            LANDING_TICKS,
        ),
        (
            not_landable(hole, false),
            None,
            "Your ship is unable to land on  Hole. The planet's environment is too hostile.",
            LANDING_TICKS,
        ),
    ];
    for (refusal, gate, text, ticks) in cases {
        let mut view = faked(sol());
        // A stellar the scene doesn't hold, named by the flight.
        renamed(&mut view, |names| {
            names.stellars.insert(hole, "Hole".to_owned());
        });
        press_l(&mut view, refused(refusal, gate));
        assert_eq!(view.take_landing(), None, "{refusal:?}");
        faked_lasts(&mut view, text, ticks);
    }
}

#[test]
fn an_offline_or_unseen_gate_not_landable_says_the_gates_line() {
    for (kind, stellar) in [
        (GateKind::Hypergate, StellarId(300)),
        (GateKind::Wormhole, StellarId(300)),
        (GateKind::Wormhole, StellarId(150)),
    ] {
        let mut view = faked(sol());
        press_l(
            &mut view,
            refused(
                LandingRefusal::NotLandable {
                    stellar,
                    station: false,
                },
                Some(kind),
            ),
        );
        faked_lasts(&mut view, &gate_refusal_message(kind), LANDING_TICKS);
    }
}

#[test]
fn a_refusal_naming_no_stellar_says_nothing_and_leaves_the_line() {
    for refusal in [
        LandingRefusal::NoStellars,
        LandingRefusal::NothingLandable,
        LandingRefusal::Jumping,
        LandingRefusal::Disabled,
    ] {
        let mut view = faked(sol());
        view.say_for("earlier", MESSAGE_TICKS);
        press_l(&mut view, refused(refusal, None));
        assert_eq!(view.take_landing(), None, "{refusal:?}");
        assert_eq!(view.message(), Some("earlier"), "{refusal:?}: unchanged");
    }
}

#[test]
fn a_new_refusal_shows_afresh() {
    let mut view = faked(sol());
    let too_far = || {
        refused(
            LandingRefusal::TooFar {
                stellar: StellarId(128),
                station: false,
            },
            None,
        )
    };
    press_l(&mut view, too_far());
    for _ in 0..300 {
        super::faked_travel::faked_tick(&mut view);
    }
    press_l(&mut view, too_far());
    faked_lasts(&mut view, TOO_FAR_PLANET, TOO_FAR_TICKS);
}

#[test]
fn landing_lets_go_of_the_line_the_effects_the_flashes_and_the_glows() {
    let mut view = world(armed(0, &[BLASTER]), sol());
    raise(
        &mut view,
        &[CombatEvent::Exploded {
            at: Vec2::ZERO,
            explosion: nova_sim::catalog::Explosion::decode(0).expect("type 0"),
            size: 0.0,
        }],
    );
    let mut flash = ShieldFlash::default();
    flash.hit(false);
    view.flashes.insert(ShipRef::Player, flash);
    view.glows.step(&[(ShipRef::Player, false)], |_| Some(1.0));
    assert!(view.glows.drawn(ShipRef::Player).is_some(), "a glow lit");
    let alt = crate::flight::held_alt::AltLayer {
        image: ImageKey::sprite(2100, 0),
        at: Point::new(0.0, 0.0),
        tint: Color::WHITE,
        colorize: None,
        blend: crate::Blend::Normal,
    };
    let held = view.held_alts.get_mut().shown(
        ShipRef::Player,
        nova_sim::combat::ship_frames::Alt::Drawn(0),
        Some(alt),
        true,
    );
    assert_eq!(held, Some(alt), "an alt held");
    view.say_for(TOO_FAR_PLANET, TOO_FAR_TICKS);
    assert_ne!(*view.effects(), Effects::default());
    press_l(&mut view, landed(128));
    assert_eq!(view.take_landing(), Some(StellarId(128)));
    assert_eq!(view.message(), None, "forgotten once the ship lands");
    assert_eq!(*view.effects(), Effects::default());
    assert!(view.effects().pods().is_empty());
    assert!(view.flashes.is_empty(), "the shield flashes");
    assert_eq!(view.glows.drawn(ShipRef::Player), None, "the weapon glow");
    assert_eq!(
        *view.held_alts.borrow(),
        crate::flight::held_alt::HeldAlts::default(),
        "the alts held"
    );
}

#[test]
fn landing_in_a_flight_that_never_started_does_nothing() {
    let mut view = faked(FlightFrame::default());
    view.land();
    assert_eq!(sent(&view), [FlightCommand::Land], "refused");
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.message(), None);
}

// The gates.

/// [`faked`] in Sol with L pressed over its cleared hypergate, which
/// offers Alpha Centauri (131) and Barnard (132): the hypergate map open.
pub(super) fn at_the_hypergate() -> Faked {
    let mut view = faked(sol());
    let opening = map_opening(&view.frame, Some(130));
    view.sim.queue_screen(opening);
    view.sim.queue_answer(at_gate(GateKind::Hypergate));
    view.sim
        .queue_answer(Answer::Offered(vec![SystemId(131), SystemId(132)]));
    view.land();
    view
}

#[test]
fn l_at_a_cleared_hypergate_opens_its_map_offering_its_links_on_the_current_system() {
    let mut view = at_the_hypergate();
    assert_eq!(
        sent_but_lines(&view),
        [FlightCommand::Land, FlightCommand::OpenHypergate]
    );
    assert!(view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Hypergate);
    assert_eq!(view.course_map().offered(), [SystemId(131), SystemId(132)]);
    assert_eq!(
        view.course_map().selected(),
        Some(SystemId(130)),
        "the current system"
    );
    assert_eq!(view.take_landing(), None, "not landed");
    assert_eq!(
        view.ui_sounds,
        [],
        "the hypergate's map opens without M's beep"
    );
}

#[test]
fn a_hypergate_without_links_says_nothing_and_opens_no_map() {
    let mut view = faked(sol());
    view.say_for("cleared", 250);
    view.sim.queue_answer(at_gate(GateKind::Hypergate));
    view.sim
        .queue_answer(Answer::GateRefused(GateRefusal::NoLinks));
    view.land();
    assert!(!view.map_open());
    assert_eq!(view.message(), Some("cleared"), "nothing said");
}

/// Closes `view`'s hypergate map on `pick`, its flight answering the
/// entry `answer`.
pub(super) fn close_on(view: &mut Faked, pick: Option<SystemId>, answer: Answer) {
    if pick.is_some() {
        view.map.select_gate(pick);
    }
    view.sim.queue_answer(answer);
    view.close_map();
}

#[test]
fn closing_the_hypergate_map_on_a_pick_comes_out_of_its_gate_fading_in_from_white() {
    let mut view = at_the_hypergate();
    view.map.centre_on_system(SystemId(132));
    view.track_held(&key(Key::Up, true));
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    close_on(&mut view, Some(SystemId(131)), arrived());
    assert_eq!(
        sent_but_lines(&view).last(),
        Some(&FlightCommand::EnterHypergate(Some(SystemId(131))))
    );
    assert!(!view.map_open());
    assert_eq!(view.course_map().mode(), MapMode::Course);
    assert_eq!(view.scene().map(SystemScene::name), Some("Alpha Centauri"));
    assert_eq!(
        view.course_map().current(),
        Some(SystemId(131)),
        "the map follows"
    );
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0))
    );
    assert!(!view.holds_keys(), "the keys let go");
    assert_eq!(view.ui_sounds, [], "closed in silence");
    let exited = "Exiting hypergate in the Alpha Centauri system on June 23rd, 1177 NC. No stellar objects present.";
    faked_lasts(&mut view, exited, 360);
    let mut recentred = faked(FlightFrame {
        system: SystemId(131),
        ..FlightFrame::default()
    });
    recentred.map.recentre();
    assert_eq!(
        view.course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll),
        recentred
            .course_map()
            .place()
            .map(crate::galaxy::place::MapPlace::scroll),
        "the scroll on the system arrived in"
    );
    view.fade_frame(FADE_IN_FOR);
    assert_eq!(view.jump_effect(), None, "the fade is over");
}

#[test]
fn without_hyperspace_effects_a_hypergate_flashes_white() {
    let mut view = at_the_hypergate().with_hyperspace_effects(false);
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    close_on(&mut view, Some(SystemId(131)), arrived());
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::Flash(0.0))
    );
    view.fade_frame(ARRIVAL_FLASH_FOR);
    assert_eq!(view.jump_effect(), None);
}

#[test]
fn closing_the_hypergate_map_without_a_pick_cancels_in_silence() {
    let mut view = at_the_hypergate();
    close_on(&mut view, None, Answer::GateRefused(GateRefusal::Cancelled));
    assert_eq!(
        sent_but_lines(&view).last(),
        Some(&FlightCommand::EnterHypergate(None))
    );
    assert!(!view.map_open());
    assert_eq!(
        view.course_map().mode(),
        MapMode::Course,
        "a course map again"
    );
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.ui_sounds, []);
    assert!(
        !sent(&view).contains(&FlightCommand::CueCourse),
        "the hypergate map cues nothing"
    );
    faked_lasts(&mut view, HYPERGATE_CANCELLED, GATE_REFUSAL_TICKS);
}

#[test]
fn closing_the_hypergate_map_redraws_the_radar_at_once_leaving_the_cadence() {
    for pick in [None, Some(SystemId(131))] {
        let mut view = at_the_hypergate();
        let stale = RadarScene {
            player: Point::new(-1.0, -1.0),
            ..RadarScene::default()
        };
        view.radar_scene = Some(stale.clone());
        let cadence = view.hud_cadence;
        let answer = if pick.is_some() {
            came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
            arrived()
        } else {
            Answer::GateRefused(GateRefusal::Cancelled)
        };
        close_on(&mut view, pick, answer);
        assert_eq!(
            view.hud_cadence, cadence,
            "{pick:?}: neither timed nor flipped"
        );
        assert_ne!(*view.radar_shown(&view.frame_now()), stale, "{pick:?}");
    }
}

#[test]
fn l_at_a_wormhole_passes_through_it_with_a_flash_whatever_the_preference() {
    for effects in [true, false] {
        let mut view = faked(sol()).with_hyperspace_effects(effects);
        view.track_held(&key(Key::Up, true));
        view.sim.queue_answer(at_gate(GateKind::Wormhole));
        came_out_in_alpha(&mut view, SimMessage::PassedWormhole(SystemId(131)));
        view.sim.queue_answer(arrived());
        view.land();
        assert_eq!(
            sent_but_lines(&view),
            [FlightCommand::Land, FlightCommand::EnterWormhole]
        );
        assert!(!view.map_open());
        assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
        assert_eq!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Flash(0.0)),
            "effects {effects}"
        );
        assert!(!view.holds_keys());
        let passed = "Passing through a wormhole into the Alpha Centauri system on June 23rd, 1177 NC. No stellar objects present.";
        faked_lasts(&mut view, passed, 240);
    }
}

/// What the view says of a wormhole with no exit.
const NO_EXIT: &str =
    "Your ship is unable to enter this wormhole - the radiation levels are too extreme.";

#[test]
fn a_wormhole_with_no_exit_says_why() {
    let mut view = faked(sol());
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    view.sim
        .queue_answer(Answer::GateRefused(GateRefusal::NoExit));
    view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
    view.land();
    assert_eq!(view.jump_effect(), None);
    assert_eq!(
        view.drained_sounds(),
        [Sound::Sim(SimSound::Refused)],
        "the flight's beep (@0x646cb), and none of the view's"
    );
    faked_lasts(&mut view, NO_EXIT, GATE_REFUSAL_TICKS);
}

#[test]
fn a_wormhole_with_no_exit_lays_the_scene_out_afresh_once_from_the_flights_events() {
    // The dock request's tail deletes the ships, kills the shots and the
    // smoke (@0x66f40-0x66f6f, @0x67096) and sets the system up again
    // (@0x674c2), in the system the ship stays in.
    let mut view = world(armed(0, &[BLASTER]), sol());
    raise(
        &mut view,
        &[CombatEvent::Exploded {
            at: Vec2::ZERO,
            explosion: nova_sim::catalog::Explosion::decode(0).expect("type 0"),
            size: 0.0,
        }],
    );
    assert_ne!(*view.effects(), Effects::default());
    let mut flash = ShieldFlash::default();
    flash.hit(false);
    view.flashes.insert(ShipRef::Player, flash);
    view.npc_previous = vec![(
        nova_sim::NpcId(0),
        ShipPlace {
            position: Vec2::ZERO,
            heading: 0.0,
        },
    )];
    let mut after = sol();
    after
        .ships
        .push(record::npc(7, 131, Vec2::new(100.0, 0.0), 0.0));
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    view.sim.queue_answer_telling(
        Answer::GateRefused(GateRefusal::NoExit),
        after,
        vec![
            FlightEvent::Session(SessionEvent::SystemChanged),
            FlightEvent::Session(SessionEvent::StellarFramesReset),
        ],
    );
    view.land();
    assert_eq!(*view.effects(), Effects::default(), "the old scene let go");
    assert!(view.flashes.is_empty());
    assert!(view.npc_previous.is_empty(), "the old ships gone");
    assert!(
        view.npc_sheets.contains_key(&ShipId(131)),
        "the new ships' sheets read"
    );
    assert!(!view.catch_up().system, "laid out once, not again");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    faked_lasts(&mut view, NO_EXIT, GATE_REFUSAL_TICKS);
}

#[test]
fn a_wormhole_with_no_exit_shuts_the_escort_menu() {
    // `_escortMenuOnscreen` = 0 (@0x6705a).
    let mut view = faked(super::faked_escorts::fleet_of(&[EscortClass::Warship]));
    assert!(view.escort_key(MENU_KEY));
    assert!(view.escort_menu().is_open());
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    view.sim
        .queue_answer(Answer::GateRefused(GateRefusal::NoExit));
    view.land();
    assert!(!view.escort_menu().is_open());
    faked_lasts(&mut view, NO_EXIT, GATE_REFUSAL_TICKS);
}

#[test]
fn passing_through_a_wormhole_lets_go_of_the_shield_flashes_and_the_effects() {
    let mut view = world(armed(0, &[BLASTER]), sol());
    let mut flash = ShieldFlash::default();
    flash.hit(false);
    view.flashes.insert(ShipRef::Player, flash);
    view.sim.queue_answer(at_gate(GateKind::Wormhole));
    came_out_in_alpha(&mut view, SimMessage::PassedWormhole(SystemId(131)));
    view.sim.queue_answer(arrived());
    view.land();
    assert!(view.flashes.is_empty());
    assert_eq!(*view.effects(), Effects::default());
}

// The course map's close.

#[test]
fn m_beeps_as_it_opens_and_the_close_beeps_and_cues_the_course() {
    let mut view = faked(sol());
    view.open_map_by_key();
    assert!(view.map_open());
    assert_eq!(view.ui_sounds, [UiSound::MapOpened], "opened");
    view.ui_sounds.clear();
    view.map_input(&key(MAP, false));
    view.close_map();
    assert!(!view.map_open());
    assert_eq!(view.ui_sounds, [UiSound::MapClosed], "closed");
    assert_eq!(
        sent(&view),
        [FlightCommand::CueCourse],
        "the close's cue (@0x13c16)"
    );
}

#[test]
fn return_closes_the_course_map_with_the_close_beep_and_the_cue() {
    let mut view = faked(sol());
    view.open_map();
    view.map_input(&key(Key::Enter, true));
    assert!(!view.map_open());
    assert_eq!(view.ui_sounds, [UiSound::MapClosed]);
    assert_eq!(sent(&view), [FlightCommand::CueCourse]);
}

#[test]
fn return_or_escape_on_the_hypergate_map_enters_the_pick_in_silence() {
    let mut view = at_the_hypergate();
    view.map.select_gate(Some(SystemId(131)));
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    view.sim.queue_answer(arrived());
    view.map_input(&key(Key::Enter, true));
    assert_eq!(
        sent_but_lines(&view).last(),
        Some(&FlightCommand::EnterHypergate(Some(SystemId(131))))
    );
    assert_eq!(view.ui_sounds, []);
}

#[test]
fn closing_the_map_abandons_its_drag() {
    let mut view = faked_sol(sol());
    view.open_map();
    let empty = Point::new(250.0, 150.0);
    view.map_input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: empty,
    });
    view.close_map();
    assert!(!view.map_open());
    view.open_map();
    let place = view.course_map().place().expect("in flight").scroll();
    view.map_input(&Input::PointerMoved(Point::new(300.0, 150.0)));
    view.map_input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at: empty,
    });
    assert_eq!(
        view.course_map().place().expect("in flight").scroll(),
        place,
        "no drag"
    );
}

#[test]
fn b_then_a_close_in_flight_saves_the_borders_off_once_and_twice_saves_nothing() {
    let mut view = faked(sol());
    view.open_map();
    assert!(view.map_borders_shown());
    view.map_input(&key(Key::Char('b'), true));
    assert!(!view.map_borders_shown());
    assert_eq!(view.take_map_borders_save(), None, "not until the close");
    view.close_map();
    assert_eq!(view.take_map_borders_save(), Some(false));
    assert_eq!(view.take_map_borders_save(), None, "taken");
    view.open_map();
    assert!(!view.course_map().borders_shown(), "the next open keeps it");
    view.map_input(&key(Key::Char('b'), true));
    view.map_input(&key(Key::Char('b'), true));
    view.close_map();
    assert_eq!(view.take_map_borders_save(), None, "b twice");
}

#[test]
fn a_borders_change_on_the_hypergate_map_saves_but_one_docked_is_kept_unsaved() {
    let mut view = at_the_hypergate();
    view.map_input(&key(Key::Char('b'), true));
    close_on(&mut view, None, Answer::GateRefused(GateRefusal::Cancelled));
    assert_eq!(
        view.take_map_borders_save(),
        Some(false),
        "the hypergate map's too"
    );
    let mut docked = faked(FlightFrame {
        landed: Some(StellarId(128)),
        ..sol()
    });
    docked.open_bar_map();
    docked.port_map_input(&key(Key::Char('b'), true));
    docked.close_map();
    assert_eq!(docked.take_map_borders_save(), None, "docked");
    assert!(!docked.map_borders_shown(), "kept for the run");
    docked.open_bar_map();
    assert!(
        !docked.course_map().borders_shown(),
        "the next open starts off"
    );
}

// The take-off and the settling.

/// The answer to a take-off from `stellar`, nothing settled.
pub(super) fn departed(stellar: i16) -> Answer {
    Answer::Departed(nova_sim::Departure {
        stellar: StellarId(stellar),
        settled: nova_sim::Settled::default(),
    })
}

#[test]
fn taking_off_gives_the_stellar_and_draws_the_ship_there_on_the_next_frame() {
    let mut view = faked(docked_at(140, Vec2::new(6.0, -8.0)));
    view.alpha = 0.5;
    let flying = FlightFrame {
        landed: None,
        ..docked_at(140, Vec2::new(6.0, -8.0))
    };
    view.sim.queue_answer_showing(departed(140), flying);
    assert_eq!(view.take_off(), Some(StellarId(140)));
    assert_eq!(sent_but_lines(&view), [FlightCommand::TakeOff]);
    assert_eq!(view.alpha(), 0.0);
    assert_eq!(view.shown_position(), at(6.0, -8.0));
    assert_eq!(view.camera().center(), at(6.0, -8.0));
    assert_eq!(view.take_off(), None, "not landed: refused");
}

#[test]
fn a_take_off_greets_in_the_flights_words_and_lets_no_line_from_before_stay() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    view.say_for(TOO_FAR_PLANET, TOO_FAR_TICKS);
    let greeting = SimMessage::TookOff {
        stellar: StellarId(128),
        lead: 0,
    };
    view.sim.queue_event(FlightEvent::Message(greeting.clone()));
    view.sim.queue_answer_showing(departed(128), sol());
    view.take_off();
    let said = take_off_greeting(0, "Earth", DATE);
    faked_lasts(&mut view, &said, greeting.ticks());
}

#[test]
fn a_held_leave_takes_off_saying_its_message_and_none_due_does_nothing() {
    let mut view = faked(FlightFrame {
        leave_due: true,
        ..docked_at(128, Vec2::ZERO)
    });
    assert!(view.leave_due(), "the frame's");
    view.sim
        .queue_event(FlightEvent::LeaveMessage("Leave now.".to_owned()));
    view.sim.queue_answer_showing(departed(128), sol());
    assert_eq!(view.leave_if_due(), Some(StellarId(128)));
    assert_eq!(sent_but_lines(&view), [FlightCommand::LeaveIfDue]);
    faked_lasts(&mut view, "Leave now.", SCRIPT_MESSAGE_TICKS);
    assert!(!view.leave_due());
    assert_eq!(view.leave_if_due(), None, "none due");
}

#[test]
fn the_escorts_landing_reports_what_the_flight_reports() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    view.sim
        .queue_answer(Answer::EscortReport(Some("Escort sold.".to_owned())));
    assert_eq!(view.land_escorts(), Some("Escort sold.".to_owned()));
    assert_eq!(view.land_escorts(), None, "refused: nothing");
    assert_eq!(
        sent(&view),
        [FlightCommand::LandEscorts, FlightCommand::LandEscorts]
    );
}

#[test]
fn taking_off_in_a_flight_that_never_started_does_nothing() {
    let mut view = faked(FlightFrame::default());
    assert_eq!(view.take_off(), None);
    assert_eq!(view.leave_if_due(), None);
    assert_eq!(view.land_escorts(), None);
    assert!(!view.leave_due());
}

#[test]
fn while_docked_the_radar_shows_only_the_player_until_the_take_off() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    assert!(view.radar_shown(&view.frame_now()).blank, "docked");
    view.sim.queue_answer_showing(departed(128), sol());
    view.take_off();
    assert!(
        !view.radar_shown(&view.frame_now()).blank,
        "at once on take-off"
    );
}

#[test]
fn a_take_off_draws_its_new_traffic_with_its_sprites_and_none_from_before_the_landing() {
    let before = npc_129(0, Vec2::new(100.0, -100.0), 0.0);
    let mut view = world(
        armed(1, &[BLASTER]),
        FlightFrame {
            ships: vec![the_player_at(Vec2::ZERO, 0.0), before],
            ..docked_at(128, Vec2::ZERO)
        },
    );
    let after = npc_129(2, Vec2::new(-100.0, 100.0), 0.0);
    view.sim.queue_answer_showing(
        departed(128),
        FlightFrame {
            traffic_ships: vec![ShipId(129)],
            ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0), after])
        },
    );
    view.take_off();
    let sprites = super::faked_world::faked_ship_sprites(&view);
    let at_screen = |place: Vec2| view.camera().world_to_screen(point(place));
    assert!(
        sprites
            .iter()
            .any(|&(_, centre)| centre == at_screen(Vec2::new(-100.0, 100.0))),
        "the new traffic drawn at once, with its own sprite: {sprites:?}"
    );
    assert!(
        !sprites
            .iter()
            .any(|&(_, centre)| centre == at_screen(Vec2::new(100.0, -100.0))),
        "none of the ships from before the landing"
    );
}

#[test]
fn landing_and_taking_off_keep_the_stars() {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(super::stars::Counting::default()));
    let mut view = world(starry(), sol()).with_effects_chance(SharedChance::new(chance));
    view.catch_up();
    assert!(view.stars.stars().iter().all(|star| star.shown));
    press_l(&mut view, landed(128));
    let before = view.stars.stars();
    view.sim.queue_answer_showing(departed(128), sol());
    view.take_off();
    view.catch_up();
    assert_eq!(view.stars.stars(), before);
}

#[test]
fn the_settling_sends_its_command_and_says_what_it_settled() {
    let mut view = faked(sol());
    view.sim.queue_answer(Answer::Settled(nova_sim::Settled {
        message: Some("Hello.".to_owned()),
        ..nova_sim::Settled::default()
    }));
    view.settle_script();
    assert_eq!(sent_but_lines(&view), [FlightCommand::SettleScript]);
    faked_lasts(&mut view, "Hello.", SCRIPT_MESSAGE_TICKS);
}

#[test]
fn fighters_abandoned_are_told_only_with_the_layout_of_the_move_settled() {
    let mut view = faked(sol());
    view.sim
        .queue_event(FlightEvent::FighterNote(nova_sim::FighterNote::Abandoned(
            2,
        )));
    view.sim
        .queue_answer(Answer::Settled(nova_sim::Settled::default()));
    view.settle_script();
    assert_eq!(view.message(), None, "no layout from this settling");
    assert_eq!(
        view.drain().take_fighter_notes(),
        [nova_sim::FighterNote::Abandoned(2)],
        "left for the next arrival"
    );
}

#[test]
fn fighters_abandoned_by_a_settled_move_are_told_in_flight_and_not_docked() {
    // Read from the kept frame the settling left
    // (rdm:phase/view-sim-seam/phase-20-view-frame-fills-per-tick).
    for (frame, told) in [(sol(), true), (docked_at(128, Vec2::ZERO), false)] {
        let mut view = faked(frame.clone());
        view.sim
            .queue_event(FlightEvent::FighterNote(nova_sim::FighterNote::Abandoned(
                2,
            )));
        view.sim.queue_answer_telling(
            Answer::Settled(nova_sim::Settled::default()),
            frame,
            vec![FlightEvent::Session(nova_sim::SessionEvent::SystemChanged)],
        );
        view.settle_script();
        let said = view.message().map(str::to_owned);
        if told {
            assert_eq!(
                said,
                Some(crate::flight::escorts::fighters_abandoned_message(2)),
                "in flight"
            );
        } else {
            assert_eq!(said, None, "docked");
        }
    }
}

#[test]
fn docked_the_settling_stages_the_bar_as_its_scripts_ran() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    assert!(view.docked());
    view.sim
        .queue_answer(Answer::Settled(nova_sim::Settled::default()));
    view.settle_script();
    assert!(view.bar_steps.is_empty(), "no text, nothing staged");
    assert_eq!(sent_but_lines(&view), [FlightCommand::SettleScript]);
}

// The gates the commands are taken under (`FlightCommand::gate`).

#[test]
fn a_refused_gate_sends_no_travel_command() {
    let mut view = faked(sol());
    view.change_sim(|sim| sim.refuse(nova_sim::PlayerCommand::Land));
    view.land();
    assert_eq!(sent(&view), [], "L under a refused Land gate");
    view.change_sim(|sim| sim.refuse(nova_sim::PlayerCommand::Jump));
    view.jump_latch = false;
    view.jump_frame(true);
    assert_eq!(sent(&view), [], "J under a refused Jump gate");
    assert!(!view.jump_latch, "its latch left as it was");
    view.change_sim(|sim| sim.take(nova_sim::PlayerCommand::Jump));
    view.jump_frame(true);
    assert_eq!(sent(&view), [FlightCommand::Jump], "taken again");
}

#[test]
fn a_command_sent_under_a_refused_gate_is_refused_unsent() {
    let mut view = faked(sol());
    view.change_sim(|sim| sim.refuse(nova_sim::PlayerCommand::Jump));
    view.sim.queue_answer(Answer::Jumped(SystemId(131)));
    assert_eq!(view.send_gated(FlightCommand::Jump), Answer::Refused);
    assert_eq!(sent(&view), [], "nothing sent");
    assert_eq!(
        view.send_gated(FlightCommand::Arrive),
        Answer::Jumped(SystemId(131)),
        "a command no key gates is sent"
    );
    assert_eq!(sent(&view), [FlightCommand::Arrive]);
}

// The request's, the greeting's and the take-off's lines over the
// flight's names and date (rdm:phase/view-sim-seam/phase-12-view-rules-and-leaks).
// When the flight raises each, its fee and its notes are nova-sim's.

/// [`sol`]'s flight, which shows Earth (128) and the Moon (129), naming
/// the player's ship Kestrel.
fn sol_named() -> Faked {
    faked(FlightFrame {
        player_ship_name: Some("Kestrel".to_owned()),
        ..sol()
    })
}

#[test]
fn the_selecting_l_shows_the_requests_line_then_a_tick_near_shows_the_greeting() {
    let mut view = sol_named();
    let request = SimMessage::DockRequested {
        stellar: StellarId(129),
        kind: DockKind::Planet,
    };
    view.sim.queue_event(FlightEvent::Message(request.clone()));
    press_l(&mut view, requested(129));
    let line = "Moon traffic control reads you, Kestrel. Begin initial approach.";
    assert_eq!(view.message(), Some(line), "the picks' first outcomes");
    faked_lasts(&mut view, line, request.ticks());
    let greeting = SimMessage::DockCleared {
        stellar: StellarId(129),
        kind: DockKind::Planet,
        fee: None,
    };
    view.sim.queue_event(FlightEvent::Message(greeting.clone()));
    view.relay_messages();
    let said = "Cleared to land, Kestrel. Commence final approach.";
    faked_lasts(&mut view, said, greeting.ticks());
}

#[test]
fn take_off_says_the_greeting_for_240() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    let greeting = SimMessage::TookOff {
        stellar: StellarId(128),
        lead: 0,
    };
    view.sim.queue_event(FlightEvent::Message(greeting.clone()));
    view.sim.queue_answer_showing(departed(128), sol());
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(greeting.ticks(), 240);
    faked_lasts(&mut view, &take_off_greeting(0, "Earth", DATE), 240);
    assert_eq!(
        take_off_greeting(2, "Earth", "June 24th, 1177 NC"),
        "Taking off from Earth on June 24th, 1177 NC."
    );
    let leads: Vec<String> = (0..5)
        .map(|lead| take_off_greeting(lead, "X", "D"))
        .collect();
    assert_eq!(
        leads,
        [
            "Launching from X on D.",
            "Blasting off from X on D.",
            "Taking off from X on D.",
            "Leaving X on D.",
            "Departing X on D."
        ]
    );
}

#[test]
fn the_greeting_leads_the_pay_notes() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::TookOff {
            stellar: StellarId(128),
            lead: 4,
        }));
    view.sim
        .queue_event(FlightEvent::PayNote(nova_sim::PayNote::Defected(1)));
    view.sim.queue_answer_showing(departed(128), sol());
    view.take_off().expect("takes off");
    let said = format!("{}  {DEFECTED_ONE}", take_off_greeting(4, "Earth", DATE));
    faked_lasts(&mut view, &said, 240);
}

#[test]
fn a_landed_q_shows_the_take_offs_pay_after_its_message() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    view.sim
        .queue_event(FlightEvent::LeaveMessage("Off you go.".to_owned()));
    view.sim
        .queue_event(FlightEvent::PayNote(nova_sim::PayNote::Defected(1)));
    view.sim.queue_answer_showing(departed(128), sol());
    view.take_off().expect("takes off");
    let said = format!("Off you go.  {DEFECTED_ONE}");
    faked_lasts(&mut view, &said, SCRIPT_MESSAGE_TICKS);
}

#[test]
fn a_new_pilots_take_off_says_nothing() {
    let mut view = faked(docked_at(128, Vec2::ZERO));
    view.say_for(TOO_FAR_PLANET, TOO_FAR_TICKS);
    view.sim.queue_answer_showing(departed(128), sol());
    assert_eq!(view.take_off(), Some(StellarId(128)));
    assert_eq!(view.message(), None, "the flight said nothing");
}
