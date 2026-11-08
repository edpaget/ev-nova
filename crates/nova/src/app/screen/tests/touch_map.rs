//! A pinch through the router to the galaxy maps
//! (rdm:phase/key-bindings/phase-14-touch-input): the app hands the
//! touch router's [`Pinch`] to the screen as [`Input::Pinch`], and it
//! reaches the course map over flight and the viewer through the
//! navigator, zooming as the zoom key does. What a pinch does to the map
//! is `nova_view::galaxy::touch`'s; this pins the wiring alone.

use super::*;
use nova_view::Pinch;
use nova_view::galaxy::dialog::map_area;

/// Two points 90 apart about `centre`.
fn apart(centre: Point) -> (Point, Point) {
    (
        Point::new(centre.x - 45.0, centre.y),
        Point::new(centre.x + 45.0, centre.y),
    )
}

/// A pinch about `centre` that makes one `step`.
fn pinch(screen: &mut AppScreen, centre: Point, step: Pinch) {
    let (primary, second) = apart(centre);
    screen.input(&Input::Pinch(Pinch::Began { primary, second }));
    screen.input(&Input::Pinch(step));
    screen.input(&Input::Pinch(Pinch::Ended));
}

fn tap_key(screen: &mut AppScreen, k: Key) {
    screen.input(&key(k, true));
    screen.input(&key(k, false));
}

/// Flight with its course map open.
fn over_flight() -> AppScreen {
    let mut screen = AppScreen::new(data());
    fly(&mut screen);
    screen.input(&key(MAP, true));
    screen.input(&key(MAP, false));
    assert!(flight(&screen).map_open());
    screen
}

#[test]
fn a_pinch_reaches_the_course_map_over_flight() {
    let mut pinched = over_flight();
    let before = flight(&pinched).course_map().zoom();
    pinch(&mut pinched, map_area().center(), Pinch::ZoomIn);
    let mut keyed = over_flight();
    tap_key(&mut keyed, Key::Equal);
    assert_ne!(flight(&pinched).course_map().zoom(), before, "zoomed");
    assert_eq!(
        flight(&pinched).course_map().zoom(),
        flight(&keyed).course_map().zoom()
    );
}

/// The galaxy viewer.
fn on_the_viewer() -> AppScreen {
    let mut screen = AppScreen::new(data());
    screen.input(&key(Key::Tab, true));
    screen.input(&key(Key::Tab, false));
    assert_eq!(screen.showing(), Showing::GalaxyMap);
    screen
}

#[test]
fn a_pinch_reaches_the_viewer_through_the_navigator() {
    let mut pinched = on_the_viewer();
    let before = *pinched.galaxy_map().view().expect("the viewer");
    pinch(&mut pinched, Point::new(512.0, 300.0), Pinch::ZoomOut);
    let mut keyed = on_the_viewer();
    tap_key(&mut keyed, Key::Minus);
    assert_ne!(*pinched.galaxy_map().view().expect("the viewer"), before);
    assert_eq!(pinched.galaxy_map().view(), keyed.galaxy_map().view());
}
