//! Flight's on-screen touch controls
//! (rdm:phase/key-bindings/phase-15-touch-flight-controls): a floating
//! stick, large fire and afterburner buttons, an edge row of flight's
//! commands and a menu behind More holding every other one, so a player
//! with no keyboard flies, fights and reaches every flight command.
//!
//! # A device of its own
//!
//! The overlay never fakes a key. It raises [`Input::TouchControl`]: a
//! button pressing or letting go of a command, or the stick's raw offset.
//! The keymap reads it as a third device beside the keyboard and the pad
//! ([`crate::keymap::touch`]), so a button runs through flight's own key
//! path (the same gates, latches, sounds and messages as the command's
//! key), and the stick feeds the same turn and throttle amounts as the
//! pad's flight axes.
//!
//! # The drain
//!
//! The overlay acts on nothing itself: what it raises is queued, and
//! [`TouchControls::take_queued`] drains it. Flight hands the queue on
//! (`FlightView::take_touch_controls`), and the app's router runs each
//! input through its own input path from the top, as it does a key, so
//! the router's own readers (Pause, which leaves flight, and Double
//! Time) see it, and each release reaches whatever is over flight by
//! then.
//!
//! # One role per finger
//!
//! A finger the overlay claims ([`TouchControls::claims`]) takes one role
//! as it lands and keeps it until it lifts or is cancelled, wherever it
//! slides:
//!
//! - on the stick zone, the stick, based where it landed;
//! - on a button, that button's command, pressed as it lands, as a key
//!   acts on its press; two fingers on one button hold it until both
//!   lift;
//! - on More, a page arrow or the open menu's background, an action as
//!   it lifts, never on a cancel, so a system gesture taking the finger
//!   opens and turns nothing.
//!
//! A lift or a cancel releases what the finger held, so a cancelled touch
//! on fire releases fire; a menu button then closes the menu.
//! [`TouchControls::let_go`] releases everything held, centres the stick,
//! closes the menu and forgets every finger: later events from those
//! fingers do nothing. Flight lets go of it whenever a dialog opens over
//! it, the map opens, or it otherwise stops flying live, so a control
//! whose own press opens a dialog is released as the dialog opens.
//!
//! # Showing
//!
//! The overlay shows while the device used last is touch
//! ([`Surroundings::device`]) and flight flies live
//! ([`TouchControls::set_live`]); any other device, or flight not live,
//! hides it, letting go of everything. The overlay makes no sound of its
//! own: every sound is its command's.
//!
//! # Where each addition is pinned
//!
//! - The layout's rules (sizes in points, the safe area, the tiers, every
//!   command reached): [`layout`](mod@layout)'s tests.
//! - The device: [`crate::keymap::touch`]'s tests.
//! - The roles and the queue: this module's tests.
//! - Flight's path (each control does what its key does):
//!   `flight/view/tests/touch_flight.rs`.
//! - The router's drain and claims: `nova`'s
//!   `app/screen/tests/touch_controls.rs`.
//! - A touch on each control through the app's viewport: `nova`'s
//!   `app/tests/touch_controls.rs`.

pub mod draw;
pub mod layout;
pub mod stick;

use std::collections::{BTreeMap, HashMap};

use crate::geometry::Point;
use crate::input::{Device, FingerId, Input, Surroundings, Touch, TouchControl, TouchPhase};
use crate::keymap::Command;
use crate::keymap::command::FlightCommand;

use layout::{ControlId, Layout, MenuView, Region, STICK_RADIUS_PT, layout};

/// What a finger the overlay claimed does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    /// The floating stick, based where it landed, with the thumb at `at`,
    /// holding Reverse Course or not.
    Stick {
        base: Point,
        at: Point,
        reverse: bool,
    },
    /// Holding a command, as a key does.
    Button(Command),
    /// Holding a menu cell's command; the menu closes as it lifts.
    MenuButton(Command),
    /// Opens or closes the menu as it lifts.
    More,
    /// Turns the menu back a page as it lifts.
    PagePrev,
    /// Turns the menu on a page as it lifts.
    PageNext,
    /// Closes the menu as it lifts.
    MenuBackdrop,
    /// Does nothing: a second finger on the stick zone.
    Inert,
}

/// The overlay's state: what it is told, the menu, each finger's role,
/// how many fingers hold each command, and the queue of what it raised.
#[derive(Clone, Debug, Default)]
pub struct TouchControls {
    /// The surroundings the app told last.
    told: Surroundings,
    /// The flight view's size, in logical units.
    view: (f32, f32),
    /// Whether flight flies live: no map, movie, dialog or end over it.
    live: bool,
    /// The menu's state, with the escort menu's.
    menu: MenuView,
    /// Each finger claimed, and its role.
    fingers: HashMap<FingerId, Role>,
    /// How many fingers hold each command down.
    held: BTreeMap<Command, usize>,
    /// What it raised, for the drain.
    queue: Vec<Input>,
}

impl TouchControls {
    /// Keeps the surroundings the app told: any device but touch hides
    /// the overlay, letting go of everything.
    pub fn tell(&mut self, told: Surroundings) {
        self.told = told;
        if !self.showing() {
            self.let_go();
        }
    }

    /// Keeps the flight view's size, in logical units.
    pub const fn set_view(&mut self, view: (f32, f32)) {
        self.view = view;
    }

    /// Keeps whether flight flies live; not live hides the overlay,
    /// letting go of everything.
    pub fn set_live(&mut self, live: bool) {
        self.live = live;
        if !self.showing() {
            self.let_go();
        }
    }

    /// Keeps whether flight's escort menu shows, which places the escort
    /// groups.
    pub const fn set_escort_menu(&mut self, shows: bool) {
        self.menu.escort_menu = shows;
    }

    /// Whether the overlay shows: touch was used last and flight flies
    /// live.
    #[must_use]
    pub fn showing(&self) -> bool {
        self.live && self.told.device == Device::Touch
    }

    /// The controls as placed now.
    #[must_use]
    pub fn layout(&self) -> Layout {
        layout(&Region::for_flight(&self.told, self.view), self.menu)
    }

    /// Whether the menu is open.
    #[must_use]
    pub const fn menu_open(&self) -> bool {
        self.menu.open
    }

    /// Whether a touch starting at `at` is the overlay's: it shows, and
    /// `at` falls on a placed control, the stick zone, or the open menu's
    /// panel.
    #[must_use]
    pub fn claims(&self, at: Point) -> bool {
        if !self.showing() {
            return false;
        }
        let layout = self.layout();
        layout.hit(at).is_some() || layout.panel.is_some_and(|panel| panel.contains(at))
    }

    /// Follows a claimed finger's touch.
    pub fn touch(&mut self, touch: Touch) {
        match touch.phase {
            TouchPhase::Started(at) => self.start(touch.finger, at),
            TouchPhase::Moved(at) => self.moved(touch.finger, at),
            TouchPhase::Ended(_) => self.end(touch.finger, true),
            TouchPhase::Cancelled => self.end(touch.finger, false),
        }
    }

    /// Whether a control holds `command` down now.
    #[must_use]
    pub fn holds(&self, command: Command) -> bool {
        self.held.contains_key(&command)
    }

    /// The stick held: where it landed and where the thumb is.
    #[must_use]
    pub fn stick(&self) -> Option<(Point, Point)> {
        self.fingers.values().find_map(|role| match *role {
            Role::Stick { base, at, .. } => Some((base, at)),
            _ => None,
        })
    }

    /// The stick's radius in logical units.
    #[must_use]
    pub fn stick_radius(&self) -> f32 {
        STICK_RADIUS_PT / self.told.points_per_pixel
    }

    /// Releases everything held and centres the stick, then forgets every
    /// finger and closes the menu: later events from those fingers do
    /// nothing.
    pub fn let_go(&mut self) {
        if self.stick().is_some() {
            self.queue.push(stick_input(0.0, 0.0));
        }
        for command in std::mem::take(&mut self.held).into_keys() {
            self.queue.push(button(command, false));
        }
        self.fingers.clear();
        self.close_menu();
    }

    /// What it raised since the last drain, in order.
    pub fn take_queued(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.queue)
    }

    fn start(&mut self, finger: FingerId, at: Point) {
        if !self.claims(at) || self.fingers.contains_key(&finger) {
            return;
        }
        let layout = self.layout();
        let on_panel = layout.panel.is_some_and(|panel| panel.contains(at));
        let role = match layout.hit(at) {
            Some(ControlId::Stick) if self.stick().is_some() => Role::Inert,
            Some(ControlId::Stick) => Role::Stick {
                base: at,
                at,
                reverse: false,
            },
            Some(ControlId::Command(command)) => {
                self.press(command);
                if on_panel {
                    Role::MenuButton(command)
                } else {
                    Role::Button(command)
                }
            }
            Some(ControlId::More) => Role::More,
            Some(ControlId::PagePrev) => Role::PagePrev,
            Some(ControlId::PageNext) => Role::PageNext,
            None => Role::MenuBackdrop,
        };
        self.fingers.insert(finger, role);
    }

    fn moved(&mut self, finger: FingerId, to: Point) {
        let radius = self.stick_radius();
        let Some(Role::Stick { base, at, reverse }) = self.fingers.get_mut(&finger) else {
            return;
        };
        *at = to;
        let (x, y) = stick::offset(*base, to, radius);
        let was = *reverse;
        let now = stick::reverse_after(was, y);
        *reverse = now;
        self.queue.push(stick_input(x, y));
        let back = Command::Flight(FlightCommand::ReverseCourse);
        if now && !was {
            self.press(back);
        } else if was && !now {
            self.release(back);
        }
    }

    fn end(&mut self, finger: FingerId, lifted: bool) {
        let Some(role) = self.fingers.remove(&finger) else {
            return;
        };
        match role {
            Role::Stick { reverse, .. } => {
                self.queue.push(stick_input(0.0, 0.0));
                if reverse {
                    self.release(Command::Flight(FlightCommand::ReverseCourse));
                }
            }
            Role::Button(command) => self.release(command),
            Role::MenuButton(command) => {
                self.release(command);
                self.close_menu();
            }
            Role::More if lifted => {
                self.menu.open = !self.menu.open;
                self.menu.page = 0;
            }
            Role::PagePrev if lifted => {
                let pages = self.layout().pages.max(1);
                self.menu.page = (self.menu.page + pages - 1) % pages;
            }
            Role::PageNext if lifted => {
                let pages = self.layout().pages.max(1);
                self.menu.page = (self.menu.page + 1) % pages;
            }
            Role::MenuBackdrop if lifted => self.close_menu(),
            Role::More | Role::PagePrev | Role::PageNext | Role::MenuBackdrop | Role::Inert => {}
        }
    }

    const fn close_menu(&mut self) {
        self.menu.open = false;
        self.menu.page = 0;
    }

    /// One more finger holds `command`: its press goes out with the
    /// first.
    fn press(&mut self, command: Command) {
        let count = self.held.entry(command).or_insert(0);
        *count += 1;
        if *count == 1 {
            self.queue.push(button(command, true));
        }
    }

    /// One finger fewer holds `command`: its release goes out with the
    /// last.
    fn release(&mut self, command: Command) {
        let Some(count) = self.held.get_mut(&command) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.held.remove(&command);
            self.queue.push(button(command, false));
        }
    }
}

const fn button(command: Command, pressed: bool) -> Input {
    Input::TouchControl(TouchControl::Button { command, pressed })
}

const fn stick_input(x: f32, y: f32) -> Input {
    Input::TouchControl(TouchControl::Stick { x, y })
}

#[cfg(test)]
mod tests;
