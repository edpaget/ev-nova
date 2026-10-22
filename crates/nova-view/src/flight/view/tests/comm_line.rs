//! The session's message line as the view keeps it in step: the lines
//! the view raises set it, Return clears it, and the session's own lines
//! relayed after a frame's steps leave it as the session set it.

use super::*;
use nova_sim::ai::{CommCall, Decision};

/// Every NPC flees itself, no threat to the player, and calls for help on
/// every think.
#[derive(Debug)]
struct CallsForHelp;

impl Behaviour for CallsForHelp {
    fn decide(
        &self,
        npc: &nova_sim::Npc,
        _around: &nova_sim::Surroundings,
        _chance: &mut dyn Chance,
    ) -> Goal {
        Goal::Flee(ShipRef::Npc(npc.id))
    }

    fn decision(
        &self,
        npc: &nova_sim::Npc,
        around: &nova_sim::Surroundings,
        chance: &mut dyn Chance,
    ) -> Decision {
        Decision {
            goal: self.decide(npc, around, chance),
            comm_call: Some(CommCall::Distress),
            ..Decision::default()
        }
    }
}

/// A flight with one independent trader in, calling for help, its first
/// tick run (the setup, which brings the ship and decides nothing).
fn called() -> View {
    let mut view =
        FlightView::new(trafficked(&[130], 1, 129, 1)).with_behaviour(Rc::new(CallsForHelp));
    view.tick(TICK);
    assert_eq!(npc_ids(&view).len(), 1, "the trader is in");
    view
}

/// Whether the message line shows the trader's call.
fn calling(view: &View) -> bool {
    view.message()
        .is_some_and(|text| text.starts_with("Ship 129:  "))
}

#[test]
fn the_trader_calls_while_nothing_shows() {
    let mut view = called();
    ticks(&mut view, 1);
    assert!(calling(&view), "{:?}", view.message());
}

#[test]
fn a_line_the_view_raises_keeps_the_call_off() {
    let mut view = called();
    tap(&mut view, MENU_KEY);
    assert_eq!(view.message(), Some(NO_ESCORTS));
    let line = view.session().expect("flying").comm_line();
    assert!(!line.free_for_calls(), "the view's line set it");
    ticks(&mut view, 3);
    assert_eq!(view.message(), Some(NO_ESCORTS), "no call over it");
}

#[test]
fn return_frees_the_line_for_the_call() {
    let mut view = called();
    tap(&mut view, MENU_KEY);
    assert_eq!(view.message(), Some(NO_ESCORTS));
    tap(&mut view, MENU_CLOSE_KEY);
    assert_eq!(view.message(), None, "acknowledged");
    ticks(&mut view, 2);
    assert!(calling(&view), "{:?}", view.message());
}

#[test]
fn a_line_cleared_by_name_frees_the_line_for_the_call() {
    let mut view = called();
    view.say_for(NO_ESCORTS, 250);
    view.clear_message(NO_ESCORTS);
    assert_eq!(view.message(), None);
    ticks(&mut view, 2);
    assert!(calling(&view), "{:?}", view.message());
}

/// How many frames of one step the line shows the call after `view`'s
/// frame of `steps` steps raised it on its first.
fn frames_left(steps: u32) -> u32 {
    let mut view = called();
    view.tick(TICK * steps);
    assert!(calling(&view));
    let mut frames = 0;
    while !view.session().expect("flying").comm_line().free_for_calls() {
        ticks(&mut view, 1);
        frames += 1;
        assert!(frames <= 240, "never free");
    }
    frames
}

#[test]
fn a_frame_of_several_steps_leaves_the_line_as_the_session_counted_it() {
    // The call (240 ticks) is raised on the frame's first step and
    // counted down on each: relayed after the steps, it is not set
    // again.
    assert_eq!(frames_left(1), 239);
    assert_eq!(frames_left(3), 237);
}
