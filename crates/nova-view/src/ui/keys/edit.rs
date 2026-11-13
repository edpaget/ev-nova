//! The keys screen's edit: a draft of the bindings, what recording a key
//! binds, the commands a rebind left unbound, and the change OK hands
//! out. No drawing and no input: the screen ([`super::KeysScreen`]) reads
//! the keys and calls this.

use std::collections::BTreeSet;

use nova_sim_frame::{RuleKey, RuleSource, Rulebook};

use crate::input::Key;
use crate::keymap::{
    AxisBinding, Bindings, Command, KeyCombo, KeyMap, Modifiers, PadBinding, PadMap,
};

/// Which modifiers a recorded key binds with: the `recorded_modifiers`
/// rule's two readings ([`RuleKey::RecordedModifiers`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Recording {
    /// The command's own default modifiers, whatever is held, and a
    /// modifier key recorded as its press goes down: the original's, as
    /// `_loadKeys` keeps each recorder's `keyCode` alone and
    /// `-[SRRecorderCell flagsChanged:]` records a modifier on its press.
    #[default]
    CommandModifiers,
    /// The modifiers held as the key goes down, and a modifier key
    /// recorded alone when it is let go with nothing pressed meanwhile.
    HeldModifiers,
}

impl Recording {
    /// The reading `rules` gives `recorded_modifiers`.
    #[must_use]
    pub fn from_rulebook(rules: &Rulebook) -> Self {
        match rules.source_for(RuleKey::RecordedModifiers) {
            RuleSource::Engine => Self::CommandModifiers,
            RuleSource::Bible => Self::HeldModifiers,
        }
    }
}

/// A tab of the keys screen: the device family whose bindings its one
/// cell column shows and edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Column {
    /// The keyboard's.
    Keyboard,
    /// The gamepad's.
    Gamepad,
}

impl Column {
    /// The tabs, left to right.
    pub const ALL: [Self; 2] = [Self::Keyboard, Self::Gamepad];

    /// Its tab's label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Keyboard => "Keyboard",
            Self::Gamepad => "Gamepad",
        }
    }

    /// Its place in [`Column::ALL`].
    pub(super) const fn index(self) -> usize {
        self as usize
    }
}

/// What recording a pad binding did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PadRecorded {
    /// It is bound, and these commands, which had pad bindings, lost them.
    Bound(Vec<Command>),
    /// The command named holds it ([`Bindings::assign_pad`]): nothing is
    /// bound.
    Refused(Command),
}

/// What recording a key did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// The key is bound, and these commands, which had keys, lost them.
    Bound(Vec<Command>),
    /// The key, as pressed or as it would bind, is a menu bar key
    /// equivalent: nothing is bound.
    Reserved,
}

/// The left key of a modifier pressed on either side, as the original
/// records Command, Option, Shift and Control by their left keys' codes
/// (0x37, 0x3a, 0x38, 0x3b); any other key as it is.
#[must_use]
pub fn left_key(key: Key) -> Key {
    match key {
        Key::ShiftRight => Key::ShiftLeft,
        Key::ControlRight => Key::ControlLeft,
        Key::AltRight => Key::AltLeft,
        Key::MetaRight => Key::MetaLeft,
        other => other,
    }
}

/// The draft of the bindings the keys screen edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEdit {
    /// The bindings as the screen opened on them.
    opened: Bindings,
    /// The bindings as edited.
    draft: Bindings,
    /// The commands a rebind left unbound on each column, until each is
    /// bound again there or that column's defaults are restored.
    marked: BTreeSet<(Column, Command)>,
    /// The menu bar's key equivalents, which no recording binds.
    reserved: Vec<KeyCombo>,
    /// What a recorded key binds with.
    recording: Recording,
}

impl KeyEdit {
    /// An edit of `bindings`, binding none of `reserved`, recording as
    /// `recording` reads.
    #[must_use]
    pub fn new(bindings: Bindings, reserved: Vec<KeyCombo>, recording: Recording) -> Self {
        Self {
            opened: bindings.clone(),
            draft: bindings,
            marked: BTreeSet::new(),
            reserved,
            recording,
        }
    }

    /// The bindings as edited.
    #[must_use]
    pub fn draft(&self) -> &Bindings {
        &self.draft
    }

    /// How a recorded key binds.
    #[must_use]
    pub fn recording(&self) -> Recording {
        self.recording
    }

    /// Whether a rebind left `command` unbound on `column`, and nothing
    /// has bound it there since.
    #[must_use]
    pub fn is_marked(&self, column: Column, command: Command) -> bool {
        self.marked.contains(&(column, command))
    }

    /// The combo recording `key` on `command` binds, with `held` the
    /// modifiers held as it went down: under
    /// [`Recording::CommandModifiers`] the key with the modifiers
    /// `command`'s default combo has (Command-K for K on Jettison Cargo,
    /// Shift-T for T on Target Select's Shift variant), under
    /// [`Recording::HeldModifiers`] the key with `held`.
    #[must_use]
    pub fn combo_for(&self, command: Command, key: Key, held: Modifiers) -> KeyCombo {
        let modifiers = match self.recording {
            Recording::CommandModifiers => KeyMap::default()
                .combo(command)
                .map_or(Modifiers::NONE, |combo| combo.modifiers),
            Recording::HeldModifiers => held,
        };
        KeyCombo::with(key, modifiers)
    }

    /// Records `key`, pressed with `held` held, on `command`: binds what
    /// [`KeyEdit::combo_for`] gives, unbinding what clashes with it
    /// ([`Bindings::assign`]) and marking those commands, unless the key
    /// as pressed or the combo it would bind is a reserved key
    /// equivalent, when nothing is bound.
    pub fn record(&mut self, command: Command, key: Key, held: Modifiers) -> Recorded {
        let pressed = KeyCombo::with(key, held);
        let combo = self.combo_for(command, key, held);
        if self.reserved.contains(&pressed) || self.reserved.contains(&combo) {
            return Recorded::Reserved;
        }
        let unbound = self.draft.assign(command, combo);
        self.mark(Column::Keyboard, &unbound);
        Recorded::Bound(unbound)
    }

    /// Records `binding` on `command`'s pad binding through the pad's
    /// rebind rule ([`Bindings::assign_pad`]), marking the commands it
    /// unbinds, or changes nothing when a fixed row or a flight axis
    /// holds it.
    pub fn record_pad(&mut self, command: Command, binding: PadBinding) -> PadRecorded {
        match self.draft.assign_pad(command, binding) {
            Ok(unbound) => {
                self.mark(Column::Gamepad, &unbound);
                PadRecorded::Bound(unbound)
            }
            Err(holder) => PadRecorded::Refused(holder),
        }
    }

    /// Marks `unbound` on `column`, then drops the mark of each command
    /// bound again.
    fn mark(&mut self, column: Column, unbound: &[Command]) {
        self.marked
            .extend(unbound.iter().map(|&command| (column, command)));
        let draft = &self.draft;
        self.marked.retain(|&(column, command)| match column {
            Column::Keyboard => draft.keyboard.combo(command).is_none(),
            Column::Gamepad => draft.gamepad.binding(command) == PadBinding::Unbound,
        });
    }

    /// Reads `command`'s pad axis the other way; nothing for a command
    /// bound to no axis.
    pub fn toggle_invert(&mut self, command: Command) {
        if let Some(binding) = self.draft.gamepad.axis_binding(command) {
            self.draft.gamepad.bind(
                command,
                PadBinding::Axis(AxisBinding {
                    inverted: !binding.inverted,
                    ..binding
                }),
            );
        }
    }

    /// Clears `command`'s binding on `column`, as Delete (or the pad's
    /// West) does; a fixed pad row ([`PadMap::is_fixed`]) clears not. It
    /// isn't marked: the player cleared it.
    pub fn clear_on(&mut self, column: Column, command: Command) {
        match column {
            Column::Keyboard => self.draft.keyboard.unbind(command),
            Column::Gamepad if !PadMap::is_fixed(command) => self.draft.gamepad.unbind(command),
            Column::Gamepad => {}
        }
    }

    /// Restores `column`'s bindings to their defaults ([`KeyMap::default`]
    /// or [`PadMap::default`], the axes and their inversion included) and
    /// clears its marks, leaving the other column's.
    pub fn restore_defaults(&mut self, column: Column) {
        match column {
            Column::Keyboard => self.draft.keyboard = KeyMap::default(),
            Column::Gamepad => self.draft.gamepad = PadMap::default(),
        }
        self.marked.retain(|&(marked, _)| marked != column);
    }

    /// The bindings as edited, if they differ from those the screen
    /// opened on.
    #[must_use]
    pub fn change(&self) -> Option<Bindings> {
        (self.draft != self.opened).then(|| self.draft.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{FlightCommand, PadBinding, PortCommand, UiCommand};

    fn edit(recording: Recording) -> KeyEdit {
        let cmd_q = KeyCombo::with(Key::KeyQ, Modifiers::COMMAND);
        KeyEdit::new(Bindings::default(), vec![cmd_q], recording)
    }

    fn combo(edit: &KeyEdit, command: impl Into<Command>) -> Option<KeyCombo> {
        edit.draft().keyboard.combo(command.into())
    }

    #[test]
    fn the_rule_s_engine_reading_takes_the_command_s_modifiers() {
        assert_eq!(
            Recording::from_rulebook(&Rulebook::default()),
            Recording::CommandModifiers
        );
        let bible =
            Rulebook::default().with_override(RuleKey::RecordedModifiers, RuleSource::Bible);
        assert_eq!(Recording::from_rulebook(&bible), Recording::HeldModifiers);
    }

    #[test]
    fn by_default_a_recorded_key_takes_its_command_s_default_modifiers() {
        let edit = edit(Recording::CommandModifiers);
        let shift = Modifiers::SHIFT;
        let jettison = FlightCommand::JettisonCargo.into();
        assert_eq!(
            edit.combo_for(jettison, Key::KeyK, Modifiers::NONE),
            KeyCombo::with(Key::KeyK, Modifiers::COMMAND)
        );
        assert_eq!(
            edit.combo_for(
                FlightCommand::TargetSelectBack.into(),
                Key::KeyT,
                Modifiers::NONE
            ),
            KeyCombo::with(Key::KeyT, shift)
        );
        assert_eq!(
            edit.combo_for(FlightCommand::TargetSelect.into(), Key::KeyT, shift),
            KeyCombo::key(Key::KeyT),
            "the Shift held is dropped"
        );
    }

    #[test]
    fn by_the_other_reading_a_recorded_key_takes_the_modifiers_held() {
        let edit = edit(Recording::HeldModifiers);
        let held = Modifiers::SHIFT.union(Modifiers::CONTROL);
        assert_eq!(
            edit.combo_for(FlightCommand::JettisonCargo.into(), Key::KeyK, held),
            KeyCombo::with(Key::KeyK, held)
        );
    }

    #[test]
    fn recording_binds_and_marks_what_lost_its_key() {
        let mut edit = edit(Recording::CommandModifiers);
        let recorded = edit.record(PortCommand::Bar.into(), Key::Escape, Modifiers::NONE);
        assert_eq!(recorded, Recorded::Bound(vec![UiCommand::Back.into()]));
        assert_eq!(
            combo(&edit, PortCommand::Bar),
            Some(KeyCombo::key(Key::Escape))
        );
        assert!(edit.is_marked(Column::Keyboard, UiCommand::Back.into()));
        assert!(!edit.is_marked(Column::Keyboard, PortCommand::Bar.into()));
    }

    #[test]
    fn a_mark_goes_once_its_command_is_bound_again() {
        let mut edit = edit(Recording::CommandModifiers);
        edit.record(PortCommand::Bar.into(), Key::Escape, Modifiers::NONE);
        edit.record(UiCommand::Back.into(), Key::F13, Modifiers::NONE);
        assert!(!edit.is_marked(Column::Keyboard, UiCommand::Back.into()));
    }

    #[test]
    fn a_reserved_combo_pressed_or_formed_binds_nothing() {
        let mut edit = edit(Recording::CommandModifiers);
        let before = edit.draft().clone();
        assert_eq!(
            edit.record(FlightCommand::Board.into(), Key::KeyQ, Modifiers::COMMAND),
            Recorded::Reserved,
            "Command-Q as pressed"
        );
        assert_eq!(
            edit.record(
                FlightCommand::JettisonCargo.into(),
                Key::KeyQ,
                Modifiers::NONE
            ),
            Recorded::Reserved,
            "Q on Jettison Cargo would bind Command-Q"
        );
        assert_eq!(edit.draft(), &before);
        let mut held = KeyEdit::new(
            Bindings::default(),
            vec![KeyCombo::with(Key::KeyQ, Modifiers::COMMAND)],
            Recording::HeldModifiers,
        );
        assert_eq!(
            held.record(FlightCommand::Board.into(), Key::KeyQ, Modifiers::COMMAND),
            Recorded::Reserved,
            "Command-Q pressed under the other reading too"
        );
        assert_eq!(
            held.record(
                FlightCommand::JettisonCargo.into(),
                Key::KeyQ,
                Modifiers::NONE
            ),
            Recorded::Bound(vec![]),
            "bare Q binds bare Q there"
        );
    }

    #[test]
    fn clearing_unbinds_unmarked_and_restoring_gives_the_defaults_back() {
        let mut edit = edit(Recording::CommandModifiers);
        edit.record(PortCommand::Bar.into(), Key::Escape, Modifiers::NONE);
        edit.clear_on(Column::Keyboard, FlightCommand::Accelerate.into());
        assert_eq!(combo(&edit, FlightCommand::Accelerate), None);
        assert!(!edit.is_marked(Column::Keyboard, FlightCommand::Accelerate.into()));
        edit.restore_defaults(Column::Keyboard);
        assert_eq!(edit.draft(), &Bindings::default());
        assert!(!edit.is_marked(Column::Keyboard, UiCommand::Back.into()));
    }

    #[test]
    fn the_change_is_the_draft_only_when_it_differs() {
        let mut edit = edit(Recording::CommandModifiers);
        assert_eq!(edit.change(), None);
        edit.record(FlightCommand::Accelerate.into(), Key::KeyT, Modifiers::NONE);
        let change = edit.change().expect("a change");
        assert_eq!(
            change.keyboard.combo(FlightCommand::Accelerate.into()),
            Some(KeyCombo::key(Key::KeyT))
        );
        edit.restore_defaults(Column::Keyboard);
        assert_eq!(edit.change(), None, "back to what it opened on");
    }

    fn pad_entry(button: crate::input::PadButton) -> PadBinding {
        PadBinding::Entry(crate::keymap::PadEntry::plain(button))
    }

    #[test]
    fn recording_a_pad_binding_binds_and_marks_what_lost_it_on_the_gamepad_column() {
        use crate::input::PadButton as P;
        let mut edit = edit(Recording::CommandModifiers);
        let autopilot: Command = FlightCommand::Autopilot.into();
        let closest: Command = FlightCommand::ClosestTarget.into();
        assert_eq!(
            edit.record_pad(autopilot, pad_entry(P::West)),
            PadRecorded::Bound(vec![closest])
        );
        assert_eq!(edit.draft().gamepad.binding(autopilot), pad_entry(P::West));
        assert!(edit.is_marked(Column::Gamepad, closest));
        assert!(!edit.is_marked(Column::Keyboard, closest));
        edit.record_pad(closest, pad_entry(P::West));
        assert!(
            !edit.is_marked(Column::Gamepad, closest),
            "bound again, by taking West back"
        );
        assert!(edit.is_marked(Column::Gamepad, autopilot));
    }

    #[test]
    fn a_refused_pad_binding_changes_nothing() {
        let mut edit = edit(Recording::CommandModifiers);
        let before = edit.draft().clone();
        assert_eq!(
            edit.record_pad(
                PortCommand::Bar.into(),
                pad_entry(crate::input::PadButton::South)
            ),
            PadRecorded::Refused(UiCommand::Activate.into())
        );
        assert_eq!(edit.draft(), &before);
    }

    #[test]
    fn toggling_inversion_flips_an_axis_binding_and_nothing_else() {
        let mut edit = edit(Recording::CommandModifiers);
        let turn: Command = FlightCommand::TurnAxis.into();
        let read = |edit: &KeyEdit| edit.draft().gamepad.axis_binding(turn).map(|b| b.inverted);
        assert_eq!(read(&edit), Some(false));
        edit.toggle_invert(turn);
        assert_eq!(read(&edit), Some(true));
        edit.toggle_invert(turn);
        assert_eq!(read(&edit), Some(false));
        let before = edit.draft().clone();
        edit.toggle_invert(FlightCommand::Autopilot.into());
        let mut unbound = edit.clone();
        unbound.clear_on(Column::Gamepad, turn);
        unbound.toggle_invert(turn);
        assert_eq!(edit.draft(), &before, "no axis, nothing to invert");
        assert_eq!(unbound.draft().gamepad.binding(turn), PadBinding::Unbound);
    }

    #[test]
    fn clearing_and_restoring_act_on_one_column_only() {
        use crate::input::PadButton as P;
        let mut edit = edit(Recording::CommandModifiers);
        let accelerate: Command = FlightCommand::Accelerate.into();
        edit.clear_on(Column::Gamepad, accelerate);
        assert_eq!(
            edit.draft().gamepad.binding(accelerate),
            PadBinding::Unbound
        );
        assert!(edit.draft().keyboard.combo(accelerate).is_some());
        edit.clear_on(Column::Gamepad, UiCommand::Activate.into());
        assert_eq!(
            edit.draft().gamepad.binding(UiCommand::Activate.into()),
            pad_entry(P::South),
            "a fixed row clears not"
        );
        edit.record(PortCommand::Bar.into(), Key::Escape, Modifiers::NONE);
        edit.record_pad(FlightCommand::Autopilot.into(), pad_entry(P::West));
        edit.toggle_invert(FlightCommand::TurnAxis.into());
        edit.restore_defaults(Column::Gamepad);
        assert_eq!(edit.draft().gamepad, crate::keymap::PadMap::default());
        assert_ne!(edit.draft().keyboard, KeyMap::default(), "the keys kept");
        assert!(edit.is_marked(Column::Keyboard, UiCommand::Back.into()));
        assert!(!edit.is_marked(Column::Gamepad, FlightCommand::ClosestTarget.into()));
        edit.record_pad(FlightCommand::Autopilot.into(), pad_entry(P::West));
        edit.restore_defaults(Column::Keyboard);
        assert_eq!(edit.draft().keyboard, KeyMap::default());
        assert!(!edit.is_marked(Column::Keyboard, UiCommand::Back.into()));
        assert!(edit.is_marked(Column::Gamepad, FlightCommand::ClosestTarget.into()));
        assert_ne!(
            edit.draft().gamepad,
            crate::keymap::PadMap::default(),
            "the pad kept"
        );
    }

    #[test]
    fn a_right_modifier_records_as_its_left_key() {
        assert_eq!(left_key(Key::ShiftRight), Key::ShiftLeft);
        assert_eq!(left_key(Key::ControlRight), Key::ControlLeft);
        assert_eq!(left_key(Key::AltRight), Key::AltLeft);
        assert_eq!(left_key(Key::MetaRight), Key::MetaLeft);
        assert_eq!(left_key(Key::CapsLock), Key::CapsLock);
        assert_eq!(left_key(Key::KeyA), Key::KeyA);
    }
}
