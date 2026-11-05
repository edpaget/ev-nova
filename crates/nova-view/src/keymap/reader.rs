//! What a screen holds to read its commands: a [`CommandReader`] over the
//! app's one [`SharedBindings`].

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use super::bindings::{Bindings, Devices};
use super::command::{Command, Context};
use super::contexts::ScreenContexts;
use super::keyboard::{self, Modifiers};
use crate::input::Input;

/// The app's bindings, shared by every screen: a cheap clone over one
/// table, which the app edge refills (phase 3 at launch, phase 4 on the
/// keys screen's OK) and every running screen then reads on its next
/// input.
#[derive(Clone, Debug, Default)]
pub struct SharedBindings(Rc<RefCell<Bindings>>);

impl SharedBindings {
    /// A handle over `bindings`.
    #[must_use]
    pub fn new(bindings: Bindings) -> Self {
        Self(Rc::new(RefCell::new(bindings)))
    }

    /// Replaces the bindings every clone reads.
    pub fn set(&self, bindings: Bindings) {
        *self.0.borrow_mut() = bindings;
    }

    /// A copy of the bindings as they are now.
    #[must_use]
    pub fn get(&self) -> Bindings {
        self.0.borrow().clone()
    }

    /// Reads the bindings as they are now.
    pub fn with<R>(&self, read: impl FnOnce(&Bindings) -> R) -> R {
        read(&self.0.borrow())
    }
}

/// One command an input pressed, repeated or let go of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandInput {
    /// The command.
    pub command: Command,
    /// Down (`true`) or up.
    pub pressed: bool,
    /// Whether the press is the OS repeating a held key; never set on a
    /// release.
    pub repeat: bool,
}

/// A screen's reader of its commands: it follows every input, says what
/// each press, repeat and release meant, and keeps the commands held.
///
/// The held set is recomputed whenever a key or a modifier changes, so a
/// modifier pressed after its key still meets the combo (Command pressed
/// after X ejects) and one let go of breaks it. Only a key going down
/// presses a command, though: a modifier changing presses nothing, as the
/// original acts on each key's own press.
#[derive(Clone, Debug)]
pub struct CommandReader {
    /// The app's bindings.
    bindings: SharedBindings,
    /// The screen's contexts.
    screen: ScreenContexts,
    /// The conditional contexts switched on.
    switched_on: BTreeSet<Context>,
    /// What the devices hold down.
    devices: Devices,
    /// The commands held.
    held: BTreeSet<Command>,
}

impl CommandReader {
    /// A reader of `screen`'s contexts over the default bindings, until
    /// [`CommandReader::with_bindings`] gives it the app's.
    #[must_use]
    pub fn new(screen: ScreenContexts) -> Self {
        Self {
            bindings: SharedBindings::default(),
            screen,
            switched_on: BTreeSet::new(),
            devices: Devices::default(),
            held: BTreeSet::new(),
        }
    }

    /// The reader reading `bindings`.
    #[must_use]
    pub fn with_bindings(mut self, bindings: SharedBindings) -> Self {
        self.set_bindings(bindings);
        self
    }

    /// Reads `bindings` from the next input on.
    pub fn set_bindings(&mut self, bindings: SharedBindings) {
        self.bindings = bindings;
        self.recompute();
    }

    /// The bindings it reads.
    #[must_use]
    pub fn bindings(&self) -> &SharedBindings {
        &self.bindings
    }

    /// Switches a conditional context of the screen's list on or off.
    pub fn set_active(&mut self, context: Context, on: bool) {
        if on {
            self.switched_on.insert(context);
        } else {
            self.switched_on.remove(&context);
        }
        self.recompute();
    }

    /// The contexts active now, in the screen's order.
    #[must_use]
    pub fn active(&self) -> Vec<Context> {
        self.screen
            .entries
            .iter()
            .filter(|entry| !entry.conditional || self.switched_on.contains(&entry.context))
            .map(|entry| entry.context)
            .collect()
    }

    /// Follows `input`, and says which command it pressed, repeated or
    /// let go of, if any.
    ///
    /// A press means the command [`Bindings::resolve`] finds; a repeat
    /// means what its press meant (or, with that forgotten, what it means
    /// now); a release lets go of what its press meant.
    pub fn follow(&mut self, input: &Input) -> Option<CommandInput> {
        let Input::Key {
            pressed, repeat, ..
        } = *input
        else {
            return None;
        };
        let meant = if pressed {
            self.devices.follow(input);
            let meant = if repeat {
                self.devices.press_of(input).or_else(|| self.resolve(input))
            } else {
                self.resolve(input)
            };
            self.devices.note_press(input, meant);
            meant
        } else {
            let meant = self.devices.press_of(input);
            self.devices.follow(input);
            meant
        };
        self.recompute();
        meant.map(|command| CommandInput {
            command,
            pressed,
            repeat: pressed && repeat,
        })
    }

    /// What `input` would mean now, without following it.
    #[must_use]
    pub fn peek(&self, input: &Input) -> Option<Command> {
        let mut devices = self.devices.clone();
        devices.follow(input);
        let active = self.active();
        self.bindings.with(|b| b.resolve(input, &devices, &active))
    }

    /// The command the key of `input` (a press, a repeat or a release)
    /// means, without following it: what its press meant while the
    /// reader holds that, otherwise what pressing it would mean now. So a
    /// release whose press came before the reader did means its key's
    /// command, for a screen that reads a key's going up as its going
    /// down (`_PlayQTMovie`'s skip keys, polled held). `None` for any
    /// other input.
    #[must_use]
    pub fn means(&self, input: &Input) -> Option<Command> {
        let Input::Key { key, .. } = *input else {
            return None;
        };
        self.devices.press_of(input).or_else(|| {
            self.peek(&Input::Key {
                key,
                pressed: true,
                repeat: false,
            })
        })
    }

    /// Every command the key of `input` (a press, a repeat or a release)
    /// holds while it is down, with the modifiers held now, without
    /// following it: what [`CommandReader::held`] would gain with it down.
    /// A screen that keeps its own state per command (a latch on each)
    /// asks this of a key going down or up. Empty for any other input.
    #[must_use]
    pub fn meanings(&self, input: &Input) -> Vec<Command> {
        let Input::Key { key, .. } = *input else {
            return Vec::new();
        };
        let mut devices = self.devices.clone();
        devices.follow(&Input::Key {
            key,
            pressed: true,
            repeat: false,
        });
        let active = self.active();
        self.bindings.with(|bindings| {
            keyboard::meanings(&bindings.keyboard, key, devices.keyboard(), &active)
        })
    }

    /// Whether the key of `input` (a press, a repeat or a release) is
    /// down; false for any other input.
    #[must_use]
    pub fn is_down(&self, input: &Input) -> bool {
        self.devices.keyboard().holds(input)
    }

    /// A fresh press of each key down, in order: what a screen hands back
    /// to the reader after letting go of some of them.
    #[must_use]
    pub fn down(&self) -> Vec<Input> {
        self.devices.keyboard().presses().collect()
    }

    /// Whether a modifier key is down, as the active contexts read the
    /// modifier keys ([`keyboard::sides`]): for a screen whose original
    /// holds an action back while any modifier is down.
    #[must_use]
    pub fn modifier_down(&self) -> bool {
        self.active().into_iter().any(|context| {
            self.devices.keyboard().modifiers(keyboard::sides(context)) != Modifiers::NONE
        })
    }

    /// Whether `command` is held.
    #[must_use]
    pub fn held(&self, command: impl Into<Command>) -> bool {
        self.held.contains(&command.into())
    }

    /// The commands held, in order.
    pub fn held_commands(&self) -> impl Iterator<Item = Command> + '_ {
        self.held.iter().copied()
    }

    /// Whether any device input is down, a command or not.
    #[must_use]
    pub fn anything_down(&self) -> bool {
        self.devices.any_down()
    }

    /// Lets go of everything held, as when the screen's keys are
    /// released or a dialog opens over it.
    pub fn let_go(&mut self) {
        self.devices.let_go();
        self.held.clear();
    }

    /// What `input` means with the devices as they are.
    fn resolve(&self, input: &Input) -> Option<Command> {
        let active = self.active();
        self.bindings
            .with(|bindings| bindings.resolve(input, &self.devices, &active))
    }

    /// Rebuilds the held set from what is down.
    fn recompute(&mut self) {
        let active = self.active();
        self.held = self
            .bindings
            .with(|bindings| bindings.held(&self.devices, &active));
    }
}

#[cfg(test)]
mod tests {
    use super::super::command::{EscortMenuCommand, FlightCommand, UiCommand};
    use super::super::contexts::{DIALOG, FLIGHT};
    use super::super::keyboard::KeyCombo;
    use super::*;
    use crate::input::Key;

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn repeat(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: true,
        }
    }

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[allow(
        clippy::unnecessary_wraps,
        reason = "compared with what follow returns"
    )]
    fn input(command: impl Into<Command>, pressed: bool, repeat: bool) -> Option<CommandInput> {
        Some(CommandInput {
            command: command.into(),
            pressed,
            repeat,
        })
    }

    #[test]
    fn a_held_command_is_held_from_its_press_to_its_release() {
        let mut reader = CommandReader::new(FLIGHT);
        assert_eq!(
            reader.follow(&press(Key::ArrowUp)),
            input(FlightCommand::Accelerate, true, false)
        );
        assert!(reader.held(FlightCommand::Accelerate));
        assert_eq!(
            reader.follow(&release(Key::ArrowUp)),
            input(FlightCommand::Accelerate, false, false)
        );
        assert!(!reader.held(FlightCommand::Accelerate));
    }

    #[test]
    fn command_pressed_after_x_holds_eject_and_letting_go_of_command_lets_it_go() {
        let mut reader = CommandReader::new(FLIGHT);
        assert_eq!(
            reader.follow(&press(Key::KeyX)),
            None,
            "X alone means nothing"
        );
        assert!(!reader.held(FlightCommand::Eject));
        assert_eq!(reader.follow(&press(Key::MetaLeft)), None);
        assert!(reader.held(FlightCommand::Eject));
        reader.follow(&release(Key::MetaLeft));
        assert!(!reader.held(FlightCommand::Eject));
        assert!(reader.anything_down());
    }

    #[test]
    fn a_release_lets_go_of_what_its_press_meant() {
        let mut reader = CommandReader::new(FLIGHT);
        reader.follow(&press(Key::ShiftLeft));
        assert_eq!(
            reader.follow(&press(Key::Tab)),
            input(FlightCommand::TargetSelectBack, true, false)
        );
        reader.follow(&release(Key::ShiftLeft));
        assert!(reader.held(FlightCommand::TargetSelect));
        assert!(!reader.held(FlightCommand::TargetSelectBack));
        assert_eq!(
            reader.follow(&release(Key::Tab)),
            input(FlightCommand::TargetSelectBack, false, false)
        );
    }

    #[test]
    fn a_repeat_is_passed_through_and_marked() {
        let mut reader = CommandReader::new(DIALOG);
        reader.follow(&press(Key::ArrowDown));
        assert_eq!(
            reader.follow(&repeat(Key::ArrowDown)),
            input(UiCommand::Down, true, true)
        );
        let mut fresh = CommandReader::new(DIALOG);
        assert_eq!(
            fresh.follow(&repeat(Key::ArrowDown)),
            input(UiCommand::Down, true, true),
            "a repeat whose press it never saw means what it means now"
        );
    }

    #[test]
    fn let_go_empties_the_held_set() {
        let mut reader = CommandReader::new(FLIGHT);
        reader.follow(&press(Key::Space));
        reader.follow(&press(Key::ArrowLeft));
        assert_eq!(reader.held_commands().count(), 2);
        reader.let_go();
        assert_eq!(reader.held_commands().count(), 0);
        assert!(!reader.anything_down());
        assert_eq!(reader.follow(&release(Key::Space)), None);
    }

    #[test]
    fn a_refilled_handle_is_read_on_the_next_input() {
        let shared = SharedBindings::default();
        let mut reader = CommandReader::new(FLIGHT).with_bindings(shared.clone());
        shared
            .set(Bindings::default().rebound(FlightCommand::Accelerate, KeyCombo::key(Key::KeyT)));
        assert_eq!(
            reader.follow(&press(Key::KeyT)),
            input(FlightCommand::Accelerate, true, false)
        );
        assert_eq!(reader.follow(&press(Key::ArrowUp)), None);
        assert!(reader.held(FlightCommand::Accelerate));
        assert_eq!(reader.bindings().get(), shared.get());
    }

    #[test]
    fn key_1_picks_stellar_slot_1_with_the_menu_hidden_and_group_1_with_it_shown() {
        let mut reader = CommandReader::new(FLIGHT);
        assert_eq!(
            reader.follow(&press(Key::Digit1)),
            input(FlightCommand::StellarSlot1, true, false)
        );
        reader.follow(&release(Key::Digit1));
        reader.set_active(Context::EscortMenu, true);
        assert_eq!(
            reader.active(),
            vec![
                Context::EscortMenu,
                Context::Flight,
                Context::Play,
                Context::Pause
            ]
        );
        assert_eq!(
            reader.follow(&press(Key::Digit1)),
            input(EscortMenuCommand::Group1, true, false)
        );
        assert!(reader.held(EscortMenuCommand::Group1));
        assert!(!reader.held(FlightCommand::StellarSlot1));
        reader.set_active(Context::EscortMenu, false);
        assert!(
            reader.held(FlightCommand::StellarSlot1),
            "the menu hiding gives 1 back"
        );
        assert!(!reader.held(EscortMenuCommand::Group1));
        assert_eq!(
            reader.active(),
            vec![Context::Flight, Context::Play, Context::Pause]
        );
    }

    #[test]
    fn peek_says_what_an_input_means_without_following_it() {
        let reader = CommandReader::new(DIALOG);
        assert_eq!(
            reader.peek(&press(Key::NumpadEnter)),
            Some(Command::Ui(UiCommand::Confirm))
        );
        assert!(!reader.anything_down());
        assert_eq!(reader.peek(&Input::Text('a')), None);
    }

    #[test]
    fn means_says_what_a_key_means_going_down_or_up_whether_its_press_was_seen_or_not() {
        let mut reader = CommandReader::new(FLIGHT);
        assert_eq!(
            reader.means(&release(Key::ArrowUp)),
            Some(Command::Flight(FlightCommand::Accelerate)),
            "a release whose press it never saw"
        );
        reader.follow(&press(Key::ShiftLeft));
        reader.follow(&press(Key::Tab));
        reader.follow(&release(Key::ShiftLeft));
        assert_eq!(
            reader.means(&release(Key::Tab)),
            Some(Command::Flight(FlightCommand::TargetSelectBack)),
            "what its press meant"
        );
        assert_eq!(
            reader.means(&press(Key::KeyX)),
            None,
            "X alone means nothing"
        );
        assert_eq!(reader.means(&Input::Text('a')), None);
        assert!(reader.held(FlightCommand::TargetSelect), "nothing followed");
    }

    #[test]
    fn meanings_lists_every_command_a_key_would_hold_with_the_modifiers_held() {
        let mut reader = CommandReader::new(FLIGHT);
        assert_eq!(
            reader.meanings(&press(Key::Tab)),
            vec![Command::Flight(FlightCommand::TargetSelect)]
        );
        reader.follow(&press(Key::ShiftLeft));
        assert_eq!(
            reader.meanings(&release(Key::Tab)),
            vec![
                Command::Flight(FlightCommand::TargetSelect),
                Command::Flight(FlightCommand::TargetSelectBack),
            ],
            "a release asks what its key holds down"
        );
        assert_eq!(reader.meanings(&press(Key::KeyX)), vec![], "X alone");
        assert_eq!(reader.meanings(&Input::Text('a')), vec![]);
        assert!(
            !reader.held(FlightCommand::TargetSelect),
            "nothing followed"
        );
    }

    #[test]
    fn is_down_and_down_say_which_keys_the_reader_holds() {
        let mut reader = CommandReader::new(FLIGHT);
        reader.follow(&press(Key::KeyX));
        reader.follow(&repeat(Key::ArrowUp));
        assert!(reader.is_down(&press(Key::KeyX)));
        assert!(
            reader.is_down(&release(Key::ArrowUp)),
            "any input of its key"
        );
        assert!(!reader.is_down(&press(Key::KeyJ)));
        assert!(!reader.is_down(&Input::Text('x')));
        assert_eq!(reader.down(), vec![press(Key::KeyX), press(Key::ArrowUp)]);
    }

    #[test]
    fn modifier_down_reads_the_modifier_keys_as_the_active_contexts_do() {
        let mut flight = CommandReader::new(FLIGHT);
        assert!(!flight.modifier_down());
        flight.follow(&press(Key::AltRight));
        assert!(!flight.modifier_down(), "flight reads the left keys alone");
        flight.follow(&press(Key::ShiftLeft));
        assert!(flight.modifier_down());
        let mut dialog = CommandReader::new(DIALOG);
        dialog.follow(&press(Key::MetaRight));
        assert!(dialog.modifier_down(), "a dialog reads either");
        dialog.follow(&press(Key::KeyA));
        dialog.follow(&release(Key::MetaRight));
        assert!(!dialog.modifier_down());
    }

    #[test]
    fn other_input_follows_nothing() {
        let mut reader = CommandReader::new(DIALOG);
        assert_eq!(reader.follow(&Input::Text('a')), None);
        assert_eq!(reader.follow(&Input::PointerLeft), None);
        assert!(!reader.anything_down());
    }

    #[test]
    fn a_clone_of_shared_bindings_sees_a_refill() {
        let shared = SharedBindings::default();
        let clone = shared.clone();
        let rebound = Bindings::default().unbound(FlightCommand::Eject);
        shared.set(rebound.clone());
        assert_eq!(clone.get(), rebound);
        assert_eq!(
            SharedBindings::new(rebound.clone()).with(Clone::clone),
            rebound
        );
    }
}
