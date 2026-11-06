//! The on-screen keyboard's state ([`EntryKeyboard`]): when it shows,
//! what it takes, and what each key gives back.
//!
//! - **Shown** while entry is active (its host has a focused field and
//!   isn't covered) and the game enters the text
//!   ([`EntryMode::OnScreen`]): with touch at once, and with a pad once
//!   South pressed the field ([`EntryKeyboard::field_pressed`]), until
//!   East or Done lowers it. Under a pad a dialog opens asking for
//!   nothing, so South at once still accepts the offered value.
//! - **Asks** for the system's entry instead in [`EntryMode::System`]
//!   (the keyboard and the mouse, touch with a soft keyboard).
//! - **Takes**, while it shows, a pointer press on a key (it acts on the
//!   release inside, [`Press`]), and under the pad the d-pad (its focus),
//!   South (types the focused key on the press) and East (lowers it).
//!   Every other input passes to the host: the shoulders (so Tab still
//!   switches fields), the keys and typed text. A pad button whose press
//!   it took has its release taken too. A held d-pad direction's repeats
//!   move the focus again, as its press did; a repeat never types a key.
//! - **Gives back** what a key types, for the host to run through its
//!   own input as real keys ([`super::intercept`]): a character
//!   ([`Input::Text`]), Space a space, Delete a Backspace press and
//!   release, and Done a Return press and release, which presses the
//!   dialog's default. Clear gives back nothing: it asks its host to
//!   empty the field ([`EntryKeyboard::take_clear`]), as Backspaces
//!   can't once a refusal selected the field's first characters (the
//!   first deletes the selection and leaves the caret at the start).
//! - **Shift** is on as a field takes the focus and turns off after one
//!   letter, as a phone's keyboard capitalises a name.
//!
//! It is silent, as the touch aids are.

use crate::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::{Device, Input, Key, PadButton, Surroundings};
use crate::keymap::{Command, CommandReader, UiCommand, contexts};
use crate::text_entry::EntryMode;
use crate::ui::pad_focus::{FocusMove, FocusState, PadRead, Target, draw_highlight};
use crate::ui::touch_aids::look::{FILL, HIGHLIGHT, draw_square};
use crate::ui::touch_aids::press::{Press, Step};
use crate::view_shape::CLASSIC_SIZE;

use super::layout::{INITIAL, KeyId, Layout};

/// The panel drawn behind the keys.
pub const PANEL: Color = Color::rgba(16, 20, 32, 200);

/// The on-screen keyboard a text-entry host keeps: see the module docs.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // Each a separate state it keeps.
pub struct EntryKeyboard {
    /// The surroundings the host was told last.
    told: Surroundings,
    /// Whether whoever holds the host covers it.
    covered: bool,
    /// The pad target (its host's item) the pad's South raised it on, if
    /// it did: it shows while the pad's focus stays there.
    pad_raised: Option<usize>,
    /// Whether the next letter is a capital.
    shift: bool,
    /// How many times a field of the host took the focus.
    focused: u32,
    /// A pointer press on a key.
    press: Press,
    /// The pad's focus among the keys.
    focus: FocusState<KeyId>,
    /// The pad's commands, read as a dialog reads them.
    keys: CommandReader,
    /// The pad buttons whose press it took, whose release it takes too.
    taken: Vec<PadButton>,
    /// Whether Clear was typed since its host last took it.
    clear: bool,
}

impl Default for EntryKeyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl EntryKeyboard {
    /// Hidden, told nothing, no field focused yet. It reads the pad over
    /// the default bindings: the shared UI context's pad rows, which are
    /// all it reads, are fixed (`PadMap::is_fixed`), so a host's bindings
    /// read them the same.
    #[must_use]
    pub fn new() -> Self {
        Self {
            told: Surroundings::default(),
            covered: false,
            pad_raised: None,
            shift: true,
            focused: 0,
            press: Press::default(),
            focus: FocusState::default(),
            keys: CommandReader::new(contexts::DIALOG),
            taken: Vec::new(),
            clear: false,
        }
    }

    /// The surroundings it was told last.
    #[must_use]
    pub const fn told(&self) -> Surroundings {
        self.told
    }

    /// Who enters the text for the device used last.
    #[must_use]
    pub const fn mode(&self) -> EntryMode {
        EntryMode::of(self.told.device, self.told.soft_keyboard)
    }

    /// Whether it shows, with entry `active` in its host (a field
    /// focused).
    #[must_use]
    pub const fn shows(&self, active: bool) -> bool {
        active
            && !self.covered
            && matches!(self.mode(), EntryMode::OnScreen)
            && (matches!(self.told.device, Device::Touch) || self.pad_raised.is_some())
    }

    /// Whether its host asks for the system's entry, with entry `active`.
    #[must_use]
    pub const fn asks(&self, active: bool) -> bool {
        active && !self.covered && matches!(self.mode(), EntryMode::System)
    }

    /// A field of its host took the focus (the dialog opening, Tab): the
    /// count goes up, so the system's entry starts again, and Shift is on
    /// for the field's first letter.
    pub const fn focus_field(&mut self) {
        self.focused = self.focused.wrapping_add(1);
        self.shift = true;
    }

    /// A click, a tap or the pad's South pressed a field of its host, with
    /// the pad's focus on `pad_target` (its host's item, the field's under
    /// the pad): it takes the focus ([`EntryKeyboard::focus_field`]), and
    /// under the pad the keyboard is raised there.
    pub fn field_pressed(&mut self, pad_target: Option<usize>) {
        self.focus_field();
        if let (Device::Gamepad(_), Some(target)) = (self.told.device, pad_target) {
            self.pad_raised = Some(target);
            if self.focus.get().is_none() {
                self.focus.set_by_pad(INITIAL);
            }
        }
    }

    /// The pad target the pad raised it on, if it did.
    #[must_use]
    pub const fn raised_on(&self) -> Option<usize> {
        self.pad_raised
    }

    /// Lowers it if the pad raised it: the pad's focus left the field.
    pub const fn lower(&mut self) {
        self.pad_raised = None;
    }

    /// How many times a field of its host took the focus.
    #[must_use]
    pub const fn focused(&self) -> u32 {
        self.focused
    }

    /// The key the pad's focus is on, if it has one.
    #[must_use]
    pub const fn pad_focus(&self) -> Option<KeyId> {
        self.focus.get()
    }

    /// Where its keys go over its host's dialog at `dialog`, in the
    /// classic canvas every text-entry host draws in.
    #[must_use]
    pub fn layout(&self, dialog: Bounds) -> Layout {
        #[allow(clippy::cast_precision_loss)] // The canvas: 1024 × 768.
        let canvas = Bounds::at(
            Point::new(0.0, 0.0),
            CLASSIC_SIZE.0 as f32,
            CLASSIC_SIZE.1 as f32,
        );
        Layout::new(&self.told, canvas, dialog)
    }

    /// Follows `input` ahead of its host, with `shown` its layout while it
    /// shows (none while hidden): the inputs the host runs instead, in
    /// order (none for an input it took). See the module docs.
    pub fn input(&mut self, input: &Input, shown: Option<&Layout>) -> Vec<Input> {
        if let Input::Surroundings(told) = *input {
            self.keys.follow(input);
            self.told = told;
            if !matches!(told.device, Device::Gamepad(_)) {
                self.pad_raised = None;
            }
            self.focus.surroundings(told.device, Some(INITIAL), false);
            return vec![*input];
        }
        if let Input::PadButton {
            button,
            pressed: false,
            ..
        } = *input
            && let Some(at) = self.taken.iter().position(|&taken| taken == button)
        {
            self.taken.remove(at);
            self.keys.follow(input);
            return Vec::new();
        }
        let Some(layout) = shown else {
            if let Input::PadButton { .. } = input {
                self.keys.follow(input);
            }
            self.press.cancel();
            return vec![*input];
        };
        match *input {
            Input::PadButton {
                button,
                pressed,
                repeat,
            } => {
                let read = self.keys.follow(input);
                if !self.focus.pad_shown() {
                    return vec![*input];
                }
                let given =
                    match read.map(|command| (command.command, self.focus.read_pad(command))) {
                        Some((_, PadRead::Move(FocusMove::Toward(direction)))) => {
                            let targets: Vec<_> = layout
                                .keys()
                                .iter()
                                .map(|&(id, rect)| Target::at(id, rect))
                                .collect();
                            self.focus
                                .move_by_pad(&targets, INITIAL, FocusMove::Toward(direction));
                            Vec::new()
                        }
                        Some((_, PadRead::Press)) => {
                            let key = self.focus.get().unwrap_or(INITIAL);
                            self.type_key(key)
                        }
                        Some((Command::Ui(UiCommand::Back), PadRead::Other)) => {
                            self.pad_raised = None;
                            Vec::new()
                        }
                        _ => return vec![*input],
                    };
                // A repeat's button is taken already, by its press: one
                // release ends it.
                if pressed && !repeat {
                    self.taken.push(button);
                }
                given
            }
            _ => match self.press.input(input, &layout.rects()) {
                Step::Passed => vec![*input],
                Step::Taken => Vec::new(),
                Step::Acted(index) => {
                    let key = layout.keys()[index].0;
                    self.type_key(key)
                }
            },
        }
    }

    /// What `key` gives back.
    fn type_key(&mut self, key: KeyId) -> Vec<Input> {
        let tap = |key| {
            [true, false].map(|pressed| Input::Key {
                key,
                pressed,
                repeat: false,
            })
        };
        match key {
            KeyId::Char(c) if c.is_alphabetic() && self.shift => {
                self.shift = false;
                c.to_uppercase().map(Input::Text).collect()
            }
            KeyId::Char(c) => vec![Input::Text(c)],
            KeyId::Shift => {
                self.shift = !self.shift;
                Vec::new()
            }
            KeyId::Space => vec![Input::Text(' ')],
            KeyId::Delete => tap(Key::Backspace).to_vec(),
            KeyId::Clear => {
                self.clear = true;
                Vec::new()
            }
            KeyId::Done => {
                self.pad_raised = None;
                tap(Key::Enter).to_vec()
            }
        }
    }

    /// Whether Clear was typed since the last call: its host then empties
    /// the focused field.
    pub const fn take_clear(&mut self) -> bool {
        std::mem::replace(&mut self.clear, false)
    }

    /// Its host's pointer was let go of: a press on a key is forgotten.
    pub fn cancel(&mut self) {
        self.press.cancel();
    }

    /// Whether whoever holds its host covers it. Covered, it hides, its
    /// host asks nothing, and a press on a key is forgotten.
    pub fn set_covered(&mut self, covered: bool) {
        if covered {
            self.press.cancel();
        }
        self.covered = covered;
    }

    /// Whether whoever holds its host covers it.
    #[must_use]
    pub const fn covered(&self) -> bool {
        self.covered
    }

    /// Draws it at `layout`: a panel behind the keys, each key as a touch
    /// control is drawn, Shift and a key held down highlighted, and the
    /// pad's highlight round the key the pad focused, under the pad.
    pub fn draw(&self, layout: &Layout, list: &mut DrawList) {
        let ppp = self.told.points_per_pixel;
        fill_rect(list, layout.bounds().grown(2.0 / ppp), PANEL);
        let held = self.press.held();
        for (index, &(id, rect)) in layout.keys().iter().enumerate() {
            let down = held == Some(index) || (id == KeyId::Shift && self.shift);
            let fill = if down { HIGHLIGHT } else { FILL };
            draw_square(list, rect, &id.label(self.shift), fill, ppp);
        }
        if let Some(focused) = self.focus.pad_highlight()
            && let Some(rect) = layout.key(focused)
        {
            draw_highlight(list, rect, HIGHLIGHT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{ControllerFamily, MouseButton};
    use crate::ui::pad_focus::every_screen::{PAD, pad, told};

    /// The field's item in its host.
    const FIELD: usize = 3;

    /// A dialog in the middle of the canvas.
    const DIALOG: Bounds = Bounds {
        min: Point::new(350.0, 280.0),
        max: Point::new(676.0, 490.0),
    };

    fn soft(device: Device) -> Input {
        Input::Surroundings(Surroundings {
            soft_keyboard: true,
            ..Surroundings::of(device)
        })
    }

    fn keyboard_told(input: &Input) -> EntryKeyboard {
        let mut keyboard = EntryKeyboard::new();
        keyboard.input(input, None);
        keyboard.focus_field();
        keyboard
    }

    /// A keyboard told the pad, raised by South on the field.
    fn raised() -> EntryKeyboard {
        let mut keyboard = keyboard_told(&told(PAD));
        keyboard.field_pressed(Some(FIELD));
        keyboard
    }

    /// `keyboard` takes `input` as it shows (or not).
    fn send(keyboard: &mut EntryKeyboard, input: &Input) -> Vec<Input> {
        let layout = keyboard.layout(DIALOG);
        let shown = keyboard.shows(true).then_some(&layout);
        keyboard.input(input, shown)
    }

    fn tap(keyboard: &mut EntryKeyboard, button: PadButton) -> Vec<Input> {
        let mut given = send(keyboard, &pad(button, true));
        given.extend(send(keyboard, &pad(button, false)));
        given
    }

    fn click(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    /// A tap on `key`, as the touch router delivers it.
    fn tap_key(keyboard: &mut EntryKeyboard, key: KeyId) -> Vec<Input> {
        let at = keyboard.layout(DIALOG).key(key).expect("a key").center();
        let mut given = send(keyboard, &click(true, at));
        given.extend(send(keyboard, &click(false, at)));
        given
    }

    fn touch() -> EntryKeyboard {
        keyboard_told(&told(Device::Touch))
    }

    fn key(key: Key, pressed: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat: false,
        }
    }

    #[test]
    fn hidden_unless_entry_is_active() {
        let keyboard = touch();
        assert!(keyboard.shows(true));
        assert!(!keyboard.shows(false), "no field focused");
        assert!(!raised().shows(false));
        assert!(!keyboard_told(&told(Device::Keyboard)).asks(false));
    }

    #[test]
    fn touch_without_a_soft_keyboard_shows_it_at_once() {
        let keyboard = touch();
        assert!(keyboard.shows(true));
        assert!(!keyboard.asks(true), "the game enters the text");
    }

    #[test]
    fn touch_with_a_soft_keyboard_never_shows_it() {
        let mut keyboard = keyboard_told(&soft(Device::Touch));
        keyboard.field_pressed(Some(FIELD));
        assert!(!keyboard.shows(true));
        assert!(keyboard.asks(true), "the system's keyboard instead");
    }

    #[test]
    fn a_pad_shows_it_only_once_raised() {
        for soft_keyboard in [false, true] {
            let mut keyboard = keyboard_told(&Input::Surroundings(Surroundings {
                soft_keyboard,
                ..Surroundings::of(PAD)
            }));
            assert!(!keyboard.shows(true), "not at open");
            assert!(!keyboard.asks(true), "nor the system's");
            keyboard.focus_field();
            assert!(!keyboard.shows(true), "a focus alone raises nothing");
            keyboard.field_pressed(Some(FIELD));
            assert!(keyboard.shows(true), "raised by South on the field");
        }
    }

    #[test]
    fn the_keyboard_and_mouse_never_show_it() {
        for device in [Device::Keyboard, Device::Mouse] {
            for input in [told(device), soft(device)] {
                let mut keyboard = keyboard_told(&input);
                keyboard.field_pressed(Some(FIELD));
                assert!(!keyboard.shows(true), "{device:?}");
                assert!(keyboard.asks(true), "{device:?}");
            }
        }
    }

    #[test]
    fn leaving_the_pad_lowers_it() {
        let mut keyboard = raised();
        send(&mut keyboard, &told(Device::Keyboard));
        assert!(!keyboard.shows(true));
        send(&mut keyboard, &told(PAD));
        assert!(!keyboard.shows(true), "until South raises it again");
    }

    #[test]
    fn east_lowers_it_under_a_pad_and_is_taken() {
        let mut keyboard = raised();
        assert_eq!(tap(&mut keyboard, PadButton::East), []);
        assert!(!keyboard.shows(true));
        assert_eq!(
            tap(&mut keyboard, PadButton::East),
            [pad(PadButton::East, true), pad(PadButton::East, false)],
            "lowered, East is the host's again"
        );
    }

    #[test]
    fn a_tap_on_a_key_types_it_on_the_release_inside() {
        let mut keyboard = touch();
        let at = keyboard
            .layout(DIALOG)
            .key(KeyId::Char('7'))
            .expect("7")
            .center();
        assert_eq!(send(&mut keyboard, &click(true, at)), []);
        assert_eq!(send(&mut keyboard, &Input::PointerMoved(at)), []);
        assert_eq!(send(&mut keyboard, &click(false, at)), [Input::Text('7')]);
    }

    #[test]
    fn a_press_that_slides_off_types_nothing() {
        let mut keyboard = touch();
        let layout = keyboard.layout(DIALOG);
        let at = layout.key(KeyId::Char('7')).expect("7").center();
        let off = Point::new(5.0, 5.0);
        send(&mut keyboard, &click(true, at));
        assert_eq!(send(&mut keyboard, &Input::PointerMoved(off)), []);
        assert_eq!(send(&mut keyboard, &click(false, off)), []);
        assert_eq!(
            send(&mut keyboard, &click(true, off)),
            [click(true, off)],
            "a press off the keys is the host's"
        );
    }

    #[test]
    fn the_dpad_moves_its_focus_and_south_types_on_the_press() {
        let mut keyboard = raised();
        assert_eq!(tap(&mut keyboard, PadButton::DPadRight), []);
        assert_eq!(tap(&mut keyboard, PadButton::DPadDown), []);
        assert_eq!(
            send(&mut keyboard, &pad(PadButton::South, true)),
            [Input::Text('W')],
            "from 1, right to 2, down to W, the first letter a capital"
        );
        assert_eq!(send(&mut keyboard, &pad(PadButton::South, false)), []);
        assert_eq!(tap(&mut keyboard, PadButton::South), [Input::Text('w')]);
    }

    #[test]
    fn shift_capitalises_one_letter() {
        let mut keyboard = touch();
        assert_eq!(tap_key(&mut keyboard, KeyId::Char('5')), [Input::Text('5')]);
        assert_eq!(tap_key(&mut keyboard, KeyId::Char('z')), [Input::Text('Z')]);
        assert_eq!(tap_key(&mut keyboard, KeyId::Char('o')), [Input::Text('o')]);
        assert_eq!(tap_key(&mut keyboard, KeyId::Shift), []);
        assert_eq!(tap_key(&mut keyboard, KeyId::Char('e')), [Input::Text('E')]);
        tap_key(&mut keyboard, KeyId::Shift);
        tap_key(&mut keyboard, KeyId::Shift);
        assert_eq!(
            tap_key(&mut keyboard, KeyId::Char('e')),
            [Input::Text('e')],
            "off again"
        );
        keyboard.focus_field();
        assert_eq!(
            tap_key(&mut keyboard, KeyId::Char('n')),
            [Input::Text('N')],
            "a new field"
        );
    }

    #[test]
    fn delete_gives_a_backspace_press_and_release() {
        let mut keyboard = touch();
        assert_eq!(
            tap_key(&mut keyboard, KeyId::Delete),
            [key(Key::Backspace, true), key(Key::Backspace, false)]
        );
    }

    #[test]
    fn clear_gives_nothing_back_and_asks_its_host_to_empty_the_field() {
        let mut keyboard = touch();
        assert!(!keyboard.take_clear());
        assert_eq!(tap_key(&mut keyboard, KeyId::Clear), []);
        assert!(keyboard.take_clear());
        assert!(!keyboard.take_clear(), "taken once");
        tap_key(&mut keyboard, KeyId::Delete);
        assert!(!keyboard.take_clear(), "Delete is no Clear");
    }

    #[test]
    fn done_gives_return() {
        let mut keyboard = touch();
        assert_eq!(
            tap_key(&mut keyboard, KeyId::Done),
            [key(Key::Enter, true), key(Key::Enter, false)]
        );
        assert!(keyboard.shows(true), "touch keeps it while entry lasts");
        let mut padded = raised();
        for _ in 0..4 {
            tap(&mut padded, PadButton::DPadDown);
        }
        assert_eq!(padded.focus.get(), Some(KeyId::Space), "the bottom row");
        for _ in 0..3 {
            tap(&mut padded, PadButton::DPadRight);
        }
        assert_eq!(padded.focus.get(), Some(KeyId::Done));
        assert_eq!(
            send(&mut padded, &pad(PadButton::South, true)),
            [key(Key::Enter, true), key(Key::Enter, false)]
        );
        assert!(!padded.shows(true), "lowered under the pad");
        assert_eq!(
            send(&mut padded, &pad(PadButton::South, false)),
            [],
            "its release taken"
        );
    }

    #[test]
    fn space_types_a_space() {
        let mut keyboard = touch();
        assert_eq!(tap_key(&mut keyboard, KeyId::Space), [Input::Text(' ')]);
    }

    #[test]
    fn the_shoulders_and_other_pad_buttons_pass_through() {
        let mut keyboard = raised();
        for button in [
            PadButton::RightShoulder,
            PadButton::LeftShoulder,
            PadButton::Start,
            PadButton::LeftStickDown,
        ] {
            assert_eq!(
                tap(&mut keyboard, button),
                [pad(button, true), pad(button, false)],
                "{button:?}"
            );
        }
        assert!(keyboard.shows(true));
    }

    #[test]
    fn hardware_keys_and_typed_text_pass_through() {
        for mut keyboard in [touch(), raised()] {
            for input in [
                key(Key::KeyA, true),
                Input::Text('a'),
                key(Key::Backspace, true),
                key(Key::Enter, true),
                key(Key::Tab, false),
            ] {
                assert_eq!(send(&mut keyboard, &input), [input]);
            }
        }
    }

    #[test]
    fn under_touch_the_pads_presses_pass_to_the_host() {
        // The pad's focus moves and South act only once the keyboard is
        // told the pad (phase 8's "Nothing hidden acts"): under touch they
        // are the host's, which holds them back too.
        let mut keyboard = touch();
        for button in [PadButton::South, PadButton::DPadDown, PadButton::East] {
            assert_eq!(
                tap(&mut keyboard, button),
                [pad(button, true), pad(button, false)],
                "{button:?}"
            );
        }
        assert!(keyboard.shows(true));
        let mut drawn = DrawList::new();
        keyboard.draw(&keyboard.layout(DIALOG), &mut drawn);
        let mut fresh = DrawList::new();
        touch().draw(&touch().layout(DIALOG), &mut fresh);
        assert_eq!(drawn, fresh, "nothing moved");
    }

    #[test]
    fn a_hidden_keyboard_passes_everything() {
        let mut keyboard = keyboard_told(&told(PAD));
        for input in [
            pad(PadButton::South, true),
            pad(PadButton::South, false),
            pad(PadButton::DPadDown, true),
            click(true, Point::new(500.0, 700.0)),
        ] {
            assert_eq!(send(&mut keyboard, &input), [input]);
        }
    }

    fn highlighted(keyboard: &EntryKeyboard, key: KeyId) -> bool {
        let layout = keyboard.layout(DIALOG);
        let mut list = DrawList::new();
        keyboard.draw(&layout, &mut list);
        let drawn: Vec<_> = list.iter().cloned().collect();
        crate::ui::pad_focus::every_screen::highlights(&drawn, layout.key(key).expect("a key"))
    }

    #[test]
    fn its_highlight_shows_only_under_the_pad() {
        let keyboard = raised();
        assert!(highlighted(&keyboard, INITIAL));
        assert!(!highlighted(&touch(), INITIAL));
        let mut moved = raised();
        tap(&mut moved, PadButton::DPadRight);
        assert!(highlighted(&moved, KeyId::Char('2')));
        assert!(!highlighted(&moved, INITIAL));
    }

    #[test]
    fn covered_it_hides_and_lets_go_of_a_press() {
        let mut keyboard = touch();
        assert!(!keyboard.covered());
        let at = keyboard
            .layout(DIALOG)
            .key(KeyId::Char('7'))
            .expect("7")
            .center();
        send(&mut keyboard, &click(true, at));
        keyboard.set_covered(true);
        assert!(keyboard.covered());
        assert!(!keyboard.shows(true));
        assert!(!keyboard.asks(true));
        keyboard.set_covered(false);
        assert!(!keyboard.covered());
        assert!(keyboard.shows(true));
        assert_eq!(
            send(&mut keyboard, &click(false, at)),
            [click(false, at)],
            "the press was forgotten"
        );
        let mut asking = keyboard_told(&told(Device::Mouse));
        asking.set_covered(true);
        assert!(!asking.asks(true));
    }

    #[test]
    fn cancel_forgets_a_press() {
        let mut keyboard = touch();
        let at = keyboard
            .layout(DIALOG)
            .key(KeyId::Char('7'))
            .expect("7")
            .center();
        send(&mut keyboard, &click(true, at));
        keyboard.cancel();
        assert_eq!(send(&mut keyboard, &click(false, at)), [click(false, at)]);
    }

    #[test]
    fn each_focus_counts() {
        let mut keyboard = EntryKeyboard::new();
        assert_eq!(keyboard.focused(), 0);
        keyboard.focus_field();
        keyboard.field_pressed(Some(FIELD));
        assert_eq!(keyboard.focused(), 2);
    }

    #[test]
    fn a_held_key_and_shift_are_drawn_lit() {
        let mut keyboard = touch();
        let layout = keyboard.layout(DIALOG);
        let drawn = |keyboard: &EntryKeyboard| {
            let mut list = DrawList::new();
            keyboard.draw(&layout, &mut list);
            list
        };
        let fill_of = |list: &DrawList, rect: Bounds| {
            [FILL, HIGHLIGHT].into_iter().find(|&color| {
                list.iter()
                    .any(|command| *command == crate::draw::fill_command(rect, color))
            })
        };
        let seven = layout.key(KeyId::Char('7')).expect("7");
        let shift = layout.key(KeyId::Shift).expect("Shift");
        assert_eq!(fill_of(&drawn(&keyboard), seven), Some(FILL));
        assert_eq!(
            fill_of(&drawn(&keyboard), shift),
            Some(HIGHLIGHT),
            "on at first"
        );
        send(&mut keyboard, &click(true, seven.center()));
        assert_eq!(fill_of(&drawn(&keyboard), seven), Some(HIGHLIGHT));
        send(&mut keyboard, &click(false, seven.center()));
        tap_key(&mut keyboard, KeyId::Shift);
        assert_eq!(fill_of(&drawn(&keyboard), shift), Some(FILL));
    }

    #[test]
    fn its_panel_reaches_two_points_past_the_keys() {
        let mut keyboard = EntryKeyboard::new();
        keyboard.input(
            &Input::Surroundings(Surroundings {
                points_per_pixel: 2.0,
                ..Surroundings::of(Device::Touch)
            }),
            None,
        );
        let layout = keyboard.layout(DIALOG);
        let mut list = DrawList::new();
        keyboard.draw(&layout, &mut list);
        assert_eq!(
            list.iter().next(),
            Some(&crate::draw::fill_command(
                layout.bounds().grown(1.0),
                PANEL
            )),
            "two points at two points a unit: one unit"
        );
    }

    #[test]
    fn a_pad_family_change_keeps_it_raised() {
        let mut keyboard = raised();
        send(
            &mut keyboard,
            &told(Device::Gamepad(ControllerFamily::Xbox)),
        );
        assert!(keyboard.shows(true));
    }
}
