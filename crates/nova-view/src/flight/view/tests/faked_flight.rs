//! Helpers for flight's key, step and ship-drawing component tests on
//! [`FakeFlight`] (rdm:phase/view-sim-seam/phase-8-hud-and-target-frame):
//! keys held as flight's input tracks them, a tick's step with the
//! player's places a step apart, and the frame's player record. Kept
//! apart from `tests/mod.rs`, which is large.

use super::faked_world::{in_sol_with, the_player_at, world};
use super::*;
use nova_sim::flight_frame::{FrameShip, StepReport, StepRun};

/// A view over a flight scripted to show the player's ship 128 at rest
/// at Sol's centre, facing up, read from `catalog`.
pub(super) fn at_rest(catalog: FakeCatalog) -> Faked {
    world(catalog, in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)]))
}

/// Presses or lets go of `k`, as flight's input takes a key: the cursor
/// told, then the key tracked as held.
pub(super) fn hold(view: &mut Faked, k: Key, pressed: bool) {
    let input = key(k, pressed);
    view.cursor_input(&input);
    view.track_held(&input);
}

/// A tick of flight with one step, as `Screen::tick` runs it: the clock
/// and the stellars moved on a tick, the step given the keys held, its
/// report with the player where it was, the frame then showing the
/// player as `now` says (as it was without), the step drawn whole
/// (alpha 0), the cursor stepped and the HUD advanced.
pub(super) fn tick_to(view: &mut Faked, now: Option<FrameShip>) {
    view.step_stellars(TICK);
    let before = view.frame.player().place();
    view.sim.queue_report(StepReport {
        steps: vec![StepRun {
            before: vec![(ShipRef::Player, before)],
            ..StepRun::default()
        }],
        stopped: false,
    });
    if let Some(now) = now {
        view.sim.frame_mut().ships[0] = now;
    }
    view.step_frame(1, view.anchor());
    view.alpha = 0.0;
    view.step_cursor(1);
    view.advance_hud();
}

/// [`tick_to`] `n` times, the player where it is.
pub(super) fn ticked(view: &mut Faked, n: u32) {
    for _ in 0..n {
        tick_to(view, None);
    }
}

/// The controls the last step was given.
pub(super) fn last_controls(view: &Faked) -> Controls {
    view.sim.inputs().last().expect("a step").controls
}

/// The player's ship 128 at `at` facing `heading`.
pub(super) fn ship_at(at: Vec2, heading: f32) -> FrameShip {
    the_player_at(at, heading)
}
