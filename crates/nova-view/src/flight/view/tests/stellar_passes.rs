//! The stellar passes the flight raises, on [`FakeFlight`]: replayed on
//! the scene of the system entered, before the next frame is drawn, the
//! entry's before the frames' reset and the rest after it. Which passes
//! the session raises, and when, is nova-sim's (its `stellar_passes`
//! module).

use std::time::Duration;

use nova_sim::flight_frame::{FlightEvent, FrameStellar};
use nova_sim::gate_sprite::StellarPass;
use nova_sim::{DrawnSheet, NeverFires, SessionEvent};

use super::faked_world::world_list;
use super::stellar_looks::{
    caught_up, changed, faked_from, looking, moon_frames, moon_on, raising, showing, sol,
    tick_stellars,
};
use super::*;
use crate::system::animation::Frames;
use crate::system::catalog::AnimationData;

/// A stellar of 6 frames with this `AnimDelay` and `Frame0Bias`.
fn animated(id: i16, delay: i16, frame0_bias: i16) -> StellarContents {
    StellarContents {
        animation: AnimationData {
            delay,
            frame0_bias,
            flags2: 0,
        },
        ..stellar(id, "Animated", (200, 100), 6)
    }
}

/// Sol, with `more`, as a frame shows it.
fn sol_and(more: &[i16]) -> Vec<FrameStellar> {
    let mut stellars = sol();
    stellars.extend(more.iter().map(|&id| looking(id, DrawnSheet::Type, false)));
    stellars
}

fn passing(view: &mut Faked, passes: &[StellarPass]) {
    for &pass in passes {
        view.sim.queue_event(FlightEvent::StellarPass(pass));
    }
}

/// The animation state of stellar `id` of the scene shown.
fn frames_of(view: &Faked, id: i16) -> Frames {
    let scene = view.scene().expect("laid out");
    scene
        .stellars()
        .iter()
        .find(|stellar| stellar.id == StellarId(id))
        .expect("shown")
        .frames
}

/// `from` after `ticks` ticks over 6 frames of a stellar with this delay
/// and bias.
fn ticked(mut from: Frames, delay: i16, frame0_bias: i16, ticks: usize) -> Frames {
    let data = AnimationData {
        delay,
        frame0_bias,
        flags2: 0,
    };
    let six = NonZeroU16::new(6).expect("non-zero");
    for _ in 0..ticks {
        from.step(data, false, six, six, &mut NeverFires);
    }
    from
}

fn reset(mut from: Frames) -> Frames {
    from.take_off_reset();
    from
}

#[test]
fn a_take_offs_still_passes_test_each_threshold_and_add_to_no_timer() {
    let mut catalog = catalog();
    catalog
        .sol_more
        .extend([animated(150, 0, 0), animated(151, 2, 0)]);
    let mut view = faked_from(catalog, showing(130, sol_and(&[150, 151])));
    tick_stellars(&mut view);
    let kept = frames_of(&view, 151);
    raising(&mut view, &[SessionEvent::StellarFramesReset]);
    passing(&mut view, &[StellarPass::Still; 2]);
    caught_up(&mut view);
    assert_eq!(
        frames_of(&view, 150).shown(),
        1,
        "no delay: next 0, then frame 1"
    );
    assert_eq!(
        frames_of(&view, 151),
        reset(kept),
        "frame 0 shown, its timer kept"
    );
}

#[test]
fn a_gate_exits_passes_step_the_new_scene_around_its_reset_and_not_the_old_one() {
    let mut catalog = catalog();
    catalog.proxima_frames = 4;
    catalog.sol_more.push(animated(150, 5, 0));
    let proxima = || showing(131, vec![looking(140, DrawnSheet::Type, false)]);
    let mut view = faked_from(catalog, proxima());
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    let left = frames_of(&view, 140);
    assert_eq!(left.shown(), 2);
    changed(&mut view, |frame| {
        *frame = showing(130, sol_and(&[150]));
    });
    raising(
        &mut view,
        &[
            SessionEvent::SystemChanged,
            SessionEvent::StellarFramesReset,
        ],
    );
    passing(
        &mut view,
        &[StellarPass::Entry, StellarPass::Ticked, StellarPass::Ticked],
    );
    caught_up(&mut view);
    assert_eq!(
        frames_of(&view, 150),
        ticked(Frames::after_reset(), 5, 0, 3),
        "never shown: its timer 3, its frames the reset's"
    );
    changed(&mut view, |frame| *frame = proxima());
    raising(&mut view, &[SessionEvent::SystemChanged]);
    caught_up(&mut view);
    assert_eq!(
        frames_of(&view, 140),
        reset(left),
        "zeroed as it was kept, and not stepped"
    );
}

#[test]
fn a_gate_entrys_pass_reads_the_frame_the_stellar_was_left_on_before_the_reset() {
    let mut catalog = catalog();
    catalog.sol_more.push(animated(150, 3, 2));
    let mut view = faked_from(catalog, showing(130, sol_and(&[150])));
    // Delay 3, bias 2: frame 1 at tick 6, then 2 ticks on it.
    for _ in 0..8 {
        tick_stellars(&mut view);
    }
    assert_eq!(frames_of(&view, 150).shown(), 1);
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Type, false)]);
    });
    raising(&mut view, &[SessionEvent::SystemChanged]);
    caught_up(&mut view);
    changed(&mut view, |frame| {
        *frame = showing(130, sol_and(&[150]));
    });
    raising(
        &mut view,
        &[
            SessionEvent::SystemChanged,
            SessionEvent::StellarFramesReset,
        ],
    );
    passing(
        &mut view,
        &[StellarPass::Entry, StellarPass::Ticked, StellarPass::Ticked],
    );
    caught_up(&mut view);
    assert_eq!(
        frames_of(&view, 150),
        ticked(Frames::after_reset(), 3, 2, 2),
        "the entry's fires on frame 1, at the delay: zeroing first would \
         have frame 0's threshold and leave its timer at 5"
    );
}

#[test]
fn a_pass_alone_steps_the_scene_shown_before_the_next_draw() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    passing(&mut view, &[StellarPass::Ticked]);
    caught_up(&mut view);
    assert_eq!(moon_on(&view), 1);
}

#[test]
fn game_entrys_passes_reach_the_scene_on_a_tick_of_no_time_with_no_step_run() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    view.enter_game(|sim| {
        for _ in 0..3 {
            sim.queue_event(FlightEvent::StellarPass(StellarPass::Ticked));
        }
        true
    });
    view.tick(Duration::ZERO);
    assert_eq!(view.sim.steps_asked(), 0, "no step run");
    assert_eq!(moon_on(&view), 3);
}

#[test]
fn a_frozen_stellars_frame_stays_shown_through_a_pass() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    changed(&mut view, |frame| frame.stellars[1].frozen = true);
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    let before = moon_frames(&view);
    passing(&mut view, &[StellarPass::Ticked]);
    caught_up(&mut view);
    assert_eq!(moon_on(&view), 2, "drawn as it was");
    assert_ne!(moon_frames(&view), before, "stepped behind it");
}

/// Sol with hypergate 141 of 6 frames, as the session has it on `frame`.
fn with_a_gate(frame: u16) -> Faked {
    let mut catalog = catalog();
    catalog.sol_more.push(StellarContents {
        animation: AnimationData {
            delay: 1,
            frame0_bias: 0,
            flags2: nova_sim::gate::HYPERGATE,
        },
        ..stellar(141, "Gate", (200, 100), 6)
    });
    let mut stellars = sol();
    stellars.push(FrameStellar {
        gate_frame: Some(frame),
        ..looking(141, DrawnSheet::Type, false)
    });
    faked_from(catalog, showing(130, stellars))
}

/// The frame of hypergate 141's sheet drawn.
fn gate_on(view: &Faked) -> Vec<u16> {
    sprites(&world_list(view))
        .into_iter()
        .filter(|(image, _)| image.id == 1141)
        .map(|(image, _)| image.frame)
        .collect()
}

#[test]
fn no_pass_changes_the_frame_a_hypergate_draws() {
    let mut view = with_a_gate(3);
    assert_eq!(gate_on(&view), [3]);
    passing(&mut view, &[StellarPass::Ticked, StellarPass::Still]);
    caught_up(&mut view);
    assert_eq!(gate_on(&view), [3]);
}

#[test]
fn the_exit_gate_draws_the_sessions_frame_after_a_reset_with_a_layout() {
    let mut view = with_a_gate(0);
    let exit = looking(141, DrawnSheet::Type, false);
    changed(&mut view, |frame| {
        let mut stellars = sol();
        stellars.push(FrameStellar {
            gate_frame: Some(5),
            ..exit
        });
        *frame = showing(130, stellars);
    });
    raising(
        &mut view,
        &[
            SessionEvent::SystemChanged,
            SessionEvent::StellarFramesReset,
        ],
    );
    passing(
        &mut view,
        &[StellarPass::Entry, StellarPass::Ticked, StellarPass::Ticked],
    );
    caught_up(&mut view);
    assert_eq!(gate_on(&view), [5]);
}
