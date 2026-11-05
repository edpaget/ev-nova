//! The keys screen's edit: a draft of the bindings, what recording a key
//! binds, the commands a rebind left unbound, and the change OK hands
//! out. No drawing and no input: the screen ([`super::KeysScreen`]) reads
//! the keys and calls this.

use std::collections::BTreeSet;

use nova_sim::{RuleKey, RuleSource, Rulebook};

use crate::input::Key;
use crate::keymap::{Bindings, Command, KeyCombo, KeyMap, Modifiers};

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
    /// The commands a rebind left unbound, until each is bound again or
    /// the defaults are restored.
    marked: BTreeSet<Command>,
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

    /// Whether a rebind left `command` unbound, and nothing has bound it
    /// since.
    #[must_use]
    pub fn is_marked(&self, command: Command) -> bool {
        self.marked.contains(&command)
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
        self.marked.extend(unbound.iter().copied());
        self.unmark_bound();
        Recorded::Bound(unbound)
    }

    /// Clears `command`'s key, as Delete does. It isn't marked: the player
    /// cleared it.
    pub fn clear(&mut self, command: Command) {
        self.draft.keyboard.unbind(command);
    }

    /// Restores every binding's default ([`KeyMap::default`]) and clears
    /// the marks.
    pub fn restore_defaults(&mut self) {
        self.draft = Bindings::default();
        self.marked.clear();
    }

    /// The bindings as edited, if they differ from those the screen
    /// opened on.
    #[must_use]
    pub fn change(&self) -> Option<Bindings> {
        (self.draft != self.opened).then(|| self.draft.clone())
    }

    /// Drops the mark of each command bound again.
    fn unmark_bound(&mut self) {
        let draft = &self.draft;
        self.marked
            .retain(|&command| draft.keyboard.combo(command).is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{FlightCommand, PortCommand, UiCommand};

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
        assert!(edit.is_marked(UiCommand::Back.into()));
        assert!(!edit.is_marked(PortCommand::Bar.into()));
    }

    #[test]
    fn a_mark_goes_once_its_command_is_bound_again() {
        let mut edit = edit(Recording::CommandModifiers);
        edit.record(PortCommand::Bar.into(), Key::Escape, Modifiers::NONE);
        edit.record(UiCommand::Back.into(), Key::F13, Modifiers::NONE);
        assert!(!edit.is_marked(UiCommand::Back.into()));
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
        edit.clear(FlightCommand::Accelerate.into());
        assert_eq!(combo(&edit, FlightCommand::Accelerate), None);
        assert!(!edit.is_marked(FlightCommand::Accelerate.into()));
        edit.restore_defaults();
        assert_eq!(edit.draft(), &Bindings::default());
        assert!(!edit.is_marked(UiCommand::Back.into()));
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
        edit.restore_defaults();
        assert_eq!(edit.change(), None, "back to what it opened on");
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
