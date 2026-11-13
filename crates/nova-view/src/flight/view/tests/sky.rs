//! The flight's layered sky in the view (see the view's `sky` module):
//! by [`RuleKey::LayeredStarfield`]'s other reading, the world's first
//! draw is one [`DrawCommand::Sky`] seen from over the camera's centre,
//! the original's stars neither scattered nor stepped, dimmed by the
//! system's murk and none in a system of negative murk; its eye moving
//! with the ship and not jumping at a wrap.

use std::cell::Cell;

use nova_sim_frame::flight_frame::StepRun;
use nova_sim_frame::player_wrap::PlayerWrap;

use crate::flight::catalog::{StarDot, StarDots};
use crate::flight::stars::murk_level;
use crate::sky::SkyPose;

use super::stepping::{MOVE, SHIFT, STOOD, frame, place, wrap, wrapping};
use super::*;

/// Draws in the middle of each range, counting every draw it is asked
/// for.
struct Tally(Rc<Cell<u32>>);

impl Chance for Tally {
    fn fires(&mut self, _percent: u8) -> bool {
        self.0.set(self.0.get() + 1);
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        self.0.set(self.0.get() + 1);
        n / 2
    }
}

/// [`catalog`] with no stars' sheet but stand-in dots, as the new game's
/// data.
fn dotted() -> FakeCatalog {
    let dot = |side, grey| StarDot {
        side,
        color: Color::rgba(grey, grey, grey, 255),
    };
    FakeCatalog {
        star_dots: StarDots::new(vec![dot(1, 100), dot(3, 240)]),
        ..catalog()
    }
}

/// The rules with the layered sky's other reading.
fn layered() -> Rulebook {
    Rulebook::default().with_override(RuleKey::LayeredStarfield, RuleSource::Bible)
}

/// A view by `rules` of the player alone at `at` in Sol of murk `murk`,
/// its effects drawn on a [`Tally`] counting into `draws`.
fn faked(rules: &Rulebook, at: Vec2, murk: i16, draws: &Rc<Cell<u32>>) -> Faked {
    let mut frame = in_sol_with(vec![the_player_at(at, 0.0)]);
    frame.system_murk = murk;
    let mut view = super::faked_world::world_by(dotted(), frame, rules);
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Tally(Rc::clone(draws))));
    view.effects_chance = SharedChance::new(chance);
    view
}

/// What a catch-up of `view` redraws, the status bar the session's.
fn caught_up(view: &mut Faked) {
    view.catch_up_then(|view, redrawn| view.redraw_scene_then(redrawn, |_| {}, |_| {}));
}

/// The skies `view`'s world draws.
fn skies(view: &Faked) -> Vec<SkyPose> {
    super::faked_world::world_list(view)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Sky { sky } => Some(*sky),
            _ => None,
        })
        .collect()
}

/// The one sky `view`'s world draws.
fn the_sky(view: &Faked) -> SkyPose {
    let skies = skies(view);
    assert_eq!(skies.len(), 1, "{skies:?}");
    skies[0]
}

#[test]
fn by_the_layered_reading_the_worlds_first_draw_is_the_systems_sky_from_the_camera() {
    let draws = Rc::new(Cell::new(0));
    let mut view = faked(&layered(), Vec2::new(30.0, -50.0), 0, &draws);
    caught_up(&mut view);
    let list = super::faked_world::world_list(&view);
    let camera = view.camera();
    assert_eq!(
        list.iter().next(),
        Some(&DrawCommand::Sky {
            sky: SkyPose {
                seed: u64::from(view.frame.system.0.cast_unsigned()),
                eye: camera.center(),
                anchor: camera.anchor(),
                level: 32,
            }
        })
    );
    assert_eq!(skies(&view).len(), 1);
    assert!(
        !list
            .iter()
            .any(|command| is_star(command) || matches!(command, DrawCommand::OrFill { .. })),
        "no star's sprite or dot"
    );
}

#[test]
fn by_the_layered_reading_no_star_is_scattered_and_the_chance_draws_nothing_for_them() {
    let draws = Rc::new(Cell::new(0));
    let mut view = faked(&layered(), Vec2::ZERO, 0, &draws);
    caught_up(&mut view);
    assert!(view.stars.stars().iter().all(|star| !star.shown));
    assert_eq!(draws.get(), 0);
    // The same flight by the engine's reading scatters its dots.
    let engine = Rc::new(Cell::new(0));
    let mut view = faked(&Rulebook::default(), Vec2::ZERO, 0, &engine);
    caught_up(&mut view);
    assert!(view.stars.stars().iter().all(|star| star.shown));
    assert!(engine.get() > 0);
    assert!(skies(&view).is_empty());
}

#[test]
fn by_the_layered_reading_no_pass_steps_the_stars() {
    let draws = Rc::new(Cell::new(0));
    let mut view = faked(&layered(), Vec2::ZERO, 0, &draws);
    caught_up(&mut view);
    let before = view.stars.stars();
    let run = StepRun {
        before: vec![(ShipRef::Player, place(Vec2::new(8.0, -4.0)))],
        passes: vec![wrapping(Vec2::ZERO, None)],
        ..StepRun::default()
    };
    frame(
        &mut view,
        StepReport {
            steps: vec![run],
            stopped: false,
        },
    );
    assert_eq!(view.stars.stars(), before);
    assert_eq!(draws.get(), 0);
}

#[test]
fn the_layered_reading_wins_over_the_procedural_starfields() {
    let rules = layered().with_override(RuleKey::FlightStarfield, RuleSource::Bible);
    let draws = Rc::new(Cell::new(0));
    let mut view = faked(&rules, Vec2::ZERO, 0, &draws);
    caught_up(&mut view);
    let list = super::faked_world::world_list(&view);
    assert!(matches!(list.iter().next(), Some(DrawCommand::Sky { .. })));
    let mut procedural = DrawList::new();
    starfield::draw(&mut procedural, &view.camera());
    let first = procedural.iter().next().expect("some dots");
    assert!(
        !list.iter().any(|command| command == first),
        "no procedural dot"
    );
}

#[test]
fn a_murky_systems_sky_is_dimmed_and_a_negative_murks_is_not_drawn() {
    let draws = Rc::new(Cell::new(0));
    let mut view = faked(&layered(), Vec2::ZERO, 8, &draws);
    caught_up(&mut view);
    assert_eq!(the_sky(&view).level, murk_level(8));
    let mut view = faked(&layered(), Vec2::ZERO, -1, &draws);
    caught_up(&mut view);
    assert_eq!(skies(&view), []);
}

/// Runs a step of `view` from the ship at `from` to `to`, `wrap` run at
/// its head, and shows its end.
fn flown(view: &mut Faked, from: Vec2, to: Vec2, wrap: Option<PlayerWrap>) {
    let mut moved = view.frame.clone();
    moved.ships[0].position = to;
    view.sim.set_frame(moved);
    let run = StepRun {
        before: vec![(ShipRef::Player, place(from))],
        passes: vec![wrapping(to, wrap)],
        ..StepRun::default()
    };
    frame(
        view,
        StepReport {
            steps: vec![run],
            stopped: false,
        },
    );
    view.alpha = 1.0;
}

#[test]
fn the_skys_eye_moves_with_the_ship() {
    let draws = Rc::new(Cell::new(0));
    let start = Vec2::new(100.0, 200.0);
    let mut view = faked(&layered(), start, 0, &draws);
    caught_up(&mut view);
    let next = Vec2::new(112.0, 191.0);
    flown(&mut view, start, next, None);
    let before = the_sky(&view).eye;
    assert_eq!(before, Point::new(next.x, next.y));
    let last = Vec2::new(150.0, 160.0);
    flown(&mut view, next, last, None);
    let after = the_sky(&view).eye;
    assert_eq!(
        (after.x - before.x, after.y - before.y),
        (last.x - next.x, last.y - next.y)
    );
}

#[test]
fn a_carried_wrap_leaves_the_skys_eye_moving_by_the_ships_step() {
    let draws = Rc::new(Cell::new(0));
    let mut view = faked(&layered(), STOOD, 0, &draws);
    caught_up(&mut view);
    flown(&mut view, STOOD, STOOD, None);
    let before = the_sky(&view).eye;
    assert_eq!(before, Point::new(STOOD.x, STOOD.y));
    flown(&mut view, STOOD, STOOD + SHIFT + MOVE, Some(wrap()));
    let after = the_sky(&view).eye;
    assert!(
        (after.x - (before.x + MOVE.x)).abs() < 1e-3
            && (after.y - (before.y + MOVE.y)).abs() < 1e-3,
        "{before:?} to {after:?}: the ship's step, not the wrap's"
    );
}
