//! The scene as the kept frame holds its stellars, on [`FakeFlight`]
//! (rdm:phase/view-sim-seam/phase-7-world-frame): laid out, placed and
//! read again from the flight's events, each stellar drawn on the sheet
//! its record says (whole, dead, graphic type 0's, none, frozen), its
//! animation over the frames its record counts, kept across layouts. What
//! the session decides (which events a set expression, a jump, a gate or
//! a move raise; which sheet a rule draws; which stellars a system
//! shows) is nova-sim's, named where a test leans on it.

use nova_sim::DrawnSheet;
use nova_sim::SessionEvent;
use nova_sim::flight_frame::{FlightEvent, FrameStellar};

use super::faked_world::{world, world_list};
use super::stellars::proxima_in_sol;
use super::*;
use crate::system::animation::Frames;

/// What a catch-up of `view` redraws, with no status bar to follow
/// (the session's, phase 8's).
fn caught_up(view: &mut Faked) -> Redrawn {
    view.catch_up_then(|view, redrawn| view.redraw_scene_then(redrawn, |_| {}, |_| {}))
}

/// Has the flight raise `events`.
fn raising(view: &mut Faked, events: &[SessionEvent]) {
    for &event in events {
        view.sim.queue_event(FlightEvent::Session(event));
    }
}

/// `view`'s frame changed by `change`, shown by the fake from now; the
/// view's kept one follows at its next fill.
fn changed(view: &mut Faked, change: impl FnOnce(&mut FlightFrame)) {
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
fn showing(system: i16, stellars: Vec<FrameStellar>) -> FlightFrame {
    FlightFrame {
        system: SystemId(system),
        stellars,
        ..in_sol_with(vec![the_player_at(Vec2::ZERO, 0.0)])
    }
}

/// Stellar `id`'s record, shown, drawing `drawn` and counting its frames,
/// destroyed as `destroyed` says.
fn looking(id: i16, drawn: DrawnSheet, destroyed: bool) -> FrameStellar {
    FrameStellar {
        destroyed,
        drawn,
        counted: drawn,
        ..record::stellar(StellarId(id))
    }
}

/// Sol's Earth and Moon, whole.
fn sol() -> Vec<FrameStellar> {
    vec![
        looking(128, DrawnSheet::Type, false),
        looking(129, DrawnSheet::Type, false),
    ]
}

/// A scripted flight from `catalog` showing `frame`.
fn faked_from(catalog: FakeCatalog, frame: FlightFrame) -> Faked {
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

// The stellars' animation (stellar-fidelity phases 1, 8, 10 and 12).

/// The Moon's frame shown.
fn moon_on(view: &Faked) -> u16 {
    sprites(&world_list(view))[1].0.frame
}

/// Steps `view`'s stellars a tick, its frozen and gate frames shown from
/// its frame as a tick's steps leave them.
fn tick_stellars(view: &mut Faked) {
    view.step_stellars(TICK);
    view.refresh_frame();
    if let Some(scene) = &mut view.scene {
        super::super::show_frozen_frames(scene, &view.frame);
        super::super::show_gate_frames(scene, &view.frame);
    }
}

#[test]
fn two_half_tick_steps_step_the_stellars_once() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    view.step_stellars(TICK / 2);
    assert_eq!(moon_on(&view), 0, "not on half a tick");
    view.step_stellars(REST_OF_TICK);
    assert_eq!(moon_on(&view), 1, "on the tick's second half");
}

/// A flight in Sol whose Moon (129) animates a frame a tick only while
/// destroyed (`Flags2` 0x0080), over a `Type` sheet of `frames` frames
/// and a 3-frame `DeadType` sheet.
fn flickering(frames: u16) -> FakeCatalog {
    FakeCatalog {
        lives: vec![life(129, 0x0080)],
        moon: Some(MoonSheets {
            width: 50,
            dead_width: Some(70),
            zero_width: 90,
            frames,
            dead_frames: 3,
            flags2: 0x0080,
        }),
        ..catalog()
    }
}

/// The Moon's frames shown over `n` ticks, each of its `DeadType` sheet.
fn dead_moon_on(view: &mut Faked, n: usize) -> Vec<u16> {
    (0..n)
        .map(|_| {
            tick_stellars(view);
            let image = sprites(&world_list(view))[1].0;
            assert_eq!(image.id, 2129, "the DeadType sheet");
            image.frame
        })
        .collect()
}

#[test]
fn a_stellar_animates_over_the_frames_of_the_sheet_its_record_counts() {
    // Which sheet it counts after `Y` and a rebuild point: the session's
    // (nova-sim's `by_the_engine_a_stellar_counts_the_sheet_of_its_last_rebuild_point`,
    // `by_the_bible_a_stellar_counts_the_sheet_it_draws`).
    let mut view = faked_from(flickering(4), showing(130, sol()));
    changed(&mut view, |frame| {
        frame.stellars[1] = FrameStellar {
            counted: DrawnSheet::Type,
            ..looking(129, DrawnSheet::Dead, true)
        };
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    assert_eq!(
        dead_moon_on(&mut view, 9),
        [1, 2, 0, 0, 1, 2, 0, 0, 1],
        "Type's 4 frames over the DeadType's 3"
    );
    // The rebuild keeps current frame 1, next 2, from the Type's count.
    changed(&mut view, |frame| {
        frame.stellars[1].counted = DrawnSheet::Dead;
    });
    raising(&mut view, &[SessionEvent::StellarsRebuilt]);
    caught_up(&mut view);
    assert_eq!(
        dead_moon_on(&mut view, 7),
        [2, 0, 1, 2, 0, 1, 2],
        "the DeadType's 3"
    );
}

#[test]
fn a_stellar_counting_a_one_frame_sheet_stays_still() {
    let mut view = faked_from(flickering(1), showing(130, sol()));
    changed(&mut view, |frame| {
        frame.stellars[1] = FrameStellar {
            counted: DrawnSheet::Type,
            ..looking(129, DrawnSheet::Dead, true)
        };
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    assert_eq!(dead_moon_on(&mut view, 6), [0; 6]);
    changed(&mut view, |frame| {
        frame.stellars[1].counted = DrawnSheet::Dead;
    });
    raising(&mut view, &[SessionEvent::StellarsRebuilt]);
    caught_up(&mut view);
    assert_eq!(dead_moon_on(&mut view, 4), [1, 2, 0, 1]);
}

/// A flight in Sol whose Moon (129) can be destroyed and animates a
/// frame a tick while whole, over a 4-frame `Type` sheet 50 square with
/// a 2-frame `DeadType` sheet 70 square.
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
fn a_stellar_drawing_no_frame_draws_nothing_and_a_frozen_one_its_kept_frame_shifted() {
    // When its slot shows none, counts none or is frozen: the session's
    // (nova-sim's
    // `a_whole_stellar_whose_type_counts_no_frames_draws_no_frame_then_keeps_its_dead_frame_frozen`).
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
            counts_none: true,
            ..looking(129, DrawnSheet::Dead, true)
        };
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    tick_stellars(&mut view);
    assert_eq!(
        moon_drawn(&view),
        [(ImageKey::sprite(2129, 0), Point::new(0.0, 0.0))],
        "its `DeadType`, still: it counts none"
    );
    changed(&mut view, |frame| {
        frame.stellars[1] = FrameStellar {
            frozen: true,
            counts_none: true,
            ..looking(129, DrawnSheet::Dead, false)
        };
    });
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    caught_up(&mut view);
    for _ in 0..4 {
        tick_stellars(&mut view);
        assert_eq!(
            moon_drawn(&view),
            [(
                ImageKey::sprite(2129, 0),
                Point::new(35.0 - 16.0, 35.0 - 16.0)
            )],
            "the `DeadType` frame kept, where a 32-pixel slot puts it"
        );
    }
}

#[test]
fn a_move_within_the_system_keeps_its_stellars_animations() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 2);
    raising(&mut view, &[SessionEvent::SystemChanged]);
    assert!(caught_up(&mut view).system, "laid out");
    assert_eq!(moon_on(&view), 2, "where it was");
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 3, "and on from there");
}

#[test]
fn a_stellar_left_on_frame_2_shows_frame_2_after_another_system_and_back() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 2);
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Type, false)]);
    });
    raising(&mut view, &[SessionEvent::SystemChanged]);
    assert!(caught_up(&mut view).system, "laid out in Alpha Centauri");
    for _ in 0..5 {
        tick_stellars(&mut view);
    }
    changed(&mut view, |frame| *frame = showing(130, sol()));
    raising(&mut view, &[SessionEvent::SystemChanged]);
    assert!(caught_up(&mut view).system, "laid out in Sol again");
    assert_eq!(moon_on(&view), 2, "nothing stepped while away");
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 3, "and on from there");
}

/// The Moon's animation state in `view`'s scene.
fn moon_frames(view: &Faked) -> Frames {
    view.scene().expect("laid out").stellars()[1].frames
}

#[test]
fn a_take_offs_reset_restarts_every_stellar_from_frame_0_with_next_0() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 2);
    let mut zeroed = moon_frames(&view);
    zeroed.take_off_reset();
    raising(&mut view, &[SessionEvent::StellarFramesReset]);
    assert!(caught_up(&mut view).frames_reset);
    assert_eq!(moon_on(&view), 0, "frame 0 shown");
    assert_eq!(moon_frames(&view), zeroed, "current 0, next 0");
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 0, "next 0");
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 1, "then on");
}

#[test]
fn a_gate_exits_reset_zeroes_the_stellars_left_behind_and_starts_the_new_ones_with_next_0() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 2);
    changed(&mut view, |frame| {
        *frame = showing(131, vec![looking(140, DrawnSheet::Type, false)]);
    });
    raising(
        &mut view,
        &[
            SessionEvent::SystemChanged,
            SessionEvent::StarfieldReset,
            SessionEvent::StellarFramesReset,
        ],
    );
    let redrawn = caught_up(&mut view);
    assert!(redrawn.system && redrawn.frames_reset);
    assert_eq!(
        view.scene().expect("laid out").stellars()[0].frames,
        Frames::after_reset(),
        "never shown before: next 0"
    );
    changed(&mut view, |frame| *frame = showing(130, sol()));
    raising(&mut view, &[SessionEvent::SystemChanged]);
    caught_up(&mut view);
    assert_eq!(moon_on(&view), 0, "zeroed as it was kept");
    tick_stellars(&mut view);
    assert_eq!(moon_on(&view), 0, "next 0");
}

#[test]
fn without_the_reset_a_catch_up_keeps_the_stellars_frames() {
    let mut view = faked_from(catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    let before = moon_frames(&view);
    raising(&mut view, &[SessionEvent::StellarsRebuilt]);
    assert!(!caught_up(&mut view).frames_reset);
    assert_eq!(moon_frames(&view), before);
}

#[test]
fn a_galaxy_change_keeps_the_stellars_animations() {
    let mut view = faked_from(destructible_catalog(), showing(130, sol()));
    tick_stellars(&mut view);
    tick_stellars(&mut view);
    raising(&mut view, &[SessionEvent::GalaxyChanged]);
    assert!(caught_up(&mut view).galaxy);
    assert_eq!(moon_on(&view), 2, "not laid out afresh");
}

/// A flight in Sol with hypergate 141 at (200, 100), which opens only
/// while destroyed (`Flags2` 0x0080) and steps every tick, its `Type`
/// sheet (1141) 20 square and its `DeadType` sheet (2141), 100 square,
/// counting 6.
fn uncounted_gate() -> FakeCatalog {
    let mut catalog = trafficked(&[130], 1, 129, 3);
    catalog.sol_more.push(StellarContents {
        dead_sprite: Some(Ok(StellarSheet {
            image: crate::SheetImage::Rled(2141),
            frames: NonZeroU16::new(6).expect("non-zero"),
            frame_width: 100,
            frame_height: 100,
        })),
        ..stellar(141, "Gate", (200, 100), 6)
    });
    catalog.lives = vec![life(141, 0x0080)];
    catalog
}

/// The frames of hypergate 141's sheets drawn.
fn gate_on(view: &Faked) -> Vec<ImageKey> {
    sprites(&world_list(view))
        .into_iter()
        .map(|(image, _)| image)
        .filter(|image| [1141, 2141].contains(&image.id))
        .collect()
}

#[test]
fn a_gate_frozen_keeps_the_frame_it_showed_before_its_gate_frame_changes() {
    // The view reads the freeze before the gate frames: on the tick the
    // slot freezes, it holds the frame drawn on the tick before, not the
    // one the record steps the gate to on this one.
    let gate = |frame, destroyed| FrameStellar {
        gate_frame: Some(frame),
        ..looking(141, DrawnSheet::Dead, destroyed)
    };
    let mut frame = showing(130, sol());
    frame.stellars.push(gate(3, true));
    let mut view = faked_from(uncounted_gate(), frame);
    let shown = gate_on(&view);
    assert_eq!(
        shown,
        [ImageKey::sprite(2141, 3)],
        "opening while destroyed"
    );
    changed(&mut view, |frame| {
        frame.stellars[2] = FrameStellar {
            frozen: true,
            counts_none: true,
            ..gate(0, false)
        };
    });
    for _ in 0..4 {
        tick_stellars(&mut view);
        assert_eq!(gate_on(&view), shown, "the frame shown before, kept");
    }
}

/// The Moon's frames over 3 ticks of a flight in Sol whose Moon (129), of
/// 4 frames a tick, has `flags2`, its effects drawn on `draws`: those
/// frames, and what the effects' chance was asked.
fn moon_frames_on(flags2: u16, draws: &[u32]) -> (Vec<u16>, Vec<u32>) {
    let (effects, effects_chance) = scripted(draws);
    let mut view = faked_from(
        FakeCatalog {
            moon: Some(MoonSheets {
                width: 50,
                dead_width: None,
                zero_width: 90,
                frames: 4,
                dead_frames: 1,
                flags2,
            }),
            ..catalog()
        },
        showing(130, sol()),
    );
    view.effects_chance = effects_chance;
    let frames = (0..3)
        .map(|_| {
            tick_stellars(&mut view);
            moon_on(&view)
        })
        .collect();
    let asked = effects.borrow().asked.clone();
    (frames, asked)
}

#[test]
fn a_random_stellars_frames_roll_on_the_effects_chance() {
    let random = nova_sim::gate_sprite::RANDOM;
    let (frames, asked) = moon_frames_on(random, &[0, 0, 0]);
    // From 1, a draw of 0 is frame 0; from 0, frame 1.
    assert_eq!(frames, [1, 0, 1]);
    assert_eq!(asked, [3, 3, 3]);
    let (plain, unasked) = moon_frames_on(0, &[]);
    assert_eq!(plain, [1, 2, 3]);
    assert!(unasked.is_empty(), "{unasked:?}");
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
