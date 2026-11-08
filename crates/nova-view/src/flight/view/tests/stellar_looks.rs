//! The scene as the kept frame holds its stellars, on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-7-world-frame): laid out, placed and
//! read again from the flight's events, each stellar drawn on the sheet
//! its record says (whole, dead, graphic type 0's, none, frozen), on the
//! frame its record reports. What
//! the session decides (which events a set expression, a jump, a gate or
//! a move raise; which sheet a rule draws; which stellars a system
//! shows) is nova-sim's, named where a test leans on it.

use nova_sim::DrawnSheet;
use nova_sim::SessionEvent;
use nova_sim::flight_frame::{FlightEvent, FrameStellar};

use super::faked_world::{world, world_list};
use super::nav::{faked_nav_icons, nav_at};
use super::stellars::proxima_in_sol;
use super::*;

/// What a catch-up of `view` redraws, with no status bar to follow
/// (the session's, phase 8's).
pub(super) fn caught_up(view: &mut Faked) -> Redrawn {
    view.catch_up_then(|view, redrawn| view.redraw_scene_then(redrawn, |_| {}, |_| {}))
}

/// Has the flight raise `events`.
pub(super) fn raising(view: &mut Faked, events: &[SessionEvent]) {
    for &event in events {
        view.sim.queue_event(FlightEvent::Session(event));
    }
}

/// `view`'s frame changed by `change`, shown by the fake from now; the
/// view's kept one follows at its next fill.
pub(super) fn changed(view: &mut Faked, change: impl FnOnce(&mut FlightFrame)) {
    let mut frame = view.sim_frame();
    change(&mut frame);
    view.sim.set_frame(frame);
}

impl Faked {
    /// The frame the fake shows now.
    fn sim_frame(&self) -> FlightFrame {
        let mut frame = FlightFrame::default();
        self.sim.frame(&mut frame);
        frame
    }
}

/// A frame of the player alone in `system`, showing `stellars` as their
/// records say.
pub(super) fn showing(system: i16, stellars: Vec<FrameStellar>) -> FlightFrame {
    FlightFrame {
        system: SystemId(system),
        stellars,
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    }
}

/// Stellar `id`'s record, shown, drawing `drawn` on frame 0, destroyed
/// as `destroyed` says.
pub(super) fn looking(id: i16, drawn: DrawnSheet, destroyed: bool) -> FrameStellar {
    FrameStellar {
        destroyed,
        drawn,
        ..record::stellar(StellarId(id))
    }
}

/// Sol's Earth and Moon, whole.
pub(super) fn sol() -> Vec<FrameStellar> {
    vec![
        looking(128, DrawnSheet::Type, false),
        looking(129, DrawnSheet::Type, false),
    ]
}

/// A scripted flight from `catalog` showing `frame`.
pub(super) fn faked_from(catalog: FakeCatalog, frame: FlightFrame) -> Faked {
    world(catalog, frame)
}

/// The systems the catalog was asked for.
fn read(view: &Faked) -> Vec<SystemId> {
    view.catalog().systems_read.borrow().clone()
}

/// Whether stellar `id` is drawn on its `Type` sheet (1000 + `id`) and
/// whether on its `DeadType` sheet (2000 + `id`), at any of the first 4
/// frames.
fn drawn_on(view: &Faked, id: i16) -> (bool, bool) {
    let images: Vec<ImageKey> = sprites(&world_list(view))
        .into_iter()
        .map(|(image, _)| image)
        .collect();
    let sheet = |image_id| (0..4).any(|f| images.contains(&ImageKey::sprite(image_id, f)));
    (sheet(1000 + id), sheet(2000 + id))
}

/// Whether graphic type 0's sheet is drawn.
fn on_type_zero(view: &Faked) -> bool {
    sprites(&world_list(view))
        .into_iter()
        .any(|(image, _)| (0..4).any(|f| image == ImageKey::sprite(TYPE_ZERO, f)))
}

const WHOLE: (bool, bool) = (true, false);
const DEAD: (bool, bool) = (false, true);

/// The IDs of the stellars `view`'s scene holds.
fn held(view: &Faked) -> Vec<StellarId> {
    view.scene()
        .expect("laid out")
        .stellars()
        .iter()
        .map(|stellar| stellar.id)
        .collect()
}

// Laid out from the events (phase 5's tests, the session's events pinned
// by nova-sim's `an_arrival_tells_the_system_changed_and_the_starfield_reset`,
// `a_gates_exit_tells_the_system_changed_and_resets_the_stars_even_into_the_same`,
// `a_move_tells_whether_the_system_or_only_the_stellar_changed` and
// `a_rebuild_point_tells_stellars_rebuilt_only_when_it_changed_them`).

#[test]
fn the_hull_is_read_afresh_from_the_ship_changed_event_once() {
    let mut view = faked_from(shipbuying(), showing(130, sol()));
    changed(&mut view, |frame| frame.ships[0].class = ShipId(129));
    assert_eq!(
        view.sheet_ship,
        Some(ShipId(128)),
        "not before the catch-up"
    );
    raising(&mut view, &[SessionEvent::ShipChanged]);
    caught_up(&mut view);
    assert_eq!(view.sheet_ship, Some(ShipId(129)));
    raising(&mut view, &[SessionEvent::ShipChanged]);
    caught_up(&mut view);
    let asked = view.catalog().sheets_asked.borrow().clone();
    assert_eq!(asked, [ShipId(128), ShipId(129)], "once");
}

#[test]
fn the_system_is_laid_out_from_the_system_changed_event() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    view.catalog().systems_read.borrow_mut().clear();
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Type, false)]);
        frame.ships[0].position = Vec2::new(-1350.0, 0.0);
    });
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(130)));
    raising(&mut view, &[SessionEvent::SystemChanged]);
    assert!(caught_up(&mut view).system);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(read(&view), [SystemId(131)]);
    assert_eq!(view.map.current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), at(-1350.0, 0.0), "where it arrived");
}

#[test]
fn several_system_events_before_a_catch_up_lay_out_once_the_last_system() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    view.catalog().systems_read.borrow_mut().clear();
    changed(&mut view, |frame| *frame = showing(132, Vec::new()));
    raising(
        &mut view,
        &[SessionEvent::SystemChanged, SessionEvent::SystemChanged],
    );
    caught_up(&mut view);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(132)));
    assert_eq!(read(&view), [SystemId(132)], "one layout");
}

/// A stellar moved to, then a system entered, before one catch-up: one
/// layout, of the system entered. (Placing the ship as well after the
/// layout would change nothing to see: its writes are the layout's.)
#[test]
fn a_system_entered_supersedes_a_stellar_moved_to_before_the_catch_up() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    view.catalog().systems_read.borrow_mut().clear();
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Type, false)]);
    });
    raising(
        &mut view,
        &[SessionEvent::StellarChanged, SessionEvent::SystemChanged],
    );
    caught_up(&mut view);
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(read(&view), [SystemId(131)], "one layout");
    assert_eq!(view.map.current(), Some(SystemId(131)));
    assert_eq!(view.shown_position(), Point::new(0.0, 0.0), "at Proxima");
}

#[test]
fn a_stellar_moved_to_draws_the_ship_there_not_on_its_way() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    view.alpha = 0.5;
    changed(&mut view, |frame| {
        frame.ships[0].position = Vec2::new(300.0, -200.0);
    });
    raising(&mut view, &[SessionEvent::StellarChanged]);
    caught_up(&mut view);
    assert_eq!(view.alpha, 0.0);
    assert_eq!(
        view.shown_position(),
        Point::new(300.0, -200.0),
        "at the Moon"
    );
    assert_eq!(read(&view), [SystemId(130)], "the scene kept");
}

#[test]
fn the_scene_holds_only_the_stellars_the_frame_shows() {
    // Alpha Centauri lists Proxima, Sol owns it: the session's (nova-sim's
    // `a_system_shows_only_the_stellars_it_holds`).
    let mut unshown = looking(140, DrawnSheet::Type, false);
    unshown.shown = false;
    let view = faked_from(proxima_in_sol(), showing(131, vec![unshown]));
    assert_eq!(held(&view), [], "listed, unshown");
}

#[test]
fn a_rebuild_that_changes_the_stellars_shown_lays_the_scene_out_again() {
    // Docked at Proxima it is shown, and in flight Sol's again: the
    // session's (nova-sim's `docked_the_stellar_docked_at_stays_shown`,
    // `which_stellars_are_shown_changes_only_at_a_rebuild_point`).
    let mut unshown = looking(140, DrawnSheet::Type, false);
    unshown.shown = false;
    let mut view = faked_from(proxima_in_sol(), showing(131, vec![unshown]));
    changed(&mut view, |frame| {
        frame.stellars = vec![looking(140, DrawnSheet::Type, false)];
    });
    raising(&mut view, &[SessionEvent::StellarsRebuilt]);
    caught_up(&mut view);
    assert_eq!(held(&view), [StellarId(140)], "docked at it");
    changed(&mut view, |frame| frame.stellars[0].shown = false);
    raising(&mut view, &[SessionEvent::StellarsRebuilt]);
    caught_up(&mut view);
    assert_eq!(held(&view), [], "in flight, Sol's again");
}

#[test]
fn a_hull_with_no_sheet_reads_its_outline_when_the_ship_changes() {
    let dart = crate::outline::Outline {
        triangles: vec![crate::outline::OutlineTriangle {
            corners: [at(0.0, 4.0), at(1.0, -2.0), at(-1.0, -2.0)],
            color: Color::WHITE,
        }],
    };
    let catalog = FakeCatalog {
        outlines: vec![(130, dart.clone())],
        ..shipbuying()
    };
    let mut view = faked_from(catalog, showing(130, sol()));
    assert_eq!(view.outline, None, "ship 128 has a sheet");
    changed(&mut view, |frame| frame.ships[0].class = ShipId(130));
    raising(&mut view, &[SessionEvent::ShipChanged]);
    caught_up(&mut view);
    assert_eq!(view.outline, Some(dart), "ship 130 has none");
    changed(&mut view, |frame| frame.ships[0].class = ShipId(129));
    raising(&mut view, &[SessionEvent::ShipChanged]);
    caught_up(&mut view);
    assert_eq!(view.outline, None, "ship 129 has a sheet");
    assert_eq!(*view.catalog().outlines_asked.borrow(), [ShipId(130)]);
}

#[test]
fn with_the_map_open_a_catch_up_still_lays_out_and_reads_the_hull() {
    let mut view = faked_from(shipbuying(), showing(130, sol()));
    queue_map(&mut view, None, sol_drawing());
    view.open_map();
    assert!(view.map_open);
    view.catalog().systems_read.borrow_mut().clear();
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Type, false)]);
        frame.ships[0].class = ShipId(129);
    });
    raising(
        &mut view,
        &[SessionEvent::SystemChanged, SessionEvent::ShipChanged],
    );
    caught_up(&mut view);
    assert!(view.map_open, "still open");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(read(&view), [SystemId(131)]);
    assert_eq!(view.sheet_ship, Some(ShipId(129)));
}

// Destroyed stellars (story-runtime phase 7; phase 4's tests). Which sheet
// a stellar draws by the rules is the session's (nova-sim's
// `a_stellar_with_a_dead_type_sheet_draws_it_while_destroyed`,
// `by_the_engine_a_stellar_with_no_dead_type_sheet_draws_type_zero_while_dead_since_a_rebuild`,
// `by_the_bible_a_stellar_with_no_dead_type_sheet_draws_its_type`,
// `y_destroys_a_stellar_for_its_dead_time_as_every_reader_sees_it`,
// `y_u_the_daily_step_and_a_galaxy_change_are_no_rebuild_points`,
// `a_new_pilot_begins_with_the_stellars_that_start_the_game_destroyed`,
// `a_pilot_flown_with_a_stellar_destroyed_is_dead_since_the_rebuild_at_once_untold`).

#[test]
fn a_stellar_its_record_draws_dead_is_drawn_dead_from_the_catch_up_after_and_whole_again() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    assert_eq!(drawn_on(&view, 128), WHOLE);
    view.catalog().systems_read.borrow_mut().clear();
    changed(&mut view, |frame| {
        frame.stellars[0] = looking(128, DrawnSheet::Dead, true);
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    assert_eq!(drawn_on(&view, 128), WHOLE, "not before the catch-up");
    caught_up(&mut view);
    assert_eq!(drawn_on(&view, 128), DEAD, "from the catch-up");
    assert_eq!(read(&view), [], "not laid out again");
    changed(&mut view, |frame| {
        frame.stellars[0] = looking(128, DrawnSheet::Type, false);
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    assert_eq!(drawn_on(&view, 128), WHOLE, "regenerated");
    assert_eq!(read(&view), []);
}

#[test]
fn a_destroyed_stellar_whose_record_draws_its_type_keeps_its_type() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    changed(&mut view, |frame| {
        frame.stellars[1] = looking(129, DrawnSheet::Type, true);
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    assert_eq!(drawn_on(&view, 129), WHOLE);
    let scene = view.scene().expect("a scene");
    assert!(scene.stellars()[1].destroyed, "destroyed all the same");
}

#[test]
fn a_system_entered_draws_its_destroyed_stellars_dead() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Dead, true)]);
    });
    raising(&mut view, &[SessionEvent::SystemChanged]);
    let redrawn = caught_up(&mut view);
    assert!(redrawn.system, "laid out");
    assert!(!redrawn.galaxy, "by the system's reaction alone");
    assert_eq!(view.scene().map(SystemScene::id), Some(SystemId(131)));
    assert_eq!(drawn_on(&view, 140), DEAD);
}

#[test]
fn a_flight_opens_on_its_destroyed_stellars_dead_and_its_type_zero_ones() {
    let view = faked_from(
        destructible_catalog(),
        showing(
            130,
            vec![
                looking(128, DrawnSheet::Dead, true),
                looking(129, DrawnSheet::TypeZero, true),
            ],
        ),
    );
    assert_eq!(drawn_on(&view, 128), DEAD, "before any frame");
    assert!(on_type_zero(&view), "the Moon on graphic type 0's");
}

/// [`destructible`]'s catalog.
fn destructible_catalog() -> FakeCatalog {
    FakeCatalog {
        lives: vec![life(128, 0), life(129, 0), life(140, 0)],
        ..catalog()
    }
}

#[test]
fn a_rebuild_that_draws_type_zero_draws_it_without_laying_out() {
    let mut view = faked_from(destructible_catalog(), showing(130, sol()));
    view.catalog().systems_read.borrow_mut().clear();
    changed(&mut view, |frame| {
        frame.stellars[1] = looking(129, DrawnSheet::TypeZero, true);
    });
    raising(&mut view, &[SessionEvent::StellarsRebuilt]);
    caught_up(&mut view);
    assert!(on_type_zero(&view), "the rebuild's");
    assert_eq!(drawn_on(&view, 129), (false, false));
    assert_eq!(read(&view), [], "not laid out again");
}

#[test]
fn a_system_entered_draws_type_zero_from_its_first_frame() {
    let mut view = faked_from(destructible_catalog(), showing(131, Vec::new()));
    changed(&mut view, |frame| {
        *frame = showing(
            130,
            vec![
                looking(128, DrawnSheet::Type, false),
                looking(129, DrawnSheet::TypeZero, true),
            ],
        );
    });
    raising(&mut view, &[SessionEvent::SystemChanged]);
    caught_up(&mut view);
    assert!(on_type_zero(&view));
}

/// Why the Moon's `Type` sheet cannot be read.
const UNREADABLE: &str = "spöb 129: no spïn 1004 for its graphic type 4";

#[test]
fn a_whole_stellar_with_no_type_sheet_draws_type_zero_or_its_placeholder_as_its_record_says() {
    // Which, by the rules: the session's (nova-sim's
    // `by_the_engine_a_whole_stellar_with_no_type_sheet_draws_type_zero_from_a_rebuild_point`,
    // `by_the_other_reading_a_whole_stellar_with_no_type_sheet_draws_its_type`).
    let unreadable = || FakeCatalog {
        moon_unreadable: Some(UNREADABLE),
        ..catalog()
    };
    let reason = format!("Sprite unavailable: {UNREADABLE}");
    let zero = faked_from(
        unreadable(),
        showing(
            130,
            vec![
                looking(128, DrawnSheet::Type, false),
                looking(129, DrawnSheet::TypeZero, false),
            ],
        ),
    );
    assert!(on_type_zero(&zero), "the entry's rebuild point");
    assert!(!texts(&world_list(&zero)).contains(&reason));
    let problems = zero.scene().expect("a scene").problems().to_vec();
    assert!(
        problems.contains(&format!("spöb 129 Type: {UNREADABLE}")),
        "{problems:?}"
    );
    let typed = faked_from(unreadable(), showing(130, sol()));
    assert!(!on_type_zero(&typed));
    assert!(texts(&world_list(&typed)).contains(&reason));
}

// The stellars' frames (rendering phase 11): the session steps every
// stellar's frame and reports it (nova-sim's `stellar_frames` module);
// the view draws the frame the record gives, and steps none of its own.

/// The Moon's frame shown.
pub(super) fn moon_on(view: &Faked) -> u16 {
    sprites(&world_list(view))[1].0.frame
}

/// A tick of `view`'s clock with no step, the stellars shown from its
/// frame as it is now.
pub(super) fn tick_stellars(view: &mut Faked) {
    view.advance_clock(TICK);
    view.refresh_frame();
    view.show_stellars();
}

/// `view`'s Moon reported on `frame`, from the next fill.
fn moon_reported(view: &mut Faked, frame: u16) {
    changed(view, |flight| flight.stellars[1].frame = frame);
}

#[test]
fn each_stellar_draws_the_frame_the_kept_frame_reports() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    moon_reported(&mut view, 3);
    tick_stellars(&mut view);
    assert_eq!(sprites(&world_list(&view))[1].0, ImageKey::sprite(1129, 3));
    moon_reported(&mut view, 0);
    tick_stellars(&mut view);
    assert_eq!(sprites(&world_list(&view))[1].0, ImageKey::sprite(1129, 0));
}

#[test]
fn the_view_steps_no_stellar_of_its_own() {
    // Ticks with no step, whatever their length, leave the frame drawn as
    // the record reports it.
    let mut view = faked_from(catalog(), showing(130, sol()));
    moon_reported(&mut view, 2);
    tick_stellars(&mut view);
    for dt in [TICK / 2, TICK, TICK * 7] {
        view.advance_clock(dt);
        view.refresh_frame();
        view.show_stellars();
        assert_eq!(moon_on(&view), 2, "{dt:?}");
    }
}

#[test]
fn a_take_offs_reset_event_leaves_the_frames_to_the_session() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    moon_reported(&mut view, 2);
    tick_stellars(&mut view);
    raising(&mut view, &[SessionEvent::StellarFramesReset]);
    caught_up(&mut view);
    assert_eq!(moon_on(&view), 2, "as the record reports it");
}

/// A flight in Sol whose Moon (129) can be destroyed, over a 4-frame
/// `Type` sheet 50 square with a 2-frame `DeadType` sheet 70 square.
fn uncounted_moon() -> FakeCatalog {
    FakeCatalog {
        lives: vec![life(129, 0)],
        moon: Some(MoonSheets {
            width: 50,
            dead_width: Some(70),
            zero_width: 90,
            frames: 4,
            dead_frames: 2,
            flags2: 0,
        }),
        ..catalog()
    }
}

/// The Moon's sprites drawn, with where each is centred less where the
/// Moon is on screen.
fn moon_drawn(view: &Faked) -> Vec<(ImageKey, Point)> {
    let moon = view.scene().expect("a scene").stellars()[1].position;
    let at = view.camera().world_to_screen(moon);
    sprites(&world_list(view))
        .into_iter()
        .filter(|(image, _)| [1129, 2129, TYPE_ZERO].contains(&image.id))
        .map(|(image, centre)| (image, Point::new(centre.x - at.x, centre.y - at.y)))
        .collect()
}

#[test]
fn a_stellar_drawing_no_frame_draws_nothing_and_a_frozen_one_its_frame_shifted() {
    // When its slot shows none or is frozen, and the frame it keeps: the
    // session's (nova-sim's
    // `a_whole_stellar_whose_type_counts_no_frames_draws_no_frame_then_keeps_its_dead_frame_frozen`,
    // `a_frozen_slot_keeps_the_frame_from_before_the_pass_that_froze_it`).
    let mut view = faked_from(
        uncounted_moon(),
        showing(
            130,
            vec![
                looking(128, DrawnSheet::Type, false),
                looking(129, DrawnSheet::NoFrame, false),
            ],
        ),
    );
    tick_stellars(&mut view);
    assert_eq!(moon_drawn(&view), [], "its slot shows no frame");
    changed(&mut view, |frame| {
        frame.stellars[1] = FrameStellar {
            frozen: true,
            frame: 1,
            ..looking(129, DrawnSheet::Dead, false)
        };
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    tick_stellars(&mut view);
    assert_eq!(
        moon_drawn(&view),
        [(
            ImageKey::sprite(2129, 1),
            Point::new(35.0 - 16.0, 35.0 - 16.0)
        )],
        "the `DeadType` frame reported, where a 32-pixel slot puts it"
    );
}

#[test]
fn the_nav_brackets_round_a_slot_whose_frames_count_none_are_32_square() {
    // `_HandleNavTargetSprites` brackets at half the slot's larger side
    // (@0x2fe00-0x2fe36), which `_GetSpriteXSize` reads as 32 for a slot
    // whose frames count none, frozen or frameless (@0x26d4-0x2702).
    let mut view = faked_from(
        uncounted_moon(),
        showing(
            130,
            vec![
                looking(128, DrawnSheet::Type, false),
                looking(129, DrawnSheet::NoFrame, false),
            ],
        ),
    );
    caught_up(&mut view);
    let moon = view.scene().expect("a scene").stellars()[1].position;
    let (x, y) = (moon.x, moon.y);
    let on_screen = view.camera().world_to_screen(moon);
    let brackets = |view: &mut Faked| faked_nav_icons(view, Some(nav_at(129, (x, y), 0.0)));
    assert_eq!(
        brackets(&mut view),
        corners_at(10000, on_screen, 16.0),
        "showing no frame"
    );
    changed(&mut view, |frame| {
        frame.stellars[1] = FrameStellar {
            frozen: true,
            ..looking(129, DrawnSheet::Dead, false)
        };
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    assert_eq!(
        brackets(&mut view),
        corners_at(10000, on_screen, 16.0),
        "its `DeadType` frame, 70 square, frozen"
    );
    changed(&mut view, |frame| {
        frame.stellars[1] = looking(129, DrawnSheet::Type, false);
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    assert_eq!(
        brackets(&mut view),
        corners_at(10000, on_screen, 25.0),
        "its `Type` frame, 50 square"
    );
}

#[test]
fn a_frame_held_between_its_passes_draws_its_own_stellar_frames() {
    // Double time's frame between its passes draws the stellars as its
    // first pass left them, then the kept frame's once it goes.
    let mut flight = showing(130, sol());
    let mut view = faked_from(catalog(), flight.clone());
    let mut first = flight.clone();
    first.stellars[1].frame = 1;
    flight.stellars[1].frame = 2;
    view.sim.set_frame(flight);
    let pass = nova_sim::flight_frame::PassReport::default();
    let report = super::stepping::doubled(pass.clone(), pass, first);
    view.sim.queue_report(report);
    view.tick(TICK);
    assert_eq!(moon_on(&view), 1, "the first pass's");
    view.sim.queue_report(nova_sim::flight_frame::StepReport {
        steps: Vec::new(),
        stopped: true,
    });
    view.tick(TICK);
    assert_eq!(moon_on(&view), 2, "the flight's");
}

#[test]
fn overlapping_stellars_are_drawn_lowest_id_first_whatever_their_navdefs_order() {
    let catalog = FakeCatalog {
        sol_more: vec![stellar(127, "Under the Moon", (300, -200), 1)],
        ..catalog()
    };
    let mut stellars = sol();
    stellars.push(looking(127, DrawnSheet::Type, false));
    let view = faked_from(catalog, showing(130, stellars));
    assert_eq!(
        held(&view),
        [StellarId(128), StellarId(129), StellarId(127)],
        "Sol lists the Moon before 127"
    );
    let images: Vec<i16> = sprites(&world_list(&view))
        .into_iter()
        .map(|(image, _)| image.id)
        .filter(|id| (1127..=1129).contains(id))
        .collect();
    assert_eq!(images, [1127, 1128, 1129], "127 under the Moon");
}
