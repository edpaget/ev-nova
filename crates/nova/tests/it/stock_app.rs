//! App frames of the ship browser, the galaxy map and a system over the
//! stock data, through the recording Gpu, a course plotted from flight,
//! and HG-Kania entered for Tichel, a new pilot offered the tutorial
//! over the spaceport, and a storyline finale's texts shown over Earth's
//! spaceport before its `Q` leaves. Skips, passing, when `NOVA_DATA` is
//! unset.

use crate::common;

use std::ops::RangeInclusive;
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

/// The ship browser's selected ship: its ID and how many sprites it may
/// draw: the base, plus each layer the `shän` defines, its lights left out
/// while a blinking `BlinkMode` (1 to 3) has them dark.
fn selected(app: &App<Rc<GameData>>) -> (ShipId, RangeInclusive<usize>) {
    assert_eq!(app.screen().showing(), Showing::ShipBrowser);
    let ship = app.screen().ship_browser().current().expect("a ship");
    let has = |layer: &Option<Result<_, _>>| usize::from(matches!(layer, Some(Ok(_))));
    let most = 1 + has(&ship.glow) + has(&ship.lights);
    let blinks = (1..=3).contains(&ship.blink.mode);
    let least = most - usize::from(blinks) * has(&ship.lights);
    (ship.id, least..=most)
}

/// The app opens on the ship browser and draws, through the renderer, the
/// first ship, the first with both glow and steady lights, and the last (reached
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
    assert!(selected(&app).1.contains(&redraw(&mut app)));

    // Each search presses Right at most once per ship, so a browser that
    // never reaches its target fails the test instead of hanging it.
    for _ in 0..ship_count {
        if selected(&app).1 == (3..=3) {
            break;
        }
        press(&mut app, Key::Right);
    }
    assert_eq!(
        selected(&app).1,
        3..=3,
        "no stock ship within {ship_count} presses of Right has both glow and steady lights"
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
    assert!(selected(&app).1.contains(&redraw(&mut app)));
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
    let at = map.transform().world_to_screen(sol.position());
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
    let at = map.transform().world_to_screen(sol.position());
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

/// A new pilot starts in Kania (128) on 23 June 1177, and a Shift-click on
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
    let at = map.transform().world_to_screen(tichel.position());
    send(
        &mut app,
        WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        },
    );
    // A Shift-click adds the hop, as `_DoSystemMap` builds a course.
    let shift = |pressed| WindowEvent::Key {
        key: Key::Shift,
        pressed,
        repeat: false,
    };
    send(&mut app, shift(true));
    for pressed in [true, false] {
        let event = WindowEvent::PointerButton {
            button: MouseButton::Left,
            pressed,
        };
        send(&mut app, event);
    }
    send(&mut app, shift(false));
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
    assert_no_tag_or_brace(about.lines());
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

/// The app driven as the window drives it: each event through the
/// recording Gpu.
struct Driver {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Driver {
    fn send(&mut self, event: WindowEvent) {
        let control = self.app.handle(event, &mut Window, &mut self.gpu);
        assert_eq!(control, Control::Continue, "{event:?}");
    }

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.send(WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            });
        }
    }

    fn click(&mut self, at: nova_view::Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed,
            });
        }
    }

    fn redraw(&mut self) {
        self.clock += Duration::from_millis(50);
        self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn flight(&self) -> &nova_view::flight::FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flying")
    }
}

/// A pilot saved at HG-Kania (`spöb` 1404, at (-70, 250) in Kania) and
/// opened from the main menu resumes in space over the gate, no spaceport
/// opened. L, a tick, and L open the hypergate map, Tichel is clicked,
/// and M brings the ship out of HG-Tichel at (-400, -500) in Tichel (129),
/// flying on from there under the fade-in, on 23 June 1177, the day it
/// started, as no take-off passed one (`_PlayerLandOnStellar` @0x63440),
/// saying so. It holds rank 147, "Have Access to Hypergate System", which
/// blesses landing at the Hypergate government's (183) stellars: its
/// record could never clear HG-Kania's `MinStatus` of 32767.
#[test]
fn a_pilot_at_hg_kania_enters_it_for_tichel() {
    use nova_sim::fixture::MemoryPilots;
    use nova_sim::{Pilot, PilotKeeper, PilotStore, Vec2};
    use nova_view::galaxy::MapMode;
    use nova_view::menu::MenuChoice;
    use nova_view::text::fixture::MonoMetrics;

    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let pilot = Pilot::new(data.as_ref(), "Gatekeeper").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(1404);
    save["ranks"] = serde_json::json!([147]);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let store = MemoryPilots::new();
    let keeper = PilotKeeper::new(Box::new(store) as Box<dyn PilotStore>);
    keeper.save(&saved).expect("saved");
    let screen = start_screen(Rc::clone(&data)).with_pilots(Some(keeper), Rc::new(MonoMetrics));
    let mut driver = Driver {
        app: App::new(&Window, data, screen),
        gpu: RecordingGpu::new(),
        clock: Duration::ZERO,
    };
    let open = driver
        .app
        .screen()
        .main_menu()
        .expect("the main menu")
        .button(MenuChoice::OpenPilot)
        .rect
        .center();
    driver.click(open);
    assert_eq!(driver.showing(), Showing::OpenPilot);
    driver.press(Key::Enter);
    assert_eq!(driver.showing(), Showing::Flight, "resumed in space");
    let session = driver.flight().session().expect("flying");
    assert_eq!(session.landed(), None);
    assert_eq!(session.system(), SystemId(128));
    assert_eq!(session.player().position, Vec2::new(-70.0, 250.0));

    driver.press(Key::Char('l'));
    // Two redraws, at least a tick, for the dock approach to clear the
    // request.
    driver.redraw();
    driver.redraw();
    driver.press(Key::Char('l'));
    assert_eq!(driver.showing(), Showing::FlightMap);
    let map = driver.flight().course_map();
    assert_eq!(map.mode(), MapMode::Hypergate);
    let tichel = map.model().system(SystemId(129)).expect("Tichel");
    let at = map.transform().world_to_screen(tichel.position());
    driver.click(at);
    driver.press(Key::Char('m'));
    assert_eq!(driver.showing(), Showing::Flight);
    let session = driver.flight().session().expect("flying");
    assert_eq!(session.system(), SystemId(129));
    assert_eq!(session.player().position, Vec2::new(-400.0, -500.0));
    // The ship flies on under the arrival fade-in, so the exit point is
    // read before the redraw.
    driver.redraw();
    let flight = driver.flight();
    let session = flight.session().expect("flying");
    assert_eq!(session.system(), SystemId(129));
    let today = session.date();
    assert_eq!((today.day(), today.month(), today.year()), (23, 6, 1177));
    assert_eq!(
        flight.message(),
        Some("Exiting hypergate in the Tichel system on June 23, 1177 NC.")
    );
}

/// Asserts that no line of a shown `dësc` still holds a tag (`<REG>` in
/// stock 32767) or conditional text, as `_LoadDescription` and
/// `_MungeBriefing(0, -1)` leave none.
fn assert_no_tag_or_brace(lines: &[String]) {
    let left: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains('<') || line.contains('{'))
        .collect();
    assert_eq!(left, Vec::<&String>::new());
}

/// X on the main menu opens the stock `dësc` 32766 in "Desc Dialog" over
/// it, as `_MainScreenKey` @0x1ac7e does, and Return closes it.
#[test]
fn x_shows_the_stock_desc_32766_over_the_main_menu() {
    use nova_sim::fixture::MemoryPilots;
    use nova_sim::{PilotKeeper, PilotStore};

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
    let keeper = PilotKeeper::new(Box::new(MemoryPilots::new()) as Box<dyn PilotStore>);
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let screen = start_screen(Rc::clone(&data))
        .with_dialogs(Rc::new(interface), Rc::clone(&metrics) as _)
        .with_pilots(Some(keeper), metrics);
    let mut app: App<_> = App::new(&Window, data, screen);
    let mut gpu = RecordingGpu::new();
    let mut press = |app: &mut App<Rc<GameData>>, key| {
        let event = WindowEvent::Key {
            key,
            pressed: true,
            repeat: false,
        };
        assert_eq!(app.handle(event, &mut Window, &mut gpu), Control::Continue);
    };

    press(&mut app, Key::Char('x'));
    assert_eq!(app.screen().showing(), Showing::Note);
    let text = app.screen().about().expect("open");
    assert!(!text.lines().is_empty());
    assert_no_tag_or_brace(text.lines());
    press(&mut app, Key::Enter);
    assert_eq!(app.screen().showing(), Showing::MainMenu);
}

/// A new pilot saved at the first landable stellar of Kania and opened
/// from the main menu resumes in space over it; L, redraws and L land it
/// in its spaceport, which offers the stock tutorial ("Head to
/// Sol;Tutorial 001", `mïsn` 251, `AvailLoc` 3) over the spaceport as it
/// opens.
#[test]
fn a_new_stock_pilot_is_offered_the_tutorial_over_the_spaceport() {
    use nova_sim::fixture::MemoryPilots;
    use nova_sim::{MissionId, Pilot, PilotCatalog, PilotKeeper, PilotStore, is_landable};
    use nova_view::menu::MenuChoice;

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
    let pilot = Pilot::new(data.as_ref(), "Stock").expect("the stock first chär starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let store = MemoryPilots::new();
    let keeper = PilotKeeper::new(Box::new(store) as Box<dyn PilotStore>);
    keeper.save(&saved).expect("saved");
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let screen = start_screen(Rc::clone(&data))
        .with_dialogs(Rc::new(interface), Rc::clone(&metrics) as _)
        .with_pilots(Some(keeper), metrics);
    let mut driver = Driver {
        app: App::new(&Window, data, screen),
        gpu: RecordingGpu::new(),
        clock: Duration::ZERO,
    };
    let open = driver
        .app
        .screen()
        .main_menu()
        .expect("the main menu")
        .button(MenuChoice::OpenPilot)
        .rect
        .center();
    driver.click(open);
    assert_eq!(driver.showing(), Showing::OpenPilot);
    driver.press(Key::Enter);
    assert_eq!(driver.showing(), Showing::Flight, "resumed in space");
    driver.press(Key::Char('l'));
    // Two redraws, at least a tick, for the dock approach to clear the
    // request.
    driver.redraw();
    driver.redraw();
    driver.press(Key::Char('l'));
    assert_eq!(driver.showing(), Showing::Spaceport, "landed");
    driver.redraw();

    let offer = driver
        .app
        .screen()
        .spaceport_view()
        .expect("the spaceport")
        .offer_dialog()
        .expect("the spaceport offers");
    assert_eq!(
        offer.prompt().mission,
        MissionId(251),
        "Head to Sol;Tutorial 001"
    );
    assert!(
        offer
            .prompt()
            .text
            .starts_with("As you wander around a spaceport for the first time"),
        "{}",
        offer.prompt().text
    );
}

/// The app over the stock data in `dir`, with a pilot saved docked at
/// Earth holding `mïsn` 474 with its cargo aboard, opened from the main
/// menu, which resumes it in space over Earth, and landed there with L, a
/// tick and L; `None`, skipping, without the interface file.
fn resumed_at_earth_holding_krane(dir: &std::path::Path) -> Option<Driver> {
    use nova_sim::fixture::MemoryPilots;
    use nova_sim::{Pilot, PilotKeeper, PilotStore};
    use nova_view::menu::MenuChoice;

    let interface = match nova_data::open_interface(dir) {
        Ok(interface) => interface,
        Err(error) => {
            eprintln!("skipping: {error}");
            return None;
        }
    };
    let data = Rc::new(GameData::open(dir, None).expect("the stock data opens"));
    let pilot = Pilot::new(data.as_ref(), "Finale").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(130);
    save["stellar"] = serde_json::json!(128);
    let earth = serde_json::json!({"stellar": 128, "system": 130});
    save["missions"] = serde_json::json!([{
        "slot": 0,
        "mission": 474,
        "destination": null,
        "return": earth,
        "cargo_type": 23,
        "cargo_tons": 0,
        "pay": 250_000,
        "days_left": null,
        "deadline": null,
        "reached": false,
        "goals_met": false,
        "aboard": true,
        "failed": null,
        "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
        "aux": {"countdown": 0, "arrived": 0, "left": 0},
    }]);
    let docked = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let store = MemoryPilots::new();
    let keeper = PilotKeeper::new(Box::new(store) as Box<dyn PilotStore>);
    keeper.save(&docked).expect("saved");
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let screen = start_screen(Rc::clone(&data))
        .with_dialogs(Rc::new(interface), Rc::clone(&metrics) as _)
        .with_pilots(Some(keeper), metrics);
    let mut driver = Driver {
        app: App::new(&Window, data, screen),
        gpu: RecordingGpu::new(),
        clock: Duration::ZERO,
    };
    let open = driver
        .app
        .screen()
        .main_menu()
        .expect("the main menu")
        .button(MenuChoice::OpenPilot)
        .rect
        .center();
    driver.click(open);
    driver.press(Key::Enter);
    assert_eq!(driver.showing(), Showing::Flight, "resumed in space");
    driver.press(Key::Char('l'));
    // Two redraws, at least a tick, for the dock approach to clear the
    // request.
    driver.redraw();
    driver.redraw();
    driver.press(Key::Char('l'));
    assert_eq!(driver.showing(), Showing::Spaceport, "landed");
    Some(driver)
}

/// A pilot saved docked at Earth (`spöb` 128, in Sol, 130) holding
/// "Take Krane to Earth" (`mïsn` 474, a Federation finale) with Krane's
/// cargo aboard resumes over it and lands there. The landing drops the cargo
/// off, showing its `DumpCargoText` (`dësc` 8064), then completes the
/// mission, showing its `CompText` (9076), both over the spaceport while
/// the ship stays landed; its `OnSuccess`, `b89 b9995 b9500 M472 L129
/// K149 Q25048`, holds a leave. Then Earth's own offer shows (`mïsn` 878,
/// which cannot be refused, as `_OfferMissionFromPort` @0x5fc37 runs
/// whatever is pending), and accepting it takes off, in S7evyn (472)
/// where `M472` moved the pilot, the take-off line `STR#` 25048's one
/// string.
#[test]
fn a_finales_texts_show_over_earths_spaceport_before_its_q_leaves() {
    use nova_sim::MissionId;
    use nova_view::spaceport::SpaceportView;

    let Some(dir) = common::nova_data() else {
        return;
    };
    let Some(mut driver) = resumed_at_earth_holding_krane(&dir) else {
        return;
    };
    driver.redraw();
    let port = |driver: &Driver| -> (String, Option<MissionId>) {
        let port = driver.app.screen().spaceport_view().expect("the spaceport");
        let text = port
            .mission_text()
            .map(|text| text.lines().join(" "))
            .unwrap_or_default();
        (
            text,
            port.offer_dialog().map(|offer| offer.prompt().mission),
        )
    };

    let (text, _) = port(&driver);
    assert!(
        text.starts_with("As soon as you land Krane leads you"),
        "DumpCargoText 8064 first: {text}"
    );
    let session = driver.flight().session().expect("a session");
    assert!(session.landed().is_some(), "still landed");
    assert_eq!(session.pilot().missions().iter().count(), 0, "474 done");
    assert!(driver.flight().leave_due(), "the Q held");

    driver.press(Key::Enter);
    assert_eq!(
        driver.showing(),
        Showing::Spaceport,
        "the CompText holds it"
    );
    let (text, _) = port(&driver);
    assert!(
        text.starts_with("\"Are you threatening me?\""),
        "CompText 9076 next: {text}"
    );

    driver.press(Key::Enter);
    assert_eq!(driver.showing(), Showing::Spaceport, "the offer holds it");
    assert_eq!(port(&driver), (String::new(), Some(MissionId(878))));
    assert!(
        driver
            .app
            .screen()
            .spaceport_view()
            .is_some_and(SpaceportView::holds_leave)
    );

    driver.press(Key::Enter);
    assert_eq!(driver.showing(), Showing::Flight, "took off");
    let flight = driver.flight();
    let session = flight.session().expect("a session");
    assert_eq!(session.landed(), None);
    let held: Vec<MissionId> = session
        .pilot()
        .missions()
        .iter()
        .map(|(_, active)| active.mission)
        .collect();
    assert_eq!(held, [MissionId(878)], "the offer accepted first");
    assert_eq!(session.system(), SystemId(472), "where M472 moved it");
    assert_eq!(
        flight.message(),
        Some("A mysterious force transports you to an unknown location...")
    );
}
