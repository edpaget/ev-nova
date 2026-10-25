//! A wheel or trackpad scroll reaches the screen at the pointer, in
//! logical units, only while the pointer is over the game and the
//! developer overlay doesn't show.

use nova_view::ScrollDelta;

use super::*;

fn scroll(delta: WheelDelta) -> WindowEvent {
    WindowEvent::Scroll { delta }
}

#[test]
fn a_scroll_over_the_game_reaches_the_screen_at_the_pointer() {
    let mut window = FakeWindow::new((2560, 1536), 2.0);
    let mut app = app(&window);
    // Pillarboxed: content x 256..2304 at scale 2.
    handle(&mut app, &mut window, moved(456.0, 100.0));
    assert_eq!(
        routed(&mut app, &mut window, scroll(WheelDelta::Lines(1.0))),
        handled(Control::Continue, Routing::Game)
    );
    handle(&mut app, &mut window, scroll(WheelDelta::Px(48.0)));
    let at = Point::new(100.0, 50.0);
    assert_eq!(
        app.screen().inputs[1..],
        [
            Input::Scroll {
                delta: ScrollDelta::Lines(1.0),
                at
            },
            Input::Scroll {
                delta: ScrollDelta::Units(24.0),
                at
            },
        ]
    );
}

#[test]
fn a_scroll_in_a_bar_or_off_the_window_reaches_no_screen() {
    let mut window = FakeWindow::new((2560, 1536), 2.0);
    let mut app = app(&window);
    handle(&mut app, &mut window, moved(10.0, 10.0));
    handle(&mut app, &mut window, scroll(WheelDelta::Lines(1.0)));
    handle(&mut app, &mut window, moved(456.0, 100.0));
    handle(&mut app, &mut window, WindowEvent::PointerLeft);
    handle(&mut app, &mut window, scroll(WheelDelta::Lines(1.0)));
    assert!(
        !app.screen()
            .inputs
            .iter()
            .any(|input| matches!(input, Input::Scroll { .. })),
        "{:?}",
        app.screen().inputs
    );
}

#[test]
fn a_scroll_goes_to_the_overlay_alone_while_it_shows() {
    let mut window = FakeWindow::new((2560, 1536), 2.0);
    let mut app = overlay_app(&window);
    handle(&mut app, &mut window, moved(456.0, 100.0));
    routed(&mut app, &mut window, key(BACKQUOTE, true));
    let handled_scroll = routed(&mut app, &mut window, scroll(WheelDelta::Lines(1.0)));
    assert_eq!(handled_scroll, handled(Control::Continue, Routing::Overlay));
    assert!(app.overlay_wants(Some(&handled_scroll)));
    assert_eq!(
        app.screen().inputs,
        [Input::PointerMoved(Point::new(100.0, 50.0))]
    );
}
