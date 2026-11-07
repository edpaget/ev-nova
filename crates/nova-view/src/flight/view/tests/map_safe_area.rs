//! The course map's safe area in its own frame
//! (rdm:phase/key-bindings/phase-16-touch-pointer-gaps): flight tells its
//! map the surroundings moved as the map's frame is (`map_offset`), when
//! told and again as the map opens, so the map's touch toggle is placed
//! inside the window's safe area over widened flight and over the
//! spaceport alike.

use super::*;
use crate::flight::PortScreen;
use crate::geometry::Bounds;
use crate::input::{Device, Surroundings};
use crate::view_shape::classic_offset;

/// A 16:9 window's widened view.
const WIDE: (f32, f32) = (1365.0, 768.0);

/// Touch, with a safe area cut 40 from each side and 20 from the top and
/// the bottom of `view`.
fn touch_in(view: (f32, f32)) -> Surroundings {
    Surroundings {
        safe_area: Bounds {
            min: Point::new(40.0, 20.0),
            max: Point::new(view.0 - 40.0, view.1 - 20.0),
        },
        ..Surroundings::of(Device::Touch)
    }
}

/// `told`'s safe area moved left by `x`.
fn moved_left(told: Surroundings, x: f32) -> Bounds {
    Bounds {
        min: Point::new(told.safe_area.min.x - x, told.safe_area.min.y),
        max: Point::new(told.safe_area.max.x - x, told.safe_area.max.y),
    }
}

#[test]
fn the_course_map_is_told_its_safe_area_in_its_own_frame_over_widened_flight() {
    let mut view = faked_sol(in_sol());
    view.view = WIDE;
    let told = touch_in(WIDE);
    view.input(&Input::Surroundings(told));
    let offset = classic_offset(WIDE.0);
    assert!(offset > 0.0);
    assert_eq!(
        view.course_map().touch_toggle().told().safe_area,
        moved_left(told, offset)
    );
}

#[test]
fn the_course_map_is_told_its_safe_area_in_its_own_frame_over_the_spaceport() {
    let mut view = faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    let told = touch_in(VIEW_SIZE);
    view.input(&Input::Surroundings(told));
    view.open_port_screen_map(PortScreen::Spaceport);
    assert_eq!(
        view.course_map().touch_toggle().told().safe_area,
        told.safe_area,
        "the classic frame, unmoved"
    );
}

#[test]
fn a_map_opened_in_widened_flight_after_the_spaceport_is_told_again_in_its_frame() {
    let mut view = faked_sol(FlightFrame {
        landed: Some(StellarId(128)),
        ..in_sol()
    });
    view.view = WIDE;
    let told = touch_in(WIDE);
    view.input(&Input::Surroundings(told));
    assert_eq!(
        view.course_map().touch_toggle().told().safe_area,
        told.safe_area,
        "docked, the map's frame is at 0"
    );
    view.sim.frame_mut().landed = None;
    view.refresh_frame();
    view.open_map();
    assert_eq!(
        view.course_map().touch_toggle().told().safe_area,
        moved_left(told, classic_offset(WIDE.0)),
        "told again as it opens over flight"
    );
}

/// The map's toggle takes the bottom-left corner of the safe area within
/// the map's own frame, 8 points in: on screen, the frame's left edge
/// (the safe area starts left of it), not 40 further right as an
/// untranslated safe area would put it.
#[test]
fn the_maps_toggle_lies_in_the_safe_areas_corner_over_widened_flight() {
    let mut view = faked_sol(in_sol());
    view.view = WIDE;
    let told = touch_in(WIDE);
    view.input(&Input::Surroundings(told));
    let rect = view.course_map().touch_toggle().rect();
    let offset = classic_offset(WIDE.0);
    let on_screen = Bounds {
        min: Point::new(rect.min.x + offset, rect.min.y),
        max: Point::new(rect.max.x + offset, rect.max.y),
    };
    assert!(
        told.safe_area.contains(on_screen.min) && told.safe_area.contains(on_screen.max),
        "{on_screen:?} in {:?}",
        told.safe_area
    );
    assert_eq!(on_screen.min.x, told.safe_area.min.x.max(offset) + 8.0);
    assert_eq!(on_screen.max.y, told.safe_area.max.y - 8.0);
}
