//! The safe area and points per pixel reach the screen in
//! [`Surroundings`]: on a resize, at a redraw that reads a new safe area
//! before that frame is drawn, on a shape change, and with every device
//! switch; never twice for the same value.

use nova_view::{Device, Surroundings};

use super::*;
use crate::app::safe_area::fit;

fn redraw_at(app: &mut TestApp, window: &mut FakeWindow, ms: u64) {
    let elapsed = Duration::from_millis(ms);
    handle(app, window, WindowEvent::Redraw { elapsed });
}

/// The surroundings `window` gives through `app`'s viewport now, with
/// `device`.
fn expected(app: &TestApp, window: &FakeWindow, device: Device) -> Surroundings {
    let (safe_area, points_per_pixel) = fit(window.safe_area(), app.viewport()).expect("an area");
    Surroundings {
        device,
        safe_area,
        points_per_pixel,
        soft_keyboard: false,
    }
}

/// A safe area cutting `top`, `right`, `bottom` and `left` px from
/// `window`'s edges.
fn inset(window: &FakeWindow, top: u32, right: u32, bottom: u32, left: u32) -> PxRect {
    let (w, h) = window.size_px;
    PxRect {
        x: left,
        y: top,
        width: w - left - right,
        height: h - top - bottom,
    }
}

#[test]
fn a_resize_tells_the_screen_its_safe_rect_and_points_per_pixel() {
    let mut window = FakeWindow::new((2048, 1536), 2.0);
    let mut app = app(&window);
    window.size_px = (2560, 1536);
    window.safe_area = Some(inset(&window, 0, 400, 0, 0));
    handle(
        &mut app,
        &mut window,
        WindowEvent::Resized {
            size_px: (2560, 1536),
            scale_factor: 2.0,
        },
    );
    let told = expected(&app, &window, Device::Keyboard);
    assert_ne!(told, Surroundings::default());
    assert_eq!(app.screen().surroundings, [told]);
}

#[test]
fn a_safe_area_change_seen_at_a_redraw_reaches_the_screen_before_that_frame_is_drawn() {
    let mut window = FakeWindow::new((2048, 1536), 2.0);
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 16);
    assert_eq!(app.screen().surroundings, [], "the default, untold");
    window.safe_area = Some(inset(&window, 94, 0, 40, 0));
    redraw_at(&mut app, &mut window, 32);
    let told = expected(&app, &window, Device::Keyboard);
    assert_ne!(told, Surroundings::default());
    assert_eq!(
        app.screen().drawn_told.borrow().last().copied().flatten(),
        Some(told)
    );
}

#[test]
fn an_unchanged_safe_area_tells_nothing_again() {
    let mut window = FakeWindow::new((2048, 1536), 2.0);
    window.safe_area = Some(inset(&window, 94, 0, 40, 0));
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 16);
    redraw_at(&mut app, &mut window, 32);
    handle(&mut app, &mut window, key(Key::KeyA, true));
    assert_eq!(app.screen().surroundings.len(), 1);
}

#[test]
fn a_shape_change_tells_the_new_shapes_safe_rect() {
    let mut window = FakeWindow::new((2388, 1668), 2.0);
    window.safe_area = Some(inset(&window, 0, 60, 40, 60));
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 16);
    let classic = expected(&app, &window, Device::Keyboard);
    app.screen.shape_on_input = Some(ViewShape::Widened);
    handle(&mut app, &mut window, key(Key::KeyA, true));
    let widened = expected(&app, &window, Device::Keyboard);
    assert_ne!(widened, classic);
    assert_eq!(app.screen().surroundings, [classic, widened]);
}

#[test]
fn a_device_switch_carries_the_current_safe_area() {
    let mut window = FakeWindow::new((2048, 1536), 2.0);
    window.safe_area = Some(inset(&window, 94, 0, 40, 0));
    let mut app = app(&window);
    redraw_at(&mut app, &mut window, 16);
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (400.0, 400.0) },
    );
    let told = expected(&app, &window, Device::Mouse);
    assert_eq!(app.screen().surroundings.last(), Some(&told));
    assert_eq!(app.screen().surroundings.len(), 2);
}
