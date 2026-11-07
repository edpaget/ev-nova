//! A touch on each of flight's touch controls reaches it through the
//! app's viewport (rdm:phase/key-bindings/phase-15-touch-flight-controls):
//! the app maps a finger's pixels to the logical point the layout placed
//! the control at, inside the safe area it told, so the router claims it
//! and the screen gets the touch where the layout hit-tests to the same
//! control. A wiring test over `nova_view`'s public layout, as
//! `touch_map` is over the map; the layout's own rules are its.

use nova_view::flight::touch_controls::layout::{MenuView, Region, layout};
use nova_view::{FingerId, Touch, TouchPhase};

use super::*;

/// The window pixel the app's viewport shows logical point `at` at.
fn pixel(app: &TestApp, at: Point) -> (f64, f64) {
    let viewport = app.viewport();
    let (rect, _) = viewport.content().expect("a window with area");
    let (across, down) = viewport.axis_scales().expect("a window with area");
    (
        f64::from(rect.x) + f64::from(at.x) * f64::from(across),
        f64::from(rect.y) + f64::from(at.y) * f64::from(down),
    )
}

/// The layout the screen claims from, with the menu as `menu` says.
fn placed(app: &TestApp, menu: MenuView) -> nova_view::flight::touch_controls::layout::Layout {
    layout(&app.screen.touch_region(), menu)
}

/// Touches the centre of every control `menu`'s layout places, each with
/// a finger of its own, and checks the screen got each where the layout
/// hit-tests to that control.
fn touch_each(app: &mut TestApp, window: &mut FakeWindow, menu: MenuView, case: &str) {
    app.screen.claims_menu = Some(menu);
    let shown = placed(app, menu);
    assert!(!shown.placed.is_empty(), "{case}: {menu:?}");
    for (n, control) in shown.placed.iter().enumerate() {
        let px = pixel(app, control.rect.center());
        handle(
            app,
            window,
            WindowEvent::TouchStarted {
                finger: FingerId::new(1000 + n),
                px,
            },
        );
        let Some(&Input::Touch(Touch {
            phase: TouchPhase::Started(at),
            ..
        })) = app.screen.inputs.last()
        else {
            panic!(
                "{case}: {:?} was not claimed: {:?}",
                control.id,
                app.screen.inputs.last()
            );
        };
        assert_eq!(
            shown.hit(at),
            Some(control.id),
            "{case}: {:?} at {at:?}",
            control.id
        );
        handle(
            app,
            window,
            WindowEvent::TouchEnded {
                finger: FingerId::new(1000 + n),
                px,
            },
        );
    }
}

/// An app over `window` drawing in `shape`, whose screen claims the
/// controls' touches, told its surroundings by a first redraw.
fn claiming(window: &mut FakeWindow, shape: ViewShape) -> TestApp {
    let screen = RecordingScreen {
        shape,
        claims_menu: Some(MenuView::default()),
        ..RecordingScreen::default()
    };
    let mut app = App::new(window, NoImages, screen).with_os(Os::Windows);
    handle(
        &mut app,
        window,
        WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        },
    );
    app
}

fn every_control(window: &mut FakeWindow, shape: ViewShape, case: &str) {
    let mut app = claiming(window, shape);
    let region = app.screen.touch_region();
    let told = app.screen.surroundings.last().copied().unwrap_or_default();
    assert_eq!(region.points_per_pixel, told.points_per_pixel);
    touch_each(&mut app, window, MenuView::default(), case);
    touch_each(
        &mut app,
        window,
        MenuView {
            open: true,
            ..MenuView::default()
        },
        case,
    );
}

#[test]
fn a_touch_on_each_control_reaches_it_through_a_pillarboxed_viewport() {
    let mut window = FakeWindow::new((2560, 1536), 2.0);
    window.safe_area = Some(PxRect {
        x: 300,
        y: 0,
        width: 2560 - 300 - 400,
        height: 1536,
    });
    every_control(&mut window, ViewShape::Classic, "pillarboxed");
}

#[test]
fn a_touch_on_each_control_reaches_it_through_a_letterboxed_viewport() {
    let mut window = FakeWindow::new((2048, 2732), 2.0);
    window.safe_area = Some(PxRect {
        x: 0,
        y: 700,
        width: 2048,
        height: 2732 - 700 - 680,
    });
    every_control(&mut window, ViewShape::Widened, "letterboxed");
}

#[test]
fn a_touch_on_each_control_reaches_it_through_a_widened_viewport() {
    let mut window = FakeWindow::new((2388, 1668), 2.0);
    window.safe_area = Some(PxRect {
        x: 120,
        y: 48,
        width: 2388 - 240,
        height: 1668 - 48 - 40,
    });
    every_control(&mut window, ViewShape::Widened, "widened");
}

#[test]
fn the_region_the_screen_claims_from_is_inside_the_safe_area_the_app_told() {
    let mut window = FakeWindow::new((2560, 1536), 2.0);
    window.safe_area = Some(PxRect {
        x: 0,
        y: 0,
        width: 2560 - 400,
        height: 1536,
    });
    let app = claiming(&mut window, ViewShape::Classic);
    let told = app.screen.surroundings.last().copied().expect("told");
    let region = Region::for_flight(&told, (1024.0, 768.0));
    assert_eq!(app.screen.touch_region(), region);
    assert!(region.rect.max.x <= told.safe_area.max.x);
}
