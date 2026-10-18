//! Pilots through the whole app: synthetic game data (a `chär` with cash
//! and a starting date, flying a ship from a system whose one planet it
//! starts over, and Nova's button pictures) and a synthetic interface file
//! holding the stock "Create a new pilot:" and "Spaceport" dialogs, laid
//! out by the real glyphon metrics, drawn through the renderer into the
//! recording Gpu and driven only by window events, keys and typed text
//! made by the platform's own translation. Pilots are kept in the
//! in-memory store, so the test sees what is saved, and "restarts" with a
//! new app over the same store.
//!
//! From the main menu, New Pilot asks for a name and flies the new pilot;
//! landing, taking off and closing the window each save it; Open Pilot in
//! a new app lists it and resumes it docked where it landed, on the same
//! date, in the same ship and with the same cash; Quit exits.

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use nova::app::screen::QUIT_DELAY;
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova::platform;
use nova_audio::recording::{AudioLog, RecordingAudio};
use nova_audio::{Audio, AudioCommand, AudioCore, AudioSettings, music_fade};
use nova_data::graphics::fixture::{DirectBits, PictBuilder, RledBuilder};
use nova_data::graphics::{PICT, RLED};
use nova_data::records::character::Character;
use nova_data::records::dialog::Dlog;
use nova_data::records::dialog_items::Ditl;
use nova_data::records::ship::Ship;
use nova_data::records::ship_anim::ShipAnim;
use nova_data::records::spin::Spin;
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::store::fs::{DirLister, EntryKind, Listing};
use nova_data::{GameData, InterfaceData, Record, SoundId};
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_render::{Batch, FontFaces, Frame};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{Fork, ForkReader, ResType};
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Pilot, PilotKeeper, PilotStore, ShipId, StellarId, SystemId};
use nova_view::MouseButton;
use nova_view::geometry::Point;
use nova_view::menu::MenuChoice;
use nova_view::menu::fixture::main_screen_resources;
use winit::event::ElementState;
use winit::keyboard::{KeyCode, PhysicalKey};

/// A 1024 x 768 window at scale 1: window pixels are logical units.
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

/// One file, holding a fork, wherever it is looked for.
struct OneFile(Vec<u8>);

impl DirLister for OneFile {
    fn list(&self, _dir: &Path) -> io::Result<Vec<Listing>> {
        Ok(vec![Listing {
            name: "Nova Data".into(),
            kind: EntryKind::File,
        }])
    }
}

impl ForkReader for OneFile {
    fn read_fork(&self, _path: &Path, fork: Fork) -> io::Result<Option<Vec<u8>>> {
        Ok((fork == Fork::Data).then(|| self.0.clone()))
    }
}

fn fork(resources: &[(ResType, i16, Vec<u8>)]) -> OneFile {
    let bytes = resources
        .iter()
        .fold(ForkBuilder::new(), |fork, (ty, id, data)| {
            fork.resource(*ty, *id, None, data)
        })
        .build()
        .bytes;
    OneFile(bytes)
}

fn be(values: &[i16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

fn put_i16s(bytes: &mut [u8], at: usize, values: &[i16]) {
    for (i, value) in values.iter().enumerate() {
        bytes[at + 2 * i..at + 2 * i + 2].copy_from_slice(&value.to_be_bytes());
    }
}

/// A `width` x `height` picture of one colour.
fn pict(width: i16, height: i16, rgb: [u8; 3]) -> Vec<u8> {
    let frame = [0, 0, height, width];
    let pixels = vec![rgb; (width * height) as usize];
    PictBuilder::new(frame)
        .direct_bits(&DirectBits::rgb888(frame, &pixels))
        .end()
        .build()
}

/// The starting credits.
const CASH: i32 = 25_000;

/// The `chär`: 25,000 credits, ship 128 in system 128 on 23 June 1177,
/// and no legal records.
fn character() -> Vec<u8> {
    let mut bytes = vec![0; Character::SIZE.expect("fixed")];
    bytes[0x00..0x04].copy_from_slice(&CASH.to_be_bytes());
    put_i16s(&mut bytes, 0x04, &[128, 128, -1, -1, -1]);
    put_i16s(&mut bytes, 0x0E, &[-1; 4]);
    put_i16s(&mut bytes, 0x134, &[23, 6, 1177]);
    bytes
}

/// An average `shïp` with shield, armour and fuel.
fn ship() -> Vec<u8> {
    let mut bytes = vec![0; Ship::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x02, &[30, 300, 300, 30, 300, 45]);
    bytes
}

/// A `shän` whose base image is `rlëD` 2000, one set of 36 rotations.
fn ship_anim() -> Vec<u8> {
    let mut bytes = vec![0; ShipAnim::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[2000, 0, 1]);
    put_i16s(&mut bytes, 0x34, &[36]);
    bytes
}

/// An `rlëD` of `frames` frames of `size` x `size`.
fn sheet(frames: u16, size: u16) -> Vec<u8> {
    (0..frames)
        .fold(RledBuilder::new(size, size), |sheet, _| {
            sheet.frame(|f| (0..size).fold(f, |f, _| f.line().pixels(&vec![0x0400; size.into()])))
        })
        .build()
}

/// An independent `sÿst` holding stellar 128 alone.
fn system() -> Vec<u8> {
    let mut bytes = vec![0; System::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x04, &[-1; 32]);
    put_i16s(&mut bytes, 0x24, &[128]);
    put_i16s(&mut bytes, 0x66, &[-1]);
    bytes
}

/// Where [`stellar`] is: (50, 50), where a new pilot's ship starts.
const ALPHA_PRIME_AT: nova_sim::Vec2 = nova_sim::Vec2::new(50.0, 50.0);

/// Alpha Prime: a landable planet with a bar at (50, 50), where a new
/// pilot's ship starts, whose landscape is `PICT` 10004.
fn stellar() -> Vec<u8> {
    let mut bytes = vec![0; Stellar::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[50, 50, 4]);
    bytes[0x06..0x0A].copy_from_slice(&0x41_u32.to_be_bytes());
    put_i16s(&mut bytes, 0x18, &[-1, -1]);
    bytes
}

/// A `spïn` naming `rlëD` 1000, one frame across.
fn spin() -> Vec<u8> {
    let mut bytes = vec![0; Spin::SIZE.expect("fixed")];
    put_i16s(&mut bytes, 0x00, &[1000, -1, 0, 0, 1, 1]);
    bytes
}

fn game_data() -> Rc<GameData> {
    let mut resources = vec![
        (Character::TYPE, 128, character()),
        (Ship::TYPE, 128, ship()),
        (ShipAnim::TYPE, 128, ship_anim()),
        (RLED, 2000, sheet(36, 1)),
        (System::TYPE, 128, system()),
        (Stellar::TYPE, 128, stellar()),
        (Spin::TYPE, 1004, spin()),
        (RLED, 1000, sheet(1, 8)),
        (PICT, 8500, pict(6, 5, [40, 40, 40])),
        (PICT, 10_004, pict(6, 3, [0, 90, 0])),
    ];
    for state in [7500, 7503, 7506] {
        resources.push((PICT, state, pict(13, 25, [200, 0, 0])));
        resources.push((PICT, state + 1, pict(2, 25, [0, 200, 0])));
        resources.push((PICT, state + 2, pict(13, 25, [0, 0, 200])));
        resources.push((PICT, state + 100, pict(13, 25, [0, 0, 0])));
        resources.push((PICT, state + 102, pict(13, 25, [0, 0, 0])));
    }
    // The main menu's art, so the menu draws the original's screen.
    resources.extend(main_screen_resources());
    let file = fork(&resources);
    Rc::new(GameData::load(&file, &file, Path::new("/data"), None).expect("opens"))
}

/// One `DITL` item: (left, top, right, bottom), its type byte and data.
fn ditl_item((l, t, r, b): (i16, i16, i16, i16), type_byte: u8, data: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 4];
    bytes.extend(be(&[t, l, b, r]));
    bytes.push(type_byte);
    bytes.push(data.len() as u8);
    bytes.extend(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

const USER: u8 = 0;
const BUTTON: u8 = 4;
const CHECK_BOX: u8 = 5;
const CONTROL: u8 = 7;
const STATIC_TEXT: u8 = 8;
const EDIT_TEXT: u8 = 16;
const PICTURE: u8 = 64;
const DISABLED: u8 = 0x80;

/// A `DLOG` of `bounds` (top, left, bottom, right), centred, naming its
/// `DITL`.
fn dlog((t, l, b, r): (i16, i16, i16, i16), ditl: i16) -> Vec<u8> {
    let mut bytes = be(&[t, l, b, r, 1]);
    bytes.extend([1, 0, 0, 0, 0, 0, 0, 0]);
    bytes.extend(be(&[ditl]));
    bytes.extend([0, 0, 0xA8, 0x0A]);
    bytes
}

fn ditl(items: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = be(&[items.len() as i16 - 1]);
    bytes.extend(items.concat());
    bytes
}

/// Stock "Create a new pilot:" (`DLOG` 3102, 326 x 213, centred) and its
/// fourteen items, and stock "Spaceport" (`DLOG` 1000) and its fifteen
/// user items.
fn interface() -> InterfaceData {
    let text = |bounds, s: &str| ditl_item(bounds, STATIC_TEXT | DISABLED, s.as_bytes());
    let field = |bounds| ditl_item(bounds, EDIT_TEXT, b"Edit Text");
    let new_pilot = [
        ditl_item((238, 183, 308, 203), BUTTON, b"OK"),
        ditl_item((156, 183, 226, 203), BUTTON, b"Cancel"),
        ditl_item((79, 141, 261, 163), PICTURE | DISABLED, &be(&[129])),
        ditl_item((59, 122, 154, 139), CHECK_BOX, b"Strict Play"),
        text((54, 36, 138, 52), "Full Name:"),
        text((54, 59, 133, 75), "Nickname:"),
        text((49, 357, 127, 373), "name3:"),
        field((149, 36, 319, 52)),
        field((149, 59, 319, 75)),
        field((144, 357, 314, 373)),
        ditl_item((51, 92, 251, 112), CONTROL, &be(&[500])),
        text((53, 6, 181, 22), "Create a new pilot:"),
        ditl_item((85, 277, 356, 297), CONTROL, &be(&[501])),
        ditl_item((7, 5, 39, 37), PICTURE | DISABLED, &be(&[130])),
    ];
    let spaceport: Vec<Vec<u8>> = [
        ((242, 551, 200, 25), true),
        ((452, 549, 68, 30), false),
        ((159, 297, 303, 18), false),
        ((471, 416, 145, 25), true),
        ((3, 3, 612, 285), true),
        ((160, 327, 301, 185), false),
        ((3, 414, 145, 25), true),
        ((471, 375, 145, 25), true),
        ((471, 333, 145, 25), true),
        ((3, 374, 145, 25), true),
        ((3, 333, 145, 25), true),
        ((471, 456, 145, 25), true),
        ((3, 456, 145, 25), true),
        ((524, 550, 68, 30), false),
        ((605, 548, 32, 32), false),
    ]
    .into_iter()
    .map(|((x, y, w, h), enabled)| {
        let byte = if enabled { USER } else { USER | DISABLED };
        ditl_item((x, y, x + w, y + h), byte, &[])
    })
    .collect();
    let file = fork(&[
        (Dlog::TYPE, 3102, dlog((78, 175, 291, 501), 3102)),
        (Ditl::TYPE, 3102, ditl(&new_pilot)),
        (Dlog::TYPE, 1000, dlog((-201, 60, 316, 678), 1000)),
        (Ditl::TYPE, 1000, ditl(&spaceport)),
    ]);
    InterfaceData::load(&file, Path::new("/Nova-DF.rsrc")).expect("loads")
}

struct Harness {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    frames: u64,
    /// What the app's audio was asked to play.
    log: AudioLog,
    /// The app's audio, to say what is still playing.
    audio: RecordingAudio,
}

impl Harness {
    /// The app on the main menu, keeping pilots in `store`, before any
    /// frame.
    fn opening(store: &MemoryPilots) -> Self {
        Self::opening_with(store, AudioSettings::default())
    }

    /// [`Harness::opening`], its audio on `settings`.
    fn opening_with(store: &MemoryPilots, settings: AudioSettings) -> Self {
        Self::built(store, settings, false)
    }

    /// The app on the main menu as `main` builds it, keeping pilots in
    /// `store` and opening the last pilot at launch, before any frame.
    fn launching(store: &MemoryPilots) -> Self {
        Self::built(store, AudioSettings::default(), true)
    }

    /// The app on the main menu, keeping pilots in `store`, its audio on
    /// `settings`, opening the last pilot at launch when `launch`.
    fn built(store: &MemoryPilots, settings: AudioSettings, launch: bool) -> Self {
        let data = game_data();
        let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
        let keeper = PilotKeeper::new(Box::new(store.clone()) as Box<dyn PilotStore>);
        let mut screen = start_screen(Rc::clone(&data))
            .with_pilots(Some(keeper), metrics.clone())
            .with_dialogs(Rc::new(interface()), metrics);
        if launch {
            screen = screen.open_last_pilot();
        }
        let audio = RecordingAudio::new();
        let log = audio.log();
        let handle = audio.clone();
        let core = AudioCore::new(Box::new(audio) as Box<dyn Audio>).with_settings(settings);
        Self {
            app: App::new(&FakeWindow, data, crate::common::appeared(screen)).with_audio(core),
            gpu: RecordingGpu::new(),
            frames: 0,
            log,
            audio: handle,
        }
    }

    fn handle(&mut self, event: WindowEvent) -> Control {
        self.app.handle(event, &mut FakeWindow, &mut self.gpu)
    }

    fn send(&mut self, event: WindowEvent) {
        assert_eq!(self.handle(event), Control::Continue, "{event:?}");
    }

    /// Presses and releases the physical key `code`, typing `text` (as
    /// winit reports it) on the press, through the platform's translation:
    /// the key, then its text.
    fn type_key(&mut self, code: KeyCode, text: Option<&str>) {
        for state in [ElementState::Pressed, ElementState::Released] {
            self.send(platform::key_event(PhysicalKey::Code(code), state, false));
            if let Some(typed) = platform::text_event(text, state) {
                self.send(typed);
            }
        }
    }

    /// Types `name`, each letter on its own key.
    fn type_name(&mut self, name: &str) {
        for c in name.chars() {
            let code = match c.to_ascii_lowercase() {
                'a' => KeyCode::KeyA,
                'd' => KeyCode::KeyD,
                'l' => KeyCode::KeyL,
                'p' => KeyCode::KeyP,
                'x' => KeyCode::KeyX,
                ' ' => KeyCode::Space,
                other => panic!("no key listed for {other:?}"),
            };
            self.type_key(code, Some(&c.to_string()));
        }
    }

    fn press(&mut self, code: KeyCode) {
        self.type_key(code, None);
    }

    /// Presses L twice, as the player lands: the first requests clearance,
    /// a few frames let the dock approach clear it, and the second lands.
    fn land(&mut self) {
        self.press(KeyCode::KeyL);
        self.approach();
        self.press(KeyCode::KeyL);
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

    /// Where the main menu's `choice` button is.
    fn menu_button(&self, choice: MenuChoice) -> Point {
        self.app
            .screen()
            .main_menu()
            .expect("a main menu")
            .press_point(choice)
    }

    /// The music commands since the last call; the effects let go.
    fn music(&self) -> Vec<AudioCommand> {
        std::mem::take(&mut *self.log.borrow_mut())
            .into_iter()
            .filter(|command| {
                matches!(
                    command,
                    AudioCommand::StartMusic { .. }
                        | AudioCommand::StopMusic
                        | AudioCommand::FadeOutMusic(_)
                        | AudioCommand::SetMusicVolume(_)
                )
            })
            .collect()
    }

    /// The `snd `s played since the last call; the rest let go.
    fn played(&self) -> Vec<SoundId> {
        std::mem::take(&mut *self.log.borrow_mut())
            .into_iter()
            .filter_map(|command| match command {
                AudioCommand::Play { sound, .. } => Some(sound),
                _ => None,
            })
            .collect()
    }

    /// Sends the next redraw, 16 ms on, and says whether the app goes on.
    fn redraw(&mut self) -> Control {
        self.frames += 1;
        let elapsed = Duration::from_millis(16 * self.frames);
        self.handle(WindowEvent::Redraw { elapsed })
    }

    /// Redraws until the app exits, and says how many frames that took.
    fn frames_until_exit(&mut self) -> u64 {
        for frames in 1..=100 {
            if self.redraw() == Control::Exit {
                return frames;
            }
        }
        panic!("the app never exits");
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }

    fn ship(&self) -> nova_sim::ShipState {
        *self
            .app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
            .player()
    }

    fn pilot(&self) -> Pilot {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .pilot()
            .expect("a pilot")
            .clone()
    }

    fn frame(&mut self) -> Frame {
        self.frames += 1;
        self.send(WindowEvent::Redraw {
            elapsed: Duration::from_millis(16 * self.frames),
        });
        assert_eq!(self.app.take_failures(), []);
        assert_eq!(self.app.take_warnings(), Vec::<String>::new());
        (*self.gpu.submits().last().expect("a frame")).clone()
    }

    /// Sends four redraws, at least a tick of flight, for the dock
    /// approach to clear a request. The nav brackets' corners (`cicn`
    /// 10000-10003), which the press's selection draws, may be missing
    /// from the fixture, so the frames' image failures are not checked.
    fn approach(&mut self) {
        for _ in 0..4 {
            self.frames += 1;
            let elapsed = Duration::from_millis(16 * self.frames);
            self.send(WindowEvent::Redraw { elapsed });
        }
        let _ = self.app.take_failures();
    }
}

fn texts(frame: &Frame) -> Vec<String> {
    frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Text(runs) => runs.iter().map(|run| run.text.clone()).collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// The main screen's six buttons are each drawn where they are hit.
fn assert_menu_drawn(game: &mut Harness) {
    let frame = game.frame();
    let quads: Vec<nova_render::Rect> = frame
        .batches
        .iter()
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.iter().map(|quad| quad.dest).collect(),
            _ => Vec::new(),
        })
        .collect();
    let menu = game.app.screen().main_menu().expect("a main menu");
    assert!(menu.draws_art());
    for choice in MenuChoice::ALL {
        let bounds = menu.button_bounds(choice);
        let drawn = nova_render::Rect {
            x: bounds.min.x,
            y: bounds.min.y,
            w: bounds.width(),
            h: bounds.height(),
        };
        assert!(quads.contains(&drawn), "{choice:?} at {drawn:?}");
    }
}

fn saved(store: &MemoryPilots, key: &str) -> Pilot {
    nova_sim::save::decode(&store.text(key).expect("saved")).expect("a pilot")
}

#[test]
fn a_new_pilot_lands_quits_and_resumes_where_it_landed() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    assert_eq!(game.showing(), Showing::MainMenu);
    assert_menu_drawn(&mut game);

    // New Pilot asks for a name; typing (with a slip, deleted) fills it.
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    assert_eq!(game.showing(), Showing::NewPilot);
    assert!(texts(&game.frame()).contains(&"Create a new pilot:".to_owned()));
    game.type_name("Adx");
    game.press(KeyCode::Backspace);
    game.type_name("a P");
    assert_eq!(
        game.showing(),
        Showing::NewPilot,
        "P typed, not Preferences"
    );
    assert!(texts(&game.frame()).contains(&"Ada P".to_owned()));
    game.press(KeyCode::Enter);
    game.click(crate::common::enter_ship_at(game.app.screen()));
    assert_eq!(game.showing(), Showing::Flight);
    let pilot = game.pilot();
    assert_eq!(pilot.name(), "Ada P");
    assert_eq!(pilot.cash(), i64::from(CASH));
    assert_eq!(pilot.ship(), ShipId(128));
    assert_eq!(pilot.system(), SystemId(128));
    assert_eq!(
        (
            pilot.date().day(),
            pilot.date().month(),
            pilot.date().year()
        ),
        (23, 6, 1177)
    );
    assert_eq!(store.keys(), ["Ada P"], "saved at once");
    game.frame();

    // Landing saves, with the stellar; leaving saves again.
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(store.writes(), 2);
    assert_eq!(saved(&store, "Ada P").stellar(), Some(StellarId(128)));
    game.press(KeyCode::Escape);
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(store.writes(), 3);

    // Landed again, closing the window saves the pilot as it is.
    game.land();
    let writes = store.writes();
    let landed = game.pilot();
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_eq!(store.writes(), writes + 1);
    assert_eq!(saved(&store, "Ada P"), landed);
    assert_eq!(game.app.take_warnings(), Vec::<String>::new());

    // A new app: Open Pilot lists it and resumes it in space over the
    // stellar it landed on, no spaceport opened.
    let mut game = Harness::opening(&store);
    let open_pilot = game.menu_button(MenuChoice::OpenPilot);
    game.click(open_pilot);
    assert_eq!(game.showing(), Showing::OpenPilot);
    assert!(texts(&game.frame()).contains(&"Ada P".to_owned()));
    game.press(KeyCode::Enter);
    game.click(crate::common::enter_ship_at(game.app.screen()));
    assert_eq!(game.showing(), Showing::Flight);
    assert!(game.app.screen().spaceport_view().is_none());
    assert_eq!(game.ship().position, ALPHA_PRIME_AT, "over Alpha Prime");
    let resumed = game.pilot();
    assert_eq!(resumed, landed);
    assert_eq!(resumed.stellar(), Some(StellarId(128)));
    assert_eq!(resumed.date(), landed.date());
    assert_eq!(resumed.ship(), landed.ship());
    assert_eq!(resumed.cash(), landed.cash());
    game.frame();

    // L, L lands it there.
    game.land();
    assert_eq!(game.showing(), Showing::Spaceport);
    assert_eq!(
        game.app
            .screen()
            .spaceport_view()
            .expect("landed")
            .stellar(),
        StellarId(128)
    );

    // Take off, back to the menu, which appears after its black second,
    // and Quit exits.
    game.press(KeyCode::Escape);
    game.press(KeyCode::Escape);
    assert_eq!(game.showing(), Showing::MainMenu);
    for _ in 0..frames_to(nova_view::fade::DEATH_FADE_FOR) {
        assert_eq!(game.redraw(), Control::Continue);
    }
    let quit = game.menu_button(MenuChoice::Quit);
    game.send(WindowEvent::PointerMoved {
        px: (f64::from(quit.x), f64::from(quit.y)),
    });
    game.send(WindowEvent::PointerButton {
        button: MouseButton::Left,
        pressed: true,
    });
    game.send(WindowEvent::PointerButton {
        button: MouseButton::Left,
        pressed: false,
    });
    assert_eq!(
        game.frames_until_exit(),
        frames_to(QUIT_DELAY),
        "Quit waits out its delay first"
    );
}

/// How many 16 ms frames it takes to reach `delay`.
fn frames_to(delay: Duration) -> u64 {
    u64::try_from(delay.as_millis().div_ceil(16)).expect("short")
}

const DOWN: SoundId = SoundId(600);
const UP: SoundId = SoundId(601);

#[test]
fn a_new_app_opens_holding_the_last_pilot_saved_and_enter_ship_flies_it() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);
    assert_eq!(store.keys(), ["Ada"]);

    let mut game = Harness::launching(&store);
    assert_eq!(game.showing(), Showing::MainMenu);
    let held = game.app.screen().game_in_progress().expect("a game held");
    assert_eq!(held.pilot().expect("a pilot").name(), "Ada");
    assert_menu_drawn(&mut game);
    game.click(crate::common::enter_ship_at(game.app.screen()));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.pilot().name(), "Ada");
    assert_eq!(game.ship().position, ALPHA_PRIME_AT, "over Alpha Prime");
}

#[test]
fn a_keyed_new_pilot_opens_only_once_its_click_has_played() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    game.frame();
    game.played();
    game.audio.set_playing(DOWN, true);
    game.type_key(KeyCode::KeyN, Some("n"));
    for _ in 0..3 {
        game.frame();
        assert_eq!(game.showing(), Showing::MainMenu, "600 still plays");
    }
    assert_eq!(game.played(), [DOWN], "no 601 yet");
    game.audio.set_playing(DOWN, false);
    game.frame();
    assert_eq!(game.played(), [UP]);
    assert_eq!(
        game.showing(),
        Showing::NewPilot,
        "in the frame that plays 601"
    );
}

#[test]
fn a_keyed_quit_exits_only_after_its_click_and_the_quit_delay() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    game.frame();
    game.played();
    game.audio.set_playing(DOWN, true);
    game.type_key(KeyCode::KeyQ, Some("q"));
    for _ in 0..frames_to(QUIT_DELAY) * 2 {
        assert_eq!(game.redraw(), Control::Continue, "600 still plays");
    }
    assert_eq!(game.played(), [DOWN]);
    game.audio.set_playing(DOWN, false);
    assert_eq!(game.redraw(), Control::Continue, "601 starts");
    assert_eq!(game.played(), [UP]);
    assert_eq!(game.frames_until_exit(), frames_to(QUIT_DELAY));
}

#[test]
fn with_sound_off_a_keyed_new_pilot_opens_at_once() {
    let store = MemoryPilots::new();
    let quiet = AudioSettings {
        sound: false,
        ..AudioSettings::default()
    };
    let mut game = Harness::opening_with(&store, quiet);
    game.audio.set_playing(DOWN, true);
    game.type_key(KeyCode::KeyN, Some("n"));
    assert_eq!(game.showing(), Showing::NewPilot, "before any frame");
    assert_eq!(game.played(), []);
}

#[test]
fn a_new_pilot_reopened_before_it_lands_resumes_over_the_stellar_its_first_save_names() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    game.type_name("Ada");
    game.press(KeyCode::Enter);
    game.click(crate::common::enter_ship_at(game.app.screen()));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.ship().position, nova_sim::Vec2::new(50.0, 50.0));
    assert_eq!(
        saved(&store, "Ada").stellar(),
        Some(StellarId(128)),
        "the nearest stellar, saved as last landed on"
    );
    assert_eq!(game.handle(WindowEvent::CloseRequested), Control::Exit);

    let mut game = Harness::opening(&store);
    let open_pilot = game.menu_button(MenuChoice::OpenPilot);
    game.click(open_pilot);
    game.press(KeyCode::Enter);
    game.click(crate::common::enter_ship_at(game.app.screen()));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(game.pilot().stellar(), Some(StellarId(128)));
    assert_eq!(game.ship().position, ALPHA_PRIME_AT, "over it");
}

#[test]
fn the_main_menu_plays_the_music_until_a_pilot_enters_the_game() {
    let store = MemoryPilots::new();
    let mut game = Harness::opening(&store);
    game.frame();
    assert_eq!(game.showing(), Showing::MainMenu);
    assert_eq!(
        game.music(),
        [AudioCommand::StartMusic {
            volume: AudioSettings::default().music_volume
        }]
    );

    let new_pilot = game.menu_button(MenuChoice::NewPilot);
    game.click(new_pilot);
    assert_eq!(game.showing(), Showing::NewPilot);
    game.frame();
    assert_eq!(game.music(), [], "the dialog keeps it");

    game.type_name("Ada");
    game.press(KeyCode::Enter);
    game.click(crate::common::enter_ship_at(game.app.screen()));
    assert_eq!(game.showing(), Showing::Flight);
    assert_eq!(
        game.music(),
        [AudioCommand::FadeOutMusic(music_fade(
            AudioSettings::default().music_volume
        ))]
    );
    game.frame();

    game.press(KeyCode::Escape);
    assert_eq!(game.showing(), Showing::MainMenu);
    game.frame();
    assert_eq!(game.music(), [], "spent for the rest of the run");
}
