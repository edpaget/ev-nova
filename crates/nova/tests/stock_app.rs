//! App frames of the ship browser, the galaxy map and a system over the
//! stock data, through the recording Gpu, and a course plotted from
//! flight. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use std::rc::Rc;
use std::time::Duration;

use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::{GameData, ShipId, SystemId};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces};
use nova_view::ships::ShipCatalog;
use nova_view::{Key, MouseButton};

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
    assert_eq!(app.screen().showing(), Showing::ShipBrowser);
    let ship = app.screen().ship_browser().current().expect("a ship");
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
        let event = WindowEvent::Key {
            key,
            pressed: true,
            repeat: false,
        };
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

/// Tab opens the galaxy map, whose first frame draws every nebula picture
/// (each decoded and packed without failing) before every hyperlink and
/// system, and a click on Sol selects it.
#[test]
fn the_galaxy_map_draws_the_stock_galaxy() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let mut app: App<_> = App::new(&Window, Rc::clone(&data), start_screen(data));
    let mut gpu = RecordingGpu::new();
    let send = |app: &mut App<Rc<GameData>>, gpu: &mut RecordingGpu, event| {
        assert_eq!(app.handle(event, &mut Window, gpu), Control::Continue);
    };
    send(
        &mut app,
        &mut gpu,
        WindowEvent::Key {
            key: Key::Tab,
            pressed: true,
            repeat: false,
        },
    );
    assert_eq!(app.screen().showing(), Showing::GalaxyMap);
    let redraw = WindowEvent::Redraw {
        elapsed: Duration::from_millis(50),
    };
    send(&mut app, &mut gpu, redraw);
    assert_eq!(app.take_failures(), []);
    let frame = (*gpu.submits().last().expect("a frame")).clone();
    let Some(Batch::Sprites { quads: nebulae, .. }) = frame.batches.first() else {
        panic!("the nebulae first: {:?}", frame.batches.first());
    };
    assert_eq!(nebulae.len(), 4);
    let Some(Batch::Solid(shapes)) = frame.batches.get(1) else {
        panic!("then the map's shapes: {:?}", frame.batches.get(1));
    };
    assert!(shapes.len() >= 930 + 545, "{} solid quads", shapes.len());

    let map = app.screen().galaxy_map();
    let sol = map.model().system(SystemId(130)).expect("Sol");
    let at = map.view().world_to_screen(sol.position());
    send(
        &mut app,
        &mut gpu,
        WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        },
    );
    for pressed in [true, false] {
        let event = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed,
        };
        send(&mut app, &mut gpu, event);
    }
    send(
        &mut app,
        &mut gpu,
        WindowEvent::Redraw {
            elapsed: Duration::from_millis(100),
        },
    );
    let frame = (*gpu.submits().last().expect("a frame")).clone();
    let shown = frame.batches.iter().any(|batch| match batch {
        Batch::Text(runs) => runs.iter().any(|run| run.text == "Sol (sÿst 130)"),
        _ => false,
    });
    assert!(shown, "Sol is selected");
    assert_eq!(app.take_failures(), []);
}

/// Clicking Sol on the map and pressing Return opens it, whose first frame
/// draws its five stellars, every sheet (the 32-frame Wormhole's included)
/// decoded and packed without failing, with Earth at the screen's centre.
/// Escape goes back to the map, and again quits.
#[test]
fn sol_opens_from_the_map_and_its_stellars_reach_the_gpu() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let mut app: App<_> = App::new(&Window, Rc::clone(&data), start_screen(data));
    let mut gpu = RecordingGpu::new();
    let send = |app: &mut App<Rc<GameData>>, gpu: &mut RecordingGpu, event| {
        app.handle(event, &mut Window, gpu)
    };
    let key = |key, pressed| WindowEvent::Key {
        key,
        pressed,
        repeat: false,
    };
    assert_eq!(
        send(&mut app, &mut gpu, key(Key::Tab, true)),
        Control::Continue
    );

    let map = app.screen().galaxy_map();
    let sol = map.model().system(SystemId(130)).expect("Sol");
    let at = map.view().world_to_screen(sol.position());
    let moved = WindowEvent::PointerMoved {
        px: (f64::from(at.x), f64::from(at.y)),
    };
    assert_eq!(send(&mut app, &mut gpu, moved), Control::Continue);
    for pressed in [true, false] {
        let event = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed,
        };
        assert_eq!(send(&mut app, &mut gpu, event), Control::Continue);
    }
    assert_eq!(app.screen().galaxy_map().selected(), Some(SystemId(130)));
    assert_eq!(
        send(&mut app, &mut gpu, key(Key::Enter, true)),
        Control::Continue
    );
    assert_eq!(app.screen().showing(), Showing::System);

    let redraw = WindowEvent::Redraw {
        elapsed: Duration::from_millis(50),
    };
    assert_eq!(send(&mut app, &mut gpu, redraw), Control::Continue);
    assert_eq!(app.take_failures(), []);
    let frame = (*gpu.submits().last().expect("a frame")).clone();
    let quads: Vec<_> = frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(quads.len(), 5);
    let centres: Vec<(f32, f32)> = quads
        .iter()
        .map(|q| (q.dest.x + q.dest.w / 2.0, q.dest.y + q.dest.h / 2.0))
        .collect();
    assert!(centres.contains(&(512.0, 384.0)), "Earth: {centres:?}");

    assert_eq!(
        send(&mut app, &mut gpu, key(Key::Escape, true)),
        Control::Continue
    );
    assert_eq!(app.screen().showing(), Showing::GalaxyMap);
    assert_eq!(app.screen().galaxy_map().selected(), Some(SystemId(130)));
    assert_eq!(
        send(&mut app, &mut gpu, key(Key::Escape, false)),
        Control::Continue
    );
    assert_eq!(
        send(&mut app, &mut gpu, key(Key::Escape, true)),
        Control::Exit
    );
}

/// A new pilot starts in Kania (128) on 23 June 1177, and a click on
/// Tichel (129) on flight's map plots a course one jump long.
#[test]
fn a_new_pilot_plots_a_one_jump_course_from_kania_to_tichel() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let mut app: App<_> = App::new(&Window, Rc::clone(&data), start_screen(data));
    let mut gpu = RecordingGpu::new();
    let mut send = |app: &mut App<Rc<GameData>>, event| {
        assert_eq!(app.handle(event, &mut Window, &mut gpu), Control::Continue);
    };
    for key in [Key::Char('f'), Key::Char('m')] {
        for pressed in [true, false] {
            let event = WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            };
            send(&mut app, event);
        }
    }
    assert_eq!(app.screen().showing(), Showing::FlightMap);
    let flight = app.screen().flight_view().expect("flying");
    let session = flight.session().expect("the stock first chär starts");
    assert_eq!(session.system(), SystemId(128));
    let today = session.date();
    assert_eq!((today.day(), today.month(), today.year()), (23, 6, 1177));
    let map = flight.course_map();
    let tichel = map.model().system(SystemId(129)).expect("Tichel");
    assert_eq!(tichel.entry.name, "Tichel");
    let at = map.view().world_to_screen(tichel.position());
    send(
        &mut app,
        WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        },
    );
    for pressed in [true, false] {
        let event = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed,
        };
        send(&mut app, event);
    }
    let flight = app.screen().flight_view().expect("flying");
    let course = flight.session().expect("flying").course();
    assert_eq!(course, [SystemId(129)]);
}

/// I opens the stock "Desc Dialog" over the ship browser, from the
/// interface file beside the stock data, with the About text laid out by
/// the real glyphon metrics; its frame, button and text reach the Gpu
/// with every picture resolved, Down scrolls it and Return closes it.
#[test]
fn i_shows_the_stock_about_text() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let interface = match nova_data::open_interface(&dir) {
        Ok(interface) => interface,
        Err(error) => {
            eprintln!("skipping: {error}");
            return;
        }
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let screen = start_screen(Rc::clone(&data)).with_dialogs(
        Rc::new(interface),
        Rc::new(GlyphonMetrics::new(&FontFaces::bundled())),
    );
    let mut app: App<_> = App::new(&Window, data, screen);
    let mut gpu = RecordingGpu::new();
    let mut send = |app: &mut App<Rc<GameData>>, event| {
        assert_eq!(app.handle(event, &mut Window, &mut gpu), Control::Continue);
    };
    let key = |key| WindowEvent::Key {
        key,
        pressed: true,
        repeat: false,
    };

    send(&mut app, key(Key::Char('i')));
    assert_eq!(app.screen().showing(), Showing::About);
    send(
        &mut app,
        WindowEvent::Redraw {
            elapsed: Duration::from_millis(16),
        },
    );
    assert_eq!(app.take_failures(), []);
    let about = app.screen().about().expect("open");
    assert!(about.lines().len() > 20, "{} lines", about.lines().len());
    send(&mut app, key(Key::Down));
    assert_eq!(app.screen().about().expect("open").first_line(), 1);
    send(&mut app, key(Key::Enter));
    assert_eq!(app.screen().showing(), Showing::ShipBrowser);

    let frame = gpu.submits()[0].clone();
    let pictures: usize = frame
        .batches
        .iter()
        .map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.len(),
            _ => 0,
        })
        .sum();
    assert!(pictures >= 6, "the frame's three and the button's three");
    let texts: Vec<String> = frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
            _ => Vec::new(),
        })
        .collect();
    assert!(texts.contains(&"Done".to_owned()), "{texts:?}");
}
