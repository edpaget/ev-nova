//! Which metrics flight lays out its text with: [`AppScreen::metrics`]'s
//! dialogs-then-menu-then-none fallbacks. The tests compare where "No
//! Target" lands under each wiring with each other, so they pin no
//! layout constant of nova-view's.

use super::*;

use nova_view::flight::target::NO_TARGET;
use nova_view::text::TextMetrics;

/// [`data`] with the status bar, so flight draws its target panel.
fn bar_data() -> Rc<GameData> {
    loaded(&barred(game_data_fork(
        false, false, false, 100, b"", b"", None,
    )))
}

/// Where flight's "No Target" starts across.
fn no_target_x(screen: &AppScreen) -> f32 {
    drawn(screen)
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { text, origin, .. } if text == NO_TARGET => Some(origin.x),
            _ => None,
        })
        .expect("flight draws No Target")
}

/// Where "No Target" starts once `screen` flies from its ship browser.
fn flown_no_target_x(mut screen: AppScreen) -> f32 {
    fly(&mut screen);
    no_target_x(&screen)
}

/// Whether two positions across are the same, to well under a pixel.
fn same(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn only_the_main_menus_metrics_lay_out_flights_text() {
    let store = MemoryPilots::default();
    let given: Rc<dyn TextMetrics> = Rc::new(MonoMetrics);
    let mut screen =
        appeared(AppScreen::new(bar_data()).with_pilots(Some(keeper(&store)), Rc::clone(&given)));
    create(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Flight);

    let metrics = screen.metrics().expect("the menu's");
    assert!(std::ptr::addr_eq(Rc::as_ptr(&metrics), Rc::as_ptr(&given)));
    let x = no_target_x(&screen);
    assert!(same(x, flown_no_target_x(with_dialogs(bar_data()))));
    assert!(!same(x, flown_no_target_x(AppScreen::new(bar_data()))));
}

#[test]
fn with_neither_dialogs_nor_pilots_flights_text_is_laid_out_without_metrics() {
    let screen = AppScreen::new(bar_data());
    assert!(screen.metrics().is_none());
    assert!(!same(
        flown_no_target_x(screen),
        flown_no_target_x(with_dialogs(bar_data()))
    ));
}
