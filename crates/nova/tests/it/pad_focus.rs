//! The pad alone through the app: Preferences, opened from the menu bar,
//! reads the fake pad's d-pad, South and East through the router, the
//! app's pad poll and the device switch, and its toggle's change reaches
//! the settings store.
//!
//! From the main menu, the pad alone opens Preferences and closes it,
//! and creates or opens a pilot and flies it, through the router's
//! telling the main menu and the screens it opens the device
//! (rdm:phase/key-bindings/phase-11-gamepad-menus), naming it on the
//! on-screen keyboard (rdm:phase/key-bindings/phase-17-text-entry-port).
//!
//! Over `preferences`' synthetic "new prefs dialog". Where the focus
//! lands, and what each button does, is nova-view's, pinned there
//! (`ui::pad_focus::every_screen`); this pins that the pieces are joined.

use std::time::Duration;

use nova::app::{Showing, WindowEvent};
use nova::gamepad::fixture::FakeGamepads;
use nova::gamepad::{PadEvent, PadId};
use nova::settings::SettingsKeeper;
use nova_sim::fixture::MemoryPilots;
use nova_view::menu::{MainMenu, MenuChoice};
use nova_view::ui::prefs::Focus;
use nova_view::{ControllerFamily, Device, MenuCommand, PadButton};

use crate::preferences::{Harness, seeded};

const PAD: PadId = PadId(0);

/// The preferences harness reading the fake pads, and every window event
/// it was sent.
struct Padded {
    harness: Harness,
    pads: FakeGamepads,
    sent: Vec<WindowEvent>,
}

impl Padded {
    fn new(store: &nova_audio::recording::MemorySettings) -> Self {
        Self::reading(Harness::new(store))
    }

    /// The game started on the main menu, reading the fake pads.
    fn on_main_menu(store: &nova_audio::recording::MemorySettings) -> Self {
        Self::reading(Harness::on_main_menu(store))
    }

    fn reading(harness: Harness) -> Self {
        let pads = FakeGamepads::new();
        let Harness {
            app,
            window,
            gpu,
            log,
            frames,
        } = harness;
        let harness = Harness {
            app: app.with_gamepads(Box::new(pads.clone())),
            window,
            gpu,
            log,
            frames,
        };
        Self {
            harness,
            pads,
            sent: Vec::new(),
        }
    }

    fn send(&mut self, event: WindowEvent) {
        self.sent.push(event);
        self.harness.send(event);
    }

    /// One frame, which polls the pads.
    fn frame(&mut self) {
        self.harness.frames += 1;
        let elapsed = Duration::from_millis(16 * self.harness.frames);
        self.send(WindowEvent::Redraw { elapsed });
    }

    /// `button` pressed, a frame, released, a frame.
    fn tap(&mut self, button: PadButton) {
        for pressed in [true, false] {
            self.pads.push(vec![PadEvent::Button {
                pad: PAD,
                button,
                pressed,
            }]);
            self.frame();
        }
    }
}

/// Whether `part` is one of the ten toggles.
fn toggle(part: Focus) -> bool {
    !matches!(
        part,
        Focus::Display
            | Focus::Scaling
            | Focus::EffectsVolume
            | Focus::MusicVolume
            | Focus::KeySettings
            | Focus::Ok
    )
}

#[test]
fn the_pad_alone_moves_the_preferences_focus_flips_a_toggle_and_closes_it() {
    let store = seeded();
    let mut app = Padded::new(&store);
    app.send(WindowEvent::Menu(MenuCommand::Preferences));
    assert_eq!(app.harness.showing(), Showing::Preferences);
    let after_menu = app.sent.len();

    app.pads.push(vec![PadEvent::connected(PAD)]);
    app.frame();
    let mut moves = 0;
    while !app.harness.dialog().focus().is_some_and(toggle) {
        assert!(moves < 20, "the d-pad reaches a toggle");
        app.tap(PadButton::DPadUp);
        moves += 1;
    }
    assert_eq!(
        app.harness.dialog().device(),
        Device::Gamepad(ControllerFamily::Generic),
        "the pad was used last"
    );
    let part = app.harness.dialog().focus().expect("a toggle");
    let before = SettingsKeeper::open(store.clone()).0.settings();

    app.tap(PadButton::South);
    let after = SettingsKeeper::open(store.clone()).0.settings();
    assert_ne!(after, before, "{part:?} flipped in the store");

    app.tap(PadButton::East);
    assert_ne!(app.harness.showing(), Showing::Preferences, "closed");

    assert!(
        app.sent[after_menu..]
            .iter()
            .all(|event| matches!(event, WindowEvent::Redraw { .. })),
        "no key or pointer event after the menu command: {:?}",
        app.sent
    );
}

/// The pad alone in the spaceport: after landing, the pad's focus reaches
/// the Trade Center, South opens it, East closes it, and East again takes
/// off, through the router's telling the spaceport the device and the
/// spaceport's telling the screen it opens. Over `trade`'s landed pilot.
#[test]
fn the_pad_alone_opens_a_service_closes_it_and_takes_off() {
    use nova_sim::fixture::MemoryPilots;
    use nova_view::spaceport::layout::SERVICE_ITEMS;
    use nova_view::ui::dialog::Dialog;

    let pads = FakeGamepads::new();
    let store = MemoryPilots::new();
    let mut game = crate::trade::Harness::landed_with_pads(&store, Box::new(pads.clone()));
    let mut events = Vec::new();
    let mut send = |game: &mut crate::trade::Harness, event: WindowEvent| {
        events.push(event);
        game.send(event);
    };
    let mut frame = |game: &mut crate::trade::Harness| {
        game.frames += 1;
        let elapsed = Duration::from_millis(16 * game.frames);
        send(game, WindowEvent::Redraw { elapsed });
    };
    pads.push(vec![PadEvent::connected(PAD)]);
    frame(&mut game);
    let mut tap = |game: &mut crate::trade::Harness, button: PadButton| {
        for pressed in [true, false] {
            pads.push(vec![PadEvent::Button {
                pad: PAD,
                button,
                pressed,
            }]);
            frame(game);
        }
    };
    let highlight = |game: &crate::trade::Harness| {
        game.app
            .screen()
            .spaceport_view()
            .and_then(|port| port.dialog())
            .and_then(Dialog::pad_highlight)
    };
    // The Trade Center, the service this stellar offers.
    let trade_center = SERVICE_ITEMS
        .iter()
        .find(|&&(service, _)| service == nova_sim::Service::TradeCenter)
        .map(|&(_, item)| item);
    let on_service = |item: Option<usize>| item.is_some() && item == trade_center;
    let mut moves = 0;
    while !on_service(highlight(&game)) {
        assert!(
            moves < 20,
            "the pad reaches the Trade Center: {:?}",
            highlight(&game)
        );
        tap(&mut game, PadButton::RightShoulder);
        moves += 1;
    }
    let port = game.app.screen().spaceport_view().expect("landed");
    assert_eq!(port.device(), Device::Gamepad(ControllerFamily::Generic));

    let opened = |game: &crate::trade::Harness| {
        let port = game.app.screen().spaceport_view().expect("landed");
        port.open_trade().is_some()
    };
    tap(&mut game, PadButton::South);
    assert!(opened(&game), "South opens the Trade Center");
    tap(&mut game, PadButton::East);
    assert!(!opened(&game), "East closes it");
    assert_eq!(game.showing(), Showing::Spaceport);
    tap(&mut game, PadButton::East);
    assert_eq!(game.showing(), Showing::Flight, "East again takes off");
    assert!(
        events
            .iter()
            .all(|event| matches!(event, WindowEvent::Redraw { .. })),
        "no key or pointer event after landing: {events:?}"
    );
}

/// The d-pad button that heads from the main menu's highlight toward
/// `goal`'s button: the way the gap between their centres is widest.
fn toward(menu: &MainMenu, goal: MenuChoice) -> PadButton {
    let Some(at) = menu.pad_highlight() else {
        return PadButton::DPadDown;
    };
    let (from, to) = (
        menu.button_bounds(at).center(),
        menu.button_bounds(goal).center(),
    );
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    match (dx.abs() > dy.abs(), dx > 0.0, dy > 0.0) {
        (true, true, _) => PadButton::DPadRight,
        (true, false, _) => PadButton::DPadLeft,
        (false, _, true) => PadButton::DPadDown,
        (false, _, false) => PadButton::DPadUp,
    }
}

/// The pad alone from the main menu: the d-pad reaches Preferences,
/// South opens it and East closes it, through the router's telling the
/// main menu the device (rdm:phase/key-bindings/phase-11-gamepad-menus).
#[test]
fn the_pad_alone_opens_preferences_from_the_main_menu_and_closes_it() {
    let store = seeded();
    let mut app = Padded::on_main_menu(&store);
    assert_eq!(app.harness.showing(), Showing::MainMenu);
    app.pads.push(vec![PadEvent::connected(PAD)]);
    app.frame();
    let menu = |app: &Padded| {
        app.harness
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .clone()
    };
    let mut taps = Vec::new();
    for _ in 0..20 {
        let now = menu(&app);
        if now.pad_highlight() == Some(MenuChoice::Preferences) {
            break;
        }
        let button = toward(&now, MenuChoice::Preferences);
        taps.push(button);
        app.tap(button);
    }
    assert_eq!(
        menu(&app).pad_highlight(),
        Some(MenuChoice::Preferences),
        "the d-pad reaches Preferences: {taps:?}"
    );
    app.tap(PadButton::South);
    assert_eq!(app.harness.showing(), Showing::Preferences);
    app.tap(PadButton::East);
    assert_eq!(app.harness.showing(), Showing::MainMenu, "East closes it");
    assert!(
        app.sent
            .iter()
            .all(|event| matches!(event, WindowEvent::Redraw { .. })),
        "no key or pointer event: {:?}",
        app.sent
    );
}

/// The pilots harness reading the fake pads.
struct PadPilots {
    game: crate::pilots::Harness,
    pads: FakeGamepads,
}

impl PadPilots {
    fn opening(store: &MemoryPilots) -> Self {
        let pads = FakeGamepads::new();
        let game = crate::pilots::Harness::opening_with_pads(store, Box::new(pads.clone()));
        let mut padded = Self { game, pads };
        padded.pads.push(vec![PadEvent::connected(PAD)]);
        padded.game.frame();
        padded
    }

    fn tap(&mut self, button: PadButton) {
        for pressed in [true, false] {
            self.pads.push(vec![PadEvent::Button {
                pad: PAD,
                button,
                pressed,
            }]);
            self.game.frame();
        }
    }

    fn menu(&self) -> MainMenu {
        self.game
            .app
            .screen()
            .main_menu()
            .expect("a main menu")
            .clone()
    }

    /// The d-pad to `goal`, then South.
    fn choose(&mut self, goal: MenuChoice) {
        for _ in 0..20 {
            let menu = self.menu();
            if menu.pad_highlight() == Some(goal) {
                self.tap(PadButton::South);
                return;
            }
            self.tap(toward(&menu, goal));
        }
        panic!("the d-pad never reaches {goal:?}");
    }

    /// Frames until flight shows, South tapped on each intro picture.
    fn fly_in(&mut self) {
        for _ in 0..2000 {
            match self.game.showing() {
                Showing::Flight if !self.game.app.screen().entering() => return,
                Showing::Intro | Showing::IntroText => self.tap(PadButton::South),
                _ => {
                    self.game.frame();
                }
            }
        }
        panic!("never into flight: {:?}", self.game.showing());
    }
}

/// The pad alone creates a pilot, accepting the drawn default name and
/// ship name with South, and flies it: New Pilot, OK, the christening's
/// OK, Enter Ship, and the intro's picture ended with South.
#[test]
fn the_pad_alone_creates_a_pilot_and_flies_it() {
    let store = MemoryPilots::new();
    let mut app = PadPilots::opening(&store);
    assert_eq!(app.game.showing(), Showing::MainMenu);
    app.choose(MenuChoice::NewPilot);
    assert_eq!(app.game.showing(), Showing::NewPilot);
    app.tap(PadButton::South);
    assert_eq!(app.game.showing(), Showing::Christening);
    app.tap(PadButton::South);
    assert_eq!(app.game.showing(), Showing::MainMenu);
    app.choose(MenuChoice::EnterShip);
    app.fly_in();
    assert_eq!(app.game.showing(), Showing::Flight);
    assert_eq!(store.keys(), ["Ada Lovelace"], "saved under the default");
}

/// The pad alone names a new pilot on the on-screen keyboard
/// (rdm:phase/key-bindings/phase-17-text-entry-port) and flies it: New
/// Pilot, the shoulder to the name, South raising the keyboard, its keys
/// by the d-pad and South (Clear, "Zoe", Done, which presses OK), the
/// christening's OK, Enter Ship, and the intro's picture ended with
/// South.
#[test]
fn the_pad_alone_names_a_new_pilot_on_the_on_screen_keyboard_and_flies_it() {
    use nova_view::ui::entry_keyboard::{self, EntryHost, KeyId};
    use nova_view::ui::new_pilot::NAME_ITEM;
    let store = MemoryPilots::new();
    let mut app = PadPilots::opening(&store);
    app.choose(MenuChoice::NewPilot);
    assert_eq!(app.game.showing(), Showing::NewPilot);
    let dialog = |app: &PadPilots| app.game.app.screen().new_pilot().expect("open").clone();
    for _ in 0..8 {
        if dialog(&app).dialog().pad_highlight() == Some(NAME_ITEM) {
            break;
        }
        app.tap(PadButton::LeftShoulder);
    }
    assert_eq!(dialog(&app).dialog().pad_highlight(), Some(NAME_ITEM));
    app.tap(PadButton::South);
    let raised = dialog(&app);
    let layout = entry_keyboard::shown(&raised).expect("South raises the keyboard");
    let from = raised.keyboard().pad_focus().expect("the pad's focus");
    let keys = [
        KeyId::Clear,
        KeyId::Char('z'),
        KeyId::Char('o'),
        KeyId::Char('e'),
        KeyId::Done,
    ];
    for input in layout.pad_presses(from, &keys) {
        let nova_view::Input::PadButton {
            button, pressed, ..
        } = input
        else {
            panic!("pad inputs alone: {input:?}");
        };
        app.pads.push(vec![PadEvent::Button {
            pad: PAD,
            button,
            pressed,
        }]);
        app.game.frame();
    }
    assert_eq!(app.game.showing(), Showing::Christening, "Done pressed OK");
    app.tap(PadButton::South);
    assert_eq!(app.game.showing(), Showing::MainMenu);
    app.choose(MenuChoice::EnterShip);
    app.fly_in();
    assert_eq!(app.game.showing(), Showing::Flight);
    assert_eq!(store.keys(), ["Zoe"], "saved under the name typed");
}

/// The pad alone opens a saved pilot from the list and flies it.
#[test]
fn the_pad_alone_opens_a_saved_pilot_and_flies_it() {
    let store = MemoryPilots::new();
    let mut keyed = crate::pilots::Harness::opening(&store);
    keyed.create_and_close("Ada");
    assert_eq!(store.keys(), ["Ada"]);

    let mut app = PadPilots::opening(&store);
    app.choose(MenuChoice::OpenPilot);
    assert_eq!(app.game.showing(), Showing::OpenPilot);
    app.tap(PadButton::South);
    assert_eq!(app.game.showing(), Showing::MainMenu);
    app.choose(MenuChoice::EnterShip);
    app.fly_in();
    assert_eq!(app.game.showing(), Showing::Flight);
    let flown = app.game.app.screen().flight_view().expect("flying");
    assert_eq!(flown.sim().pilot().expect("a pilot").name(), "Ada");
}
