//! `Screen::input`'s routing, filters and gates, and `Screen::tick`'s and
//! `Screen::draw`'s order, on `FakeFlight`, now that the `Screen` impl is
//! generic (rdm:phase/view-sim-seam/phase-13-lifecycle-at-the-edge): the
//! tests kept real for this phase, converted under their own names. What
//! each command does, and the jump the flight flies, are the session's,
//! named in the commits that moved them.

use super::faked_flight::last_controls;
use super::faked_travel::{jump_now, lit};
use super::faked_world::world;
use super::*;
use crate::SoundPrefs;
use nova_sim::flight_frame::FlightEvent;
use nova_sim::{MissionInfoPress, SessionEvent, StellarPick, TargetPick};

/// A view of Sol's flight, its ship alone and intact at the centre, the
/// course map drawing Sol and preselecting it.
pub(super) fn in_flight() -> Faked {
    let mut view = world(catalog(), sol());
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.map_preselection = Some(SystemId(130));
    view
}

/// [`in_flight`]'s view braking and turning for a jump its J began.
pub(super) fn braking() -> Faked {
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
pub(super) fn streaking() -> Faked {
    let mut view = in_flight();
    jump_now(&mut view);
    view.tick(ms(300));
    assert!(view.jump_effect().is_some(), "warping");
    view
}

/// The view braking for a jump and the one 300 ms into its warp, as
/// [`through_a_jump`] has them over a real flight. Boxed: two views are
/// too large for the stack together.
pub(super) fn faked_through_a_jump() -> [Box<Faked>; 2] {
    [Box::new(braking()), Box::new(streaking())]
}

/// Which stage of the jump `view` is in, for a failing assertion.
pub(super) fn faked_stage(view: &Faked) -> &'static str {
    if view.jump_effect().is_none() {
        "braking"
    } else {
        "warping"
    }
}

/// The commands `view` sent since `from` of them, the message line's
/// and the settings' left out.
pub(super) fn sent_since(view: &Faked, from: usize) -> Vec<FlightCommand> {
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
pub(super) fn sent_count(view: &Faked) -> usize {
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

/// Brings `view`'s warp to its end: the frame says the end is due, and
/// the flight answers the arrival showing [`in_alpha`], telling the
/// system entered as it arrives.
pub(super) fn warp_ends(view: &mut Faked) {
    view.sim.set_frame(FlightFrame {
        warp_end_due: true,
        ..under_way(true)
    });
    view.refresh_frame();
    view.sim.queue_answer_telling(
        Answer::Arrived {
            system: SystemId(131),
            combat: Vec::new(),
        },
        in_alpha(),
        vec![FlightEvent::Session(SessionEvent::SystemChanged)],
    );
}

/// The stellar picks `view` sent since `from` of its commands.
pub(super) fn picks_since(view: &Faked, from: usize) -> Vec<FlightCommand> {
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

// Double time and the pointer.

/// Whether the last step was given the double-time key on.
fn double_time_stepped(view: &Faked) -> bool {
    view.sim.inputs().last().expect("a step").double_time_key
}

#[test]
fn double_time_reaches_the_session_on_the_next_tick_and_when_set() {
    let mut view = in_flight();
    view.set_double_time(true);
    assert_eq!(sent_count(&view), 1, "the setting sent when set");
    assert_eq!(view.sim.applied(), [FlightCommand::SetDoubleTimeKey(true)]);
    view.tick(TICK);
    assert!(double_time_stepped(&view), "and on each tick's steps");
}

#[test]
fn double_time_survives_the_events_that_let_go_of_the_flight_keys() {
    let mut view = in_flight().with_double_time(true);
    tap(&mut view, MAP);
    tap(&mut view, MAP);
    view.tick(TICK);
    assert!(double_time_stepped(&view), "the map");
    jump_now(&mut view);
    warp_ends(&mut view);
    view.tick(TICK);
    view.tick(TICK);
    assert!(double_time_stepped(&view), "a jump");
    view.sim
        .queue_answer(Answer::Hailed(nova_sim::Hailed::Stellar(StellarId(140))));
    tap(&mut view, HAIL_KEY);
    view.tick(TICK);
    assert!(double_time_stepped(&view), "a hail");
    assert!(view.double_time());
}

#[test]
fn cancelling_the_pointer_forgets_the_cursor_until_the_next_move() {
    use super::controls::{NOWHERE, cursor_sprites};
    let mut view = in_flight();
    view.input(&Input::PointerMoved(NOWHERE));
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, NOWHERE)]);
    assert_eq!(view.cursor_at(), Some(NOWHERE));
    view.cancel_pointer();
    assert_eq!(cursor_sprites(&drawn(&view)), [], "not at the old point");
    assert_eq!(view.cursor_at(), None);
    let other = Point::new(60.0, 420.0);
    view.input(&Input::PointerMoved(other));
    assert_eq!(cursor_sprites(&drawn(&view)), [(0, other)]);
    assert_eq!(view.cursor_at(), Some(other));
}

#[test]
fn flight_hides_the_system_pointer_unless_the_map_is_open_or_it_has_no_cursor() {
    let mut view = in_flight();
    assert!(view.hides_pointer());
    view.input(&key(MAP_KEY, true));
    assert!(!view.hides_pointer(), "the map");
    let no_cursor = FakeCatalog {
        cursor_sheet: Err("no spïn 650".to_owned()),
        ..catalog()
    };
    assert!(!world(no_cursor, sol()).hides_pointer());
    let mut fake = FakeFlight::new();
    fake.fail("no chär to start from");
    let failed = FlightView::over(
        Rc::new(catalog()),
        fake,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert!(!failed.hides_pointer(), "a flight that never started");
}

#[test]
fn a_click_on_a_ship_targets_it_during_a_jump() {
    let mut view = braking();
    let from = sent_count(&view);
    let over = Point::new(300.0, 200.0);
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: over,
        });
    }
    assert!(
        matches!(
            sent_since(&view, from).as_slice(),
            [FlightCommand::ClickInSpace { .. }]
        ),
        "the click reaches the flight, which targets the ship under it: {:?}",
        sent_since(&view, from)
    );
}

#[test]
fn a_slot_key_held_during_a_jump_fades_the_cursor_only_by_its_press() {
    use super::controls::NOWHERE;
    /// The cursor's alpha two ticks after `k` is pressed and held.
    fn alpha_holding(mut view: Faked, k: Key) -> Option<u8> {
        view.input(&Input::PointerMoved(NOWHERE));
        view.input(&key(k, true));
        view.tick(TICK);
        view.tick(TICK);
        drawn(&view)
            .iter()
            .find_map(|command| match *unclipped(command) {
                DrawCommand::Sprite { image, tint, .. } if image.id == 651 => Some(tint.a),
                _ => None,
            })
    }
    for (view, other) in faked_through_a_jump()
        .into_iter()
        .zip(faked_through_a_jump())
    {
        let stage = faked_stage(&view);
        assert_eq!(
            alpha_holding(*view, STELLAR_SLOT_KEYS[1][1]),
            alpha_holding(*other, Key::Char('q')),
            "{stage}"
        );
    }
}

// The nav keys held across the arrival.

/// The nav blocks `view` sent since `from` of its commands.
fn nav_blocks_since(view: &Faked, from: usize) -> Vec<FlightCommand> {
    sent_since(view, from)
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                FlightCommand::NavModeKeys { .. } | FlightCommand::SelectNextSystem
            )
        })
        .collect()
}

/// Ticks `view` through its warp's end, asserting that no nav block was
/// sent before or on the arrival's tick, then once more, giving what that
/// tick sent; then once more again, asserting it sends nothing more.
fn faked_after_arrival(view: &mut Faked) -> Vec<FlightCommand> {
    faked_after_arrival_answering(view, |_| {})
}

/// [`faked_after_arrival`], `answer` queuing the flight's answers to the
/// commands after the arrival's.
fn faked_after_arrival_answering(
    view: &mut Faked,
    answer: impl FnOnce(&mut Faked),
) -> Vec<FlightCommand> {
    let from = sent_count(view);
    view.tick(TICK);
    warp_ends(view);
    answer(view);
    view.tick(TICK);
    assert_eq!(
        nav_blocks_since(view, from),
        [],
        "not through the jump nor on arrival"
    );
    let acted_from = sent_count(view);
    view.tick(TICK);
    let acted = nav_blocks_since(view, acted_from);
    let again = sent_count(view);
    view.tick(TICK);
    assert_eq!(nav_blocks_since(view, again), [], "once only");
    acted
}

const H: FlightCommand = FlightCommand::NavModeKeys {
    nav_off: false,
    h: true,
};

#[test]
fn h_pressed_before_j_and_held_through_the_jump_acts_the_tick_after_arrival() {
    let mut view = in_flight();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    assert_eq!(nav_blocks_since(&view, 0), [H], "before the jump");
    jump_now(&mut view);
    assert_eq!(faked_after_arrival(&mut view), [H]);
}

#[test]
fn h_held_through_a_jump_that_cues_the_next_hop_only_beeps() {
    // What the block does after the arrival, the cue kept and its beep,
    // is the session's; the view sends it once, the tick after.
    let mut view = streaking();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    assert_eq!(faked_after_arrival(&mut view), [H]);
}

#[test]
fn h_first_pressed_during_the_jump_and_held_acts_after_arrival() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        if stage == "braking" {
            // The jump itself begins under the key held.
            view.input(&key(HYPERSPACE_MODE_KEY, true));
            lit(&mut view, 0.0);
        } else {
            view.input(&key(HYPERSPACE_MODE_KEY, true));
        }
        assert_eq!(faked_after_arrival(&mut view), [H], "{stage}");
    }
}

#[test]
fn h_pressed_during_the_jump_and_let_go_before_arrival_does_nothing_after() {
    let mut view = streaking();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.tick(TICK);
    view.input(&key(HYPERSPACE_MODE_KEY, false));
    assert_eq!(faked_after_arrival(&mut view), []);
}

#[test]
fn nav_off_held_through_the_jump_turns_the_nav_off_after_arrival() {
    let mut view = streaking();
    view.input(&key(NAV_OFF_KEY, true));
    assert_eq!(
        faked_after_arrival(&mut view),
        [FlightCommand::NavModeKeys {
            nav_off: true,
            h: false
        }]
    );
}

#[test]
fn h_and_nav_off_held_through_the_jump_act_once_together_after_arrival() {
    let mut view = streaking();
    view.input(&key(HYPERSPACE_MODE_KEY, true));
    view.input(&key(NAV_OFF_KEY, true));
    assert_eq!(
        faked_after_arrival(&mut view),
        [FlightCommand::NavModeKeys {
            nav_off: true,
            h: true
        }]
    );
}

/// Answers `view`'s next command, Hyper Select's, with Alpha Centauri's
/// first link selected in mode 3, which shows the floating map while `\`
/// is held.
fn hyper_selects(view: &mut Faked) {
    view.sim.queue_answer_showing(
        Answer::System(Some(SystemId(130))),
        FlightFrame {
            nav_mode: NavMode::Hyperspace(Some(SystemId(130))),
            ..in_alpha()
        },
    );
}

#[test]
fn backslash_first_pressed_during_the_jump_and_held_acts_after_arrival() {
    for mut view in faked_through_a_jump() {
        let stage = faked_stage(&view);
        view.input(&key(HYPER_SELECT_KEY, true));
        if stage == "braking" {
            lit(&mut view, 0.0);
        }
        assert_eq!(
            faked_after_arrival_answering(&mut view, hyper_selects),
            [FlightCommand::SelectNextSystem],
            "{stage}"
        );
        assert_eq!(floating_alpha(&view), Some(255), "{stage}: the map up");
    }
}

#[test]
fn backslash_pressed_before_j_and_held_through_does_nothing_after() {
    let mut view = in_flight();
    view.input(&key(HYPER_SELECT_KEY, true));
    view.tick(TICK);
    assert_eq!(
        nav_blocks_since(&view, 0),
        [FlightCommand::SelectNextSystem],
        "before the jump"
    );
    jump_now(&mut view);
    assert_eq!(faked_after_arrival(&mut view), [], "latched through it");
}

#[test]
fn backslash_let_go_and_pressed_again_during_the_jump_acts_after_arrival() {
    let mut view = in_flight();
    view.input(&key(HYPER_SELECT_KEY, true));
    view.tick(TICK);
    jump_now(&mut view);
    view.input(&key(HYPER_SELECT_KEY, false));
    view.tick(TICK);
    view.input(&key(HYPER_SELECT_KEY, true));
    assert_eq!(
        faked_after_arrival_answering(&mut view, hyper_selects),
        [FlightCommand::SelectNextSystem]
    );
}

// The player's fate's gates.

/// [`in_flight`]'s view, its flight refusing `gates`, as a breakup or an
/// explosion has the session refuse them.
fn refusing(gates: &[PlayerCommand]) -> Faked {
    let mut view = in_flight();
    for &gate in gates {
        view.sim.refuse(gate);
    }
    view
}

/// The gates a breakup shuts: the map, landing and mission info
/// (`_HandlePlayer`'s breakup-timer gates at 0x6a7b3, 0x6a8cb and
/// 0x7044b).
const BREAKING_UP: [PlayerCommand; 3] = [
    PlayerCommand::Map,
    PlayerCommand::Land,
    PlayerCommand::MissionInfo,
];

/// Every gate, shut once the player has exploded (the dead branch,
/// 0x6870d-0x6888c).
const EXPLODED: [PlayerCommand; 13] = [
    PlayerCommand::Land,
    PlayerCommand::Board,
    PlayerCommand::Hail,
    PlayerCommand::Map,
    PlayerCommand::Jump,
    PlayerCommand::Jettison,
    PlayerCommand::Eject,
    PlayerCommand::MissionInfo,
    PlayerCommand::PlayerInfo,
    PlayerCommand::SelectSystem,
    PlayerCommand::Nav,
    PlayerCommand::Target,
    PlayerCommand::Escort,
];

#[test]
fn mission_info_and_landing_do_nothing_while_the_player_breaks_up() {
    let mut view = refusing(&BREAKING_UP);
    view.take_sounds();
    tap(&mut view, MISSION_INFO_KEY);
    assert_eq!(view.take_sounds(), [], "silently (@0x70445, @0x7044f)");
    tap(&mut view, LAND_KEY);
    assert_eq!(sent_since(&view, 0), [], "nothing sent");
    assert!(view.take_mission_info_request().is_none());
    assert_eq!(view.take_landing(), None);
    assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS));
}

#[test]
fn an_exploded_player_takes_no_command() {
    let mut view = refusing(&EXPLODED);
    for k in [MAP_KEY, TARGET_KEY, LAND_KEY, MISSION_INFO_KEY, MENU_KEY] {
        tap(&mut view, k);
    }
    view.tick(TICK);
    assert_eq!(sent_since(&view, 0), [], "nothing sent");
    assert!(!view.map_open());
    assert!(view.take_mission_info_request().is_none());
    assert_eq!(view.take_landing(), None);
    assert_ne!(view.message(), Some(NO_ACTIVE_MISSIONS));
    assert_ne!(view.message(), Some(NO_ESCORTS));
    assert!(!view.escort_menu().is_open());
}

/// Home (`Key::Other` on this platform) has no binding, so it does
/// nothing in every state, including the dead one the original reads it
/// in (0x6884c, `_ResetPlayer(1)`, left out). This pins the unbound key,
/// not the dead-state lockout, which `an_exploded_player_takes_no_command`
/// and `a_click_picks_no_stellar_once_the_player_has_exploded` cover.
#[test]
fn home_is_unbound_whether_flying_or_exploded() {
    for build in [in_flight, || refusing(&EXPLODED)] {
        let mut view = build();
        assert_eq!(view.input(&key(Key::Other, true)), ScreenAction::None);
        assert_eq!(view.input(&key(Key::Other, false)), ScreenAction::None);
        view.tick(TICK);
        assert_eq!(sent_since(&view, 0), []);
        assert!(view.take_mission_info_request().is_none());
        assert_eq!(view.take_landing(), None);
        assert!(view.take_hail().is_none());
        assert!(view.take_boarding().is_none());
    }
}

#[test]
fn the_map_does_not_open_while_the_player_breaks_up() {
    let mut view = refusing(&BREAKING_UP);
    tap(&mut view, MAP_KEY);
    assert!(!view.map_open());
    let steps = view.sim.steps();
    view.tick(TICK);
    assert_eq!(view.sim.steps(), steps + 1, "the flight goes on, unpaused");
}

#[test]
fn targeting_and_escort_keys_still_work_while_the_player_breaks_up() {
    let mut view = refusing(&BREAKING_UP);
    tap(&mut view, TARGET_KEY);
    assert_eq!(
        sent_since(&view, 0),
        [FlightCommand::SelectTarget(TargetPick::Next)]
    );
    tap(&mut view, MENU_KEY);
    assert_eq!(
        view.message(),
        Some(NO_ESCORTS),
        "the escort menu's key is taken"
    );
}

#[test]
fn a_refused_nav_stops_the_held_slot_keys_once_the_player_has_exploded() {
    for k in [
        Key::Function(1),
        STELLAR_SLOT_KEYS[1][0],
        NEAREST_STELLAR_KEY,
    ] {
        let mut view = refusing(&EXPLODED);
        view.input(&key(k, true));
        for _ in 0..3 {
            view.tick(TICK);
        }
        view.input(&key(k, false));
        tap(&mut view, k);
        view.tick(TICK);
        assert_eq!(picks_since(&view, 0), [], "{k:?}: no pick");
    }
}

/// A left click at `at`.
pub(super) fn faked_click(view: &mut Faked, at: Point) {
    for pressed in [true, false] {
        view.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

#[test]
fn a_click_still_picks_a_stellar_while_the_player_breaks_up() {
    let mut view = refusing(&BREAKING_UP);
    faked_click(&mut view, Point::new(300.0, 200.0));
    assert!(
        matches!(
            sent_since(&view, 0).as_slice(),
            [FlightCommand::ClickInSpace { .. }]
        ),
        "the nav's gate is open: {:?}",
        sent_since(&view, 0)
    );
}

#[test]
fn a_click_picks_no_stellar_once_the_player_has_exploded() {
    let mut view = refusing(&EXPLODED);
    faked_click(&mut view, Point::new(300.0, 200.0));
    assert_eq!(sent_since(&view, 0), [], "the nav's gate is shut");
}

#[test]
fn npcs_stand_still_while_the_map_is_open_or_the_ship_is_landed() {
    let mut view = in_flight();
    view.tick(TICK);
    tap(&mut view, MAP);
    let steps = view.sim.steps();
    for _ in 0..10 {
        view.tick(TICK);
    }
    assert_eq!(view.sim.steps(), steps, "the map is open: no steps");
    tap(&mut view, MAP);
    view.sim.frame_mut().landed = Some(StellarId(128));
    view.refresh_frame();
    let steps = view.sim.steps();
    for _ in 0..10 {
        view.tick(TICK);
    }
    assert_eq!(
        view.sim.steps(),
        steps + 10,
        "landed, the flight's steps hold its NPCs still"
    );
}

#[test]
fn the_fire_keys_pressed_and_let_go_during_a_jump_fire_nothing() {
    let mut view = braking();
    for k in [FIRE_KEY, SECONDARY_KEY] {
        view.input(&key(k, true));
    }
    view.tick(TICK);
    assert_eq!(
        view.sim.inputs().last().expect("a step").fire,
        (true, true),
        "held: the flight holds its fire through the jump"
    );
    for k in [FIRE_KEY, SECONDARY_KEY] {
        view.input(&key(k, false));
    }
    lit(&mut view, 0.0);
    warp_ends(&mut view);
    view.tick(TICK);
    view.tick(TICK);
    assert_eq!(
        view.sim.inputs().last().expect("a step").fire,
        (false, false),
        "let go before the arrival"
    );
}

// The nav area, the HUD and the stellars.

#[test]
fn tab_in_a_flight_that_never_started_does_nothing() {
    let mut fake = FakeFlight::new();
    fake.fail("no chär to start from");
    let mut view = FlightView::over(
        Rc::new(catalog()),
        fake,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert_eq!(view.input(&key(Key::Tab, true)), ScreenAction::None);
    assert_eq!(view.failure(), Some("no chär to start from"));
    assert!(view.scene().is_none());
}

#[test]
fn with_the_map_open_a_click_goes_to_the_map_not_the_stellars() {
    let mut view = in_flight();
    tap(&mut view, MAP);
    let from = sent_count(&view);
    faked_click(&mut view, Point::new(415.0 + 40.0, 384.0));
    tap(&mut view, MAP);
    assert!(!view.map_open());
    assert!(
        !sent_since(&view, from)
            .iter()
            .any(|command| matches!(command, FlightCommand::ClickInSpace { .. })),
        "no click in space: {:?}",
        sent_since(&view, from)
    );
}

#[test]
fn the_hud_cadence_and_blink_advance_through_a_jumps_warp() {
    let mut view = in_flight();
    jump_now(&mut view);
    // Mark the snapshot, then tick through the streak: the radar is
    // taken afresh, and the blink flips, every 8 ticks.
    let marker = crate::flight::radar::RadarScene {
        player: Point::new(-1.0, -1.0),
        ..crate::flight::radar::RadarScene::default()
    };
    let mut refreshes = Vec::new();
    for tick in 1..=32 {
        view.radar_scene = Some(marker.clone());
        let blink = view.hud_cadence.blink_on();
        view.tick(TICK);
        let retaken = view.radar_scene.as_ref() != Some(&marker);
        assert_eq!(retaken, view.hud_cadence.blink_on() != blink, "tick {tick}");
        if retaken {
            refreshes.push(tick);
        }
    }
    assert!(
        matches!(
            view.jump_effect().map(JumpEffect::phase),
            Some(JumpPhase::Warp)
        ),
        "still streaking"
    );
    assert!(refreshes.len() >= 3, "{refreshes:?}");
    let gaps: Vec<u32> = refreshes.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(gaps.iter().all(|&gap| gap == 8), "{refreshes:?}");
}

#[test]
fn the_sound_prefs_reach_the_session_when_set() {
    let mut view = in_flight();
    view.set_sound_prefs(QUIET);
    assert!(view.alert_quiet());
    view.set_sound_prefs(SoundPrefs {
        sound: false,
        ..SoundPrefs::default()
    });
    assert!(view.alert_quiet(), "sound off is quiet");
    view.set_sound_prefs(SoundPrefs::default());
    assert!(!view.alert_quiet());
    assert_eq!(
        view.sim.applied(),
        [
            FlightCommand::SetSoundQuiet(true),
            FlightCommand::SetSoundQuiet(true),
            FlightCommand::SetSoundQuiet(false),
        ],
        "each told the flight"
    );
}

#[test]
fn a_missing_alert_icon_draws_nothing_and_says_so_once() {
    let iconless = FakeCatalog {
        alert_icon: None,
        ..armed(2, &[])
    };
    let mut fake = FakeFlight::new();
    fake.set_frame(sol());
    let mut view = FlightView::over(
        Rc::new(iconless),
        fake,
        &Rulebook::default(),
        SharedChance::default(),
    )
    .with_sound_prefs(QUIET);
    let missing =
        Diagnostic::Unreadable("no cïcn 18000: the Red Alert icon is not drawn".to_owned());
    let told = |view: &mut Faked| {
        let diagnostics = view.take_diagnostics();
        diagnostics.iter().filter(|&d| *d == missing).count()
    };
    assert_eq!(told(&mut view), 1);
    view.sim.frame_mut().hud.red_alert = Some(nova_sim::AlertIcon::Red);
    for _ in 0..10 {
        view.tick(TICK);
        assert_eq!(super::nav::icons_drawn(&view), []);
    }
    assert_eq!(told(&mut view), 0, "once");
    let mut whole_fake = FakeFlight::new();
    whole_fake.set_frame(sol());
    let mut whole = FlightView::over(
        Rc::new(catalog()),
        whole_fake,
        &Rulebook::default(),
        SharedChance::default(),
    );
    assert_eq!(told(&mut whole), 0, "the default catalog has it");
}

#[test]
fn a_pilot_flown_with_a_galaxy_overlay_shows_it_on_the_course_map() {
    // 132 hidden by its `Visibility`, as the load evaluates it.
    let hiding = FakeCatalog {
        visibility: vec![nova_sim::catalog::SystemVisibility {
            id: SystemId(132),
            visibility: nova_sim::Test::parse("b50"),
        }],
        ..catalog()
    };
    let galaxy = SessionBuilder::flying(&hiding).build().galaxy().clone();
    let mut fake = FakeFlight::new();
    fake.set_frame(sol());
    fake.map_mut().galaxy = Some(galaxy);
    let view = FlightView::over(
        Rc::new(hiding),
        fake,
        &Rulebook::default(),
        SharedChance::default(),
    );
    let on_the_course_map = |id| view.course_map().model().system(SystemId(id)).is_some();
    assert!(!on_the_course_map(132), "the overlay's");
    assert!(on_the_course_map(131));
}

#[test]
fn the_status_bar_follows_the_ship_type_on_the_next_frame() {
    let mut view = super::faked_hud::barred(governed(catalog(), 128, 129), sol());
    assert_eq!(bar_shown(&view), Some(700));
    view.sim.frame_mut().hud.interface_government = Some(nova_sim::GovtId(150));
    view.sim
        .queue_event(FlightEvent::Session(SessionEvent::ShipChanged));
    assert_eq!(bar_shown(&view), Some(700), "not before the frame");
    view.tick(TICK);
    assert_eq!(bar_shown(&view), Some(705), "the ship's change caught up");
    view.tick(TICK);
    assert_eq!(
        *view.catalog().bars_asked.borrow(),
        [128, 129],
        "129's read once"
    );
}

/// The Moon's frame shown.
fn moon_shown(view: &Faked) -> u16 {
    sprites(&drawn(view))[1].0.frame
}

/// The Moon's animation state as `view` keeps it while Sol is not shown.
fn moon_kept(view: &Faked) -> Option<u16> {
    view.kept_frames
        .get(StellarId(129))
        .map(|frames| frames.shown())
}

/// `view`'s flight arriving back in Sol from Alpha Centauri, its warp
/// ended and the system entered told.
fn back_in_sol(view: &mut Faked) {
    view.sim.set_frame(FlightFrame {
        warp_end_due: true,
        system: SystemId(131),
        stellars: vec![record::stellar(StellarId(140))],
        ..under_way(true)
    });
    view.refresh_frame();
    view.sim.queue_answer_telling(
        Answer::Arrived {
            system: SystemId(130),
            combat: Vec::new(),
        },
        sol(),
        vec![FlightEvent::Session(SessionEvent::SystemChanged)],
    );
}

#[test]
fn a_real_jump_away_and_back_resumes_the_frame_shown_on_leaving() {
    let mut view = in_flight();
    jump_now(&mut view);
    for _ in 0..5 {
        view.tick(TICK);
    }
    let last = moon_shown(&view);
    warp_ends(&mut view);
    view.tick(TICK);
    // The tick the jump ends in steps the stellars before the flight
    // arrives (`step_stellars` runs before the steps): one frame on from
    // the last drawn in Sol.
    let left = moon_kept(&view).expect("kept as Sol was let go");
    assert_eq!(left, (last + 1) % 4);
    view.tick(crate::flight::jump::FADE_IN_FOR);
    view.tick(TICK);
    lit(&mut view, 0.0);
    for _ in 0..3 {
        view.tick(TICK);
    }
    assert_eq!(moon_kept(&view), Some(left), "still while away");
    back_in_sol(&mut view);
    view.tick(TICK);
    // Sol is laid out on the tick the jump ends, after that tick's step:
    // the frame drawn on arrival is the one it was left on.
    assert_eq!(moon_shown(&view), left, "resumed on arrival");
    view.tick(TICK);
    assert_eq!(moon_shown(&view), (left + 1) % 4, "and on from there");
}
