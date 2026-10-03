//! App frames of the ship browser over the stock data, through the
//! recording Gpu. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, AppScreen, Control, WindowEvent, WindowPort, start_screen};
use nova_data::{GameData, ShipId};
use nova_render::Batch;
use nova_render::recording::RecordingGpu;
use nova_view::Key;
use nova_view::ships::ShipCatalog;

struct Window;

impl WindowPort for Window {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}
}

/// The ship browser's selected ship: its ID and how many sprites it draws
/// (the base, plus each layer the `shän` defines).
fn selected(app: &App<Rc<GameData>>) -> (ShipId, usize) {
    let AppScreen::ShipBrowser(browser) = app.screen();
    let ship = browser.current().expect("a ship");
    let layers = [&ship.glow, &ship.lights]
        .into_iter()
        .filter(|layer| matches!(layer, Some(Ok(_))))
        .count();
    (ship.id, 1 + layers)
}

/// The app opens on the ship browser and draws, through the renderer, the
/// first ship, the first with both glow and lights, and the last (reached
/// by going left from the first). Only a few ships go through the renderer,
/// to bound the decoding this debug-build test does; `nova-view`'s stock
/// test browses them all.
#[test]
fn the_ship_browser_draws_stock_ships() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let ship_count = data.ship_ids().len();
    let mut app: App<_> = App::new(&Window, Rc::clone(&data), start_screen(data));
    let mut gpu = RecordingGpu::new();
    let mut clock = Duration::ZERO;
    let mut redraw = |app: &mut App<Rc<GameData>>| {
        clock += Duration::from_millis(50);
        app.handle(
            WindowEvent::Redraw { elapsed: clock },
            &mut Window,
            &mut gpu,
        );
        assert_eq!(app.take_failures(), []);
        let frame = *gpu.submits().last().expect("a frame");
        let quads: usize = frame
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { quads, .. } => quads.len(),
                _ => 0,
            })
            .sum();
        quads
    };
    let press = |app: &mut App<Rc<GameData>>, key| {
        let event = WindowEvent::Key { key, pressed: true };
        assert_eq!(
            app.handle(event, &mut Window, &mut RecordingGpu::new()),
            Control::Continue
        );
    };

    assert_eq!(selected(&app).0, ShipId(128));
    assert_eq!(redraw(&mut app), selected(&app).1);

    // Each search presses Right at most once per ship, so a browser that
    // never reaches its target fails the test instead of hanging it.
    for _ in 0..ship_count {
        if selected(&app).1 == 3 {
            break;
        }
        press(&mut app, Key::Right);
    }
    assert_eq!(
        selected(&app).1,
        3,
        "no stock ship within {ship_count} presses of Right has both glow and lights"
    );
    assert_eq!(redraw(&mut app), 3);
    assert_eq!(redraw(&mut app), 3);

    for _ in 0..ship_count {
        if selected(&app).0 == ShipId(128) {
            break;
        }
        press(&mut app, Key::Right);
    }
    assert_eq!(
        selected(&app).0,
        ShipId(128),
        "Right did not wrap back to the first ship within {ship_count} presses"
    );
    press(&mut app, Key::Left);
    assert_eq!(selected(&app).0, ShipId(895));
    assert_eq!(redraw(&mut app), selected(&app).1);
}
