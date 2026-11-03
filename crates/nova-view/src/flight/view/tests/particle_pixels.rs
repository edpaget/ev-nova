//! The particles drawn at their whole pixels, by the scroll offset the last
//! frame left, as `_DrawParticlesOffscreen` (@0xc5a29-0xc5a3b) and
//! `_PostAnimateHandler` (@0x38f07-0x38f53) have it (rdm play-feel-8 phase
//! 13). The rules are the camera's
//! ([`Camera::particle_pixel`](crate::system::camera::Camera::particle_pixel))
//! and the `scroll_offset` module's, each pinned in its own tests. These
//! pin the wiring: which player the offset comes from, across steps, a
//! wrap and double time.

use nova_sim::combat::ParticleBurst;
use nova_sim::flight_frame::{PassReport, StepReport, StepRun};

use super::faked_world::world_list;
use super::stepping::{MOVE, SHIFT, STOOD, frame, place, stepping, wrap, wrapping};
use super::*;
use crate::flight::particle::Fixed;

/// The engine's anchor, the play area's centre at 1024 x 768.
const ANCHOR: Point = Point::new(415.0, 384.0);

/// One still particle at `at`, long-lived.
fn still_at(at: Vec2) -> CombatEvent {
    CombatEvent::Particles(ParticleBurst {
        at,
        speed: 0.0,
        variation: 0,
        life: (300, 300),
        color: 0x00FF_8000,
        count: 1,
        spread: 0.0,
    })
}

/// One particle at `at`, moving 2 a tick, long-lived.
fn moving_at(at: Vec2) -> CombatEvent {
    CombatEvent::Particles(ParticleBurst {
        speed: 2.0,
        ..match still_at(at) {
            CombatEvent::Particles(burst) => burst,
            _ => unreachable!(),
        }
    })
}

/// A pass that ends with the player at `at`, raising `combat`.
fn pass_to(at: Vec2, combat: Vec<CombatEvent>) -> PassReport {
    PassReport {
        combat,
        ..wrapping(at, None)
    }
}

/// `sol()` with the player at `at`.
fn sol_with_player_at(at: Vec2) -> FlightFrame {
    let mut frame = sol();
    frame.ships[0].position = at;
    frame
}

/// A frame of one step from `before` of `passes`, the flight left with the
/// player at `now`.
fn step_to(view: &mut Faked, before: Vec2, passes: Vec<PassReport>, now: Vec2) {
    view.sim.set_frame(sol_with_player_at(now));
    let run = StepRun {
        before: vec![(ShipRef::Player, place(before))],
        passes,
        ..StepRun::default()
    };
    frame(
        view,
        StepReport {
            steps: vec![run],
            stopped: false,
        },
    );
}

/// A frame of one double-time step from `before`: its first pass to
/// `first`, raising `combat_first`, which the frame is drawn on, and its
/// second to `second`, raising `combat_second`.
fn double_step_to(
    view: &mut Faked,
    before: Vec2,
    (first, combat_first): (Vec2, Vec<CombatEvent>),
    (second, combat_second): (Vec2, Vec<CombatEvent>),
) {
    view.sim.set_frame(sol_with_player_at(second));
    let doubled = |pass: PassReport| PassReport {
        double_time: true,
        ..pass
    };
    let run = StepRun {
        before: vec![(ShipRef::Player, place(before))],
        between_passes: Some(Box::new(sol_with_player_at(first))),
        passes: vec![
            doubled(pass_to(first, combat_first)),
            doubled(pass_to(second, combat_second)),
        ],
        ..StepRun::default()
    };
    frame(
        view,
        StepReport {
            steps: vec![run],
            stopped: false,
        },
    );
}

/// The dots `view` draws.
fn dots(view: &Faked) -> Vec<Point> {
    world_list(view)
        .iter()
        .filter_map(|command| match *command {
            DrawCommand::Dot { center, .. } => Some(center),
            _ => None,
        })
        .collect()
}

/// The centre of the pixel a particle on world pixel `whole` is plotted on
/// by the offset taken from the player at `from`: `whole - trunc(from -
/// anchor)`.
#[allow(clippy::cast_precision_loss)]
fn plotted(whole: (i64, i64), from: Vec2) -> Point {
    Point::new(
        whole.0 as f32 - (from.x - ANCHOR.x).trunc() + 0.5,
        whole.1 as f32 - (from.y - ANCHOR.y).trunc() + 0.5,
    )
}

/// The player's move a step: off a whole pixel on both axes.
const STRIDE: Vec2 = Vec2::new(3.7, -2.4);

/// Where the particle is laid: off a whole pixel.
const LAID: Vec2 = Vec2::new(50.6, -30.3);

/// A view whose player moved [`STRIDE`] a step for two steps from the
/// origin, a still particle laid at [`LAID`] on the first.
fn moved_twice() -> Faked {
    let mut view = stepping(catalog());
    let (p0, p1, p2) = (Vec2::ZERO, STRIDE, STRIDE * 2.0);
    step_to(&mut view, p0, vec![pass_to(p1, vec![still_at(LAID)])], p1);
    step_to(&mut view, p1, vec![pass_to(p2, vec![])], p2);
    view
}

#[test]
fn a_moving_players_particles_draw_by_the_offset_a_pass_old() {
    let mut view = moved_twice();
    view.alpha = 0.0;
    assert_eq!(view.anchor().point(view.view), ANCHOR);
    // Laid at (50, -30), truncated toward zero.
    let piece = view.effects.particles()[0];
    assert_eq!(piece.at.whole(), (50, -30));
    // By P(n-1) = (3.7, -2.4): 50 - trunc(-411.3), -30 - trunc(-386.4).
    let by_pass_old = plotted((50, -30), STRIDE);
    assert_eq!(by_pass_old, at(461.5, 356.5));
    assert_eq!(dots(&view), [by_pass_old]);
    let by_now = plotted((50, -30), STRIDE * 2.0);
    assert_ne!(by_now, by_pass_old, "P(n)'s pixel is another");
}

#[test]
fn between_steps_the_particles_move_with_the_camera_on_whole_pixels() {
    let mut view = moved_twice();
    view.alpha = 0.5;
    let shown = view.camera().center();
    assert_eq!(shown, at(STRIDE.x * 1.5, STRIDE.y * 1.5));
    // The held P(n-1) moved by the camera's motion: the camera itself.
    let [dot] = dots(&view)[..] else {
        panic!("one dot");
    };
    assert_eq!(dot, plotted((50, -30), Vec2::new(shown.x, shown.y)));
    assert_eq!(((dot.x - 0.5).fract(), (dot.y - 0.5).fract()), (0.0, 0.0));
}

#[test]
fn on_a_wrap_particles_draw_by_the_offset_before_it() {
    let mut view = stepping(catalog());
    let laid = STOOD + Vec2::new(-100.3, 20.6);
    let before = STOOD - MOVE;
    step_to(
        &mut view,
        before,
        vec![pass_to(STOOD, vec![still_at(laid)])],
        STOOD,
    );
    view.alpha = 1.0;
    let [drawn_before] = dots(&view)[..] else {
        panic!("one dot before the wrap");
    };
    let after = STOOD + SHIFT + MOVE;
    step_to(&mut view, STOOD, vec![wrapping(after, Some(wrap()))], after);
    view.alpha = 0.0;
    assert_eq!(view.camera().center(), point(STOOD + SHIFT), "carried");
    let whole = view.effects.particles()[0].at.whole();
    assert_eq!(
        dots(&view),
        [plotted(whole, STOOD)],
        "by the offset from before the wrap, uncarried"
    );
    assert_eq!(dots(&view), [drawn_before], "where it was, as it is still");
    let further = after + MOVE;
    step_to(&mut view, after, vec![pass_to(further, vec![])], further);
    let [gone] = dots(&view)[..] else {
        panic!("one dot");
    };
    assert!(
        gone.x > view.view.0 - STATUS_BAR_WIDTH,
        "past the play area, clipped, on the next frame: {gone:?}"
    );
}

#[test]
fn in_double_time_the_particles_draw_by_the_first_pass_a_frame_old_and_move_once_a_frame() {
    let mut view = stepping(catalog());
    let (first_a, second_a) = (Vec2::new(4.6, -1.3), Vec2::new(9.2, -2.6));
    let (first_b, second_b) = (Vec2::new(13.8, -3.9), Vec2::new(18.4, -5.2));
    let (laid, late) = (Vec2::new(60.4, 10.7), Vec2::new(-40.2, 30.9));
    double_step_to(
        &mut view,
        Vec2::ZERO,
        (first_a, vec![moving_at(laid)]),
        (second_a, vec![still_at(late)]),
    );
    double_step_to(&mut view, second_a, (first_b, vec![]), (second_b, vec![]));
    // Half a step in: the camera on where the step began.
    view.alpha = 0.5;
    assert_eq!(view.camera().center(), point(second_a));
    let pieces = view.effects.particles();
    let moving = pieces[0];
    assert_eq!(
        moving.at,
        Fixed::spawned(at(60.0, 10.0)).moved_by(moving.velocity),
        "moved once for the two passes since the frame it was laid"
    );
    assert_ne!(moving.velocity, Fixed::default());
    assert_eq!(
        pieces[1].at,
        Fixed::spawned(at(-40.0, 30.0)),
        "laid on the second pass, after the move: unmoved"
    );
    let by_first_a = |piece: &crate::flight::effects::Particle| plotted(piece.at.whole(), first_a);
    assert_eq!(
        dots(&view),
        [by_first_a(&pieces[0]), by_first_a(&pieces[1])]
    );
    assert_ne!(
        plotted(pieces[0].at.whole(), second_a),
        by_first_a(&pieces[0]),
        "not by where the step began"
    );
}
