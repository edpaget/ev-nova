//! The frame-rate key and its readout on `FakeFlight`: F12 toggles it
//! (`_HandlePlayer` @0x6e454-0x6e4bd), during a jump too, and each frame
//! drawn is counted and the readout said on the message line as
//! `_PlayGame` says it (@0x45bcd-0x45d57). The meter's own rules are
//! `flight::frame_rate`'s.

use super::faked_keys::{fleet, fleet_jumping};
use super::faked_screen::sent_count;
use super::faked_travel::shown_to_flight;
use super::*;
use crate::flight::frame_rate::CALCULATING_FPS;

/// One Mac tick, 1/60 s.
fn sixtieth() -> Duration {
    Duration::from_secs(1) / 60
}

/// Ticks `view` `n` times, 1/60 s each.
fn tick_sixtieths(view: &mut Faked, n: u32) {
    for _ in 0..n {
        view.tick(sixtieth());
    }
}

/// A view in Sol with the readout turned on.
fn shown() -> Faked {
    let mut view = faked(in_sol());
    tap(&mut view, FRAME_RATE_KEY);
    assert!(view.frame_rate_shown());
    view
}

#[test]
fn f12_toggles_the_readout_and_a_repeat_does_not() {
    let mut view = faked(in_sol());
    assert!(!view.frame_rate_shown(), "off at launch");
    tap(&mut view, FRAME_RATE_KEY);
    assert!(view.frame_rate_shown());
    view.input(&Input::Key {
        key: FRAME_RATE_KEY,
        pressed: true,
        repeat: true,
    });
    assert!(view.frame_rate_shown(), "a repeat does nothing");
    tap(&mut view, FRAME_RATE_KEY);
    assert!(!view.frame_rate_shown());
}

#[test]
fn f12_asks_the_session_for_its_command() {
    let view = faked(in_sol());
    assert_eq!(
        view.command_of(FRAME_RATE_KEY),
        Some(PlayerCommand::FrameRate)
    );
}

#[test]
fn f12_is_taken_while_the_ship_brakes_and_while_it_warps() {
    for warping in [false, true] {
        let mut view = fleet_jumping(&[], warping);
        tap(&mut view, FRAME_RATE_KEY);
        assert!(view.frame_rate_shown(), "warping {warping}");
        view.tick(sixtieth());
        assert_eq!(
            view.message(),
            Some(CALCULATING_FPS),
            "warping {warping}: the readout is said during the jump too"
        );
    }
}

#[test]
fn f12_does_nothing_when_the_flight_refuses_it() {
    let mut view = faked(in_sol());
    view.change_sim(|sim| sim.refuse(PlayerCommand::FrameRate));
    let from = sent_count(&view);
    tap(&mut view, FRAME_RATE_KEY);
    assert!(!view.frame_rate_shown());
    assert_eq!(&sent(&view)[from..], []);
}

#[test]
fn turning_it_on_frees_the_line_and_off_shows_an_empty_one_for_a_tick() {
    let mut view = faked(in_sol());
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    tap(&mut view, FRAME_RATE_KEY);
    assert_eq!(shown_to_flight(&view), [ESCORT_TICKS, 0], "on: 0");
    assert_eq!(view.message(), None, "the slot in step with the line");
    view.say_for(NO_ESCORTS, ESCORT_TICKS);
    tap(&mut view, FRAME_RATE_KEY);
    assert_eq!(
        shown_to_flight(&view),
        [ESCORT_TICKS, 0, ESCORT_TICKS, 1],
        "off: 1"
    );
    assert_eq!(view.message(), Some(NO_ESCORTS), "kept for that tick");
    view.tick(TICK);
    assert_eq!(view.message(), None, "then the line is empty");
}

#[test]
fn cutting_the_line_brings_back_none_already_run_out() {
    let mut view = faked(in_sol());
    view.say_for(NO_ESCORTS, 1);
    view.tick(TICK);
    assert_eq!(view.message(), None, "run out");
    view.cut_message(1);
    assert_eq!(view.message(), None, "still gone");
    assert_eq!(shown_to_flight(&view), [1, 1]);
}

#[test]
fn f12_makes_no_sound() {
    let mut view = faked(in_sol());
    view.take_sounds();
    tap(&mut view, FRAME_RATE_KEY);
    tap(&mut view, FRAME_RATE_KEY);
    assert_eq!(view.take_sounds(), []);
}

#[test]
fn the_next_frame_says_calculating_in_the_message_box() {
    let mut view = shown();
    view.tick(sixtieth());
    assert_eq!(view.message(), Some(CALCULATING_FPS));
    assert_eq!(shown_to_flight(&view).last(), Some(&100));
    assert!(
        super::faked_hud::flown(&view)
            .iter()
            .any(|command| *command == message_drawn(CALCULATING_FPS)),
        "drawn as every flight line is"
    );
}

#[test]
fn after_two_seconds_the_readout_says_the_frames_a_second() {
    let mut view = shown();
    view.tick(sixtieth());
    tick_sixtieths(&mut view, 120);
    assert_eq!(view.message(), Some(CALCULATING_FPS), "not past 120 yet");
    view.tick(sixtieth());
    assert_eq!(view.message(), Some("60 fps"));
    assert_eq!(shown_to_flight(&view).last(), Some(&999));
}

#[test]
fn turning_it_off_leaves_the_readout_for_a_tick() {
    let mut view = shown();
    view.tick(sixtieth());
    tap(&mut view, FRAME_RATE_KEY);
    assert_eq!(view.message(), Some(CALCULATING_FPS));
    assert_eq!(shown_to_flight(&view).last(), Some(&1));
    view.tick(TICK);
    assert_eq!(view.message(), None);
    tick_sixtieths(&mut view, 200);
    assert_eq!(view.message(), None, "nothing said while hidden");
}

#[test]
fn no_frames_are_counted_while_the_map_is_open() {
    let mut view = fleet(&[]);
    tap(&mut view, FRAME_RATE_KEY);
    view.tick(sixtieth());
    tap(&mut view, MAP_KEY);
    assert!(view.map_open);
    tick_sixtieths(&mut view, 200);
    view.close_map();
    view.tick(sixtieth());
    // One frame counted over 201 Mac ticks: the map's 200 counted none.
    assert_eq!(view.message(), Some("0 fps"));
}

#[test]
fn the_readout_goes_on_once_the_hold_has_expired() {
    let mut view = shown();
    view.sim.set_frame(FlightFrame {
        fate: PlayerFate::Holding { ticks_left: 0 },
        hold_expired: true,
        ..in_sol()
    });
    view.refresh_frame();
    view.tick(sixtieth());
    assert_eq!(view.message(), Some(CALCULATING_FPS));
    tick_sixtieths(&mut view, 121);
    assert_eq!(view.message(), Some("60 fps"));
}

#[test]
fn nothing_is_said_while_docked() {
    let mut view = shown();
    view.sim.set_frame(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    view.refresh_frame();
    let from = sent_count(&view);
    tick_sixtieths(&mut view, 200);
    assert_eq!(view.message(), None, "no readout line");
    assert!(
        !sent(&view)[from..]
            .iter()
            .any(|command| matches!(command, FlightCommand::CommShown(_))),
        "no line shown to the flight"
    );
}

#[test]
fn a_flight_built_with_it_shown_says_a_number_not_calculating() {
    let mut view = faked(in_sol()).with_frame_rate_shown(true);
    assert!(view.frame_rate_shown());
    tick_sixtieths(&mut view, 120);
    assert_eq!(view.message(), None, "no Calculating");
    view.tick(sixtieth());
    assert_eq!(view.message(), Some("60 fps"));
}

#[test]
fn coming_back_to_the_front_says_calculating_again() {
    let mut view = faked(in_sol()).with_frame_rate_shown(true);
    view.tick(sixtieth());
    view.focus_gained();
    view.tick(sixtieth());
    assert_eq!(view.message(), Some(CALCULATING_FPS));
}
