//! How long the first flight steps and take-offs of a run take over the
//! stock data: the entry's slowest frame (the handover's warm-up, which
//! reads the first system's traffic ships' sheets and target cards under
//! black), flight's first stepped frame, and each of three take-offs'
//! Leave click plus its next six frames.
//!
//! The tests here are `#[ignore]`d, so CI never runs them; they only read
//! the clock and print, asserting the flow but no timing. Run them by
//! hand, in debug and in release:
//!
//! ```text
//! NOVA_DATA=".../Nova Files" cargo nextest run -p nova --test it --run-ignored only --no-capture stock_take_off_cost::
//! NOVA_DATA=".../Nova Files" cargo nextest run --release -p nova --test it --run-ignored only --no-capture stock_take_off_cost::
//! ```
//!
//! They skip, passing, without `NOVA_DATA`; the `OffscreenGpu` one also
//! skips without a GPU adapter.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use nova::app::{App, Control, WindowEvent, WindowPort, start_screen};
use nova_data::GameData;
use nova_data::graphics::Image;
use nova_render::recording::RecordingGpu;
use nova_render::upscale::{Upscaler, WorldResolution};
use nova_render::wgpu::{GlyphonMetrics, InitError, OffscreenGpu};
use nova_render::{FontFaces, Frame, Gpu, ImageError, ImageSource, PageId, PixelRect};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Pilot, PilotCatalog, PilotKeeper, PilotStore, is_landable};
use nova_view::menu::MenuChoice;
use nova_view::{ImageKind, Key, MouseButton, Showing};

use crate::common::{self, EntryHarness};

/// The window: the classic frame at scale 1.
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

/// What the renderer asked of its ports over one span.
#[derive(Default)]
struct Stats {
    decodes: usize,
    decode_time: Duration,
    pages: usize,
    uploads: usize,
    gpu_time: Duration,
    submit_time: Duration,
}

/// The stock data's images, timed.
struct TimedImages {
    data: Rc<GameData>,
    stats: Rc<RefCell<Stats>>,
}

impl ImageSource for TimedImages {
    fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
        let start = Instant::now();
        let frames = self.data.frames(kind, id);
        let mut stats = self.stats.borrow_mut();
        stats.decodes += 1;
        stats.decode_time += start.elapsed();
        frames
    }
}

/// A GPU, timed.
struct TimedGpu<G> {
    inner: G,
    stats: Rc<RefCell<Stats>>,
}

impl<G: Gpu> Gpu for TimedGpu<G> {
    fn create_page(&mut self, page: PageId, size: u32) {
        let start = Instant::now();
        self.inner.create_page(page, size);
        let mut stats = self.stats.borrow_mut();
        stats.pages += 1;
        stats.gpu_time += start.elapsed();
    }

    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
        let start = Instant::now();
        self.inner.upload(page, at, image);
        let mut stats = self.stats.borrow_mut();
        stats.uploads += 1;
        stats.gpu_time += start.elapsed();
    }

    fn clear_pages(&mut self) {
        self.inner.clear_pages();
    }

    fn submit(&mut self, frame: &Frame) {
        let start = Instant::now();
        self.inner.submit(frame);
        self.stats.borrow_mut().submit_time += start.elapsed();
    }

    fn world_resolution(&self) -> WorldResolution {
        self.inner.world_resolution()
    }

    fn set_upscaler(&mut self, upscaler: Upscaler) {
        self.inner.set_upscaler(upscaler);
    }

    fn take_warnings(&mut self) -> Vec<String> {
        self.inner.take_warnings()
    }
}

/// The app over the stock data, driven a frame at a time.
struct Driver<G> {
    app: App<TimedImages>,
    gpu: TimedGpu<G>,
    clock: Duration,
    stats: Rc<RefCell<Stats>>,
    /// The slowest frame since it was last cleared.
    slowest: Duration,
}

impl<G: Gpu> Driver<G> {
    /// Sends `event`, and how long the app took over it.
    fn send(&mut self, event: WindowEvent) -> Duration {
        let start = Instant::now();
        let control = self.app.handle(event, &mut Window, &mut self.gpu);
        let took = start.elapsed();
        assert_eq!(control, Control::Continue, "{event:?}");
        took
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

    /// A 60 Hz frame, and how long it took.
    fn redraw(&mut self) -> Duration {
        self.clock += Duration::from_millis(16);
        let took = self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
        self.slowest = self.slowest.max(took);
        took
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.redraw();
        }
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn reset(&self) {
        *self.stats.borrow_mut() = Stats::default();
    }

    /// Prints `label`'s time and what the ports were asked since the
    /// last reset.
    fn report(&self, label: &str, took: Duration) {
        let stats = self.stats.borrow();
        println!(
            "{label:<32} {:>8.1} ms | decodes {:>3} ({:>6.1} ms) | pages {} uploads {:>4} \
             ({:>5.1} ms) | submit {:>6.1} ms",
            ms(took),
            stats.decodes,
            ms(stats.decode_time),
            stats.pages,
            stats.uploads,
            ms(stats.gpu_time),
            ms(stats.submit_time),
        );
    }

    fn land(&mut self) {
        self.press(Key::Char('l'));
        self.frames(4);
        self.press(Key::Char('l'));
        assert_eq!(self.showing(), Showing::Spaceport, "landed");
    }

    /// Clicks Leave, and how long the click took.
    fn click_leave(&mut self) -> Duration {
        let leave = self
            .app
            .screen()
            .spaceport_view()
            .expect("in port")
            .dialog()
            .expect("laid out")
            .item_bounds(12)
            .expect("a Leave button")
            .center();
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(leave.x), f64::from(leave.y)),
        });
        self.reset();
        let mut took = Duration::ZERO;
        for pressed in [true, false] {
            took += self.send(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed,
            });
        }
        took
    }

    /// Accepts any offer and closes any mission text, clicks Leave, and
    /// times the click that took off and the next six frames; returns
    /// that total.
    fn take_off(&mut self, label: &str) -> Duration {
        self.frames(30);
        let mut total = Duration::ZERO;
        for _ in 0..5 {
            let port = self.app.screen().spaceport_view().expect("in port");
            if port.offer_dialog().is_some() {
                self.press(Key::Enter);
                self.frames(5);
                continue;
            }
            total = self.click_leave();
            if self.showing() == Showing::Flight {
                break;
            }
            // A dialog over the port (the accepted offer's text) took the
            // click.
            self.press(Key::Enter);
            self.frames(5);
        }
        assert_eq!(self.showing(), Showing::Flight, "{label}: took off");
        for _ in 0..6 {
            total += self.redraw();
        }
        self.report(&format!("{label}: Leave + 6 frames"), total);
        total
    }
}

impl<G: Gpu> crate::common::Frames for Driver<G> {
    fn router(&self) -> &nova::app::screen::AppScreen {
        self.app.screen()
    }

    fn next_frame(&mut self) {
        self.redraw();
    }
}

impl<G: Gpu> EntryHarness for Driver<G> {
    fn send_event(&mut self, event: WindowEvent) {
        self.send(event);
    }

    fn click_point(&mut self, at: nova_view::Point) {
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
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Opens a fresh stock pilot saved at its first landable stellar, enters
/// play, then lands and takes off three times, printing each span.
fn run(gpu: impl Gpu, name: &str) {
    let Some(dir) = common::nova_data() else {
        return;
    };
    println!("--- {name} ---");
    let interface = nova_data::open_interface(&dir).expect("the stock interface");
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let pilot = Pilot::new(data.as_ref(), "Stock").expect("a pilot starts");
    let site = data
        .landing_sites(pilot.system())
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["stellar"] = serde_json::json!(site.id.0);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let keeper = PilotKeeper::new(Box::new(MemoryPilots::new()) as Box<dyn PilotStore>);
    keeper.save(&saved).expect("saved");
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let screen = start_screen(Rc::clone(&data))
        .with_dialogs(Rc::new(interface), Rc::clone(&metrics) as _)
        .with_pilots(Some(keeper), metrics);
    let stats = Rc::new(RefCell::new(Stats::default()));
    let images = TimedImages {
        data: Rc::clone(&data),
        stats: Rc::clone(&stats),
    };
    let mut driver = Driver {
        app: App::new(&Window, images, common::appeared(screen)),
        gpu: TimedGpu {
            inner: gpu,
            stats: Rc::clone(&stats),
        },
        clock: Duration::ZERO,
        stats,
        slowest: Duration::ZERO,
    };
    driver.frames(5);
    let open = driver
        .app
        .screen()
        .main_menu()
        .expect("the main menu")
        .press_point(MenuChoice::OpenPilot);
    driver.click_point(open);
    driver.press(Key::Enter);
    driver.slowest = Duration::ZERO;
    driver.enter_ship();
    assert_eq!(driver.showing(), Showing::Flight, "entered play");
    let entry = driver.slowest;
    println!("{:<32} {:>8.1} ms", "entry: slowest frame", ms(entry));
    // Flight's first frames hold the pilot still; the first that steps
    // populates the system's traffic.
    let mut first_step = Duration::ZERO;
    for i in 0..3 {
        driver.reset();
        let took = driver.redraw();
        driver.report(&format!("entry flight frame {i}"), took);
        first_step = first_step.max(took);
    }
    driver.frames(60);
    driver.land();
    let first = driver.take_off("1st take-off");
    driver.frames(60);
    driver.land();
    let second = driver.take_off("2nd take-off");
    driver.frames(60);
    driver.land();
    driver.take_off("3rd take-off");
    println!(
        "SUMMARY {name}: entry's slowest frame {:.1} ms; first stepped frame {:.1} ms; 1st \
         take-off {:.1} ms vs 2nd {:.1} ms (difference {:.1} ms)",
        ms(entry),
        ms(first_step),
        ms(first),
        ms(second),
        ms(first) - ms(second),
    );
}

#[test]
#[ignore = "reads the clock; run by hand (see the module doc)"]
fn take_offs_cost_with_a_recording_gpu() {
    run(RecordingGpu::new(), "recording");
}

#[test]
#[ignore = "reads the clock; run by hand (see the module doc)"]
fn take_offs_cost_with_an_offscreen_gpu() {
    if common::nova_data().is_none() {
        return;
    }
    let gpu = match OffscreenGpu::new(1024, 768, &FontFaces::bundled()) {
        Ok(gpu) => gpu,
        Err(error @ (InitError::NoAdapter(_) | InitError::NoDevice(_))) => {
            eprintln!("skipping: no GPU adapter ({error})");
            return;
        }
        Err(error) => panic!("offscreen GPU: {error}"),
    };
    run(gpu, "wgpu");
}
