//! The app fits each screen's shape into the window: a widened screen
//! fills a 16:9 window whole, a classic one is pillarboxed, and the
//! pointer maps through whichever shows.

use super::*;

/// A screen that draws in `shape`.
fn shaped(window: &FakeWindow, shape: ViewShape) -> TestApp {
    let screen = RecordingScreen {
        shape,
        ..RecordingScreen::default()
    };
    App::new(window, NoImages, screen)
}

fn redraw(app: &mut TestApp, window: &mut FakeWindow) -> RecordingGpu {
    let mut gpu = RecordingGpu::new();
    app.handle(
        WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        },
        window,
        &mut gpu,
    );
    gpu
}

fn full(window: (u32, u32)) -> PixelRect {
    PixelRect {
        x: 0,
        y: 0,
        w: window.0,
        h: window.1,
    }
}

#[test]
fn a_widened_screen_fills_a_16_by_9_window_768_high() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Widened);
    assert_eq!(
        app.viewport().logical(),
        nova_render::LogicalSize { w: 1365, h: 768 }
    );
    let gpu = redraw(&mut app, &mut window);
    let frame = &gpu.submits()[0];
    assert_eq!(frame.logical, nova_render::LogicalSize { w: 1365, h: 768 });
    assert_eq!(frame.viewport, full((1920, 1080)), "no bars");
}

#[test]
fn a_classic_screen_in_the_same_window_is_pillarboxed() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Classic);
    let gpu = redraw(&mut app, &mut window);
    let frame = &gpu.submits()[0];
    assert_eq!(frame.logical, nova_render::LOGICAL);
    assert_eq!(
        frame.viewport,
        PixelRect {
            x: 240,
            y: 0,
            w: 1440,
            h: 1080
        }
    );
}

#[test]
fn the_screen_is_told_the_widened_size_at_the_start_and_on_each_resize() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Classic);
    assert_eq!(app.screen().widened, [(1365.0, 768.0)]);
    handle(
        &mut app,
        &mut window,
        WindowEvent::Resized {
            size_px: (1366, 768),
            scale_factor: 1.0,
        },
    );
    assert_eq!(app.screen().widened, [(1365.0, 768.0), (1366.0, 768.0)]);
}

#[test]
fn a_resize_refits_a_widened_screen() {
    let mut window = FakeWindow::new((1024, 768), 1.0);
    let mut app = shaped(&window, ViewShape::Widened);
    assert_eq!(app.viewport().logical(), nova_render::LOGICAL, "4:3");
    handle(
        &mut app,
        &mut window,
        WindowEvent::Resized {
            size_px: (3440, 1440),
            scale_factor: 1.0,
        },
    );
    assert_eq!(
        app.viewport().logical(),
        nova_render::LogicalSize { w: 1835, h: 768 }
    );
    assert_eq!(
        app.viewport().content().map(|(rect, _)| rect),
        Some(full((3440, 1440)))
    );
}

#[test]
fn a_screen_switching_to_widened_renders_widened_from_the_next_frame_and_maps_the_pointer_so() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Classic);
    app.screen.shape_on_input = Some(ViewShape::Widened);
    handle(&mut app, &mut window, key(Key::Char('f'), true));
    assert_eq!(
        app.viewport().logical(),
        nova_render::LogicalSize { w: 1365, h: 768 },
        "refitted after the input that switched it"
    );
    let gpu = redraw(&mut app, &mut window);
    assert_eq!(gpu.submits()[0].viewport, full((1920, 1080)));
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (5.0, 540.0) },
    );
    let Some(Input::PointerMoved(at)) = app.screen().inputs.last() else {
        panic!("{:?}", app.screen().inputs);
    };
    assert!((at.x - 3.555).abs() < 0.01, "{at:?}: no bar at the left");
    assert_eq!(at.y, 384.0);
}

#[test]
fn a_screen_switching_back_to_classic_is_pillarboxed_again() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Widened);
    app.screen.shape_on_input = Some(ViewShape::Classic);
    handle(&mut app, &mut window, key(Key::Char('l'), true));
    assert_eq!(app.viewport().logical(), nova_render::LOGICAL);
}

#[test]
fn a_click_across_a_widened_window_lands_under_the_pointer() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Widened);
    let click = WindowEvent::PointerButton {
        button: MouseButton::Left,
        pressed: true,
    };
    for px in [(1900.0, 540.0), (5.0, 540.0)] {
        handle(&mut app, &mut window, WindowEvent::PointerMoved { px });
        handle(&mut app, &mut window, click);
    }
    let clicks: Vec<Point> = app
        .screen()
        .inputs
        .iter()
        .filter_map(|input| match input {
            Input::PointerButton { at, .. } => Some(*at),
            _ => None,
        })
        .collect();
    assert_eq!(clicks.len(), 2, "neither is in a bar");
    assert!((clicks[0].x - 1350.78).abs() < 0.01, "{clicks:?}");
    assert!((clicks[1].x - 3.555).abs() < 0.01, "{clicks:?}");
    assert!(clicks.iter().all(|at| at.y == 384.0));
}

#[test]
fn a_screen_drawing_its_pointer_is_told_where_it_is_after_its_shape_changes() {
    let mut window = FakeWindow::new((1920, 1080), 1.0);
    let mut app = shaped(&window, ViewShape::Classic);
    app.screen.hides_pointer = true;
    handle(
        &mut app,
        &mut window,
        WindowEvent::PointerMoved { px: (960.0, 540.0) },
    );
    assert_eq!(
        app.screen().inputs.last(),
        Some(&Input::PointerMoved(Point::new(512.0, 384.0)))
    );
    app.screen.shape_on_input = Some(ViewShape::Widened);
    handle(&mut app, &mut window, key(Key::Char('f'), true));
    redraw(&mut app, &mut window);
    let Some(Input::PointerMoved(at)) = app.screen().inputs.last() else {
        panic!("{:?}", app.screen().inputs);
    };
    assert!(
        (at.x - 682.5).abs() < 0.01 && at.y == 384.0,
        "{at:?}: the same pixel, re-mapped through the widened viewport"
    );
}
