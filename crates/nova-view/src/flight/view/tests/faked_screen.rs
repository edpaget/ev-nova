//! `Screen::input`'s routing, filters and gates, and `Screen::tick`'s and
//! `Screen::draw`'s order, on `FakeFlight`, now that the `Screen` impl is
//! generic (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge): the
//! tests kept real for this phase, converted under their own names. What
//! each command does, and the jump the flight flies, are the session's,
//! named in the commits that moved them.

use super::faked_flight::last_controls;
use super::faked_travel::{jump_now, lit, sol, under_way};
use super::faked_world::world;
use super::*;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{MissionInfoPress, SessionEvent, StellarPick, TargetPick};

/// A view of Sol's flight, its ship alone and intact at the centre, the
/// course map drawing Sol and preselecting it.
fn in_flight() -> Faked {
    let mut view = world(catalog(), sol());
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.map_preselection = Some(SystemId(130));
    view
}

/// [`in_flight`]'s view braking and turning for a jump its J began.
fn braking() -> Faked {
    let mut view = in_flight();
    view.sim
        .queue_answer_showing(Answer::Jumped(SystemId(131)), under_way(false));
    tap(&mut view, JUMP);
    assert!(
        view.jump_under_way() && view.jump_effect().is_none(),
        "braking"
    );
    view
}

/// [`in_flight`]'s view 300 ms into its jump's warp.
fn streaking() -> Faked {
    let mut view = in_flight();
    jump_now(&mut view);
    view.tick(ms(300));
    assert!(view.jump_effect().is_some(), "warping");
    view
}

/// The view braking for a jump and the one 300 ms into its warp, as
/// [`through_a_jump`] has them over a real flight. Boxed: two views are
/// too large for the stack together.
fn faked_through_a_jump() -> [Box<Faked>; 2] {
    [Box::new(braking()), Box::new(streaking())]
}

/// Which stage of the jump `view` is in, for a failing assertion.
fn faked_stage(view: &Faked) -> &'static str {
    if view.jump_effect().is_none() {
        "braking"
    } else {
        "warping"
    }
}

/// The commands `view` sent since `from` of them, the message line's
/// and the settings' left out.
fn sent_since(view: &Faked, from: usize) -> Vec<FlightCommand> {
    view.sim.applied()[from..]
        .iter()
        .filter(|command| {
            !matches!(
                command,
                FlightCommand::CommShown(_)
                    | FlightCommand::CommAcknowledged
                    | FlightCommand::CommTicked
                    | FlightCommand::SetWindowWidth(_)
                    | FlightCommand::SetDoubleTimeKey(_)
                    | FlightCommand::SetSoundQuiet(_)
            )
        })
        .cloned()
        .collect()
}

/// How many commands `view` has sent.
fn sent_count(view: &Faked) -> usize {
    view.sim.applied().len()
}

// The course map.

#[test]
fn m_opens_the_map_over_everything_and_lets_go_of_the_flight_keys() {
    let mut view = in_flight();
    view.input(&key(Key::Up, true));
    assert_eq!(view.input(&key(MAP, true)), ScreenAction::None);
    assert!(view.map_open());
    let mut map = DrawList::new();
    view.course_map().draw(&mut map);
    let list = drawn(&view);
    let over = list.len() - map.len();
    assert_eq!(
        list.iter().skip(over).collect::<Vec<_>>(),
        map.iter().collect::<Vec<_>>(),
        "the map over everything"
    );
    // Holding M, or letting it go, keeps it open.
    view.input(&held(MAP));
    view.input(&key(MAP, false));
    assert!(view.map_open());
    view.input(&key(MAP, true));
    assert!(!view.map_open());
    view.tick(TICK);
    assert_eq!(last_controls(&view), Controls::default(), "Up was let go");
}

#[test]
fn while_the_map_is_open_flight_is_paused_and_keys_go_to_the_map() {
    let mut view = in_flight();
    view.input(&key(Key::Up, true));
    view.tick(TICK);
    tap(&mut view, MAP);
    let steps = view.sim.steps();
    let fitted = view.course_map().transform();
    for _ in 0..30 {
        view.tick(TICK);
    }
    assert_eq!(view.sim.steps(), steps, "paused: no step asked");
    let from = sent_count(&view);
    view.input(&key(Key::Left, true));
    assert_ne!(
        view.course_map().transform(),
        fitted,
        "Left scrolls the map"
    );
    view.input(&key(LAND, true));
    view.input(&key(JUMP, true));
    assert_eq!(
        sent_since(&view, from),
        [],
        "neither L nor J reaches the flight"
    );
    assert_eq!(view.message(), None);
    assert_eq!(view.take_landing(), None);
    assert_eq!(view.jump_effect(), None);
    tap(&mut view, MAP);
    view.tick(TICK);
    assert_eq!(
        last_controls(&view).turn,
        Controls::default().turn,
        "Left went to the map"
    );
}

#[test]
fn the_course_map_makes_no_map_sound_on_its_own_input_or_ms_repeats() {
    let mut view = in_flight();
    tap(&mut view, MAP);
    view.take_sounds();
    view.input(&held(MAP));
    view.input(&key(MAP, false));
    for k in [Key::Up, Key::Down, Key::Left, Key::Right] {
        tap(&mut view, k);
    }
    let somewhere = faked_on_map(&mut view, 131);
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: somewhere,
        });
    }
    assert!(view.map_open());
    assert_eq!(map_sounds(&mut view), []);
}

#[test]
fn a_refused_m_makes_no_map_sound() {
    let mut view = in_flight();
    view.sim.refuse(PlayerCommand::Map);
    view.take_sounds();
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open(), "the map's gate refused");
    assert_eq!(map_sounds(&mut view), [], "refused");
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        view.take_sounds();
        tap(&mut view, MAP);
        assert!(!view.map_open(), "{stage}");
        assert_eq!(map_sounds(&mut view), [], "{stage}");
    }
}

#[test]
fn cancelling_the_pointer_abandons_a_drag_on_the_open_map() {
    let mut view = in_flight();
    tap(&mut view, MAP);
    let empty = Point::new(250.0, 150.0);
    let left = |pressed| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at: empty,
    };
    view.input(&left(true));
    view.cancel_pointer();
    let place = view.course_map().place().expect("in flight").scroll();
    let from = sent_count(&view);
    view.input(&Input::PointerMoved(Point::new(300.0, 150.0)));
    view.input(&left(false));
    assert_eq!(
        view.course_map().place().expect("in flight").scroll(),
        place,
        "no drag"
    );
    assert_eq!(sent_since(&view, from), [], "no click on the map");
    assert!(view.map_open());
}

// The jump's filter.

#[test]
fn a_repeat_or_release_of_j_does_nothing() {
    let mut view = in_flight();
    view.input(&held(JUMP));
    view.input(&key(JUMP, false));
    assert_eq!(sent_since(&view, 0), [], "no J");
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.message(), None);
}

#[test]
fn keys_are_ignored_while_the_ship_turns_to_jump() {
    let mut view = braking();
    let from = sent_count(&view);
    for k in [Key::Up, Key::Left, Key::Down, MAP, LAND, JUMP] {
        assert_eq!(view.input(&key(k, true)), ScreenAction::None);
    }
    assert_eq!(sent_since(&view, from), [], "no command");
    assert!(!view.map_open());
    assert_eq!(view.message(), None);
    assert_eq!(view.jump_effect(), None);
    assert_eq!(view.take_landing(), None);
    view.tick(TICK);
    assert_eq!(
        last_controls(&view).thrust,
        THRUST.thrust,
        "the keys held reach the flight, which flies its jump"
    );
}

#[test]
fn keys_are_ignored_while_the_jump_plays() {
    let mut view = streaking();
    view.input(&key(Key::Up, true));
    view.input(&key(Key::Up, false));
    let from = sent_count(&view);
    view.input(&key(Key::Left, true));
    view.input(&key(MAP, true));
    view.input(&key(LAND, true));
    view.input(&key(JUMP, true));
    assert_eq!(sent_since(&view, from), [], "no command");
    assert!(!view.map_open());
    assert_eq!(view.message(), None);
    assert_eq!(view.take_landing(), None);
    view.input(&key(Key::Left, false));
    view.tick(TICK);
    assert_eq!(last_controls(&view), Controls::default(), "nothing held");
}

#[test]
fn keys_are_ignored_while_the_old_system_fades_out() {
    let mut view = in_flight();
    jump_now(&mut view);
    lit(&mut view, 0.5);
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeOut(_))
    ));
    let from = sent_count(&view);
    for k in [Key::Up, Key::Left, MAP, LAND, JUMP] {
        assert_eq!(view.input(&key(k, true)), ScreenAction::None);
        assert_eq!(view.input(&key(k, false)), ScreenAction::None);
    }
    assert_eq!(sent_since(&view, from), [], "no command");
    assert!(!view.map_open());
    assert_eq!(view.message(), None);
    assert_eq!(view.take_landing(), None);
}

#[test]
fn the_target_keys_are_taken_through_a_jump() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        tap(&mut view, TARGET_KEY);
        view.input(&key(Key::Shift, true));
        tap(&mut view, TARGET_KEY);
        view.input(&key(Key::Shift, false));
        view.input(&key(Key::Alt, true));
        tap(&mut view, NEAREST_KEY);
        view.input(&key(Key::Alt, false));
        tap(&mut view, NEAREST_KEY);
        assert_eq!(
            sent_since(&view, from),
            [
                FlightCommand::SelectTarget(TargetPick::Next),
                FlightCommand::SelectTarget(TargetPick::Previous),
                FlightCommand::SelectTarget(TargetPick::Nearest),
                FlightCommand::SelectTarget(TargetPick::NearestThreat),
            ],
            "{stage}"
        );
    }
}

#[test]
fn option_backquote_clears_the_target_through_a_jump_and_nav_off_is_ignored() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        tap(&mut view, NAV_OFF_KEY);
        view.tick(TICK);
        assert_eq!(sent_since(&view, from), [], "{stage}: Nav Off ignored");
        view.input(&key(Key::Alt, true));
        tap(&mut view, NAV_OFF_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(
            sent_since(&view, from),
            [FlightCommand::ClearTarget],
            "{stage}: Option-`"
        );
    }
}

#[test]
fn weapon_select_and_safety_are_taken_through_a_jump() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        tap(&mut view, SELECT_KEY);
        view.input(&key(Key::Alt, true));
        tap(&mut view, SELECT_KEY);
        view.input(&key(Key::Alt, false));
        assert_eq!(
            sent_since(&view, from),
            [
                FlightCommand::SelectWeapon { backwards: false },
                FlightCommand::SelectWeapon { backwards: true },
            ],
            "{stage}"
        );
        view.input(&key(DESELECT_KEY, true));
        view.tick(TICK);
        let input = view.sim.inputs().last().copied().expect("a step");
        assert!(input.weapon_safety, "{stage}: Weapon Safety held");
    }
}

#[test]
fn board_is_taken_through_a_jump() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        tap(&mut view, BOARD_KEY);
        assert_eq!(sent_since(&view, from), [FlightCommand::Board], "{stage}");
        assert_eq!(view.take_boarding(), None, "{stage}: refused");
    }
}

#[test]
fn hail_is_refused_silently_through_a_jump() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        tap(&mut view, HAIL_KEY);
        assert_eq!(
            sent_since(&view, from),
            [FlightCommand::Hail { option: false }],
            "{stage}"
        );
        assert_eq!(view.take_hail(), None, "{stage}");
        assert_eq!(view.take_offer(), None, "{stage}");
        assert_eq!(view.message(), None, "{stage}");
    }
}

#[test]
fn command_k_jettisons_through_a_jump() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        view.input(&key(Key::Command, true));
        tap(&mut view, JETTISON_KEY);
        view.input(&key(Key::Command, false));
        assert_eq!(
            sent_since(&view, from),
            [FlightCommand::Jettison { all: true }],
            "{stage}"
        );
    }
}

#[test]
fn nav_and_screen_keys_are_ignored_through_a_jump() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        let from = sent_count(&view);
        for k in [
            LAND,
            MAP,
            JUMP,
            MISSION_INFO_KEY,
            HYPERSPACE_MODE_KEY,
            HYPER_SELECT_KEY,
            NAV_OFF_KEY,
            STELLAR_SLOT_KEYS[0][0],
            STELLAR_SLOT_KEYS[0][1],
            NEAREST_STELLAR_KEY,
        ] {
            tap(&mut view, k);
        }
        view.input(&key(Key::Command, true));
        tap(&mut view, NEAREST_STELLAR_COMMAND_KEY);
        view.input(&key(Key::Command, false));
        // The slot block's frame, still in the jump, takes nothing.
        view.tick(TICK);
        assert_eq!(sent_since(&view, from), [], "{stage}: nothing sent");
        assert!(!view.map_open(), "{stage}");
        assert_eq!(view.take_landing(), None, "{stage}");
        assert!(view.take_mission_info_request().is_none(), "{stage}");
        assert_eq!(view.message(), None, "{stage}");
    }
}

#[test]
fn a_repeat_or_release_of_the_hyper_select_key_does_not_cycle() {
    let mut view = in_flight();
    view.input(&held(HYPER_SELECT));
    view.input(&key(HYPER_SELECT, false));
    view.tick(TICK);
    assert_eq!(sent_since(&view, 0), [], "no press, no cycle");
    tap(&mut view, HYPER_SELECT);
    view.input(&held(HYPER_SELECT));
    view.input(&key(HYPER_SELECT, false));
    view.tick(TICK);
    assert_eq!(
        sent_since(&view, 0),
        [FlightCommand::SelectNextSystem],
        "the press, once"
    );
}

#[test]
fn hyper_select_with_the_map_open_changes_nothing() {
    let mut view = in_flight();
    tap(&mut view, MAP);
    let from = sent_count(&view);
    tap(&mut view, HYPER_SELECT);
    view.tick(TICK);
    assert!(view.map_open());
    assert_eq!(
        sent_since(&view, from),
        [FlightCommand::CycleMapSelection],
        "the map's own cycle, not Hyper Select"
    );
}

#[test]
fn i_during_a_jump_does_nothing_and_says_nothing() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        view.take_sounds();
        let from = sent_count(&view);
        tap(&mut view, MISSION_INFO_KEY);
        assert_eq!(sent_since(&view, from), [], "{stage}");
        assert!(view.take_mission_info_request().is_none(), "{stage}");
        assert_eq!(view.message(), None, "{stage}");
        assert_eq!(view.take_sounds(), [], "{stage}");
    }
}

#[test]
fn i_in_the_escape_pod_does_nothing_and_says_nothing() {
    let mut view = in_flight();
    view.sim.refuse(PlayerCommand::MissionInfo);
    view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(sent_since(&view, 0), [], "the gate refused (@0x7042e)");
    assert!(view.take_mission_info_request().is_none());
    assert_eq!(view.message(), None);
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn i_while_disabled_is_taken() {
    let mut view = in_flight();
    view.sim.frame_mut().ships[0].condition = nova_sim::Condition::Disabled;
    view.refresh_frame();
    view.sim
        .queue_answer(Answer::MissionInfo(MissionInfoPress::NoActiveMissions));
    view.sim.queue_event(FlightEvent::Sound(SimSound::Refused));
    view.sim
        .queue_event(FlightEvent::Message(SimMessage::NoActiveMissions));
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(sent_since(&view, 0), [FlightCommand::PressMissionInfo]);
    assert_eq!(view.take_mission_info_request(), None);
    assert_eq!(view.message(), Some(NO_ACTIVE_MISSIONS));
    assert_eq!(view.take_sounds(), [Sound::Sim(SimSound::Refused)]);
}

// The keys held across the arrival.

/// The frame Alpha Centauri (131) shows once arrived: the player at its
/// centre, Proxima (140) its one stellar, no jump under way.
fn in_alpha() -> FlightFrame {
    FlightFrame {
        system: SystemId(131),
        stellars: vec![record::stellar(StellarId(140))],
        ..sol()
    }
}

/// Brings `view`'s warp to its end: the frame says the end is due, and
/// the flight answers the arrival showing [`in_alpha`], telling the
/// system entered.
fn warp_ends(view: &mut Faked) {
    view.sim.set_frame(FlightFrame {
        warp_end_due: true,
        ..under_way(true)
    });
    view.refresh_frame();
    view.sim.queue_answer_showing(
        Answer::Arrived {
            system: SystemId(131),
            combat: Vec::new(),
        },
        in_alpha(),
    );
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::SystemChanged));
}

/// The stellar picks `view` sent since `from` of its commands.
fn picks_since(view: &Faked, from: usize) -> Vec<FlightCommand> {
    sent_since(view, from)
        .into_iter()
        .filter(|command| matches!(command, FlightCommand::SelectStellar(_)))
        .collect()
}

#[test]
fn a_slot_key_pressed_in_a_jump_and_held_selects_after_it() {
    let mut view = streaking();
    view.input(&key(STELLAR_SLOT_KEYS[0][0], true));
    let from = sent_count(&view);
    warp_ends(&mut view);
    view.tick(TICK);
    assert!(sent_since(&view, from).contains(&FlightCommand::Arrive));
    assert_eq!(
        picks_since(&view, from),
        [],
        "the arrival frame is the jump's"
    );
    view.tick(TICK);
    assert_eq!(
        picks_since(&view, from),
        [FlightCommand::SelectStellar(StellarPick::Slot(0))],
        "the first after it"
    );
}

#[test]
fn a_slot_key_held_from_before_a_jump_does_nothing_after_it() {
    let mut view = in_flight();
    view.input(&key(STELLAR_SLOT_KEYS[0][0], true));
    view.tick(TICK);
    assert_eq!(
        picks_since(&view, 0),
        [FlightCommand::SelectStellar(StellarPick::Slot(0))],
        "selected before the jump"
    );
    jump_now(&mut view);
    let from = sent_count(&view);
    warp_ends(&mut view);
    for _ in 0..4 {
        view.tick(TICK);
    }
    assert_eq!(picks_since(&view, from), [], "latched through it");
}

#[test]
fn a_flight_key_held_through_arrival_acts_at_once() {
    let mut view = streaking();
    view.input(&key(Key::Up, true));
    warp_ends(&mut view);
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(
        last_controls(&view).thrust,
        THRUST.thrust,
        "thrusting the first tick after"
    );
}

#[test]
fn a_key_held_when_j_is_pressed_stays_held() {
    let mut view = in_flight();
    view.input(&key(Key::Up, true));
    jump_now(&mut view);
    warp_ends(&mut view);
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(last_controls(&view).thrust, THRUST.thrust, "still held");
}

#[test]
fn fire_held_through_a_jump_fires_only_after_arrival() {
    let mut view = braking();
    view.input(&key(FIRE_KEY, true));
    view.tick(TICK);
    assert!(
        view.sim.inputs().last().expect("a step").fire.0,
        "the trigger held reaches the flight, which holds its fire"
    );
    // The jump itself begins.
    lit(&mut view, 0.0);
    assert!(view.jump_effect().is_some(), "warping");
    warp_ends(&mut view);
    view.tick(TICK);
    view.tick(TICK);
    assert!(view.sim.inputs().last().expect("a step").fire.0, "after it");
}

// The fades.

/// The fade quad's alpha in what `view` draws, if any.
fn faked_fade_alpha(view: &Faked) -> Option<u8> {
    fade(&drawn(view)).map(|(_, color)| color.a)
}

/// [`streaking`]'s view arrived in Alpha Centauri, 50 ms into the
/// arrival's fade-in.
fn fading_in() -> Faked {
    let mut view = streaking();
    lit(&mut view, 1.0);
    warp_ends(&mut view);
    view.tick(TICK);
    view.tick(ms(50));
    assert!(matches!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(_))
    ));
    view
}

#[test]
fn the_ship_flies_under_the_arrival_fade_in() {
    let mut view = fading_in();
    let before = faked_fade_alpha(&view).expect("fading in");
    let steps = view.sim.steps();
    view.input(&key(Key::Up, true));
    for _ in 0..3 {
        view.tick(TICK);
    }
    assert_eq!(view.sim.steps(), steps + 3, "the flight steps");
    assert_eq!(last_controls(&view).thrust, THRUST.thrust, "under the keys");
    let after = faked_fade_alpha(&view).expect("still fading in");
    assert!(after < before, "{after} < {before}");
}

#[test]
fn the_ship_flies_under_a_hypergates_fade_in() {
    use super::faked_landing::{arrived, at_the_hypergate, came_out_in_alpha, close_on};
    let mut view = at_the_hypergate();
    came_out_in_alpha(&mut view, SimMessage::ExitedHypergate(SystemId(131)));
    close_on(&mut view, Some(SystemId(131)), arrived());
    assert_eq!(
        view.jump_effect().map(JumpEffect::phase),
        Some(JumpPhase::FadeIn(0.0))
    );
    let steps = view.sim.steps();
    view.input(&key(Key::Up, true));
    for _ in 0..3 {
        view.tick(TICK);
    }
    assert_eq!(view.sim.steps(), steps + 3, "the flight steps");
    assert_eq!(last_controls(&view).thrust, THRUST.thrust, "under the keys");
    assert!(
        faked_fade_alpha(&view).is_some_and(|alpha| alpha < u8::MAX),
        "still fading in"
    );
}

#[test]
fn the_arrival_fade_plays_on_and_shows_over_the_map() {
    let mut view = fading_in();
    tap(&mut view, MAP);
    assert!(view.map_open(), "the map opens under the fade");
    let list = drawn(&view);
    let (at_fade, color) = fade(&list).expect("the fade, over the map");
    assert_eq!(at_fade, list.len() - 1, "drawn last");
    let before = color.a;
    view.tick(ms(100));
    let after = faked_fade_alpha(&view).expect("fading on with the map open");
    assert!(after < before, "{after} < {before}");
    view.tick(crate::flight::jump::FADE_IN_FOR);
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert_eq!(view.jump_effect(), None, "the fade played out");
    assert_eq!(fade(&drawn(&view)), None);
}

#[test]
fn the_stellars_go_on_animating_while_the_jump_plays() {
    let mut view = in_flight();
    jump_now(&mut view);
    let moon = |view: &Faked| sprites(&drawn(view))[1].0.frame;
    let before = moon(&view);
    view.tick(TICK);
    assert_eq!(moon(&view), (before + 1) % 4, "a frame a tick");
    view.tick(TICK * 2);
    assert_eq!(moon(&view), (before + 3) % 4);
    assert!(view.jump_effect().is_some());
}
