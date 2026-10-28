use super::*;
use crate::flight::sim_port::fixture::MapAsk;
use crate::galaxy::update::MapUpdate;
use nova_sim::MapClicked;
use nova_sim::flight_frame::{FlightEvent, FloatingLinks};
use nova_sim::galaxy_map::{MapDrawing, MapLink};
use nova_sim::iff::SystemColorCode;
use nova_sim::map_panel::{
    Column, ColumnBody, Listing, MapPanel, PanelWords, Ports, Services, Strip,
};

// Tab, `\`, Return and the map key on the in-flight maps (`_MapFilter`).

/// The course map's drawing in Sol after a cycle selected `to` in mode
/// 3: [`sol_drawing`] with Barnard (132) drawn too, and the link to `to`
/// drawn again.
fn cycled_to(to: i16) -> MapDrawing {
    let mut drawing = sol_drawing();
    drawing
        .systems
        .push(drawn_system(132, SystemColorCode::Unexplored));
    drawing.mode3_link = Some((SystemId(130), SystemId(to)));
    drawing
}

#[test]
fn tab_on_the_course_map_sends_the_cycle_and_shows_what_the_flight_selects_and_draws() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    for (k, expected) in [(Key::Tab, 131), (Key::Char('\\'), 132), (Key::Tab, 131)] {
        view.sim.queue_answer(Answer::Done);
        let map = view.sim.map_mut();
        map.map_preselection = Some(SystemId(expected));
        map.drawing = Some(cycled_to(expected));
        assert!(!faked_map_input(&mut view, &key(k, true)), "{k:?}");
        assert_eq!(
            sent(&view).last(),
            Some(&FlightCommand::CycleMapSelection),
            "{k:?}"
        );
        assert_eq!(view.map.selected(), Some(SystemId(expected)), "{k:?}");
        assert_eq!(
            view.map.hyper_selection(),
            Some(SystemId(expected)),
            "{k:?}"
        );
        faked_map_input(&mut view, &key(k, false));
    }
    faked_map_input(&mut view, &held(Key::Tab));
    assert_eq!(
        sent(&view),
        vec![FlightCommand::CycleMapSelection; 4],
        "a repeat cycles too"
    );
}

#[test]
fn tab_on_the_course_map_with_no_link_drawn_selects_what_the_flight_preselects() {
    let mut view = faked_sol(FlightFrame {
        system: SystemId(132),
        ..in_sol()
    });
    view.open_map();
    view.sim.queue_answer(Answer::Done);
    view.sim.map_mut().map_preselection = Some(SystemId(132));
    faked_map_tap(&mut view, Key::Tab);
    assert_eq!(sent(&view), [FlightCommand::CycleMapSelection]);
    assert_eq!(view.map.selected(), Some(SystemId(132)));
    assert_eq!(view.map.hyper_selection(), None);
}

#[test]
fn tab_on_the_course_map_plays_no_sound_of_its_own() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    view.sim.queue_answer(Answer::Done);
    faked_map_tap(&mut view, Key::Tab);
    assert_eq!(sent(&view), [FlightCommand::CycleMapSelection]);
    assert_eq!(view.ui_sounds, []);
}

#[test]
fn tab_on_the_course_map_retakes_the_nav_area_at_once() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    let alpha = NavMode::Hyperspace(Some(SystemId(131)));
    let stale = NavDisplay::Stellar(Some("Stale".to_owned()));
    view.nav_scene = Some((alpha, stale.clone()));
    view.sim.set_frame(FlightFrame {
        nav_mode: alpha,
        ..in_sol()
    });
    view.sim.queue_answer(Answer::Done);
    faked_map_tap(&mut view, Key::Tab);
    let retaken = Some((alpha, view.nav_now()));
    assert_eq!(view.nav_scene, retaken);
    assert_ne!(view.nav_now(), stale);
}

#[test]
fn closing_the_map_after_a_tab_shows_its_selection_in_the_nav_area() {
    // Tab's selection and the cue's, with no course to cue, are nova-sim's
    // (`the_maps_cycle_selects_the_next_link_in_mode_3`,
    // `the_cue_selects_the_courses_next_hop`).
    let mut view = super::faked_hud::barred(catalog(), among(Vec::new()));
    view.sim.map_mut().drawing = Some(sol_drawing());
    view.open_map();
    let alpha = NavMode::Hyperspace(Some(SystemId(131)));
    view.sim.frame_mut().nav_mode = alpha;
    view.sim.queue_answer(Answer::Done);
    faked_map_tap(&mut view, Key::Tab);
    view.close_map_then(|_, _| panic!("no hypergate"));
    assert_eq!(
        super::faked_hud::nav_of(&view),
        cueing_alpha(),
        "shown once the map closes"
    );
}

/// A view over a scripted flight in Sol with the hypergate map open,
/// offering Alpha Reborn (133) and Barnard (132), its flight's cycle
/// stepping through hidden Alpha Centauri (131) and Barnard.
fn at_a_faked_gate() -> Faked {
    let mut view = faked(in_sol());
    view.sim.map_mut().map_preselection = Some(SystemId(130));
    view.sim.map_mut().hypergate_cycle = vec![Some(SystemId(131)), Some(SystemId(132))];
    view.map.offer_gates(vec![SystemId(132), SystemId(133)]);
    view.open_map();
    assert_eq!(view.map.mode(), MapMode::Hypergate);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    view
}

#[test]
fn tab_on_the_hypergate_map_steps_through_the_flights_cycle_shift_reversing_and_writes_no_nav() {
    let mut view = at_a_faked_gate();
    for expected in [131, 132, 131] {
        faked_map_tap(&mut view, Key::Tab);
        assert_eq!(view.map.selected(), Some(SystemId(expected)));
    }
    faked_map_input(&mut view, &key(Key::Shift, true));
    for expected in [132, 131] {
        faked_map_tap(&mut view, Key::Char('\\'));
        assert_eq!(view.map.selected(), Some(SystemId(expected)));
    }
    faked_map_input(&mut view, &key(Key::Shift, false));
    assert_eq!(sent(&view), [], "no nav mode written");
    assert_eq!(view.map.hyper_selection(), None);
    assert_eq!(view.ui_sounds, []);
    assert_eq!(
        view.map.take_gate_choice(),
        None,
        "hidden 131 is no offer (@0x638cd-0x638ff)"
    );
}

#[test]
fn tab_on_the_bars_map_is_refused_and_changes_nothing() {
    let mut view = faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    view.open_bar_map();
    let selected = view.map.selected();
    let nav = view.nav_scene.clone();
    for k in [Key::Tab, Key::Char('\\')] {
        assert!(!faked_map_tap(&mut view, k), "{k:?}");
        assert_eq!(sent(&view).last(), Some(&FlightCommand::CycleMapSelection));
        assert_eq!(view.map.selected(), selected, "{k:?}");
        assert_eq!(view.map.hyper_selection(), None, "{k:?}");
        assert_eq!(view.nav_scene, nav, "{k:?}");
    }
}

// Item 3's update: what the owner's settling redraws (`_MapFilter`'s Tab
// and `\`, `_DoSystemMap`'s Clear Route).

/// Drags the open map from empty space a pixel left, too little for the
/// scroll to follow, and releases: the partial update kept.
fn jiggle_map(view: &mut Faked) {
    let (from, to) = (at(300.0, 200.0), at(299.0, 200.0));
    let button = |pressed, at| Input::PointerButton {
        button: MouseButton::Left,
        pressed,
        at,
    };
    faked_map_input(view, &button(true, from));
    faked_map_input(view, &Input::PointerMoved(to));
    faked_map_input(view, &button(false, to));
    assert_eq!(view.map.update(), Some(MapUpdate::Partial));
}

#[test]
fn a_course_cycle_the_flight_takes_redraws_item_3_with_a_link_or_none() {
    // The flight takes the cycle with no link too, selecting none: the
    // course cycle redraws anyway (@0x14032).
    for k in [Key::Tab, Key::Char('\\')] {
        let mut view = faked_sol(in_sol());
        view.open_map();
        jiggle_map(&mut view);
        view.sim.queue_answer(Answer::Done);
        faked_map_tap(&mut view, k);
        assert_eq!(view.map.update(), Some(MapUpdate::Full), "{k:?}");
    }
}

#[test]
fn tab_on_the_hypergate_map_redraws_item_3_as_it_picks() {
    let mut view = at_a_faked_gate();
    jiggle_map(&mut view);
    faked_map_tap(&mut view, Key::Tab);
    assert_eq!(view.map.update(), Some(MapUpdate::Full));
}

#[test]
fn tab_on_a_hypergate_map_with_nothing_to_pick_redraws_nothing() {
    let mut view = at_a_faked_gate();
    view.sim.map_mut().hypergate_cycle.clear();
    jiggle_map(&mut view);
    faked_map_tap(&mut view, Key::Tab);
    assert_eq!(
        view.map.update(),
        Some(MapUpdate::Partial),
        "no destination: nothing invalidated (`jle` @0x13daf)"
    );
}

#[test]
fn tab_on_the_bars_map_redraws_nothing() {
    let mut view = faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    view.open_bar_map();
    jiggle_map(&mut view);
    faked_map_input(&mut view, &key(Key::Tab, true));
    assert_eq!(sent(&view), [FlightCommand::CycleMapSelection]);
    assert_eq!(
        view.map.update(),
        Some(MapUpdate::Partial),
        "_onPlanet: the key falls through, the flight refusing it"
    );
}

#[test]
fn clear_route_redraws_item_3_by_c_or_its_button() {
    let plotted = FlightFrame {
        course_shown: vec![SystemId(131)],
        course_plotted: true,
        ..in_sol()
    };
    for (frame, k) in [(plotted, Key::Char('c')), (in_sol(), Key::Char('C'))] {
        let mut view = faked_sol(frame);
        view.open_map();
        jiggle_map(&mut view);
        view.sim.queue_answer(Answer::Done);
        faked_map_tap(&mut view, k);
        assert_eq!(sent(&view), [FlightCommand::ClearRoute], "{k:?}");
        assert_eq!(
            view.map.update(),
            Some(MapUpdate::Full),
            "{k:?}: with a course or none, whatever the button"
        );
    }
}

// The course map's nebulae by `RuleKey::NebulaPicture`.

/// The `PICT` each nebula `list`'s open course map draws is drawn from,
/// and whether it is drawn with `addMax`.
fn nebulae_in(list: &DrawList) -> Vec<(i16, bool)> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { command, .. } => match **command {
                DrawCommand::BlendedPicture { image, blend, .. } => {
                    Some((image.id, blend == crate::draw::Blend::Max))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A view over a scripted flight in Sol by `rules`, over Sol's nebula,
/// the flight's galaxy holding it explored, with the course map open: at
/// 0.5625 its rect is 106 pixels a side. The nebulae its map draws.
fn faked_nebulae_by(rules: &Rulebook) -> Vec<(i16, bool)> {
    let catalog = FakeCatalog {
        nebula: true,
        ..catalog()
    };
    let galaxy = galaxy_of(&catalog, |overlay| {
        overlay.set_nebula_explored(nova_sim::NebulaId(128), true);
    });
    let mut fake = FakeFlight::new();
    fake.set_frame(in_sol());
    fake.map_mut().drawing = Some(sol_drawing());
    fake.map_mut().galaxy = Some(galaxy);
    fake.queue_event(FlightEvent::Session(nova_sim::SessionEvent::GalaxyChanged));
    let mut view = scripted_over(catalog, fake, rules);
    view.catch_up_then(|_, _| {});
    view.open_map();
    let mut list = DrawList::new();
    view.map.draw(&mut list);
    nebulae_in(&list)
}

#[test]
fn the_course_map_draws_a_nebula_from_the_largest_picture_that_fits() {
    assert_eq!(
        faked_nebulae_by(&Rulebook::default()),
        [(9500, true)],
        "30 fits, 120 doesn't"
    );
}

#[test]
fn by_the_other_reading_the_course_map_draws_the_smallest_picture_that_covers() {
    let rules = Rulebook::default().with_override(RuleKey::NebulaPicture, RuleSource::Bible);
    assert_eq!(faked_nebulae_by(&rules), [(9502, true)]);
}

#[test]
fn return_closes_the_course_map_and_a_repeat_does_not() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    assert!(!faked_map_input(&mut view, &held(Key::Enter)), "a repeat");
    assert!(faked_map_input(&mut view, &key(Key::Enter, true)));
}

#[test]
fn a_key_down_the_map_doesnt_take_closes_it_while_the_map_key_is_held() {
    let mut view = faked_sol(in_sol());
    view.open_map_by_key();
    // Every key `_MapFilter` tests before the map key's close.
    for k in [
        Key::Tab,
        Key::Char('\\'),
        Key::Space,
        Key::Char('b'),
        Key::Char('c'),
        Key::Char('-'),
        Key::Char('='),
        Key::Char('+'),
        Key::Left,
        Key::Right,
        Key::Up,
        Key::Down,
        Key::Char('f'),
        Key::Char('s'),
    ] {
        assert!(!faked_map_tap(&mut view, k), "{k:?}");
    }
    assert!(
        !faked_map_input(&mut view, &held(LAND)),
        "a repeat never closes it"
    );
    assert!(
        faked_map_input(&mut view, &key(LAND, true)),
        "L with M held"
    );
    faked_map_input(&mut view, &key(LAND, false));
    view.open_map_by_key();
    faked_map_input(&mut view, &key(MAP, false));
    assert!(!faked_map_tap(&mut view, LAND), "M let go");
}

#[test]
fn a_modifier_is_no_key_down_and_never_closes_the_map() {
    // A Mac sends no key-down for a modifier alone, so `_MapFilter`
    // never sees one.
    let mut view = faked_sol(in_sol());
    view.open_map_by_key();
    for k in [
        Key::Shift,
        Key::Control,
        Key::Alt,
        Key::Command,
        Key::CapsLock,
    ] {
        assert!(!faked_map_tap(&mut view, k), "{k:?}");
    }
}

#[test]
fn in_port_tab_or_backslash_with_the_map_key_held_closes_the_map() {
    // Both fall through on a planet (@0x13d20).
    for k in [Key::Tab, Key::Char('\\')] {
        let mut view = faked_sol(FlightFrame {
            landed: Some(StellarId(128)),
            ..in_sol()
        });
        view.open_bar_map();
        faked_map_input(&mut view, &held(MAP));
        assert!(faked_map_input(&mut view, &key(k, true)), "{k:?}");
    }
}

#[test]
fn closing_the_map_redraws_the_radar_at_once_leaving_the_timer_and_blink() {
    // The course map, closed by its key or as Escape does, both through
    // `close_map`'s port half; the hypergate map's closes are
    // `closing_the_hypergate_map_redraws_the_radar_at_once_leaving_the_timer_and_blink`'s.
    let mut view = super::faked_hud::barred(catalog(), among(Vec::new()));
    view.sim.map_mut().drawing = Some(sol_drawing());
    for _ in 0..3 {
        super::faked_hud::hud_tick(&mut view);
    }
    view.open_map();
    assert!(view.map_open);
    let stale = RadarScene::default();
    assert_ne!(view.radar_now(&view.frame), stale);
    view.radar_scene = Some(stale);
    let cadence = view.hud_cadence;
    view.close_map_then(|_, _| panic!("no hypergate"));
    assert!(!view.map_open);
    assert_eq!(view.hud_cadence, cadence, "neither timed nor flipped");
    assert_eq!(*view.radar_shown(&view.frame), view.radar_now(&view.frame));
}

// The floating map.

/// The floating map's frame's centre, where the current system is drawn.
const FRAME_CENTRE: Point = Point::new(686.0, 278.0);
/// A point in the frame that is in no box.
const FRAME_MISS: Point = Point::new(700.0, 300.0);
/// A point outside the frame.
const OUTSIDE_FRAME: Point = Point::new(415.0, 284.0);

/// Where the floating map in `commands` draws systems' ovals, in draw order:
/// the centre of each run of 1-pixel rows in a colour code's colour,
/// each row the one after the last, inside the frame. Its black fills
/// cover but draw nothing.
fn floating_dots(commands: &DrawList) -> Vec<Point> {
    let codes = [
        DARK_GREY,
        LIGHT_GREY,
        BRIGHT_GREEN,
        CLEARED,
        DENIED,
        HOSTILE,
    ];
    let frame = floating::frame(VIEW_SIZE);
    let mut ovals: Vec<(Color, f32, Bounds)> = Vec::new();
    for command in commands {
        let DrawCommand::Line {
            from,
            to,
            width,
            color,
        } = *command
        else {
            continue;
        };
        let opaque = Color { a: 255, ..color };
        if width != 1.0 || from.y != to.y || !codes.contains(&opaque) {
            continue;
        }
        if !floating::contains(frame, from) {
            continue;
        }
        let row = Bounds {
            min: Point::new(from.x.min(to.x), from.y - 0.5),
            max: Point::new(from.x.max(to.x), from.y + 0.5),
        };
        match ovals.last_mut() {
            Some((last, top, area))
                if *last == opaque && (row.min.y == *top || row.min.y == *top + 1.0) =>
            {
                *top = row.min.y;
                area.min.x = area.min.x.min(row.min.x);
                area.max.x = area.max.x.max(row.max.x);
                area.max.y = row.max.y;
            }
            _ => ovals.push((opaque, row.min.y, row)),
        }
    }
    ovals
        .into_iter()
        .map(|(_, _, area)| area.center())
        .collect()
}

/// Where the floating map draws Proxima Near (134) at the zoom whose
/// scale is `scale`: the pixel its point truncates to.
fn near_at(scale: f32) -> Point {
    Point::new(
        (686.0 + 40.0 * scale).trunc(),
        (278.0 + 30.0 * scale).trunc(),
    )
}

/// The course map's drawing in Sol over [`FakeCatalog::near`]'s galaxy,
/// as a session there draws it: [`sol_drawing`] with Proxima Near (134,
/// at (40, 30)), unexplored, linked from Sol too.
fn near_drawing() -> MapDrawing {
    let mut drawing = sol_drawing();
    drawing.current_links.push((SystemId(130), SystemId(134)));
    drawing.links.push(MapLink {
        from: SystemId(130),
        to: SystemId(134),
        selected: false,
    });
    drawing
        .systems
        .push(drawn_system(134, SystemColorCode::Unexplored));
    drawing
}

/// The floating map's places in Sol: Sol at the galaxy's origin, and
/// each of `links` at its place.
fn sol_places(links: &[(i16, (f32, f32))]) -> FloatingLinks {
    FloatingLinks {
        current: Vec2::ZERO,
        links: links
            .iter()
            .map(|&(id, (x, y))| (SystemId(id), Vec2::new(x, y)))
            .collect(),
    }
}

/// A view over a scripted flight in `frame`, drawing the map as
/// [`sol_drawing`], with Sol's floating map places: its link to Alpha
/// Centauri (131) at (600, 0).
pub(super) fn floating_in(frame: FlightFrame) -> Faked {
    let mut view = faked_sol(frame);
    view.sim.map_mut().floating = sol_places(&[(131, (600.0, 0.0))]);
    view
}

/// [`floating_in`] over [`FakeCatalog::near`]'s galaxy, drawing it as
/// [`near_drawing`], Proxima Near (134) linked at (40, 30) too.
fn floating_near(frame: FlightFrame) -> Faked {
    let catalog = FakeCatalog {
        near: true,
        ..catalog()
    };
    let mut view = faked_in(catalog, frame);
    let map = view.sim.map_mut();
    map.drawing = Some(near_drawing());
    map.map_preselection = Some(SystemId(130));
    map.floating = sol_places(&[(131, (600.0, 0.0)), (134, (40.0, 30.0))]);
    view
}

/// Presses `k` or lets it go, as flight's input does for the floating
/// map's keys: a press of H or `\` runs its key
/// ([`FlightView::hyperspace_key`]) and a zoom key's zooms
/// ([`FlightView::zoom_floating_map`]), and a flight key is tracked as
/// held either way.
fn floating_key(view: &mut Faked, k: Key, pressed: bool) {
    if pressed {
        match k {
            HYPER_SELECT_KEY | HYPERSPACE_MODE_KEY => view.hyperspace_key(k),
            ZOOM_OUT_KEY | ZOOM_IN_KEY | ZOOM_IN_KEYPAD_KEY => view.zoom_floating_map(k),
            _ => {}
        }
    }
    view.track_held(&key(k, pressed));
}

/// Presses and lets go of `k`, as [`floating_key`] does.
fn floating_tap(view: &mut Faked, k: Key) {
    floating_key(view, k, true);
    floating_key(view, k, false);
}

/// Holds `k` down without pressing it, as flight's input tracks a key it
/// takes no press of.
fn hold(view: &mut Faked, k: Key) {
    view.track_held(&key(k, true));
}

/// One frame of `dt` as flight's tick runs the floating map's: the clock
/// moved on, the nav keys held polled, then the map shown while its keys
/// are held, or hidden once it has expired.
fn floating_frame_of(view: &mut Faked, dt: Duration) {
    view.elapsed += dt;
    view.nav_held_frame();
    view.floating_frame();
}

/// What the floating map draws now.
fn floating_drawn(view: &Faked) -> DrawList {
    let mut list = DrawList::new();
    view.draw_floating_map(&mut list);
    list
}

/// The alpha the floating map's border is drawn at, if it is drawn.
pub(super) fn alpha_of(view: &Faked) -> Option<u8> {
    floating_border(&floating_drawn(view))
        .first()
        .map(|command| match command {
            DrawCommand::Line { color, .. } => color.a,
            _ => unreachable!(),
        })
}

#[test]
fn holding_h_shows_the_floating_map_bordered_and_centred_on_the_current_system() {
    let mut view = floating_in(in_sol());
    assert_eq!(floating_border(&floating_drawn(&view)), [], "down at first");
    floating_key(&mut view, HYPERSPACE_MODE_KEY, true);
    floating_frame_of(&mut view, TICK);
    assert_eq!(
        sent(&view),
        [FlightCommand::NavModeKeys {
            nav_off: false,
            h: true
        }]
    );
    let list = floating_drawn(&view);
    let edge = |from: (f32, f32), to: (f32, f32)| DrawCommand::Line {
        from: at(from.0, from.1),
        to: at(to.0, to.1),
        width: 1.0,
        color: MENU_COLORS.border,
    };
    assert_eq!(
        floating_border(&list),
        [
            edge((558.0, 150.0), (814.0, 150.0)),
            edge((814.0, 150.0), (814.0, 406.0)),
            edge((814.0, 406.0), (558.0, 406.0)),
            edge((558.0, 406.0), (558.0, 150.0)),
        ]
    );
    assert_eq!(floating_dots(&list), [FRAME_CENTRE], "Sol, at the centre");
    assert!(
        list.iter().any(|command| matches!(command,
            DrawCommand::Line { color, .. } if *color == CYAN)),
        "the current system's dot in it"
    );
}

#[test]
fn holding_h_keeps_it_full_and_letting_go_starts_the_fade() {
    let mut view = floating_in(in_sol());
    floating_key(&mut view, HYPERSPACE_MODE_KEY, true);
    view.track_held(&held(HYPERSPACE_MODE_KEY));
    for _ in 0..100 {
        floating_frame_of(&mut view, TICK);
    }
    floating_key(&mut view, HYPERSPACE_MODE_KEY, false);
    floating_frame_of(&mut view, mac_ticks(118));
    assert_eq!(alpha_of(&view), Some(255), "shown again each frame");
    floating_frame_of(&mut view, mac_ticks(1));
    assert_eq!(alpha_of(&view), Some(247), "31 of 32 at 119");
    floating_frame_of(&mut view, mac_ticks(31));
    assert_eq!(alpha_of(&view), None, "faded out at 150");
}

#[test]
fn a_tap_of_h_shows_it_before_the_next_frame() {
    let mut view = floating_in(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(alpha_of(&view), Some(255));
    floating_frame_of(&mut view, mac_ticks(118));
    assert_eq!(alpha_of(&view), Some(255));
    floating_frame_of(&mut view, mac_ticks(1));
    assert_eq!(alpha_of(&view), Some(247), "not shown again");
}

#[test]
fn in_double_time_the_fade_keeps_its_ticks() {
    // Each frame's floating map, then its step of two passes.
    let frame = |view: &mut Faked, dt| {
        floating_frame_of(view, dt);
        let passes = vec![nova_sim::flight_frame::PassReport::default(); 2];
        view.sim.queue_report(nova_sim::flight_frame::StepReport {
            steps: vec![nova_sim::flight_frame::StepRun {
                passes,
                ..nova_sim::flight_frame::StepRun::default()
            }],
            stopped: false,
        });
        view.step_frame(
            1,
            crate::system::camera::Anchor::in_flight(RuleSource::Engine),
        );
    };
    let mut view = floating_in(in_sol());
    view.double_time = true;
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    frame(&mut view, mac_ticks(118));
    assert_eq!(alpha_of(&view), Some(255));
    frame(&mut view, mac_ticks(1));
    assert_eq!(alpha_of(&view), Some(247));
    frame(&mut view, mac_ticks(31));
    assert_eq!(alpha_of(&view), None);
}

#[test]
fn h_held_through_a_jump_doesnt_show_it() {
    // Flight's input takes no press of H during a jump.
    let mut view = floating_in(jumping_to_alpha());
    hold(&mut view, HYPERSPACE_MODE_KEY);
    floating_frame_of(&mut view, TICK);
    assert_eq!(alpha_of(&view), None);
}

#[test]
fn backslash_held_in_mode_3_shows_it_through_a_jump() {
    // Flight's input takes no press of `\` during a jump.
    let mut view = floating_in(jumping_to_alpha());
    hold(&mut view, HYPER_SELECT_KEY);
    assert_eq!(alpha_of(&view), None, "no press");
    floating_frame_of(&mut view, mac_ticks(1));
    assert_eq!(alpha_of(&view), Some(255));
    assert_eq!(sent(&view), [], "no selection during the jump");
}

#[test]
fn backslash_held_out_of_mode_3_doesnt_show_it() {
    // In Barnard (132), which lists no links: Hyper Select leaves the
    // nav off.
    let mut view = floating_in(FlightFrame {
        system: SystemId(132),
        ..in_sol()
    });
    floating_key(&mut view, HYPER_SELECT_KEY, true);
    assert_eq!(sent(&view), [FlightCommand::SelectNextSystem]);
    floating_frame_of(&mut view, TICK);
    assert_eq!(alpha_of(&view), None);
}

#[test]
fn h_held_in_the_take_off_grace_shows_it() {
    // The grace refuses the map key, and only it of these.
    let mut view = floating_in(in_sol());
    view.change_sim(|sim| sim.refuse(PlayerCommand::Map));
    floating_key(&mut view, HYPERSPACE_MODE_KEY, true);
    for _ in 0..14 {
        floating_frame_of(&mut view, TICK);
    }
    floating_key(&mut view, HYPERSPACE_MODE_KEY, false);
    floating_frame_of(&mut view, mac_ticks(118));
    assert_eq!(alpha_of(&view), Some(255), "held through the grace");
}

/// [`floating_in`] `frame` once the player has exploded: the flight takes
/// no command.
fn exploded_in(frame: FlightFrame) -> Faked {
    let mut view = floating_in(frame);
    for gate in [PlayerCommand::Nav, PlayerCommand::SelectSystem] {
        view.change_sim(|sim| sim.refuse(gate));
    }
    view
}

#[test]
fn h_held_once_exploded_doesnt_show_it() {
    // Flight's input takes no press the flight refuses.
    let mut view = exploded_in(in_sol());
    hold(&mut view, HYPERSPACE_MODE_KEY);
    floating_frame_of(&mut view, TICK);
    assert_eq!(alpha_of(&view), None);
}

#[test]
fn backslash_held_in_mode_3_once_exploded_doesnt_show_it() {
    let mut view = exploded_in(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        ..in_sol()
    });
    hold(&mut view, HYPER_SELECT_KEY);
    floating_frame_of(&mut view, TICK);
    assert_eq!(alpha_of(&view), None);
}

#[test]
fn it_hides_once_more_than_500_ticks_have_passed() {
    let mut view = floating_in(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    floating_frame_of(&mut view, mac_ticks(500));
    assert!(
        view.click_floating_map(FRAME_MISS),
        "still up at 500: swallowed"
    );
    floating_frame_of(&mut view, mac_ticks(1));
    assert!(
        !view.click_floating_map(FRAME_MISS),
        "hidden at 501: passed on to space"
    );
}

#[test]
fn a_click_on_the_centre_sends_mode_3_with_nothing_and_shows_it_again() {
    let mut view = floating_in(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        course_shown: vec![SystemId(131)],
        course_plotted: true,
        ..in_sol()
    });
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    let pressed = sent(&view).len();
    view.sim.queue_answer(Answer::Done);
    assert!(view.click_floating_map(FRAME_CENTRE));
    assert_eq!(
        sent(&view)[pressed..],
        [FlightCommand::SelectOnFloatingMap(None)]
    );
}

#[test]
fn a_click_in_a_links_box_sends_it_in_mode_3() {
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    view.sim.queue_answer(Answer::Done);
    // Proxima Near, 40 right and 30 down at 16/9: (757.1, 331.3).
    assert!(view.click_floating_map(at(757.0, 331.0)));
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::SelectOnFloatingMap(Some(SystemId(134))))
    );
}

#[test]
fn away_from_the_origin_the_map_centres_on_the_current_system_and_clicks_follow() {
    let mut view = floating_near(FlightFrame {
        system: SystemId(134),
        ..in_sol()
    });
    let mut drawing = near_drawing();
    drawing.current = SystemId(134);
    let map = view.sim.map_mut();
    map.drawing = Some(drawing);
    map.floating = FloatingLinks {
        current: Vec2::new(40.0, 30.0),
        links: vec![(SystemId(130), Vec2::ZERO)],
    };
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    // Sol, 40 left and 30 up at 16/9: (614.9, 224.7).
    let sol = Point::new(
        (686.0 - 40.0 * (16.0_f32 / 9.0)).trunc(),
        (278.0 - 30.0 * (16.0_f32 / 9.0)).trunc(),
    );
    assert_eq!(
        floating_dots(&floating_drawn(&view)),
        [sol, FRAME_CENTRE],
        "Sol, then Proxima Near at the centre, by ascending ID"
    );
    view.sim.queue_answer(Answer::Done);
    assert!(view.click_floating_map(at(614.0, 224.0)));
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::SelectOnFloatingMap(Some(SystemId(130))))
    );
}

#[test]
fn a_miss_in_the_frame_sends_nothing_and_plays_nothing() {
    let mut view = floating_in(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    let pressed = sent(&view).len();
    assert!(view.click_floating_map(FRAME_MISS), "swallowed");
    assert_eq!(sent(&view).len(), pressed);
    assert_eq!(view.ui_sounds, []);
    assert!(
        !view.click_floating_map(OUTSIDE_FRAME),
        "outside: passed on to space"
    );
    let mut view = floating_in(in_sol());
    assert!(!view.click_floating_map(FRAME_MISS), "the map down");
}

#[test]
fn a_click_is_taken_while_the_map_is_faded_out() {
    let mut view = floating_in(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    floating_frame_of(&mut view, mac_ticks(200));
    assert_eq!(alpha_of(&view), None, "faded out");
    assert!(view.click_floating_map(FRAME_MISS), "swallowed");
    view.sim.queue_answer(Answer::Done);
    assert!(view.click_floating_map(FRAME_CENTRE));
    assert_eq!(
        sent(&view).last(),
        Some(&FlightCommand::SelectOnFloatingMap(None))
    );
}

#[test]
fn a_hit_the_flight_takes_shows_the_map_again() {
    let mut view = floating_in(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    floating_frame_of(&mut view, mac_ticks(100));
    view.sim.queue_answer(Answer::Done);
    view.click_floating_map(FRAME_CENTRE);
    floating_frame_of(&mut view, mac_ticks(118));
    assert_eq!(alpha_of(&view), Some(255));
    floating_frame_of(&mut view, mac_ticks(1));
    assert_eq!(alpha_of(&view), Some(247));
}

#[test]
fn a_hit_the_flight_refuses_during_a_jump_doesnt_show_the_map_again() {
    let mut view = floating_in(jumping_to_alpha());
    hold(&mut view, HYPER_SELECT_KEY);
    floating_frame_of(&mut view, TICK);
    assert_eq!(alpha_of(&view), Some(255));
    view.track_held(&key(HYPER_SELECT_KEY, false));
    floating_frame_of(&mut view, mac_ticks(100));
    assert!(view.click_floating_map(FRAME_CENTRE), "taken");
    assert_eq!(
        sent(&view),
        [FlightCommand::SelectOnFloatingMap(None)],
        "and refused"
    );
    floating_frame_of(&mut view, mac_ticks(19));
    assert_eq!(alpha_of(&view), Some(247), "not shown again");
}

#[test]
fn minus_zooms_out_a_step_with_snd_150() {
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(
        floating_dots(&floating_drawn(&view)),
        [FRAME_CENTRE, near_at(16.0 / 9.0)]
    );
    floating_tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(
        floating_dots(&floating_drawn(&view)),
        [FRAME_CENTRE, near_at(4.0 / 3.0)]
    );
    assert_eq!(view.ui_sounds, [UiSound::FloatingMapZoomed]);
    assert_eq!(ZOOM_OUT_KEY, Key::Char('-'));
}

/// Where the floating map draws Proxima Near now.
fn near_dot(view: &Faked) -> Point {
    floating_dots(&floating_drawn(view))[1]
}

#[test]
fn command_and_minus_the_self_destruct_chord_do_not_zoom() {
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    hold(&mut view, Key::Command);
    floating_tap(&mut view, SELF_DESTRUCT_KEY);
    assert_eq!(near_dot(&view), near_at(16.0 / 9.0));
    assert!(!view.ui_sounds.contains(&UiSound::FloatingMapZoomed));
}

#[test]
fn minus_held_alone_does_not_hold_the_zoom_in_back() {
    // `_HandlePlayer` checks `-` only with Command down (@0x6fdc8), so
    // `-` alone never sets `_quickKeyCheckResult`, which the zoom reads
    // (@0x70f5d).
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    // The press zooms out a step.
    floating_key(&mut view, SELF_DESTRUCT_KEY, true);
    assert_eq!(near_dot(&view), near_at(4.0 / 3.0));
    view.ui_sounds.clear();
    floating_tap(&mut view, ZOOM_IN_KEY);
    assert_eq!(near_dot(&view), near_at(16.0 / 9.0));
    assert_eq!(view.ui_sounds, [UiSound::FloatingMapZoomed]);
}

#[test]
fn equals_and_keypad_plus_zoom_in_a_step() {
    for k in [ZOOM_IN_KEY, ZOOM_IN_KEYPAD_KEY] {
        let mut view = floating_near(in_sol());
        floating_tap(&mut view, HYPERSPACE_MODE_KEY);
        floating_tap(&mut view, k);
        assert_eq!(near_dot(&view), near_at(64.0 / 27.0), "{k:?}");
        assert_eq!(view.ui_sounds, [UiSound::FloatingMapZoomed], "{k:?}");
    }
    assert_eq!(ZOOM_IN_KEY, Key::Char('='));
    assert_eq!(ZOOM_IN_KEYPAD_KEY, Key::Char('+'));
}

#[test]
fn a_zoom_at_its_limit_changes_nothing_and_says_nothing() {
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    floating_tap(&mut view, ZOOM_IN_KEY);
    view.ui_sounds.clear();
    floating_tap(&mut view, ZOOM_IN_KEY);
    assert_eq!(near_dot(&view), near_at(64.0 / 27.0));
    assert_eq!(view.ui_sounds, []);
    for _ in 0..6 {
        floating_tap(&mut view, ZOOM_OUT_KEY);
    }
    assert_eq!(near_dot(&view), near_at(27.0 / 64.0));
    view.ui_sounds.clear();
    floating_tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(near_dot(&view), near_at(27.0 / 64.0));
    assert_eq!(view.ui_sounds, []);
}

#[test]
fn a_zoom_key_does_nothing_while_another_key_is_held() {
    for k in [
        Key::Shift,
        Key::Alt,
        Key::Control,
        Key::Command,
        HYPERSPACE_MODE_KEY,
        HYPER_SELECT_KEY,
        NAV_OFF_KEY,
        JUMP_KEY,
        Key::Up,
    ] {
        let mut view = floating_near(in_sol());
        floating_tap(&mut view, HYPERSPACE_MODE_KEY);
        hold(&mut view, k);
        floating_tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(near_dot(&view), near_at(16.0 / 9.0), "{k:?}");
        assert_eq!(view.ui_sounds, [], "{k:?}");
        view.track_held(&key(k, false));
        floating_tap(&mut view, ZOOM_OUT_KEY);
        assert_eq!(near_dot(&view), near_at(4.0 / 3.0), "{k:?} let go");
    }
}

#[test]
fn a_zoom_key_does_nothing_while_b_is_held() {
    // B's `_QuickKeyCheck` (@0x6a98b) sets `_quickKeyCheckResult`, which
    // the zoom reads (@0x70f5d).
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    hold(&mut view, BOARD_KEY);
    floating_tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(near_dot(&view), near_at(16.0 / 9.0));
    assert_eq!(view.ui_sounds, []);
    view.track_held(&key(BOARD_KEY, false));
    floating_tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(near_dot(&view), near_at(4.0 / 3.0), "let go");
}

#[test]
fn a_zoom_key_does_nothing_with_the_map_down() {
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, ZOOM_OUT_KEY);
    floating_tap(&mut view, ZOOM_IN_KEY);
    assert_eq!(view.ui_sounds, []);
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(near_dot(&view), near_at(16.0 / 9.0));
}

#[test]
fn a_zoom_shows_the_map_again() {
    let mut view = floating_near(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    floating_frame_of(&mut view, mac_ticks(140));
    floating_tap(&mut view, ZOOM_OUT_KEY);
    floating_frame_of(&mut view, mac_ticks(118));
    assert_eq!(alpha_of(&view), Some(255));
}

#[test]
fn the_map_is_drawn_after_the_double_time_icon_and_before_the_escort_menu() {
    // The escort menu's rows are the frame's.
    let mut frame = among(Vec::new());
    frame.double_time = true;
    frame.hud.escort_rows[0].present = true;
    let mut view = super::faked_world::world(catalog(), frame);
    view.escort_menu
        .toggle(view.elapsed, &view.frame.hud.escort_rows);
    assert!(view.escort_menu.is_open());
    view.sim.map_mut().drawing = Some(sol_drawing());
    view.hyperspace_key(HYPERSPACE_MODE_KEY);
    let list = super::faked_world::world_list(&view);
    let commands: Vec<&DrawCommand> = list.iter().collect();
    let icon = commands
        .iter()
        .position(|command| icon_in(20000..=20000)(command))
        .expect("the double-time icon");
    let border = floating_border(&list);
    let map = commands
        .iter()
        .position(|command| **command == border[0])
        .expect("the floating map");
    let menu = commands
        .iter()
        .position(
            |command| matches!(command, DrawCommand::Line { color, .. } if *color == Color::BLACK),
        )
        .expect("the escort menu's black");
    assert!(icon < map && map < menu, "{icon} < {map} < {menu}");
}

/// The rects of `color`, whatever their alpha, in `list`, as (left,
/// top, right, bottom).
fn rects_of(list: &DrawList, of: Color) -> Vec<(f32, f32, f32, f32)> {
    list.iter()
        .filter_map(|command| match *command {
            DrawCommand::Line {
                from,
                to,
                width,
                color,
            } if Color { a: 255, ..color } == of && from.y == to.y => Some((
                from.x.min(to.x),
                from.y - width / 2.0,
                from.x.max(to.x),
                from.y + width / 2.0,
            )),
            _ => None,
        })
        .collect()
}

/// [`sol_drawing`] with the link to Alpha Centauri (131) selected in mode
/// 3, and the course to it if `course`, as a session draws them.
fn drawn_to_alpha(course: bool) -> MapDrawing {
    let (sol, alpha) = (SystemId(130), SystemId(131));
    MapDrawing {
        course: if course {
            vec![(sol, alpha)]
        } else {
            Vec::new()
        },
        links: vec![MapLink {
            from: sol,
            to: alpha,
            selected: true,
        }],
        mode3_link: Some((sol, alpha)),
        ..sol_drawing()
    }
}

#[test]
fn the_floating_map_draws_the_flights_selection_clipped_to_the_frame() {
    let mut view = floating_in(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        ..in_sol()
    });
    view.sim.map_mut().drawing = Some(drawn_to_alpha(false));
    view.sim.queue_answer(Answer::System(Some(SystemId(131))));
    floating_tap(&mut view, HYPER_SELECT_KEY);
    assert_eq!(sent(&view), [FlightCommand::SelectNextSystem]);
    assert_eq!(view.map.hyper_selection(), None, "never opened");
    let list = floating_drawn(&view);
    assert_eq!(
        rects_of(&list, DIM_GREEN),
        [(690.0, 278.0, 814.0, 279.0)],
        "from the edge of Sol's oval to the frame's, drawn once"
    );
    assert_eq!(
        rects_of(&list, LESS_DARK_GREY),
        [],
        "the link under it is cut away"
    );
}

#[test]
fn the_floating_map_draws_the_course_as_its_show_took_it() {
    let mut view = floating_in(in_sol());
    view.sim.set_frame(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(131))),
        course_shown: vec![SystemId(131)],
        course_plotted: true,
        ..in_sol()
    });
    view.sim.map_mut().drawing = Some(drawn_to_alpha(true));
    assert_eq!(view.map.route(), [], "the course map never shown it");
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    let course = rects_of(&floating_drawn(&view), BRIGHT_GREEN);
    assert_eq!(
        course,
        [(690.0, 279.0, 814.0, 280.0)],
        "from the edge of Sol's oval to the frame's, at pen 2, its top \
         row under Sol's link"
    );
    assert_eq!(
        rects_of(&floating_drawn(&view), DIM_GREEN),
        [(690.0, 278.0, 814.0, 279.0)],
        "the link to 131, the hop the plot cued, drawn after the course \
         over it, and over its own grey"
    );
    assert_eq!(rects_of(&floating_drawn(&view), LESS_DARK_GREY), []);
    view.sim.map_mut().drawing = Some(sol_drawing());
    assert_eq!(
        rects_of(&floating_drawn(&view), BRIGHT_GREEN),
        course,
        "until the next show"
    );
}

/// The names the floating map in `list` draws, clipped to its frame.
fn floating_names(list: &DrawList) -> Vec<String> {
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Clipped { command, .. } => match command.as_ref() {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn above_a_zoom_of_1_1_the_floating_map_names_only_the_system_its_show_selected() {
    let mut view = floating_near(FlightFrame {
        nav_mode: NavMode::Hyperspace(Some(SystemId(134))),
        ..in_sol()
    });
    let mut drawing = near_drawing();
    drawing.named.push(SystemId(134));
    let map = view.sim.map_mut();
    map.drawing = Some(drawing);
    map.floating_preselection = Some(SystemId(134));
    floating_tap(&mut view, HYPER_SELECT_KEY);
    assert_eq!(
        floating_names(&floating_drawn(&view)),
        ["Sol", "Proxima Near"],
        "every system named in the frame at 16/9"
    );
    for _ in 0..3 {
        floating_tap(&mut view, ZOOM_OUT_KEY);
    }
    assert_eq!(
        floating_names(&floating_drawn(&view)),
        ["Proxima Near"],
        "at 3/4"
    );
    view.sim.map_mut().floating_preselection = None;
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(
        floating_names(&floating_drawn(&view)),
        [] as [&str; 0],
        "shown with none"
    );
}

#[test]
fn the_floating_map_takes_its_mission_arrows_as_it_shows() {
    let mut view = floating_in(in_sol());
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    let red = |view: &Faked| {
        view.floating_picture
            .as_ref()
            .expect("shown")
            .systems
            .iter()
            .any(|system| system.id == SystemId(131) && system.red_arrow)
    };
    let asked = |missions: Vec<SystemId>| MapAsk::Drawing {
        hypergate: None,
        missions,
        marks: nova_sim::MapMarks::NONE,
    };
    assert_eq!(view.sim.asked().last(), Some(&asked(Vec::new())));
    assert!(!red(&view), "no mission");
    let mut arrowed = sol_drawing();
    arrowed.systems[1].red_arrow = true;
    let map = view.sim.map_mut();
    map.mission_systems = vec![SystemId(131)];
    map.drawing = Some(arrowed);
    assert!(!red(&view), "not until the next show");
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    assert_eq!(
        view.sim.asked().last(),
        Some(&asked(vec![SystemId(131)])),
        "the mission systems taken as it shows"
    );
    assert!(red(&view), "Alpha Centauri, the mission's system, arrowed");
}

/// The systems the course map draws ovals for, by ascending ID.
fn drawn_ids(map: &GalaxyMap) -> Vec<SystemId> {
    map.picture()
        .expect("drawn")
        .systems
        .iter()
        .map(|system| system.id)
        .collect()
}

/// [`sol_drawing`] on the hypergate map, its arrow pointing to Barnard
/// (132).
fn gate_drawing() -> MapDrawing {
    MapDrawing {
        hypergate: true,
        gate_arrows: vec![SystemId(132)],
        ..sol_drawing()
    }
}

/// [`gate_drawing`] once Barnard is known: drawn, unexplored.
fn gate_drawing_known() -> MapDrawing {
    let mut drawing = gate_drawing();
    drawing
        .systems
        .push(drawn_system(132, SystemColorCode::Unexplored));
    drawing
}

/// A view over a scripted flight in Sol with the hypergate map open,
/// offering Barnard (132), drawn as [`gate_drawing`].
fn at_a_faked_gate_to_barnard() -> Faked {
    let mut view = faked(in_sol());
    let map = view.sim.map_mut();
    map.drawing = Some(gate_drawing());
    map.map_preselection = Some(SystemId(130));
    map.hypergate_cycle = vec![Some(SystemId(132))];
    view.map.offer_gates(vec![SystemId(132)]);
    view.open_map();
    view
}

#[test]
fn the_hypergate_map_marks_its_arrows_known_and_its_next_draw_shows_them() {
    let mut view = at_a_faked_gate_to_barnard();
    assert_eq!(
        view.sim.asked().first(),
        Some(&MapAsk::Drawing {
            hypergate: Some(vec![SystemId(132)]),
            missions: Vec::new(),
            marks: nova_sim::MapMarks::NONE,
        })
    );
    let marked = FlightCommand::MarkKnown(vec![SystemId(132)]);
    assert_eq!(
        sent(&view),
        std::slice::from_ref(&marked),
        "marked known as its arrow was drawn"
    );
    assert!(!drawn_ids(&view.map).contains(&SystemId(132)), "first draw");
    view.sim.map_mut().drawing = Some(gate_drawing_known());
    faked_map_tap(&mut view, Key::Tab);
    assert!(drawn_ids(&view.map).contains(&SystemId(132)), "after Tab");
    assert_eq!(sent(&view), [marked.clone(), marked]);
}

#[test]
fn the_hypergate_maps_own_mark_takes_no_redraw_a_frame_on() {
    // The gate leads to Barnard (132), which nothing has made known: the
    // map's draw marks it known, which changes the flight's map key.
    let mut view = faked(in_sol());
    let map = view.sim.map_mut();
    map.drawing = Some(gate_drawing());
    map.map_preselection = Some(SystemId(130));
    map.hypergate_cycle = vec![Some(SystemId(132))];
    let known = FlightFrame {
        map_key: nova_sim::flight_frame::MapKey {
            known: [SystemId(132)].into(),
            ..in_sol().map_key
        },
        ..in_sol()
    };
    view.sim.queue_answer_showing(Answer::Done, known);
    view.map.offer_gates(vec![SystemId(132)]);
    view.open_map();
    let drawings = |view: &Faked| {
        view.sim
            .asked()
            .iter()
            .filter(|ask| matches!(ask, MapAsk::Drawing { .. }))
            .count()
    };
    assert_eq!(sent(&view), [FlightCommand::MarkKnown(vec![SystemId(132)])]);
    assert_eq!(drawings(&view), 1, "first draw");
    view.tick(TICK);
    assert_eq!(drawings(&view), 1, "a frame on: no redraw for its own mark");
    view.sim.set_frame(FlightFrame {
        map_key: nova_sim::flight_frame::MapKey {
            known: [SystemId(131), SystemId(132)].into(),
            ..in_sol().map_key
        },
        ..in_sol()
    });
    view.tick(TICK);
    assert_eq!(drawings(&view), 2, "while a change behind it redraws");
}

/// What the flight answers a press on the hypergate map that selects
/// `selected` and picks `pick` (`None` for no pick).
fn gate_picked(selected: i16, pick: Option<i16>) -> Answer {
    Answer::Clicked(MapClicked {
        selected: Some(SystemId(selected)),
        gate_pick: Some(pick.map(SystemId)),
        clear_route: None,
        course_changed: false,
        nav_redraw: false,
    })
}

#[test]
fn a_click_on_the_hypergate_map_sends_the_press_and_draws_it_again() {
    let mut view = at_a_faked_gate_to_barnard();
    let sol = faked_on_map(&mut view, 130);
    view.sim.queue_answer(gate_picked(130, None));
    view.sim.map_mut().drawing = Some(gate_drawing_known());
    faked_map_click(&mut view, sol);
    assert_eq!(
        sent(&view)[1],
        FlightCommand::MapClick(MapClick {
            hits: vec![SystemId(130)],
            shift: false,
            hypergate: Some(vec![SystemId(132)]),
            missions: Vec::new(),
            green: None,
        })
    );
    assert!(drawn_ids(&view.map).contains(&SystemId(132)));
}

#[test]
fn the_course_map_draws_the_flights_drawing() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    assert_eq!(
        view.sim.asked().first(),
        Some(&MapAsk::Drawing {
            hypergate: None,
            missions: Vec::new(),
            marks: nova_sim::MapMarks::NONE,
        })
    );
    let picture = view.map.picture().expect("drawn");
    assert_eq!(drawn_ids(&view.map), [SystemId(130), SystemId(131)]);
    assert_eq!(picture.named, [SystemId(130)]);
    assert_eq!(picture.current, Some(SystemId(130)));
    assert_eq!(sent(&view), [], "the course map marks nothing known");
}

/// How many drawings `view`'s flight was asked for.
fn drawings_asked(view: &Faked) -> usize {
    view.sim
        .asked()
        .iter()
        .filter(|ask| matches!(ask, MapAsk::Drawing { .. }))
        .count()
}

#[test]
fn a_change_to_what_the_open_map_is_drawn_from_redraws_it_at_the_catch_up() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    assert_eq!(drawings_asked(&view), 1, "as it opened");
    view.catch_up_then(|_, _| {});
    assert_eq!(drawings_asked(&view), 1, "nothing changed");
    let mut explored = sol_drawing();
    explored.systems.push(drawn_system(
        132,
        nova_sim::iff::SystemColorCode::Uninhabited,
    ));
    view.sim.map_mut().drawing = Some(explored);
    let mut frame = in_sol();
    frame.map_key.explored = vec![SystemId(132)];
    view.sim.set_frame(frame);
    view.catch_up_then(|_, _| {});
    assert_eq!(drawings_asked(&view), 2, "explored while the map is open");
    assert!(drawn_ids(&view.map).contains(&SystemId(132)));
    view.catch_up_then(|_, _| {});
    assert_eq!(drawings_asked(&view), 2, "noted");
}

#[test]
fn a_galaxy_event_redraws_the_open_map_though_its_key_is_the_same() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    view.sim
        .queue_event(FlightEvent::Session(nova_sim::SessionEvent::GalaxyChanged));
    view.catch_up_then(|_, _| {});
    assert_eq!(drawings_asked(&view), 2);
}

#[test]
fn a_shut_map_is_not_redrawn_at_the_catch_up() {
    let mut view = faked_sol(in_sol());
    view.sim
        .queue_event(FlightEvent::Session(nova_sim::SessionEvent::GalaxyChanged));
    let mut frame = in_sol();
    frame.map_key.explored = vec![SystemId(132)];
    view.sim.set_frame(frame);
    view.catch_up_then(|_, _| {});
    assert_eq!(drawings_asked(&view), 0);
}

// The map panel (`_MapDialogUpdate`): the session's panels, drawn from
// the dialog's origin (212, 128), so the strip's left is 220 and top 564,
// and the column's left is 686 and top 136.

/// `STR#` 2002 with the panel's words, every other string empty.
fn panel_strings() -> (i16, Vec<String>) {
    let mut list = vec![String::new(); 400];
    for (n, word) in [
        (308, "Pick a destination"),
        (309, "Ports:"),
        (310, "<Unknown>"),
        (327, "Goods Traded:"),
        (328, "Services:"),
        (329, "Trading"),
        (330, "Outfitting"),
        (331, "Shipyard"),
        (335, "None"),
        (337, "Current System:"),
        (338, "Selected System:"),
        (339, "Destination System:"),
        (340, "Hypergate Destination:"),
    ] {
        list[n - 1] = word.to_owned();
    }
    (nova_sim::news::MISC_STRINGS, list)
}

/// The Geneva 9 text drawn with its baseline at (`x`, `baseline`), if
/// any: the panels' lines.
fn panel_text(list: &DrawList, (x, baseline): (f32, f32)) -> Option<String> {
    list.iter().find_map(|command| match command {
        DrawCommand::Text {
            text, origin, size, ..
        } if *size == 9.0 && *origin == Point::new(x, baseline - 9.0) => Some(text.clone()),
        _ => None,
    })
}

/// Where the column's title, the Ports list, the first goods row and the
/// first two services rows are drawn.
const TITLE_AT: (f32, f32) = (696.0, 148.0);
const PORTS_AT: (f32, f32) = (265.0, 576.0);
const GOODS_AT: (f32, f32) = (701.0, 304.0);
const SERVICES_AT: (f32, f32) = (701.0, 399.0);
const SECOND_SERVICE_AT: (f32, f32) = (701.0, 411.0);

/// The open course map drawn, as flight draws it over the scene.
fn map_drawn(view: &Faked) -> DrawList {
    let mut list = DrawList::new();
    view.draw_open_map(&mut list);
    list
}

/// The column's title the open course map draws.
fn faked_title(view: &Faked) -> Option<String> {
    panel_text(&map_drawn(view), TITLE_AT)
}

/// A panel with the panels' words ([`panel_strings`]): the strip
/// listing `ports` with no hazards or date, the column titled `title`
/// over `body`.
fn info_panel(ports: Ports, title: &str, body: ColumnBody) -> MapPanel {
    MapPanel {
        strip: Strip::Info {
            ports,
            hazards: String::new(),
            date: String::new(),
        },
        column: Column {
            title: title.to_owned(),
            body,
        },
        words: PanelWords::from_list(&panel_strings().1),
    }
}

/// A view over a scripted flight in Sol with the course map open, its
/// flight giving `panel` whatever is selected.
fn faked_panel(panel: MapPanel) -> Faked {
    let mut view = faked_sol(in_sol());
    view.sim.map_mut().panel = Some(panel);
    view.open_map();
    view
}

/// The panels `view`'s flight was asked for, in order.
fn panels_asked(view: &Faked) -> Vec<MapAsk> {
    view.sim
        .asked()
        .into_iter()
        .filter(|ask| matches!(ask, MapAsk::Panel { .. }))
        .collect()
}

/// The panel question for `selected` on the course map.
fn panel_for(selected: i16) -> MapAsk {
    MapAsk::Panel {
        selected: Some(SystemId(selected)),
        hypergate: None,
    }
}

#[test]
fn the_course_maps_panel_is_the_flights_for_the_system_selected_and_lists_its_ports() {
    // Phase 15: `_MapDialogUpdate` @0x10373-0x10504.
    let ports = Ports::Listed(vec!["Earth".to_owned()]);
    let view = faked_panel(info_panel(ports, "Current System:", ColumnBody::Unknown));
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(panels_asked(&view), [panel_for(130)]);
    let list = map_drawn(&view);
    assert_eq!(panel_text(&list, (230.0, 576.0)).as_deref(), Some("Ports:"));
    assert_eq!(panel_text(&list, PORTS_AT).as_deref(), Some("Earth"));
}

#[test]
fn the_course_maps_panel_follows_a_click_to_the_flights_panel_for_it() {
    // Phase 16: `_MapDialogUpdate` @0x10b3f-0x10c76.
    let sols_ports = Ports::Listed(vec![String::new(), String::new()]);
    let mut view = faked_panel(info_panel(
        sols_ports,
        "Current System:",
        ColumnBody::Unknown,
    ));
    assert_eq!(faked_title(&view).as_deref(), Some("Current System:"));
    assert_eq!(
        panel_text(&map_drawn(&view), PORTS_AT).as_deref(),
        Some(", "),
        "Sol's two unnamed ports"
    );
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(Answer::Clicked(MapClicked {
        selected: Some(SystemId(131)),
        gate_pick: None,
        clear_route: None,
        course_changed: false,
        nav_redraw: true,
    }));
    view.sim.map_mut().panel = Some(info_panel(
        Ports::Unknown,
        "Destination System:",
        ColumnBody::Unknown,
    ));
    faked_map_click(&mut view, alpha);
    assert_eq!(view.map.selected(), Some(SystemId(131)));
    assert_eq!(panels_asked(&view).last(), Some(&panel_for(131)));
    assert_eq!(faked_title(&view).as_deref(), Some("Destination System:"));
    assert_eq!(
        panel_text(&map_drawn(&view), PORTS_AT).as_deref(),
        Some("<Unknown>"),
        "the panel follows the click to unexplored Alpha Centauri"
    );
}

#[test]
fn the_hypergate_maps_panel_is_asked_with_its_offers_and_draws_the_prompt_then_the_title() {
    // Phase 16: `_MapDialogUpdate` @0x10213-0x102cd and @0x10ba2-0x10c34.
    let mut view = faked_in(
        FakeCatalog {
            near: true,
            ..catalog()
        },
        in_sol(),
    );
    let prompt = MapPanel {
        strip: Strip::HypergatePrompt {
            prompt: "Pick a destination".to_owned(),
            date: String::new(),
        },
        ..info_panel(Ports::None, "Current System:", ColumnBody::Unknown)
    };
    let map = view.sim.map_mut();
    map.drawing = Some(gate_drawing());
    map.map_preselection = Some(SystemId(130));
    map.panel = Some(prompt);
    view.map.offer_gates(vec![SystemId(134)]);
    view.open_map();
    let offered = Some(vec![SystemId(134)]);
    assert_eq!(
        panels_asked(&view),
        [MapAsk::Panel {
            selected: Some(SystemId(130)),
            hypergate: offered.clone(),
        }]
    );
    // The prompt at 430 + 594 / 2, unmeasured, on the strip's top + 24.
    let list = map_drawn(&view);
    assert_eq!(
        panel_text(&list, (517.0, 588.0)).as_deref(),
        Some("Pick a destination")
    );
    assert_eq!(panel_text(&list, PORTS_AT), None, "the prompt alone");
    assert_eq!(faked_title(&view).as_deref(), Some("Current System:"));
    let near = faked_on_map(&mut view, 134);
    view.sim.queue_answer(gate_picked(134, Some(134)));
    view.sim.map_mut().panel = Some(info_panel(
        Ports::Unknown,
        "Hypergate Destination:",
        ColumnBody::Unknown,
    ));
    faked_map_click(&mut view, near);
    assert_eq!(view.map.selected(), Some(SystemId(134)));
    assert_eq!(
        panels_asked(&view).last(),
        Some(&MapAsk::Panel {
            selected: Some(SystemId(134)),
            hypergate: offered,
        })
    );
    assert_eq!(
        faked_title(&view).as_deref(),
        Some("Hypergate Destination:")
    );
}

#[test]
fn the_course_maps_panel_draws_goods_and_services_unknown_or_listed_as_the_flight_says() {
    // Phase 21: `_MapDialogUpdate` @0x10f34, @0x1107d.
    let inhabited = |goods, services| ColumnBody::Inhabited {
        name: "Sol".to_owned(),
        government: String::new(),
        legal: String::new(),
        goods,
        services,
    };
    let level_1 = inhabited(Listing::Unknown, Services::Unknown { any: true });
    let list = map_drawn(&faked_panel(info_panel(
        Ports::None,
        "Current System:",
        level_1,
    )));
    assert_eq!(panel_text(&list, GOODS_AT).as_deref(), Some("<Unknown>"));
    assert_eq!(panel_text(&list, SERVICES_AT).as_deref(), Some("<Unknown>"));
    let level_2 = inhabited(
        Listing::Items(vec!["Food".to_owned()]),
        Services::Items(vec!["Trading".to_owned(), "Outfitting".to_owned()]),
    );
    let list = map_drawn(&faked_panel(info_panel(
        Ports::None,
        "Current System:",
        level_2,
    )));
    assert_eq!(panel_text(&list, GOODS_AT).as_deref(), Some("Food"));
    assert_eq!(panel_text(&list, SERVICES_AT).as_deref(), Some("Trading"));
    assert_eq!(
        panel_text(&list, SECOND_SERVICE_AT).as_deref(),
        Some("Outfitting")
    );
}

#[test]
fn a_click_on_empty_space_drags_and_keeps_the_selection_and_its_panel() {
    // A press that selects nothing drags (`_DoSystemMap` @0x137e9),
    // leaving `_selSystem` as it was.
    let sols_ports = Ports::Listed(vec![String::new(), String::new()]);
    let mut view = faked_panel(info_panel(
        sols_ports,
        "Current System:",
        ColumnBody::Unknown,
    ));
    faked_map_click(&mut view, Point::new(250.0, 150.0));
    assert_eq!(sent(&view), [], "no system hit: nothing sent");
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(panels_asked(&view), [panel_for(130)], "not asked again");
    assert_eq!(faked_title(&view).as_deref(), Some("Current System:"));
    assert_eq!(
        panel_text(&map_drawn(&view), PORTS_AT).as_deref(),
        Some(", "),
        "Sol's two unnamed ports"
    );
}

// Where the in-flight maps look (`_mapZoom`, `_mapScroll`), and what is
// drawn behind their dialog.

/// `map`'s zoom and scroll.
pub(super) fn place_of(map: &GalaxyMap) -> (f64, (i32, i32)) {
    let place = map.place().expect("an in-flight map");
    (place.zoom(), place.scroll())
}

/// Where system `id` is in `map`'s galaxy, as its scroll holds it.
fn scroll_on(map: &GalaxyMap, id: i16) -> (i32, i32) {
    let at = map.model().position(SystemId(id)).expect("on the map");
    (at.x as i32, at.y as i32)
}

#[test]
fn the_maps_zoom_and_scroll_are_kept_between_opens() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    faked_map_tap(&mut view, Key::Char('='));
    faked_map_tap(&mut view, Key::Right);
    faked_map_input(&mut view, &held(Key::Right));
    assert_eq!(place_of(&view.map), (0.421_875, (20, 0)));
    assert!(faked_map_tap(&mut view, Key::Enter), "closed");
    view.open_map();
    assert_eq!(
        place_of(&view.map),
        (0.421_875, (20, 0)),
        "where it was left"
    );
}

#[test]
fn opening_the_map_guards_a_zoom_of_0_or_less() {
    let mut view = faked_sol(in_sol());
    view.map = view.map.clone().with_zoom(0.0);
    view.open_map();
    assert_eq!(place_of(&view.map).0, 1.0);
}

#[test]
fn a_floating_map_show_puts_the_scroll_back_on_the_current_system() {
    let mut view = floating_in(in_sol());
    view.map.centre_on_system(SystemId(131));
    assert_eq!(place_of(&view.map).1, (600, 0));
    floating_key(&mut view, HYPERSPACE_MODE_KEY, true);
    floating_frame_of(&mut view, TICK);
    assert_eq!(alpha_of(&view), Some(255), "the floating map up");
    assert_eq!(place_of(&view.map).1, scroll_on(&view.map, 130));
}

#[test]
fn the_bars_map_shows_the_scroll_the_flight_left() {
    let mut view = faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    view.map.centre_on_system(SystemId(132));
    view.open_bar_map();
    assert_eq!(place_of(&view.map).1, (0, 600));
}

#[test]
fn the_hypergate_map_draws_the_dialog_with_clear_route_disabled() {
    let mut view = faked_sol(FlightFrame {
        course_plotted: true,
        ..in_sol()
    });
    view.map.offer_gates(vec![SystemId(131)]);
    view.open_map();
    let clear_route = map_drawn(&view).iter().find_map(|command| match command {
        DrawCommand::Text { text, color, .. } if text == "Clear Route" => Some(*color),
        _ => None,
    });
    assert_eq!(
        clear_route,
        Some(crate::ui::button::ButtonStyle::STOCK.grey)
    );
}

/// The index in `list` of the map's background, `PICT` 8509.
fn background_at(list: &DrawList) -> Option<usize> {
    list.iter().position(|command| {
        matches!(command, DrawCommand::StretchedPicture { image, .. }
            if *image == ImageKey::picture(crate::galaxy::dialog::BACKGROUND))
    })
}

#[test]
fn with_the_map_open_in_flight_the_paused_scene_is_drawn_behind_the_dialog() {
    let mut view = super::faked_screen::in_flight();
    let shut = drawn(&view);
    tap(&mut view, MAP);
    let list = drawn(&view);
    let background = background_at(&list).expect("the dialog");
    let sprite = list
        .iter()
        .position(|command| matches!(command, DrawCommand::Sprite { .. }))
        .expect("the scene's sprites");
    assert!(sprite < background, "{sprite} {background}");
    // The frame shut, less the cursor, then the dialog.
    let scene: Vec<&DrawCommand> = list.iter().take(background - 1).collect();
    let before: Vec<&DrawCommand> = shut.iter().take(scene.len()).collect();
    assert_eq!(scene, before);
}

#[test]
fn in_port_the_map_is_drawn_alone() {
    let mut view = super::faked_shops::docked();
    view.sim.map_mut().drawing = Some(sol_drawing());
    view.open_bar_map();
    let list = drawn(&view);
    assert_eq!(background_at(&list), Some(1), "the dialog's black, then it");
}

// Clear Route, C and the open latch (`_DoSystemMap` item 8,
// @0x12c69-0x12dbf; `showClearRouteButton`, @0x12396-0x123c3).

/// Where Clear Route is on the screen.
fn clear_route_button() -> Point {
    crate::galaxy::buttons::MapButtons::new(crate::galaxy::dialog::DIALOG_ORIGIN)
        .rect(crate::galaxy::buttons::MapButton::ClearRoute)
        .center()
}

/// What the flight answers a plain click on the course map that selects
/// `selected` and writes the nav.
fn selected_by_click(selected: i16) -> Answer {
    Answer::Clicked(MapClicked {
        selected: Some(SystemId(selected)),
        gate_pick: None,
        clear_route: None,
        course_changed: false,
        nav_redraw: true,
    })
}

#[test]
fn the_map_opens_with_clear_route_enabled_by_a_course_and_never_on_the_hypergate_map() {
    for (frame, enabled) in [(in_sol(), false), (plotted_to_alpha(), true)] {
        let mut view = faked_sol(frame);
        view.open_map();
        assert_eq!(view.map.clear_route_enabled(), enabled);
    }
    let mut view = faked_sol(plotted_to_alpha());
    view.map.offer_gates(vec![SystemId(131)]);
    view.open_map();
    assert_eq!(view.map.mode(), MapMode::Hypergate);
    assert!(!view.map.clear_route_enabled(), "the hypergate map");
}

#[test]
fn clear_route_by_its_button_sends_the_clear_shows_the_course_left_and_retakes_the_nav() {
    let mut view = faked_sol(plotted_to_alpha());
    view.open_map();
    assert_eq!(view.map.route(), [SystemId(131)]);
    let let_go = NavMode::Hyperspace(None);
    view.sim.set_frame(FlightFrame {
        nav_mode: let_go,
        ..in_sol()
    });
    view.sim.queue_answer(Answer::Done);
    assert!(!faked_map_click(&mut view, clear_route_button()), "open");
    assert_eq!(sent(&view), [FlightCommand::ClearRoute]);
    assert_eq!(view.map.route(), [], "the flight's course shown");
    assert_eq!(
        view.nav_scene,
        Some((
            let_go,
            NavDisplay::Hyperspace {
                destination: hud::NavDestination::None,
                readiness: nova_sim::JumpReadiness::Blocked,
            }
        )),
        "the nav area retaken at once"
    );
    assert!(!view.map.clear_route_enabled(), "disabled after");
}

#[test]
fn c_clears_the_route_whether_clear_route_is_enabled_or_not() {
    let mut view = faked_sol(plotted_to_alpha());
    view.open_map();
    view.sim.queue_answer(Answer::Done);
    faked_map_tap(&mut view, Key::Char('c'));
    assert_eq!(sent(&view), [FlightCommand::ClearRoute]);
    assert!(!view.map.clear_route_enabled());
    // Clear Route is disabled now, and C still clears.
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(selected_by_click(131));
    faked_map_click(&mut view, alpha);
    view.sim.queue_answer(Answer::Done);
    assert!(
        !faked_map_tap(&mut view, Key::Char('c')),
        "C doesn't close the map"
    );
    assert_eq!(sent(&view).len(), 3);
    assert_eq!(sent(&view)[2], FlightCommand::ClearRoute, "C again");
}

#[test]
fn c_on_the_hypergate_map_sends_the_clear_and_drops_the_pick_so_the_close_cancels() {
    let mut view = faked_sol(in_sol());
    view.map.offer_gates(vec![SystemId(131)]);
    view.open_map();
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(gate_picked(131, Some(131)));
    faked_map_click(&mut view, alpha);
    assert_eq!(view.map.selected(), Some(SystemId(131)));
    assert_eq!(view.map.clone().take_gate_choice(), Some(SystemId(131)));
    faked_map_tap(&mut view, Key::Char('c'));
    assert!(sent(&view).contains(&FlightCommand::ClearRoute));
    assert_eq!(view.map.take_gate_choice(), None, "no pick to enter");
}

#[test]
fn a_click_on_the_hypergate_maps_current_system_drops_the_pick_so_the_close_cancels() {
    let mut view = faked_sol(in_sol());
    view.map.offer_gates(vec![SystemId(131)]);
    view.open_map();
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(gate_picked(131, Some(131)));
    faked_map_click(&mut view, alpha);
    let sol = faked_on_map(&mut view, 130);
    view.sim.queue_answer(gate_picked(130, None));
    faked_map_click(&mut view, sol);
    assert_eq!(view.map.selected(), Some(SystemId(130)));
    assert_eq!(view.map.take_gate_choice(), None, "no pick to enter");
}

#[test]
fn shift_on_the_hypergate_map_sends_a_shifted_press_the_flight_picks_from() {
    let mut view = faked_sol(in_sol());
    view.map.offer_gates(vec![SystemId(131)]);
    view.open_map();
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(gate_picked(131, Some(131)));
    faked_map_shift_click(&mut view, alpha);
    assert!(sent(&view).contains(&FlightCommand::MapClick(MapClick {
        hits: vec![SystemId(131)],
        shift: true,
        hypergate: Some(vec![SystemId(131)]),
        missions: Vec::new(),
        green: None,
    })));
    assert_eq!(view.map.take_gate_choice(), Some(SystemId(131)), "entered");
}

#[test]
fn a_press_the_flight_says_wrote_a_hop_enables_clear_route_and_shows_the_course() {
    let mut view = faked_sol(in_sol());
    view.open_map();
    assert!(!view.map.clear_route_enabled());
    let alpha = faked_on_map(&mut view, 131);
    view.sim.queue_answer(selected_by_click(131));
    faked_map_click(&mut view, alpha);
    assert!(
        !view.map.clear_route_enabled(),
        "a plain click writes no hop"
    );
    view.sim.set_frame(plotted_to_alpha());
    view.sim.queue_answer(Answer::Clicked(MapClicked {
        selected: Some(SystemId(131)),
        gate_pick: None,
        clear_route: Some(true),
        course_changed: true,
        nav_redraw: false,
    }));
    faked_map_shift_click(&mut view, alpha);
    assert_eq!(
        sent(&view)[1],
        FlightCommand::MapClick(MapClick {
            hits: vec![SystemId(131)],
            shift: true,
            hypergate: None,
            missions: Vec::new(),
            green: None,
        })
    );
    assert!(view.map.clear_route_enabled());
    assert_eq!(view.map.route(), [SystemId(131)], "shown");
}

/// Whether a zoom key's press zooms `view`'s floating map, shown by H,
/// with `k` held.
fn faked_zooms_holding(mut view: Faked, k: Key) -> bool {
    floating_tap(&mut view, HYPERSPACE_MODE_KEY);
    hold(&mut view, k);
    let zoom = view.floating.zoom();
    floating_tap(&mut view, ZOOM_OUT_KEY);
    view.floating.zoom() != zoom
}

#[test]
fn a_slot_key_held_holds_the_zoom_back_while_the_original_polls_it() {
    for k in [
        STELLAR_SLOT_KEYS[1][0],
        STELLAR_SLOT_KEYS[2][1],
        NEAREST_STELLAR_KEY,
    ] {
        assert!(!faked_zooms_holding(floating_near(in_sol()), k), "{k:?}");
    }
    assert!(
        faked_zooms_holding(floating_near(in_sol()), NEAREST_STELLAR_COMMAND_KEY),
        "5 without Command is not polled"
    );
}

#[test]
fn a_slot_key_held_holds_the_zoom_back_while_the_escort_menus_loop_polls_it() {
    let menu_holding = |k: Key, menu: bool| {
        let mut frame = in_sol();
        frame.hud.escort_rows[2].present = true;
        frame.hud.escorts_flying = true;
        let mut view = floating_near(frame);
        floating_tap(&mut view, HYPERSPACE_MODE_KEY);
        if menu {
            view.escort_key(MENU_KEY);
            assert!(view.escort_menu().is_open());
        }
        hold(&mut view, k);
        let zoom = view.floating.zoom();
        floating_tap(&mut view, ZOOM_OUT_KEY);
        view.floating.zoom() != zoom
    };
    assert!(
        !menu_holding(STELLAR_SLOT_KEYS[1][0], true),
        "the escort loop polls 2 with the menu open"
    );
    assert!(!menu_holding(NEAREST_STELLAR_COMMAND_KEY, true), "and 5");
    assert!(
        menu_holding(NEAREST_STELLAR_COMMAND_KEY, false),
        "5 with the menu shut"
    );
}

#[test]
fn a_slot_key_held_during_a_jump_does_not_hold_the_zoom_back() {
    let mut view = floating_near(jumping_to_alpha());
    hold(&mut view, HYPER_SELECT_KEY);
    floating_frame_of(&mut view, TICK);
    view.track_held(&key(HYPER_SELECT_KEY, false));
    hold(&mut view, STELLAR_SLOT_KEYS[2][1]);
    floating_tap(&mut view, ZOOM_OUT_KEY);
    assert_eq!(view.floating.zoom(), 4, "F3 is not polled");
}

/// The port's course map, open over a flight docked at Earth whose map
/// draws Sol, and how many panels its flight was asked for.
fn port_map_open() -> (Faked, usize) {
    let mut view = super::faked_shops::docked();
    let map = view.sim.map_mut();
    map.drawing = Some(sol_drawing());
    map.map_preselection = Some(SystemId(130));
    view.open_bar_map();
    assert!(view.map_open());
    let asked = panels_asked(&view).len();
    assert!(asked > 0, "asked as it opens");
    view.tick(TICK);
    assert_eq!(panels_asked(&view).len(), asked, "nothing changed");
    (view, asked)
}

#[test]
fn the_open_maps_panel_follows_a_system_explored_between_frames() {
    // What the panel says of a system explored is the session's.
    let (mut view, asked) = port_map_open();
    view.sim.frame_mut().map_key.explored.push(SystemId(131));
    assert_eq!(panels_asked(&view).len(), asked, "not yet");
    view.tick(TICK);
    assert_eq!(panels_asked(&view).len(), asked + 1, "the next frame");
}

#[test]
fn the_open_maps_panel_follows_a_system_charted_between_frames() {
    // What the panel lists of a system charted is the session's.
    let (mut view, asked) = port_map_open();
    view.sim.frame_mut().map_key.charted.push(SystemId(130));
    assert_eq!(panels_asked(&view).len(), asked, "not yet");
    view.tick(TICK);
    assert_eq!(panels_asked(&view).len(), asked + 1, "the next frame");
}
