//! App frames of the placeholder and the ship browser over the stock data,
//! through the recording Gpu. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{
    App, AppScreen, Control, Placeholder, PlaceholderContent, WindowEvent, WindowPort, start_screen,
};
use nova_data::{GameData, ShipId};
use nova_render::Batch;
use nova_render::recording::{GpuCall, RecordingGpu};
use nova_view::Key;

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

#[test]
fn the_placeholder_draws_the_stock_data() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let content = PlaceholderContent::from_data(&data);
    let (Some(_picture), Some((_sprite, frames))) = (content.picture, content.sprite) else {
        panic!("no picture or sprite in the stock data: {content:?}");
    };
    let screen = AppScreen::Placeholder(Placeholder::new(content));
    let mut app: App<_> = App::new(&Window, &data, screen);
    let mut gpu = RecordingGpu::new();

    app.handle(
        WindowEvent::Redraw {
            elapsed: Duration::ZERO,
        },
        &mut Window,
        &mut gpu,
    );

    assert_eq!(app.take_failures(), []);
    let uploads = gpu
        .calls
        .iter()
        .filter(|call| matches!(call, GpuCall::Upload { .. }))
        .count();
    assert_eq!(uploads, 1 + usize::from(frames));
    let frame = gpu.submits()[0];
    assert!(
        matches!(&frame.batches[0], Batch::Sprites { quads, .. } if quads.len() == 301),
        "{:?}",
        frame.batches.first()
    );
}

/// The ship browser's selected ship: its ID and how many sprites it draws
/// (the base, plus each layer the `shän` defines).
fn selected(app: &App<Rc<GameData>>) -> (ShipId, usize) {
    let AppScreen::ShipBrowser(browser) = app.screen() else {
        panic!("the ship browser");
    };
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

    while selected(&app).1 < 3 {
        press(&mut app, Key::Right);
    }
    assert_eq!(redraw(&mut app), 3);
    assert_eq!(redraw(&mut app), 3);

    while selected(&app).0 != ShipId(128) {
        press(&mut app, Key::Right);
    }
    press(&mut app, Key::Left);
    assert_eq!(selected(&app).0, ShipId(895));
    assert_eq!(redraw(&mut app), selected(&app).1);
}
