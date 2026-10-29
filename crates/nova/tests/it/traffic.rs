//! The app flying among NPC traffic over synthetic game data, wired to the
//! renderer and the recording Gpu and driven only by key and mouse events
//! and redraws, as the window sends them: a system whose one `düde` flies
//! a trader, one ship on average. Entering flight populates it on the
//! app's source of chance, and the trader's own sprite reaches the Gpu.
//! Every flight's NPCs decide as the router's behaviour says, whether
//! flight was entered with the developer's F or a new pilot flies it, and
//! its ships are disabled as the router's rule says. A flight's
//! diagnostics reach the app's writer as lines.
//!
//! Where the trader is placed, how it flies and lands, the radar's blips
//! and which diagnostics a flight reports are nova-sim's, nova-sim-ai's,
//! nova-view's and nova-sim-combat's, pinned there.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::Duration;

use crate::common::{EntryHarness, Frames};
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::graphics::fixture::{Cicn, Ctab, DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{CICN, PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dude::Dude;
use nova_data::records::fixture::{ShipAnimBuilder, ShipBuilder};
use nova_data::records::interface::Interface;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::records::weapon::Weapon;
use nova_data::store::fixture::one_file;
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_render::{Batch, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{
    Behaviour, Chance, DisableRule, Gauge, Goal, HullSpec, Npc, PilotKeeper, PilotStore, Session,
    ShipId, Surroundings,
};
use nova_view::flight::FlightView;
use nova_view::menu::MenuChoice;
use nova_view::text::fixture::MonoMetrics;
use nova_view::{Key, MouseButton, Point};

/// A 1024x768 window at scale 1: window pixels are logical units.
struct FakeWindow;

impl WindowPort for FakeWindow {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

fn put_u32s(bytes: &mut [u8], at: usize, values: &[u32]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 4 * i..at + 4 * i + 4].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `chär` starting in ship 128 in system 128, with no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    bytes
}

/// A `shïp` with this `Shield`, `Accel`, `Speed`, `Maneuver` and `Fuel`,
/// 45 `Armor`, and `InherentAI` 1, a wimpy trader.
fn ship(accel: i16, speed: i16) -> Vec<u8> {
    ShipBuilder::new()
        .shield(30)
        .accel(accel)
        .speed(speed)
        .maneuver(30)
        .fuel(300)
        .armor(45)
        .inherent_ai(1)
        .build()
}

/// A trader's `shïp`: as [`ship`], carrying one `wëap` 181 in its first
/// weapon slot (the rest 0, none).
fn trader() -> Vec<u8> {
    ShipBuilder::from_bytes(ship(600, 600))
        .weap_types(&[181])
        .weap_counts(&[1])
        .build()
}

/// An exclusive `wëap`: unguided, 10 pixels a tick for 12 ticks, every
/// 20, of `Flags3` 0x0020, which the simulation does not do yet for the
/// player.
fn exclusive_gun() -> Vec<u8> {
    let mut bytes = vec![0; Weapon::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[20, 12, 2, 2, -1, 1000, -1]);
    put_i16s(&mut bytes, 0x16, &[-1]);
    bytes[0x66..0x68].copy_from_slice(&0x0020_u16.to_be_bytes());
    bytes
}

/// A `shän` whose base image is `rlëD` `image`, one set of 36 rotations.
fn ship_anim(image: i16) -> Vec<u8> {
    ShipAnimBuilder::new()
        .base_image_id(image)
        .base_set_count(1)
        .frames_per(36)
        .build()
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, frame| {
            let color = 0x0400 + frame;
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![color; size.into()])))
        })
        .build()
}

/// The dim radar colour of the status bar fixture.
const DIM_RADAR: u32 = 0x0000_8000;

/// Stock `ïntf` 128's areas and font, over background `PICT` 700.
fn interface() -> Vec<u8> {
    let mut bytes = vec![0; Interface::SIZE.expect("fixed")];
    put_u32s(&mut bytes, 0x00, &[0x00FF_FFFF, 0x0080_8080]);
    put_i16s(&mut bytes, 0x08, &[8, 8, 184, 184]);
    put_u32s(&mut bytes, 0x10, &[0x0000_FF00, DIM_RADAR]);
    put_i16s(&mut bytes, 0x18, &[199, 35, 206, 184]);
    put_u32s(&mut bytes, 0x20, &[0x0000_00FF]);
    put_i16s(&mut bytes, 0x24, &[216, 35, 223, 184]);
    put_u32s(&mut bytes, 0x2C, &[0x00FF_0000]);
    put_i16s(&mut bytes, 0x30, &[234, 35, 241, 184]);
    put_u32s(&mut bytes, 0x38, &[0x00FF_FF00, 0x0080_8000]);
    put_i16s(&mut bytes, 0x40, &[254, 8, 286, 184]);
    bytes[0x60..0x66].copy_from_slice(b"Geneva");
    put_i16s(&mut bytes, 0xA0, &[12]);
    put_i16s(&mut bytes, 0xA4, &[700]);
    bytes
}

/// A grey 194 x 16 `PICT`: the status bar's background, cut short.
fn status_picture() -> Vec<u8> {
    let bounds = [0, 0, 16, 194];
    PictBuilder::new(bounds)
        .direct_bits(&DirectBits::rgb555(bounds, &[0x4210; 194 * 16]))
        .end()
        .build()
}

/// A black 8 x 8 `cicn`: the Red Alert icon's first frame.
fn alert_icon() -> Vec<u8> {
    let ctab = Ctab {
        device: false,
        entries: vec![(0, [0, 0, 0])],
    };
    Cicn::new(8, 8, 8, &ctab, &[0; 64], &[true; 64]).bytes()
}

/// An independent `sÿst` holding stellar 128, with `düde` 128 at 100 % and
/// one ship on average.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x44, &[128, -1, -1, -1, -1, -1, -1, -1]);
    put_i16s(&mut bytes, 0x54, &[100]);
    put_i16s(&mut bytes, 0x64, &[1, -1]);
    bytes
}

/// A landable `spöb` at (0, -300), graphic type 0.
fn planet() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[0, -300, 0]);
    put_u32s(&mut bytes, 0x06, &[0x01]);
    bytes
}

/// A `spïn` naming `rlëD` 1000, one frame across.
fn spin() -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1000, -1, 0, 0, 1, 1]);
    bytes
}

/// A `düde` of wimpy traders flying ship 129, independent.
fn dude() -> Vec<u8> {
    let mut bytes = vec![0; Dude::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1, -1]);
    put_i16s(&mut bytes, 0x08, &[-1; 16]);
    put_i16s(&mut bytes, 0x08, &[129]);
    put_i16s(&mut bytes, 0x28, &[100]);
    bytes
}

/// The first `chär` flies ship 128 (a 1 x 1 sheet) from Alpha (128),
/// which holds the planet Alpha Prime (128) at (0, -300), 8 x 8, and
/// whose one `düde` flies ship 129, a trader with a 2 x 2 sheet and a
/// mining blaster.
fn data() -> Rc<GameData> {
    let fork = ForkBuilder::new()
        .resource(Character::TYPE, 128, Some(b"Pilot"), &character())
        .resource(Ship::TYPE, 128, Some(b"Shuttle"), &ship(300, 300))
        .resource(Ship::TYPE, 129, Some(b"Trader"), &trader())
        .resource(Weapon::TYPE, 181, Some(b"Exclusive Gun"), &exclusive_gun())
        .resource(ShipAnim::TYPE, 128, None, &ship_anim(2000))
        .resource(ShipAnim::TYPE, 129, None, &ship_anim(2001))
        .resource(RLED, 2000, None, &sheet(36, 1))
        .resource(RLED, 2001, None, &sheet(36, 2))
        .resource(System::TYPE, 128, Some(b"Alpha"), &system())
        .resource(Stellar::TYPE, 128, Some(b"Alpha Prime"), &planet())
        .resource(Spin::TYPE, 1000, None, &spin())
        .resource(RLED, 1000, None, &sheet(1, 8))
        .resource(Dude::TYPE, 128, Some(b"Traders"), &dude())
        .resource(
            Interface::TYPE,
            128,
            Some(b"Default status bar"),
            &interface(),
        )
        .resource(PICT, 700, Some(b"Status Bar"), &status_picture())
        .resource(CICN, 18000, Some(b"Red Alert"), &alert_icon());
    Rc::new(one_file(fork.build().bytes).load())
}

/// Every NPC idles, counting the times it is asked.
#[derive(Debug, Default)]
struct Still {
    asked: std::cell::Cell<usize>,
}

impl Behaviour for Still {
    fn decide(&self, _npc: &Npc, _around: &Surroundings, _chance: &mut dyn Chance) -> Goal {
        self.asked.set(self.asked.get() + 1);
        Goal::Idle
    }
}

/// Says no ship is disabled, counting the times it is asked.
#[derive(Debug, Default)]
struct Counting {
    asked: std::cell::Cell<usize>,
}

impl DisableRule for Counting {
    fn disabled(&self, _armor: Gauge, _hull: &HullSpec) -> bool {
        self.asked.set(self.asked.get() + 1);
        false
    }
}

/// Bytes written, kept where the test can read them.
#[derive(Clone, Default)]
struct Written(Rc<RefCell<Vec<u8>>>);

impl io::Write for Written {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    /// Frames sent, at 60 a second.
    frames: u64,
}

impl Harness {
    /// The app on the ship browser, its NPCs deciding as `behaviour` says
    /// when one is given.
    fn new(behaviour: Option<Rc<dyn Behaviour>>) -> Self {
        Self::built(behaviour, None, None)
    }

    /// The app on the ship browser, its NPCs deciding as `behaviour` says
    /// and its ships disabled as `rule` says, each when one is given, and
    /// writing its diagnostics to `out` when it is given.
    fn built(
        behaviour: Option<Rc<dyn Behaviour>>,
        rule: Option<Rc<dyn DisableRule>>,
        out: Option<Written>,
    ) -> Self {
        let data = data();
        let screen = start_screen(Rc::clone(&data));
        let screen = match behaviour {
            Some(behaviour) => screen.with_behaviour(behaviour),
            None => screen,
        };
        let screen = match rule {
            Some(rule) => screen.with_disable_rule(rule),
            None => screen,
        };
        let app = App::new(&FakeWindow, data, screen);
        let app = match out {
            Some(out) => app.with_diagnostics(Box::new(out)),
            None => app,
        };
        Self {
            app,
            gpu: RecordingGpu::new(),
            frames: 0,
        }
    }

    /// The app in flight, entered from the ship browser with F.
    fn flying(behaviour: Option<Rc<dyn Behaviour>>) -> Self {
        let mut harness = Self::new(behaviour);
        harness.press(Key::Char('f'));
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    /// A new pilot flying, from the main menu, its NPCs deciding as
    /// `behaviour` says.
    fn new_pilot(behaviour: Rc<dyn Behaviour>) -> Self {
        let data = data();
        let keeper = PilotKeeper::new(Box::new(MemoryPilots::new()) as Box<dyn PilotStore>);
        let screen = start_screen(Rc::clone(&data))
            .with_behaviour(behaviour)
            .with_pilots(Some(keeper), Rc::new(MonoMetrics));
        let mut harness = Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)),
            gpu: RecordingGpu::new(),
            frames: 0,
        };
        let at = harness
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .press_point(MenuChoice::NewPilot);
        harness.click(at);
        for c in "Trader".chars() {
            harness.send(WindowEvent::Text(c));
        }
        harness.press(Key::Enter);
        // The ship's christening, confirmed with its default.
        harness.press(Key::Enter);
        harness.enter_ship();
        assert_eq!(harness.showing(), Showing::Flight);
        harness
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(
            self.app.handle(event, &mut FakeWindow, &mut self.gpu),
            Control::Continue,
            "{event:?}"
        );
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

    fn click(&mut self, at: Point) {
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

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn flight(&self) -> &FlightView<Rc<GameData>> {
        self.app.screen().flight_view().expect("flight entered")
    }

    fn session(&self) -> &Session {
        self.flight().session().expect("flying")
    }

    /// Sends the next redraw and returns its frame.
    fn frame(&mut self) -> Frame {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        let frame = (*self.gpu.submits().last().expect("a frame")).clone();
        assert_eq!(self.app.take_failures(), []);
        frame
    }

    /// Draws frames until the system's traffic is populated.
    fn populated(&mut self) {
        self.settle_until("the system's population", |harness| {
            !harness.session().npcs().is_empty()
        });
    }
}

#[test]
fn entering_flight_populates_the_system_and_the_traders_sprite_reaches_the_gpu() {
    let mut harness = Harness::flying(None);
    harness.populated();
    let ships: Vec<ShipId> = harness
        .session()
        .npcs()
        .iter()
        .map(|npc| npc.ship)
        .collect();
    assert!(ships.contains(&ShipId(129)), "the düde's trader: {ships:?}");
    let frame = harness.frame();
    // The trader's sheet is the data's only 2 x 2 one.
    let drawn = frame.batches.iter().any(|batch| match batch {
        Batch::Sprites { quads, .. } => quads
            .iter()
            .any(|quad| (quad.dest.w, quad.dest.h) == (2.0, 2.0)),
        _ => false,
    });
    assert!(drawn, "the trader's sprite");
}

/// Asserts the router's `still` reached `harness`'s flight: it was asked,
/// and the trader idles as it said.
fn assert_still_decides(harness: &mut Harness, still: &Still) {
    harness.populated();
    harness.settle_until("the router's behaviour asked", |_| still.asked.get() > 0);
    harness.settle();
    let trader = harness.session().npcs()[0].clone();
    assert_eq!(trader.goal, Goal::Idle);
}

#[test]
fn the_routers_behaviour_decides_for_a_flight_entered_with_f() {
    let still = Rc::new(Still::default());
    let mut harness = Harness::flying(Some(still.clone()));
    assert_still_decides(&mut harness, &still);
}

#[test]
fn the_routers_behaviour_decides_for_a_new_pilots_flight() {
    let still = Rc::new(Still::default());
    let mut harness = Harness::new_pilot(still.clone());
    assert_still_decides(&mut harness, &still);
}

#[test]
fn the_routers_disable_rule_decides_for_a_flight_entered_with_f() {
    let rule = Rc::new(Counting::default());
    let mut harness = Harness::built(None, Some(rule.clone()), None);
    harness.press(Key::Char('f'));
    assert_eq!(harness.showing(), Showing::Flight);
    assert_eq!(rule.asked.get(), 0);
    harness.settle_until("the router's disable rule asked", |_| rule.asked.get() > 0);
}

#[test]
fn a_flights_diagnostics_reach_the_apps_writer_as_lines() {
    let written = Written::default();
    let mut harness = Harness::built(None, None, Some(written.clone()));
    harness.press(Key::Char('f'));
    harness.settle_until("a diagnostic written", |_| !written.0.borrow().is_empty());
    let text = String::from_utf8(written.0.borrow().clone()).expect("UTF-8");
    assert!(text.ends_with('\n'), "{text:?}");
    for line in text.lines() {
        assert!(line.starts_with("nova: "), "{line:?}");
    }
}

impl crate::common::EntryHarness for Harness {
    fn send_event(&mut self, event: WindowEvent) {
        self.send(event);
    }

    fn click_point(&mut self, at: Point) {
        self.click(at);
    }
}

/// Its entry frames leave the stand-in main menu's missing button
/// pictures, which this data has none of, unasserted: the menu shows
/// through the entry's fade out.
impl crate::common::Frames for Harness {
    fn router(&self) -> &nova::app::screen::AppScreen {
        self.app.screen()
    }

    fn next_frame(&mut self) {
        self.frames += 1;
        let elapsed = Duration::from_nanos(self.frames * 1_000_000_000 / 60);
        self.send(WindowEvent::Redraw { elapsed });
        let _ = self.app.take_failures();
    }
}
