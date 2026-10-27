//! The flight's starfield in the view (see the view's `stars` module):
//! scattered when the flight is first caught up with and at the
//! original's randomizing sites, stepped and scrolled each pass, and
//! drawn from the first pass's placement, dimmed by the system's murk.

use crate::flight::stars::{Star, Starfield};
use crate::system::camera::Anchor;

use super::*;

/// Draws in the middle of each range, so every star is scattered onto
/// the ship and none wraps; each `Rand` roll counts up, so every
/// scattering rolls frames the last did not.
#[derive(Default)]
pub(super) struct Counting {
    rolls: u32,
}

impl Chance for Counting {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n / 2
    }

    fn roll(&mut self, sides: u16) -> u16 {
        self.rolls += 1;
        u16::try_from(self.rolls % u32::from(sides.max(1))).expect("below sides")
    }
}

fn stars_of(view: &View) -> [Star; 20] {
    view.stars.stars()
}

/// A scripted flight from `catalog` of the player alone at `at` in Sol of
/// murk `murk` as loaded, its effects rolled on a [`Counting`] chance.
fn faked_counted(catalog: FakeCatalog, at: Vec2, murk: i16) -> Faked {
    let mut frame =
        super::faked_world::in_sol_with(vec![super::faked_world::the_player_at(at, 0.0)]);
    frame.system_murk = murk;
    let mut view = super::faked_world::world(catalog, frame);
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Counting::default()));
    view.effects_chance = SharedChance::new(chance);
    view
}

/// What a catch-up of `view` redraws, the status bar the session's.
fn caught_up(view: &mut Faked) -> Redrawn {
    view.catch_up_then(|view, redrawn| view.redraw_scene_then(redrawn, |_| {}, |_| {}))
}

fn faked_stars(view: &Faked) -> [Star; 20] {
    view.stars.stars()
}

#[test]
fn the_flight_scatters_its_stars_when_it_is_first_caught_up_with() {
    let mut view = faked_counted(starry(), Vec2::ZERO, 0);
    assert!(faked_stars(&view).iter().all(|star| !star.shown), "not yet");
    caught_up(&mut view);
    assert!(faked_stars(&view).iter().all(|star| star.shown));
    let list = super::faked_world::world_list(&view);
    assert_eq!(list.iter().filter(|c| is_star(c)).count(), 20);
}

#[test]
fn without_the_stars_sheet_none_is_scattered_or_drawn() {
    let mut view = faked_counted(catalog(), Vec2::ZERO, 0);
    caught_up(&mut view);
    assert!(faked_stars(&view).iter().all(|star| !star.shown));
    assert!(!super::faked_world::world_list(&view).iter().any(is_star));
}

#[test]
fn by_the_other_reading_the_stand_in_is_drawn_and_no_star_scattered() {
    let rules = Rulebook::default().with_override(RuleKey::FlightStarfield, RuleSource::Bible);
    let mut view = super::faked_world::world_by(
        starry(),
        super::faked_world::in_sol_with(vec![super::faked_world::the_player_at(Vec2::ZERO, 0.0)]),
        &rules,
    );
    caught_up(&mut view);
    assert!(faked_stars(&view).iter().all(|star| !star.shown));
    let list = super::faked_world::world_list(&view);
    assert!(!list.iter().any(is_star));
    let mut dots = DrawList::new();
    starfield::draw(&mut dots, &view.camera());
    assert!(!dots.is_empty());
    assert!(
        list.iter().take(dots.len()).eq(dots.iter()),
        "the dots first"
    );
}

#[test]
fn a_starfield_reset_scatters_the_stars_afresh_about_the_frames_ship() {
    // Which flows reset it (a jump's arrival, a gate's exit, the rescue)
    // is the session's (nova-sim's
    // `an_arrival_tells_the_system_changed_and_the_starfield_reset`).
    let mut view = faked_counted(starry(), Vec2::ZERO, 0);
    caught_up(&mut view);
    let before: Vec<u16> = faked_stars(&view).iter().map(|star| star.frame).collect();
    let ship = Vec2::new(-1350.0, 0.0);
    let mut frame = view.frame.clone();
    frame.ships[0].position = ship;
    view.sim.set_frame(frame);
    view.sim
        .queue_event(nova_sim::flight_frame::FlightEvent::Session(
            nova_sim::SessionEvent::StarfieldReset,
        ));
    caught_up(&mut view);
    let after: Vec<u16> = faked_stars(&view).iter().map(|star| star.frame).collect();
    assert_ne!(after, before);
    for star in faked_stars(&view) {
        assert!(star.shown);
        assert!((star.x - ship.x).abs() <= 415.0, "{star:?} about {ship:?}");
        assert!((star.y - ship.y).abs() <= 384.0, "{star:?} about {ship:?}");
    }
}

#[test]
fn a_system_entered_with_no_starfield_reset_keeps_the_stars() {
    // An `M` move resets none: the session's (nova-sim's
    // `a_move_tells_whether_the_system_or_only_the_stellar_changed`).
    let mut view = faked_counted(starry(), Vec2::ZERO, 0);
    caught_up(&mut view);
    let before = faked_stars(&view);
    view.sim.set_frame(FlightFrame {
        system: SystemId(131),
        ..super::faked_world::in_sol_with(vec![super::faked_world::the_player_at(Vec2::ZERO, 0.0)])
    });
    view.sim
        .queue_event(nova_sim::flight_frame::FlightEvent::Session(
            nova_sim::SessionEvent::SystemChanged,
        ));
    caught_up(&mut view);
    assert_eq!(view.frame.system, SystemId(131));
    assert_eq!(faked_stars(&view), before);
}

#[test]
fn landing_and_taking_off_keep_the_stars() {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Counting::default()));
    let mut view = over_earth(starry()).with_effects_chance(SharedChance::new(chance));
    view.tick(Duration::ZERO);
    assert!(stars_of(&view).iter().all(|star| star.shown));
    land_now(&mut view);
    assert!(view.take_landing().is_some(), "landed");
    let before = stars_of(&view);
    view.take_off();
    view.tick(Duration::ZERO);
    assert_eq!(stars_of(&view), before);
}

#[test]
fn a_system_of_negative_murk_shows_no_stars() {
    let mut view = faked_counted(starry(), Vec2::ZERO, -1);
    caught_up(&mut view);
    assert!(faked_stars(&view).iter().all(|star| !star.shown));
    assert!(!super::faked_world::world_list(&view).iter().any(is_star));
}

#[test]
fn a_murky_systems_stars_are_dimmed_to_its_level() {
    let mut view = faked_counted(starry(), Vec2::ZERO, 8);
    caught_up(&mut view);
    let tints: Vec<Color> = super::faked_world::world_list(&view)
        .iter()
        .filter(|c| is_star(c))
        .filter_map(|command| match command {
            DrawCommand::Sprite { tint, .. } => Some(*tint),
            _ => None,
        })
        .collect();
    assert_eq!(tints, [lights_tint(7); 20], "trunc(8 × 0.9) of 32");
}

/// A field scattered on a [`Counting`] chance about a ship at `ship`.
fn scattered(ship: Vec2, looks: &Looks) -> (Starfield, Counting) {
    let mut chance = Counting::default();
    let mut field = Starfield::new();
    let at =
        super::super::stars::surroundings_at((ship, 0), VIEW_SIZE, Anchor::PlayAreaCentre, looks);
    super::super::stars::reset(&mut field, &at, true, &mut chance);
    (field, chance)
}

#[test]
fn a_step_scrolls_the_stars_by_the_ships_move_twice_over_in_double_time() {
    let catalog = starry();
    let looks = Looks::read(&catalog, []);
    for double in [false, true] {
        // Off the origin on both axes.
        let ship = Vec2::new(50.0, 50.0);
        let (mut field, mut chance) = scattered(ship, &looks);
        let before = field.stars();
        let from = Vec2::new(ship.x + 8.0, ship.y - 4.0);
        let place = (VIEW_SIZE, Anchor::PlayAreaCentre);
        let starfield = (RuleSource::Engine, &looks);
        let pass = nova_sim::flight_frame::PassReport {
            player_position: ship,
            double_time: double,
            ..nova_sim::flight_frame::PassReport::default()
        };
        super::super::stars::step(&mut field, &pass, starfield, place, Some(from), &mut chance);
        let times = if double { 2.0 } else { 1.0 };
        for (star, was) in field.stars().iter().zip(before) {
            assert!(was.factor > 0.0, "{was:?}");
            assert_eq!(star.x, was.x - was.factor * (8.0 * times), "{double}");
            assert_eq!(star.y, was.y - was.factor * (-4.0 * times), "{double}");
        }
        // The second pass wraps and places, and scrolls nothing.
        let placed = field.stars();
        super::super::stars::step(&mut field, &pass, starfield, place, None, &mut chance);
        assert_eq!(field.stars(), placed, "{double}");
    }
}

#[test]
fn the_frame_draws_the_first_passs_placement_in_double_time() {
    for double in [false, true] {
        let mut view = faked_counted(starry(), Vec2::ZERO, 0);
        caught_up(&mut view);
        // Twenty steps of the ship thrusting up, a pass each or two.
        for step in 0..20_u16 {
            let y = -f32::from(step);
            let pass = |at: f32| nova_sim::flight_frame::PassReport {
                player_position: Vec2::new(0.0, at),
                double_time: double,
                ..nova_sim::flight_frame::PassReport::default()
            };
            let passes = if double {
                vec![pass(y), pass(y - 0.5)]
            } else {
                vec![pass(y)]
            };
            view.sim.queue_report(nova_sim::flight_frame::StepReport {
                steps: vec![nova_sim::flight_frame::StepRun {
                    passes,
                    ..nova_sim::flight_frame::StepRun::default()
                }],
                stopped: false,
            });
            view.step_frame(1, view.anchor());
        }
        view.alpha = 1.0;
        let placed: Vec<Point> = view
            .stars
            .placements()
            .iter()
            .map(|&(left, top)| {
                #[allow(clippy::cast_precision_loss)]
                let centre = at(left as f32 + 2.5, top as f32 + 2.5);
                centre
            })
            .collect();
        let mut list = DrawList::new();
        let stars = (&view.stars, &view.looks);
        super::super::stars::draw(&mut list, stars, RuleSource::Engine, &view.camera(), 1.0);
        let shown: Vec<Point> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Sprite { center, .. } => Some(*center),
                _ => None,
            })
            .collect();
        assert_eq!(shown.len(), 20);
        if double {
            assert_ne!(shown, placed, "the second pass placed them again, undrawn");
        } else {
            assert_eq!(shown, placed, "one pass, one placement");
        }
    }
}

#[test]
fn a_scattering_due_scatters_the_stars_afresh_on_the_frames_ship() {
    // Entering play and beginning a game make one due: the concrete
    // flight's (`FlightView::enter_game`, `FlightView::begin`, phase 9).
    let ship = Vec2::new(50.0, 50.0);
    let mut view = faked_counted(starry(), ship, 0);
    caught_up(&mut view);
    let before: Vec<u16> = faked_stars(&view).iter().map(|star| star.frame).collect();
    view.stars_due = true;
    caught_up(&mut view);
    let after: Vec<u16> = faked_stars(&view).iter().map(|star| star.frame).collect();
    assert_ne!(after, before);
    for star in faked_stars(&view) {
        assert_eq!((star.x, star.y), (ship.x, ship.y), "on the ship");
    }
}
