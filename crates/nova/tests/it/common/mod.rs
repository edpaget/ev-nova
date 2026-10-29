//! The program edge shared by the test modules: the stock data's
//! directory, the router past its start-up appear ([`appeared`]), where
//! the main menu's Enter Ship is ([`enter_ship_at`]), and a click on it
//! with the way into play stepped through ([`EntryHarness::enter_ship`]),
//! and the frames drawn until the screen has settled ([`Frames::settle`],
//! [`Frames::settle_until`]).
//!
//! This module is the only place `NOVA_DATA` is read. The game data is
//! copyrighted and never committed, so each stock test asks [`nova_data`] for
//! the `Nova Files` directory and skips, passing, when it is unset.

use std::path::PathBuf;

/// The `Nova Files` directory named by `NOVA_DATA`, or `None` (after printing
/// a skip message) when it is unset.
pub fn nova_data() -> Option<PathBuf> {
    let dir = std::env::var_os("NOVA_DATA").map(PathBuf::from);
    if dir.is_none() {
        eprintln!("skipping: NOVA_DATA not set");
    }
    dir
}

/// `screen` once its main screen has appeared: the second's fade from
/// black ticked out, then a click off every button, which finishes the
/// slides silently, and the sounds taken, so a test starts on the menu as
/// a player sees it settled.
#[must_use]
pub fn appeared(mut screen: nova::app::screen::AppScreen) -> nova::app::screen::AppScreen {
    use nova_view::{Input, MouseButton, Point, Screen};
    screen.tick(nova_view::fade::DEATH_FADE_FOR);
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: Point::new(1.0, 1.0),
        });
    }
    screen.take_sounds();
    screen
}

/// Stock "Text Input" (`DITL` 3001)'s six items, the new pilot's ship
/// christening and the shipyard's name prompt, each built by `item` from
/// its rectangle (left, top, right, bottom), its type byte and its data:
/// OK, a picture, the prompt, a picture, the field and Cancel.
pub fn text_input_items(item: impl Fn((i16, i16, i16, i16), u8, &[u8]) -> Vec<u8>) -> Vec<Vec<u8>> {
    const BUTTON: u8 = 4;
    const STATIC_TEXT: u8 = 8;
    const EDIT_TEXT: u8 = 16;
    const PICTURE: u8 = 64;
    const DISABLED: u8 = 0x80;
    vec![
        item((252, 106, 322, 126), BUTTON, b"OK"),
        item(
            (7, 147, 39, 179),
            PICTURE | DISABLED,
            &129_i16.to_be_bytes(),
        ),
        item((52, 5, 347, 55), STATIC_TEXT | DISABLED, b"^0"),
        item((7, 5, 39, 37), PICTURE | DISABLED, &130_i16.to_be_bytes()),
        item((91, 64, 291, 80), EDIT_TEXT, b""),
        item((170, 106, 240, 126), BUTTON, b"Cancel"),
    ]
}

/// Where the main menu's Enter Ship button is pressed. A pilot created or
/// opened from the menu is held there, not flown, and a click here enters
/// play with it.
#[must_use]
pub fn enter_ship_at(screen: &nova::app::screen::AppScreen) -> nova_view::Point {
    screen
        .main_menu()
        .expect("a main menu")
        .press_point(nova_view::menu::MenuChoice::EnterShip)
}

/// The most frames [`Frames::settle`] draws: a slot key settles in two
/// (its pick, then its latch clearing), so eight leaves room for a block
/// a frame or two slower and still catches a queue that never empties.
pub const SETTLE_FRAMES: usize = 8;

/// The most frames [`Frames::settle_until`] draws: ten seconds at 60 a
/// second.
pub const UNTIL_FRAMES: usize = 600;

/// A harness that draws the app's frames: its router and a frame sent.
/// Its tests assert after [`Frames::settle`] or [`Frames::settle_until`],
/// never on a particular frame, so a screen that takes a frame more or
/// less to act fails in the crate that owns it, not here.
pub trait Frames {
    /// The app's router.
    fn router(&self) -> &nova::app::screen::AppScreen;

    /// Sends a frame.
    fn next_frame(&mut self);

    /// Draws frames until the screen has taken the input sent: at least
    /// one, as a press is polled at the next frame's tick, then until the
    /// router holds no queued input
    /// ([`AppScreen::input_queued`](nova::app::screen::AppScreen::input_queued)).
    /// Bounded by [`SETTLE_FRAMES`], so a queue that never empties fails.
    fn settle(&mut self) {
        for _ in 0..SETTLE_FRAMES {
            self.next_frame();
            if !self.router().input_queued() {
                return;
            }
        }
        panic!("the screen still holds queued input after {SETTLE_FRAMES} frames");
    }

    /// Draws frames until `done` holds, checked before each: for a wait
    /// whose length is the game's time, not input (the dock approach's
    /// clearance, a jump's arrival, a fade). `done` reads a routing fact or
    /// a read of the flight or session naming the state waited for, never
    /// wording or a frame number. Bounded by [`UNTIL_FRAMES`], panicking
    /// with `what` never came.
    fn settle_until(&mut self, what: &str, mut done: impl FnMut(&mut Self) -> bool)
    where
        Self: Sized,
    {
        for _ in 0..UNTIL_FRAMES {
            if done(self) {
                return;
            }
            self.next_frame();
        }
        assert!(done(self), "{what} never came in {UNTIL_FRAMES} frames");
    }
}

/// Implements [`Frames`] for a harness `$harness` whose `app` is the
/// [`App`](nova::app::App) and whose `$frame` sends a frame.
macro_rules! frames {
    ($harness:ty, $frame:ident) => {
        impl crate::common::Frames for $harness {
            fn router(&self) -> &nova::app::screen::AppScreen {
                self.app.screen()
            }

            fn next_frame(&mut self) {
                let _ = self.$frame();
            }
        }
    };
}
pub(crate) use frames;

/// A harness driving the whole app, which [`through_entry`] can step: its
/// frames, an event sent and a click.
pub trait EntryHarness: Frames {
    /// Sends `event` to the app.
    fn send_event(&mut self, event: nova::app::WindowEvent);

    /// Clicks the primary button at `at`, in the classic frame.
    fn click_point(&mut self, at: nova_view::Point);

    /// Clicks the main menu's Enter Ship and steps the way into play
    /// through ([`through_entry`]).
    fn enter_ship(&mut self)
    where
        Self: Sized,
    {
        let at = enter_ship_at(self.router());
        self.click_point(at);
        through_entry(self);
    }
}

/// Steps play's entry through: frames until the router no longer enters
/// play, the skip key tapped while the intro's pictures show and Return
/// while its text does. Bounded, so an entry that never ends fails.
pub fn through_entry(harness: &mut impl EntryHarness) {
    use nova::app::WindowEvent;
    use nova_view::Showing;
    for _ in 0..2000 {
        if !harness.router().entering() {
            return;
        }
        let tap = match harness.router().showing() {
            Showing::Intro => Some(nova_view::menu::intro::SKIP_KEY),
            Showing::IntroText => Some(nova_view::Key::Enter),
            _ => None,
        };
        if let Some(key) = tap {
            for pressed in [true, false] {
                harness.send_event(WindowEvent::Key {
                    key,
                    pressed,
                    repeat: false,
                });
            }
        }
        harness.next_frame();
    }
    panic!("play is still being entered");
}

/// Implements [`EntryHarness`] and [`Frames`] for a harness `$harness`
/// whose `app` is the [`App`](nova::app::App), whose `send` sends an
/// event, `$click` clicks a point and `$frame` sends a frame.
macro_rules! entry_harness {
    ($harness:ty, $click:ident, $frame:ident) => {
        crate::common::frames!($harness, $frame);

        impl crate::common::EntryHarness for $harness {
            fn send_event(&mut self, event: nova::app::WindowEvent) {
                self.send(event);
            }

            fn click_point(&mut self, at: nova_view::Point) {
                self.$click(at);
            }
        }
    };
}
pub(crate) use entry_harness;
