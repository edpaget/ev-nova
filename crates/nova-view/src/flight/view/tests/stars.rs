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
struct Counting {
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

/// `catalog`'s flight, its effects rolled on a [`Counting`] chance,
/// caught up with once.
fn counted(catalog: FakeCatalog) -> View {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Counting::default()));
    let mut view = FlightView::new(catalog).with_effects_chance(SharedChance::new(chance));
    view.tick(Duration::ZERO);
    view
}

fn stars_of(view: &View) -> [Star; 20] {
    view.stars.stars()
}

fn frames(view: &View) -> Vec<u16> {
    stars_of(view).iter().map(|star| star.frame).collect()
}

#[test]
fn the_flight_scatters_its_stars_when_it_is_first_caught_up_with() {
    let mut view = FlightView::new(starry());
    assert!(stars_of(&view).iter().all(|star| !star.shown), "not yet");
    view.tick(Duration::ZERO);
    assert!(stars_of(&view).iter().all(|star| star.shown));
    assert_eq!(drawn(&view).iter().filter(|c| is_star(c)).count(), 20);
}

#[test]
fn without_the_stars_sheet_none_is_scattered_or_drawn() {
    let view = counted(catalog());
    assert!(stars_of(&view).iter().all(|star| !star.shown));
    assert!(!drawn(&view).iter().any(is_star));
}

#[test]
fn by_the_other_reading_the_stand_in_is_drawn_and_no_star_scattered() {
    let rules = Rulebook::default().with_override(RuleKey::FlightStarfield, RuleSource::Bible);
    let mut view = FlightView::new(starry()).with_rules(rules);
    view.tick(TICK);
    assert!(stars_of(&view).iter().all(|star| !star.shown));
    let list = drawn(&view);
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
fn a_jump_arrival_scatters_the_stars_afresh() {
    let mut view = counted(starry());
    let before = frames(&view);
    jump_to_alpha(&mut view);
    assert_ne!(frames(&view), before);
    let ship = player(&view).position;
    for star in stars_of(&view) {
        assert!(star.shown);
        assert!((star.x - ship.x).abs() <= 415.0, "{star:?} about {ship:?}");
        assert!((star.y - ship.y).abs() <= 384.0, "{star:?} about {ship:?}");
    }
}

#[test]
fn an_m_move_keeps_the_stars() {
    let mut view = counted(starry());
    let before = stars_of(&view);
    run_set(&mut view, "M131");
    view.settle_script();
    view.tick(Duration::ZERO);
    assert_eq!(view.session().expect("flying").system(), SystemId(131));
    assert_eq!(stars_of(&view), before);
}

#[test]
fn landing_and_taking_off_keep_the_stars() {
    let chance: Rc<RefCell<dyn Chance>> = Rc::new(RefCell::new(Counting::default()));
    let mut view = FlightView::with_pilot(starry(), docked_pilot("Ada"))
        .with_effects_chance(SharedChance::new(chance));
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
    let mut catalog = starry();
    catalog.sol_murk = -1;
    let mut view = FlightView::new(catalog);
    view.tick(TICK);
    assert!(stars_of(&view).iter().all(|star| !star.shown));
    assert!(!drawn(&view).iter().any(is_star));
}

#[test]
fn a_murky_systems_stars_are_dimmed_to_its_level() {
    let mut catalog = starry();
    catalog.sol_murk = 8;
    let mut view = FlightView::new(catalog);
    view.tick(TICK);
    let tints: Vec<Color> = drawn(&view)
        .iter()
        .filter(|c| is_star(c))
        .filter_map(|command| match command {
            DrawCommand::Sprite { tint, .. } => Some(*tint),
            _ => None,
        })
        .collect();
    assert_eq!(tints, [lights_tint(7); 20], "trunc(8 × 0.9) of 32");
}

/// A field scattered on a [`Counting`] chance about `session`'s ship.
fn scattered(session: &Session, looks: &Looks) -> (Starfield, Counting) {
    let mut chance = Counting::default();
    let mut field = Starfield::new();
    let at = super::super::stars::surroundings(session, VIEW_SIZE, Anchor::PlayAreaCentre, looks);
    super::super::stars::reset(&mut field, &at, &mut chance);
    (field, chance)
}

#[test]
fn a_step_scrolls_the_stars_by_the_ships_move_twice_over_in_double_time() {
    let catalog = starry();
    let looks = Looks::read(&catalog, []);
    for double in [false, true] {
        let mut session = Session::start(&catalog).expect("starts");
        // Begun, the ship is at (50, 50): off the origin on both axes.
        session.begin(&catalog, &mut NeverFires);
        assert_eq!(session.player().position, Vec2::new(50.0, 50.0));
        session.set_double_time_key(double);
        assert_eq!(session.double_time(), double);
        let (mut field, mut chance) = scattered(&session, &looks);
        let before = field.stars();
        let ship = session.player().position;
        let from = Vec2::new(ship.x + 8.0, ship.y - 4.0);
        let place = (VIEW_SIZE, Anchor::PlayAreaCentre, &looks);
        super::super::stars::step(&mut field, &session, place, Some(from), &mut chance);
        let times = if double { 2.0 } else { 1.0 };
        for (star, was) in field.stars().iter().zip(before) {
            assert!(was.factor > 0.0, "{was:?}");
            assert_eq!(star.x, was.x - was.factor * (8.0 * times), "{double}");
            assert_eq!(star.y, was.y - was.factor * (-4.0 * times), "{double}");
        }
        // The second pass wraps and places, and scrolls nothing.
        let placed = field.stars();
        super::super::stars::step(&mut field, &session, place, None, &mut chance);
        assert_eq!(field.stars(), placed, "{double}");
    }
}

#[test]
fn the_frame_draws_the_first_passs_placement_in_double_time() {
    for double in [false, true] {
        let mut view = counted(starry()).with_double_time(double);
        view.input(&key(Key::Up, true));
        ticks(&mut view, 20);
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
        let shown: Vec<Point> = drawn_stars(&view)
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
fn entering_play_and_beginning_a_game_scatter_the_stars_afresh() {
    for enter in [true, false] {
        let mut view = counted(starry());
        let before = frames(&view);
        if enter {
            view.enter_game();
        } else {
            view.begin();
        }
        view.tick(Duration::ZERO);
        assert_ne!(frames(&view), before, "{enter}");
        let ship = player(&view).position;
        for star in stars_of(&view) {
            assert_eq!((star.x, star.y), (ship.x, ship.y), "{enter}: on the ship");
        }
    }
}
