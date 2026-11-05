//! The keys screen, which Preferences' Key Settings opens: every command
//! listed under its context's heading with the key it is bound to,
//! recorded afresh, cleared or restored to the defaults.
//!
//! The original's Keys window (`Keys.nib`'s `ShortcutRecorder` fields on
//! four tabs) is replaced by one scrolling list, as the roadmap chose.
//! What its recorders do is followed, as rdm:phase/key-bindings/phase-4-keys-screen's
//! findings record them.
//!
//! # Rows
//!
//! Every [`Command`], in declaration order, under a heading for each
//! context ([`Context::label`]): flight first, in `Keys.nib`'s order, then
//! the other contexts; a context with no command (the system view's) has
//! no heading. Each command row is its label ([`Command::label`]) and one
//! cell a device family: the keyboard's today, its combo's name
//! ([`combo_name`]), or "Click to record shortcut" when it is unbound, as
//! an empty recorder reads (`-[SRRecorderCell drawWithFrame:inView:]`
//! @0x5d76). A gamepad's cell goes beside it (the key-bindings roadmap's
//! phase 12).
//!
//! # Fixed keys
//!
//! The screen reads raw keys, never a [`CommandReader`](crate::keymap::CommandReader),
//! so its own controls answer whatever the key map holds:
//!
//! - Up and Down move the selection, repeats included, passing over the
//!   headings.
//! - Return, the keypad's Enter and Space start recording on the selected
//!   row, as a click or Space starts a recorder (@0x53ee, @0x36ca-0x36e9),
//!   or press the focused button.
//! - Escape is Cancel.
//! - Tab moves the focus: the list, Restore Defaults, Cancel, OK, and
//!   round.
//! - Delete and Backspace clear the selected command's key.
//! - A click on a row selects it and, on its release over the row, starts
//!   recording.
//!
//! # Recording
//!
//! The row shows "Type shortcut" (@0x5d0a). While it records, every key
//! records, Escape, Backspace and Delete included, as the Keys window's
//! recorders set `escapeKeysRecord` (@0x30a4-0x30c7): the fixed keys
//! above act only outside a recording. A click anywhere but the
//! recording row cancels it, as the recorder ends when it loses focus
//! (@0x2ef2); so only Restore Defaults puts Escape back on Back once
//! it is recorded elsewhere. Repeats and fn record nothing.
//!
//! - **What binds** is the `recorded_modifiers` rule's ([`Recording`]):
//!   by default the key with its command's own default modifiers, and a
//!   modifier key on its press, as its left key; by the other reading the
//!   key with the modifiers held, and a modifier key let go with nothing
//!   pressed meanwhile.
//! - **The rebind rule** ([`Bindings::assign`]): a command holding the
//!   combo in a context active together with this one loses it, with
//!   any variant following it, and its row is marked until it is bound
//!   again or the defaults are restored. Commands of contexts never
//!   active together (a flight key and a spaceport key) keep it both. The
//!   original refuses a key another recorder holds instead
//!   (@0xd0162-0xd0b00); that is rdm:phase/front-end/phase-15-keys-window-left-outs.
//! - **Reserved combos**: the menu bar's key equivalents bind nothing,
//!   whether the key as pressed or the combo it would bind is one, so Q
//!   on Jettison Cargo, Command-Q by default, binds nothing. The recording
//!   goes on, as the original's does after its alert (@0x3265-0x3298).
//!
//! The Keys window plays no sound (the findings), and neither does this
//! screen.
//!
//! # Buttons
//!
//! Restore Defaults sets every binding to [`KeyMap::default`](crate::keymap::KeyMap).
//! OK closes the screen, and [`KeysScreen::take_change`] gives the edited
//! bindings once, if they differ from those it opened on; Cancel closes
//! it with nothing to take, as `cancel:` reverts the recorders (@0xcff7c).

mod edit;

use std::rc::Rc;
use std::time::Duration;

use nova_sim::Rulebook;

pub use edit::{KeyEdit, Recorded, Recording, left_key};

use super::button::{Button, ButtonSkin, ButtonStyle, ButtonTracker};
use super::dialog::outline;
use super::prefs::{BACKDROP, BORDER};
use super::row_list::{BarRoom, ListPress, RowList};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key, MouseButton};
use crate::key_names::{KeyNames, combo_name};
use crate::keymap::keyboard::{Sides, is_modifier};
use crate::keymap::{Bindings, Command, Context, KeyCombo, KeysDown};
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;

/// The screen's title.
pub const TITLE: &str = "Key Settings";
/// Where the title is drawn, from the screen's top left.
pub const TITLE_AT: Point = Point::new(12.0, 8.0);
/// The title's size.
pub const TITLE_SIZE: f32 = 12.0;
/// The list's box, from the screen's top left (left, top, right, bottom).
pub const LIST_AREA: (f32, f32, f32, f32) = (11.0, 30.0, 325.0, 238.0);
/// Where a row's key is drawn, from the row's left.
pub const KEY_COLUMN: f32 = 170.0;
/// Restore Defaults' rectangle, from the screen's top left.
pub const RESTORE_AREA: (f32, f32, f32, f32) = (11.0, 245.0, 131.0, 265.0);
/// Cancel's rectangle.
pub const CANCEL_AREA: (f32, f32, f32, f32) = (149.0, 245.0, 219.0, 265.0);
/// OK's rectangle, where the Preferences dialog's OK is.
pub const OK_AREA: (f32, f32, f32, f32) = (225.0, 245.0, 295.0, 265.0);
/// An unbound command's cell, as an empty recorder reads.
pub const UNBOUND: &str = "Click to record shortcut";
/// The recording row's cell.
pub const RECORDING: &str = "Type shortcut";
/// A command row's indent under its heading.
const INDENT: &str = "  ";

/// One row of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    /// A context's heading.
    Heading(Context),
    /// A command, with one cell a device family.
    Command(Command),
}

/// The rows: each context that has commands, its heading then its
/// commands, in declaration order.
#[must_use]
pub fn rows() -> Vec<Row> {
    Context::ALL
        .iter()
        .filter(|context| context.commands().next().is_some())
        .flat_map(|&context| {
            std::iter::once(Row::Heading(context)).chain(context.commands().map(Row::Command))
        })
        .collect()
}

/// A button of the screen's, in Tab order after the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysButton {
    /// Restore Defaults.
    RestoreDefaults,
    /// Cancel.
    Cancel,
    /// OK.
    Ok,
}

impl KeysButton {
    /// The buttons, left to right.
    pub const ALL: [Self; 3] = [Self::RestoreDefaults, Self::Cancel, Self::Ok];

    /// Its label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::RestoreDefaults => "Restore Defaults",
            Self::Cancel => "Cancel",
            Self::Ok => "OK",
        }
    }

    /// Its rectangle, from the screen's top left.
    const fn area(self) -> (f32, f32, f32, f32) {
        match self {
            Self::RestoreDefaults => RESTORE_AREA,
            Self::Cancel => CANCEL_AREA,
            Self::Ok => OK_AREA,
        }
    }

    /// Its place in [`KeysButton::ALL`].
    const fn index(self) -> usize {
        self as usize
    }
}

/// What has the keyboard focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysFocus {
    /// The list.
    List,
    /// A button.
    Button(KeysButton),
}

impl KeysFocus {
    /// The next in Tab order, round from OK to the list.
    const fn next(self) -> Self {
        match self {
            Self::List => Self::Button(KeysButton::RestoreDefaults),
            Self::Button(KeysButton::RestoreDefaults) => Self::Button(KeysButton::Cancel),
            Self::Button(KeysButton::Cancel) => Self::Button(KeysButton::Ok),
            Self::Button(KeysButton::Ok) => Self::List,
        }
    }
}

/// A recording under way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Capture {
    /// Its row.
    row: usize,
    /// Its command.
    command: Command,
    /// Under [`Recording::HeldModifiers`], a modifier key down with
    /// nothing pressed since, and the modifiers held as it went down: it
    /// records on its release.
    pending: Option<(Key, crate::keymap::Modifiers)>,
}

/// Where a pointer press went, so the rest of its click follows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Press {
    /// On the list: on its rows, or on its bar.
    List { rows: bool },
    /// On a button.
    Button(KeysButton),
}

/// The bounds `area`, from `origin`.
fn placed(origin: Point, (left, top, right, bottom): (f32, f32, f32, f32)) -> Bounds {
    Bounds {
        min: Point::new(origin.x + left, origin.y + top),
        max: Point::new(origin.x + right, origin.y + bottom),
    }
}

/// The keys screen (see the module docs).
#[derive(Clone)]
pub struct KeysScreen {
    bounds: Bounds,
    edit: KeyEdit,
    names: Rc<dyn KeyNames>,
    metrics: Rc<dyn TextMetrics>,
    style: ButtonStyle,
    rows: Vec<Row>,
    list: RowList,
    buttons: [Button; 3],
    trackers: [ButtonTracker; 3],
    focus: KeysFocus,
    keys: KeysDown,
    capture: Option<Capture>,
    press: Option<Press>,
    closed: bool,
    change: Option<Bindings>,
}

impl std::fmt::Debug for KeysScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeysScreen")
            .field("edit", &self.edit)
            .field("focus", &self.focus)
            .field("capture", &self.capture)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl KeysScreen {
    /// The screen over `bounds` (the Preferences dialog's), editing
    /// `bindings`, naming keys through `names`, binding none of
    /// `reserved` (the menu bar's key equivalents), recording as `rules`
    /// read `recorded_modifiers`, its buttons in `style` and its text
    /// measured by `metrics`.
    #[must_use]
    pub fn new(
        bounds: Bounds,
        bindings: Bindings,
        names: Rc<dyn KeyNames>,
        reserved: Vec<KeyCombo>,
        rules: &Rulebook,
        style: ButtonStyle,
        metrics: Rc<dyn TextMetrics>,
    ) -> Self {
        let rows = rows();
        let labels = rows
            .iter()
            .map(|row| match row {
                Row::Heading(context) => context.label().to_owned(),
                Row::Command(command) => format!("{INDENT}{}", command.label()),
            })
            .collect();
        let headings: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| matches!(row, Row::Heading(_)).then_some(index))
            .collect();
        let origin = bounds.min;
        let list = RowList::new(labels, placed(origin, LIST_AREA), &metrics, BarRoom::Inside)
            .with_column(KEY_COLUMN, Vec::new())
            .with_headings(&headings);
        let buttons = KeysButton::ALL
            .map(|button| Button::new(placed(origin, button.area()), button.label()));
        let mut screen = Self {
            bounds,
            edit: KeyEdit::new(bindings, reserved, Recording::from_rulebook(rules)),
            names,
            metrics,
            style,
            rows,
            list,
            buttons,
            trackers: [ButtonTracker::default(); 3],
            focus: KeysFocus::List,
            keys: KeysDown::default(),
            capture: None,
            press: None,
            closed: false,
            change: None,
        };
        screen.refresh();
        screen
    }

    /// The rows, in order.
    #[must_use]
    pub fn row_list(&self) -> &RowList {
        &self.list
    }

    /// Each row's command or heading.
    #[must_use]
    pub fn row_kinds(&self) -> &[Row] {
        &self.rows
    }

    /// The row of `command`.
    #[must_use]
    pub fn row_of(&self, command: Command) -> Option<usize> {
        self.rows
            .iter()
            .position(|&row| row == Row::Command(command))
    }

    /// The bindings as edited.
    #[must_use]
    pub fn draft(&self) -> &Bindings {
        self.edit.draft()
    }

    /// Whether a rebind left `command` unbound and it is still marked.
    #[must_use]
    pub fn is_marked(&self, command: Command) -> bool {
        self.edit.is_marked(command)
    }

    /// The command being recorded, if any.
    #[must_use]
    pub fn recording(&self) -> Option<Command> {
        self.capture.map(|capture| capture.command)
    }

    /// What has the keyboard focus.
    #[must_use]
    pub fn focus(&self) -> KeysFocus {
        self.focus
    }

    /// A button's rectangle on the screen.
    #[must_use]
    pub fn button_bounds(&self, button: KeysButton) -> Bounds {
        self.buttons[button.index()].rect
    }

    /// Whether OK or Cancel closed the screen.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// The bindings OK accepted, once, if they differ from those the
    /// screen opened on.
    pub fn take_change(&mut self) -> Option<Bindings> {
        self.change.take()
    }

    /// The keyboard cell of `command`'s row.
    fn keyboard_cell(&self, command: Command) -> String {
        self.edit.draft().keyboard.combo(command).map_or_else(
            || UNBOUND.to_owned(),
            |combo| combo_name(&*self.names, combo),
        )
    }

    /// Redraws every command row's cell and mark from the draft.
    fn refresh(&mut self) {
        for index in 0..self.rows.len() {
            if let Row::Command(command) = self.rows[index] {
                let cell = if self.capture.is_some_and(|capture| capture.row == index) {
                    RECORDING.to_owned()
                } else {
                    self.keyboard_cell(command)
                };
                self.list.set_cell(index, &cell);
                self.list.set_marked(index, self.edit.is_marked(command));
            }
        }
    }

    /// Starts recording on the selected row, if it is a command's.
    fn start_capture(&mut self) {
        let Some(row) = self.list.selected() else {
            return;
        };
        if let Some(&Row::Command(command)) = self.rows.get(row) {
            self.capture = Some(Capture {
                row,
                command,
                pending: None,
            });
            self.refresh();
        }
    }

    /// Ends the recording, binding nothing more.
    fn end_capture(&mut self) {
        self.capture = None;
        self.refresh();
    }

    /// Records `key`, pressed with `held`, on the recording's command: a
    /// bound key ends the recording, a reserved one leaves it going.
    fn bind(&mut self, key: Key, held: crate::keymap::Modifiers) {
        let Some(capture) = self.capture else {
            return;
        };
        match self.edit.record(capture.command, key, held) {
            Recorded::Bound(_) => self.end_capture(),
            Recorded::Reserved => {
                self.capture = Some(Capture {
                    pending: None,
                    ..capture
                });
            }
        }
    }

    /// A key while recording: `held` the modifiers held before it.
    fn capture_key(
        &mut self,
        key: Key,
        pressed: bool,
        repeat: bool,
        held: crate::keymap::Modifiers,
    ) {
        let Some(mut capture) = self.capture else {
            return;
        };
        if !pressed {
            if let Some((pending, then)) = capture.pending
                && pending == key
            {
                self.bind(key, then);
            }
            return;
        }
        if repeat || matches!(key, Key::Fn | Key::FnLock) {
            return;
        }
        if is_modifier(key) {
            match self.edit.recording() {
                Recording::CommandModifiers => self.bind(left_key(key), held),
                Recording::HeldModifiers => {
                    capture.pending = Some((key, held));
                    self.capture = Some(capture);
                }
            }
            return;
        }
        self.bind(key, held);
    }

    /// A key outside a recording: the fixed keys.
    fn fixed_key(&mut self, key: Key, repeat: bool) {
        match key {
            Key::ArrowUp | Key::ArrowDown => {
                if self.focus == KeysFocus::List {
                    self.list.step_wrapping(key == Key::ArrowDown);
                }
            }
            _ if repeat => {}
            Key::Enter | Key::NumpadEnter | Key::Space => match self.focus {
                KeysFocus::List => self.start_capture(),
                KeysFocus::Button(button) => self.press_button(button),
            },
            Key::Escape => self.press_button(KeysButton::Cancel),
            Key::Tab => self.focus = self.focus.next(),
            Key::Backspace | Key::Delete => {
                if self.focus == KeysFocus::List
                    && let Some(&Row::Command(command)) =
                        self.list.selected().and_then(|row| self.rows.get(row))
                {
                    self.edit.clear(command);
                    self.refresh();
                }
            }
            _ => {}
        }
    }

    /// Does what `button` does.
    fn press_button(&mut self, button: KeysButton) {
        match button {
            KeysButton::RestoreDefaults => {
                self.edit.restore_defaults();
                self.refresh();
            }
            KeysButton::Cancel => {
                self.closed = true;
                self.change = None;
            }
            KeysButton::Ok => {
                self.closed = true;
                self.change = self.edit.change();
            }
        }
    }

    /// A pointer input: a press goes to the button or the list under it,
    /// and the rest of its click to the same.
    fn pointer(&mut self, input: &Input) {
        if let Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        } = *input
        {
            if let Some(capture) = self.capture {
                let on_it = self
                    .list
                    .row_bounds(capture.row)
                    .is_some_and(|row| row.contains_half_open(at));
                if on_it {
                    return;
                }
                self.end_capture();
            }
            self.press = if let Some(&button) = KeysButton::ALL
                .iter()
                .find(|button| self.buttons[button.index()].contains(at))
            {
                self.trackers[button.index()].input(&self.buttons[button.index()], input);
                Some(Press::Button(button))
            } else if self.list.input(input) == ListPress::Tracking {
                self.focus = KeysFocus::List;
                let on_bar = self
                    .list
                    .bar()
                    .is_some_and(|bar| bar.contains_half_open(at));
                Some(Press::List { rows: !on_bar })
            } else {
                None
            };
            return;
        }
        let release = match *input {
            Input::PointerButton {
                button: MouseButton::Left,
                pressed: false,
                at,
            } => Some(at),
            _ => None,
        };
        match self.press {
            Some(Press::Button(button)) => {
                let index = button.index();
                if self.trackers[index].input(&self.buttons[index], input) {
                    self.press_button(button);
                }
                if release.is_some() {
                    self.press = None;
                }
            }
            Some(Press::List { rows }) => {
                if let ListPress::Released { .. } = self.list.input(input) {
                    self.press = None;
                    let on_selected = release.is_some_and(|at| {
                        self.list
                            .selected()
                            .and_then(|row| self.list.row_bounds(row))
                            .is_some_and(|row| row.contains_half_open(at))
                    });
                    if rows && on_selected {
                        self.start_capture();
                    }
                }
            }
            None => {}
        }
    }
}

impl Screen for KeysScreen {
    /// The fixed keys and the mouse, or a recording (see the module docs).
    /// Nothing once closed. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.closed {
            return ScreenAction::None;
        }
        match *input {
            Input::Key {
                key,
                pressed,
                repeat,
            } => {
                let held = self.keys.modifiers(Sides::Either);
                self.keys.follow(input);
                if self.capture.is_some() {
                    self.capture_key(key, pressed, repeat, held);
                } else if pressed {
                    self.fixed_key(key, repeat);
                }
            }
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
            Input::Text(_)
            | Input::PointerLeft
            | Input::PointerMovedOutside(_)
            | Input::Scroll { .. } => {}
        }
        ScreenAction::None
    }

    /// The list's held press scrolls on.
    fn tick(&mut self, dt: Duration) {
        self.list.tick(dt);
    }

    /// The backdrop and its outline, the title, the list, then the
    /// buttons; the focus is outlined.
    fn draw(&self, list: &mut DrawList) {
        fill_rect(list, self.bounds, BACKDROP);
        outline(list, self.bounds, BORDER);
        list.text(
            TITLE,
            Point::new(
                self.bounds.min.x + TITLE_AT.x,
                self.bounds.min.y + TITLE_AT.y,
            ),
            TITLE_SIZE,
            None,
            Color::WHITE,
        );
        self.list.draw(list);
        for button in KeysButton::ALL {
            let index = button.index();
            self.buttons[index].draw(
                self.trackers[index].pressed(),
                &ButtonSkin::NOVA,
                &self.style,
                &self.metrics,
                list,
            );
        }
        let focused = match self.focus {
            KeysFocus::List => self.list.view(),
            KeysFocus::Button(button) => self.buttons[button.index()].rect,
        };
        outline(list, focused, self.style.up);
    }

    /// Lets go of the press held, anywhere, activating nothing.
    fn cancel_pointer(&mut self) {
        self.list.cancel_pointer();
        for tracker in &mut self.trackers {
            tracker.cancel_pointer();
        }
        self.press = None;
    }

    /// Lets go of the keys held, as when the window loses the keyboard.
    fn release_keys(&mut self) {
        self.keys.let_go();
    }
}

#[cfg(test)]
mod tests;
