//! The keyboard's side of the key map: its binding table, [`KeyMap`],
//! with the original's defaults and fixed twins, and the keys it holds
//! down, [`KeysDown`]. This is the only place outside the platform
//! adapter and `input` that names a [`Key`].

use std::collections::{BTreeMap, BTreeSet};

use super::command::{
    Command, Context, EscortMenuCommand, FlightCommand, PauseCommand, PlayCommand, UiCommand,
};
use super::contexts::shadows;
use crate::input::{Input, Key};

/// The modifiers a combo requires, by flag, naming no side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No modifier.
    pub const NONE: Self = Self(0);
    /// Shift.
    pub const SHIFT: Self = Self(1);
    /// Control.
    pub const CONTROL: Self = Self(2);
    /// Option (Alt).
    pub const OPTION: Self = Self(4);
    /// Command (the Windows or Super key off a Mac).
    pub const COMMAND: Self = Self(8);

    /// Both sets' flags.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether every flag of `other` is set here.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// How many flags are set.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }
}

/// Which of a modifier's two keys set its flag, as a context's original
/// reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sides {
    /// Only the left key: `_HandlePlayer` polls Option, Shift and Command
    /// by their left keys' codes (0x3a, 0x38, 0x37).
    LeftOnly,
    /// Either key: a filter reads `EventRecord.modifiers`' bits
    /// (`shiftKey`, `optionKey`, `cmdKey`, `controlKey`), which Mac OS X
    /// sets for either key.
    Either,
}

/// Each modifier flag with its left and right keys.
const MODIFIER_KEYS: [(Modifiers, Key, Key); 4] = [
    (Modifiers::SHIFT, Key::ShiftLeft, Key::ShiftRight),
    (Modifiers::CONTROL, Key::ControlLeft, Key::ControlRight),
    (Modifiers::OPTION, Key::AltLeft, Key::AltRight),
    (Modifiers::COMMAND, Key::MetaLeft, Key::MetaRight),
];

/// Whether `key` is a modifier key, or Caps Lock: a key no command
/// listing letters takes, as `_MapFilter`'s and `_SingleMissionFilter`'s
/// "any key" reads skip them.
#[must_use]
pub fn is_modifier(key: Key) -> bool {
    key == Key::CapsLock
        || MODIFIER_KEYS
            .iter()
            .any(|&(_, left, right)| key == left || key == right)
}

/// A key and the modifiers it needs held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyCombo {
    /// The key that goes down.
    pub key: Key,
    /// The modifiers that must be held as it does.
    pub modifiers: Modifiers,
}

impl KeyCombo {
    /// `key` alone.
    #[must_use]
    pub const fn key(key: Key) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// `key` with `modifiers` held.
    #[must_use]
    pub const fn with(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Whether the combo is met with `held` modifiers: its key is down
    /// and every modifier it needs is held. More modifiers held than it
    /// needs still meet it, as the original reads only what it polls.
    #[must_use]
    pub const fn met_by(self, held: Modifiers) -> bool {
        held.contains(self.modifiers)
    }
}

/// How the keyboard binds one command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// To this combo.
    Combo(KeyCombo),
    /// To whatever its base command is bound to, plus these modifiers: a
    /// variant the original makes by polling a modifier by fixed key code
    /// beside the base's `_keyPrefs` key, so a rebound base moves it.
    Follows {
        /// The command whose combo it takes.
        base: Command,
        /// The modifiers it adds.
        plus: Modifiers,
    },
    /// To nothing: the player cleared it, or it lost its key.
    Unbound,
}

/// One fixed twin: a key the original reads beside a bindable command's
/// key, which stays put whatever that command is bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Twin {
    /// The context it answers in, when active.
    pub context: Context,
    /// The fixed key, with any modifier held.
    pub key: Key,
    /// The command it means.
    pub command: Command,
}

/// The keyboard's binding table: one [`Binding`] per command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyMap {
    /// Every command's binding.
    bindings: BTreeMap<Command, Binding>,
}

impl Default for KeyMap {
    /// Every command bound to the original's default, as `_loadKeys`
    /// gives a pref with no saved entry (see the phase's findings).
    fn default() -> Self {
        Self {
            bindings: Command::all()
                .map(|command| (command, default_binding(command)))
                .collect(),
        }
    }
}

impl KeyMap {
    /// The fixed twins, read beside the bindable commands whatever they
    /// are bound to. A command bound to the same key in an active context
    /// beats a twin.
    pub const TWINS: [Twin; 7] = [
        // `_loadKeys` hard-codes F1-F4 at `_keyPrefs`+0x34..+0x3a
        // (@0xce6c8-0xce6da), read beside 1-4 by `_HandlePlayer`
        // @0x69d92.
        twin(
            Context::Flight,
            Key::F1,
            flight(FlightCommand::StellarSlot1),
        ),
        twin(
            Context::Flight,
            Key::F2,
            flight(FlightCommand::StellarSlot2),
        ),
        twin(
            Context::Flight,
            Key::F3,
            flight(FlightCommand::StellarSlot3),
        ),
        twin(
            Context::Flight,
            Key::F4,
            flight(FlightCommand::StellarSlot4),
        ),
        // F5 (0x60, @0x69dac), beside Command-5 (@0x69dbc-0x69dda).
        twin(
            Context::Flight,
            Key::F5,
            flight(FlightCommand::NearestStellar),
        ),
        // The keypad's `+` (0x45), beside `=` (0x18), @0x70f4e-0x71110.
        twin(
            Context::Flight,
            Key::NumpadAdd,
            flight(FlightCommand::ZoomIn),
        ),
        // The keypad's Enter beside Return, as every dialog filter reads
        // both (phase 1's findings).
        twin(
            Context::Ui,
            Key::NumpadEnter,
            Command::Ui(UiCommand::Confirm),
        ),
    ];

    /// A key map built from saved entries: each command an entry names
    /// takes its combo, or is unbound for `None` (a command the player
    /// cleared stays cleared), and each command no entry names takes its
    /// default, as `_loadKeys` gives a pref with no saved entry its
    /// constant (@0xcdde9-0xcde0c for weapon safety's S).
    pub fn from_entries(entries: impl IntoIterator<Item = (Command, Option<KeyCombo>)>) -> Self {
        let mut map = Self::default();
        for (command, combo) in entries {
            map.bindings
                .insert(command, combo.map_or(Binding::Unbound, Binding::Combo));
        }
        map
    }

    /// How `command` is bound.
    #[must_use]
    pub fn binding(&self, command: Command) -> Binding {
        self.bindings
            .get(&command)
            .copied()
            .unwrap_or(Binding::Unbound)
    }

    /// The combo `command` is bound to, a variant's through its base, or
    /// `None` when it, or its base, is unbound.
    #[must_use]
    pub fn combo(&self, command: Command) -> Option<KeyCombo> {
        let mut command = command;
        let mut plus = Modifiers::NONE;
        // A variant's base is never itself a variant, but bound the walk
        // anyway, so a cycle a saved map makes can't hang.
        for _ in 0..4 {
            match self.binding(command) {
                Binding::Combo(combo) => {
                    return Some(KeyCombo::with(combo.key, combo.modifiers.union(plus)));
                }
                Binding::Follows { base, plus: more } => {
                    command = base;
                    plus = plus.union(more);
                }
                Binding::Unbound => return None,
            }
        }
        None
    }

    /// Binds `command` to `combo`.
    pub fn bind(&mut self, command: Command, combo: KeyCombo) {
        self.bindings.insert(command, Binding::Combo(combo));
    }

    /// Leaves `command` unbound.
    pub fn unbind(&mut self, command: Command) {
        self.bindings.insert(command, Binding::Unbound);
    }
}

/// A [`Twin`], for the const table.
const fn twin(context: Context, key: Key, command: Command) -> Twin {
    Twin {
        context,
        key,
        command,
    }
}

/// A flight command, for the const tables.
const fn flight(command: FlightCommand) -> Command {
    Command::Flight(command)
}

/// `key` alone.
const fn bare(key: Key) -> Binding {
    Binding::Combo(KeyCombo::key(key))
}

/// `key` with `modifiers`.
const fn chord(key: Key, modifiers: Modifiers) -> Binding {
    Binding::Combo(KeyCombo::with(key, modifiers))
}

/// `base`'s binding plus `plus`.
const fn follows(base: FlightCommand, plus: Modifiers) -> Binding {
    Binding::Follows {
        base: Command::Flight(base),
        plus,
    }
}

/// Every command's default: the original's, as the phase's findings
/// record (`_loadKeys` @0xcd5f8-0xce6c4 for the 34 `Keys.nib` commands).
const fn default_binding(command: Command) -> Binding {
    match command {
        Command::Flight(command) => default_flight(command),
        Command::EscortMenu(command) => default_escort_menu(command),
        Command::Play(command) => default_play(command),
        Command::Pause(PauseCommand::Pause) => bare(Key::Escape),
        Command::Ui(command) => default_ui(command),
    }
}

/// Flight's defaults.
const fn default_flight(command: FlightCommand) -> Binding {
    use FlightCommand as F;
    match command {
        F::Accelerate => bare(Key::ArrowUp),
        F::ReverseCourse => bare(Key::ArrowDown),
        F::RotateRight => bare(Key::ArrowRight),
        F::RotateLeft => bare(Key::ArrowLeft),
        F::Afterburner => bare(Key::KeyZ),
        F::Autopilot => bare(Key::KeyA),
        F::HyperspaceMode => bare(Key::KeyH),
        F::HyperSelect => bare(Key::Backslash),
        F::HyperJump => bare(Key::KeyJ),
        F::NavOff => bare(Key::Backquote),
        F::Communicate => bare(Key::KeyY),
        F::LandDock => bare(Key::KeyL),
        F::FirePrimary => bare(Key::Space),
        F::FireSecondary => bare(Key::ControlLeft),
        F::SelectSecondary => bare(Key::KeyW),
        F::WeaponSafety => bare(Key::KeyS),
        F::TargetSelect => bare(Key::Tab),
        F::ClosestTarget => bare(Key::KeyR),
        F::EscortMenu => bare(Key::KeyE),
        F::AttackTarget => bare(Key::KeyF),
        F::DefendMe => bare(Key::KeyD),
        F::HoldPosition => bare(Key::KeyV),
        F::Recall => bare(Key::KeyC),
        F::Acknowledge => bare(Key::Enter),
        F::Board => bare(Key::KeyB),
        F::JettisonCargo => chord(Key::KeyK, Modifiers::COMMAND),
        F::Eject => chord(Key::KeyX, Modifiers::COMMAND),
        F::SelfDestruct => chord(Key::Minus, Modifiers::COMMAND),
        F::EngageCloak => bare(Key::KeyU),
        F::ShowFramerate => bare(Key::F12),
        F::FaceStellar => follows(F::Autopilot, Modifiers::OPTION),
        F::SelectSecondaryBack => follows(F::SelectSecondary, Modifiers::OPTION),
        F::TargetSelectBack => follows(F::TargetSelect, Modifiers::SHIFT),
        F::TargetEscort => follows(F::TargetSelect, Modifiers::OPTION),
        F::TargetEscortBack => follows(F::TargetSelect, Modifiers::SHIFT.union(Modifiers::OPTION)),
        F::ClosestShip => follows(F::ClosestTarget, Modifiers::OPTION),
        F::ClearTarget => follows(F::NavOff, Modifiers::OPTION),
        F::HailStellar => follows(F::Communicate, Modifiers::OPTION),
        F::RecallToHangar => follows(F::Recall, Modifiers::OPTION),
        F::JettisonNonMission => follows(F::JettisonCargo, Modifiers::OPTION),
        F::StellarSlot1 => bare(Key::Digit1),
        F::StellarSlot2 => bare(Key::Digit2),
        F::StellarSlot3 => bare(Key::Digit3),
        F::StellarSlot4 => bare(Key::Digit4),
        F::NearestStellar => chord(Key::Digit5, Modifiers::COMMAND),
        F::ZoomOut => bare(Key::Minus),
        F::ZoomIn => bare(Key::Equal),
    }
}

/// The escort menu's defaults: 1-5 (`_loadKeys` @0xce722-0xce73a).
const fn default_escort_menu(command: EscortMenuCommand) -> Binding {
    match command {
        EscortMenuCommand::Group1 => bare(Key::Digit1),
        EscortMenuCommand::Group2 => bare(Key::Digit2),
        EscortMenuCommand::Group3 => bare(Key::Digit3),
        EscortMenuCommand::Group4 => bare(Key::Digit4),
        EscortMenuCommand::Group5 => bare(Key::Digit5),
    }
}

/// The shared play keys' defaults.
const fn default_play(command: PlayCommand) -> Binding {
    match command {
        PlayCommand::GalaxyMap => bare(Key::KeyM),
        PlayCommand::PlayerInfo => bare(Key::KeyP),
        PlayCommand::MissionInfo => bare(Key::KeyI),
    }
}

/// The shared UI context's defaults.
const fn default_ui(command: UiCommand) -> Binding {
    match command {
        UiCommand::Confirm => bare(Key::Enter),
        UiCommand::Back => bare(Key::Escape),
        UiCommand::Up => bare(Key::ArrowUp),
        UiCommand::Down => bare(Key::ArrowDown),
        UiCommand::Left => bare(Key::ArrowLeft),
        UiCommand::Right => bare(Key::ArrowRight),
        UiCommand::Tab => bare(Key::Tab),
        UiCommand::Activate => bare(Key::Space),
        UiCommand::Home => bare(Key::Home),
        UiCommand::End => bare(Key::End),
        UiCommand::PageUp => bare(Key::PageUp),
        UiCommand::PageDown => bare(Key::PageDown),
    }
}

/// Which modifier keys a context's original reads.
#[must_use]
pub const fn sides(context: Context) -> Sides {
    match context {
        Context::Flight | Context::EscortMenu | Context::Play | Context::Pause => Sides::LeftOnly,
        Context::Ui => Sides::Either,
    }
}

/// Every command `key` means with the modifiers `keys` holds, over the
/// `active` contexts in their order:
///
/// 1. A shadowing context's commands on the key ([`shadows`]), if any is
///    met: it wins its keys.
/// 2. Otherwise every other active context's commands met on the key,
///    leaving out a shadowed context's commands on a key its shadowing
///    context binds.
/// 3. Otherwise the fixed twins of the active contexts on the key: a
///    bound command beats a twin.
///
/// Every command returned is held while the key stays down; a press
/// means the most specific of them ([`most_specific`]).
pub(super) fn meanings(
    map: &KeyMap,
    key: Key,
    keys: &KeysDown,
    active: &[Context],
) -> Vec<Command> {
    let met = |context: Context| -> Vec<Command> {
        let held = keys.modifiers(sides(context));
        context
            .commands()
            .filter(|&command| {
                map.combo(command)
                    .is_some_and(|combo| combo.key == key && combo.met_by(held))
            })
            .collect()
    };
    let shadowing = |context: Context| active.iter().any(|&other| shadows(context, other));
    let shadowed_here = |context: Context| {
        active.iter().any(|&other| {
            shadows(other, context)
                && other
                    .commands()
                    .any(|command| map.combo(command).is_some_and(|combo| combo.key == key))
        })
    };
    let first: Vec<Command> = active
        .iter()
        .filter(|&&context| shadowing(context))
        .flat_map(|&context| met(context))
        .collect();
    if !first.is_empty() {
        return first;
    }
    let bound: Vec<Command> = active
        .iter()
        .filter(|&&context| !shadowing(context) && !shadowed_here(context))
        .flat_map(|&context| met(context))
        .collect();
    if !bound.is_empty() {
        return bound;
    }
    KeyMap::TWINS
        .iter()
        .filter(|twin| twin.key == key && active.contains(&twin.context))
        .map(|twin| twin.command)
        .collect()
}

/// The command a press means among `meanings`, which [`meanings`] lists
/// in the active contexts' order: the one whose combo needs the most
/// modifiers, so Option-A beats A, and the first of those.
pub(super) fn most_specific(map: &KeyMap, meanings: &[Command]) -> Option<Command> {
    let needs = |command: Command| {
        map.combo(command)
            .map_or(0, |combo| combo.modifiers.count())
    };
    meanings
        .iter()
        .copied()
        .rev()
        .max_by_key(|&command| needs(command))
}

/// The keyboard's device state: the keys down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeysDown {
    /// Every key down.
    down: BTreeSet<Key>,
}

impl KeysDown {
    /// Follows a key going down or up; anything else changes nothing.
    /// Says whether it was a key.
    pub fn follow(&mut self, input: &Input) -> bool {
        let Input::Key { key, pressed, .. } = *input else {
            return false;
        };
        if pressed {
            self.down.insert(key);
        } else {
            self.down.remove(&key);
        }
        true
    }

    /// Whether `key` is down.
    #[must_use]
    pub fn is_down(&self, key: Key) -> bool {
        self.down.contains(&key)
    }

    /// The keys down, in order.
    pub fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.down.iter().copied()
    }

    /// The modifiers held, as `sides` reads them.
    #[must_use]
    pub fn modifiers(&self, sides: Sides) -> Modifiers {
        MODIFIER_KEYS
            .iter()
            .filter(|&&(_, left, right)| {
                self.is_down(left) || (sides == Sides::Either && self.is_down(right))
            })
            .fold(Modifiers::NONE, |held, &(flag, ..)| held.union(flag))
    }

    /// Lets go of every key.
    pub fn let_go(&mut self) {
        self.down.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(map: &KeyMap, command: impl Into<Command>) -> Option<KeyCombo> {
        map.combo(command.into())
    }

    /// Every command of `context` with its default combo, as the map has
    /// them.
    fn defaults(context: Context) -> Vec<(Command, Option<KeyCombo>)> {
        let map = KeyMap::default();
        context.commands().map(|c| (c, map.combo(c))).collect()
    }

    fn row(
        command: impl Into<Command>,
        key: Key,
        modifiers: Modifiers,
    ) -> (Command, Option<KeyCombo>) {
        (command.into(), Some(KeyCombo::with(key, modifiers)))
    }

    const NO: Modifiers = Modifiers::NONE;
    const CMD: Modifiers = Modifiers::COMMAND;
    const OPT: Modifiers = Modifiers::OPTION;
    const SHIFT: Modifiers = Modifiers::SHIFT;

    #[test]
    fn flight_s_defaults_are_the_original_s() {
        use FlightCommand as F;
        assert_eq!(
            defaults(Context::Flight),
            vec![
                row(F::Accelerate, Key::ArrowUp, NO),
                row(F::ReverseCourse, Key::ArrowDown, NO),
                row(F::RotateRight, Key::ArrowRight, NO),
                row(F::RotateLeft, Key::ArrowLeft, NO),
                row(F::Afterburner, Key::KeyZ, NO),
                row(F::Autopilot, Key::KeyA, NO),
                row(F::HyperspaceMode, Key::KeyH, NO),
                row(F::HyperSelect, Key::Backslash, NO),
                row(F::HyperJump, Key::KeyJ, NO),
                row(F::NavOff, Key::Backquote, NO),
                row(F::Communicate, Key::KeyY, NO),
                row(F::LandDock, Key::KeyL, NO),
                row(F::FirePrimary, Key::Space, NO),
                row(F::FireSecondary, Key::ControlLeft, NO),
                row(F::SelectSecondary, Key::KeyW, NO),
                row(F::WeaponSafety, Key::KeyS, NO),
                row(F::TargetSelect, Key::Tab, NO),
                row(F::ClosestTarget, Key::KeyR, NO),
                row(F::EscortMenu, Key::KeyE, NO),
                row(F::AttackTarget, Key::KeyF, NO),
                row(F::DefendMe, Key::KeyD, NO),
                row(F::HoldPosition, Key::KeyV, NO),
                row(F::Recall, Key::KeyC, NO),
                row(F::Acknowledge, Key::Enter, NO),
                row(F::Board, Key::KeyB, NO),
                row(F::JettisonCargo, Key::KeyK, CMD),
                row(F::Eject, Key::KeyX, CMD),
                row(F::SelfDestruct, Key::Minus, CMD),
                row(F::EngageCloak, Key::KeyU, NO),
                row(F::ShowFramerate, Key::F12, NO),
                row(F::FaceStellar, Key::KeyA, OPT),
                row(F::SelectSecondaryBack, Key::KeyW, OPT),
                row(F::TargetSelectBack, Key::Tab, SHIFT),
                row(F::TargetEscort, Key::Tab, OPT),
                row(F::TargetEscortBack, Key::Tab, SHIFT.union(OPT)),
                row(F::ClosestShip, Key::KeyR, OPT),
                row(F::ClearTarget, Key::Backquote, OPT),
                row(F::HailStellar, Key::KeyY, OPT),
                row(F::RecallToHangar, Key::KeyC, OPT),
                row(F::JettisonNonMission, Key::KeyK, CMD.union(OPT)),
                row(F::StellarSlot1, Key::Digit1, NO),
                row(F::StellarSlot2, Key::Digit2, NO),
                row(F::StellarSlot3, Key::Digit3, NO),
                row(F::StellarSlot4, Key::Digit4, NO),
                row(F::NearestStellar, Key::Digit5, CMD),
                row(F::ZoomOut, Key::Minus, NO),
                row(F::ZoomIn, Key::Equal, NO),
            ]
        );
    }

    #[test]
    fn the_escort_menu_s_defaults_are_1_to_5() {
        use EscortMenuCommand as E;
        assert_eq!(
            defaults(Context::EscortMenu),
            vec![
                row(E::Group1, Key::Digit1, NO),
                row(E::Group2, Key::Digit2, NO),
                row(E::Group3, Key::Digit3, NO),
                row(E::Group4, Key::Digit4, NO),
                row(E::Group5, Key::Digit5, NO),
            ]
        );
    }

    #[test]
    fn the_play_keys_defaults_are_m_p_and_i() {
        assert_eq!(
            defaults(Context::Play),
            vec![
                row(PlayCommand::GalaxyMap, Key::KeyM, NO),
                row(PlayCommand::PlayerInfo, Key::KeyP, NO),
                row(PlayCommand::MissionInfo, Key::KeyI, NO),
            ]
        );
    }

    #[test]
    fn pause_s_default_is_escape() {
        assert_eq!(
            defaults(Context::Pause),
            vec![row(PauseCommand::Pause, Key::Escape, NO)]
        );
    }

    #[test]
    fn the_shared_ui_defaults_are_return_escape_the_arrows_tab_space_and_the_scroll_keys() {
        use UiCommand as U;
        assert_eq!(
            defaults(Context::Ui),
            vec![
                row(U::Confirm, Key::Enter, NO),
                row(U::Back, Key::Escape, NO),
                row(U::Up, Key::ArrowUp, NO),
                row(U::Down, Key::ArrowDown, NO),
                row(U::Left, Key::ArrowLeft, NO),
                row(U::Right, Key::ArrowRight, NO),
                row(U::Tab, Key::Tab, NO),
                row(U::Activate, Key::Space, NO),
                row(U::Home, Key::Home, NO),
                row(U::End, Key::End, NO),
                row(U::PageUp, Key::PageUp, NO),
                row(U::PageDown, Key::PageDown, NO),
            ]
        );
    }

    #[test]
    fn every_command_is_bound_by_default() {
        let map = KeyMap::default();
        for command in Command::all() {
            assert!(map.combo(command).is_some(), "{command:?}");
        }
    }

    #[test]
    fn a_variant_follows_its_rebound_base() {
        let mut map = KeyMap::default();
        map.bind(FlightCommand::TargetSelect.into(), KeyCombo::key(Key::KeyT));
        assert_eq!(
            combo(&map, FlightCommand::TargetSelectBack),
            Some(KeyCombo::with(Key::KeyT, SHIFT))
        );
        assert_eq!(
            combo(&map, FlightCommand::TargetEscortBack),
            Some(KeyCombo::with(Key::KeyT, SHIFT.union(OPT)))
        );
        map.bind(
            FlightCommand::JettisonCargo.into(),
            KeyCombo::with(Key::KeyG, CMD),
        );
        assert_eq!(
            combo(&map, FlightCommand::JettisonNonMission),
            Some(KeyCombo::with(Key::KeyG, CMD.union(OPT)))
        );
    }

    #[test]
    fn a_variant_whose_base_is_unbound_is_unbound_and_one_bound_alone_stays() {
        let mut map = KeyMap::default();
        map.unbind(FlightCommand::Autopilot.into());
        assert_eq!(combo(&map, FlightCommand::FaceStellar), None);
        map.bind(FlightCommand::FaceStellar.into(), KeyCombo::key(Key::KeyQ));
        map.bind(FlightCommand::Autopilot.into(), KeyCombo::key(Key::KeyA));
        assert_eq!(
            combo(&map, FlightCommand::FaceStellar),
            Some(KeyCombo::key(Key::KeyQ))
        );
        assert_eq!(
            map.binding(FlightCommand::FaceStellar.into()),
            Binding::Combo(KeyCombo::key(Key::KeyQ))
        );
    }

    #[test]
    fn a_cycle_of_follows_resolves_to_nothing() {
        let mut map = KeyMap::default();
        let (a, b) = (
            Command::from(FlightCommand::FaceStellar),
            Command::from(FlightCommand::Autopilot),
        );
        map.bindings
            .insert(a, Binding::Follows { base: b, plus: OPT });
        map.bindings
            .insert(b, Binding::Follows { base: a, plus: OPT });
        assert_eq!(map.combo(a), None);
    }

    #[test]
    fn a_map_from_entries_fills_each_missing_command_with_its_default() {
        let weapon_safety = Command::from(FlightCommand::WeaponSafety);
        let map = KeyMap::from_entries([(
            Command::from(FlightCommand::Accelerate),
            Some(KeyCombo::key(Key::KeyT)),
        )]);
        assert_eq!(map.combo(weapon_safety), Some(KeyCombo::key(Key::KeyS)));
        assert_eq!(
            combo(&map, FlightCommand::Accelerate),
            Some(KeyCombo::key(Key::KeyT))
        );
        assert_eq!(
            combo(&map, FlightCommand::ShowFramerate),
            Some(KeyCombo::key(Key::F12))
        );
    }

    #[test]
    fn a_map_from_entries_keeps_a_cleared_command_unbound() {
        let map = KeyMap::from_entries([(Command::from(FlightCommand::WeaponSafety), None)]);
        assert_eq!(combo(&map, FlightCommand::WeaponSafety), None);
        assert_eq!(
            map.binding(FlightCommand::WeaponSafety.into()),
            Binding::Unbound
        );
    }

    #[test]
    fn modifiers_combine_count_and_contain() {
        let both = SHIFT.union(OPT);
        assert!(both.contains(SHIFT));
        assert!(both.contains(OPT));
        assert!(!both.contains(CMD));
        assert!(both.contains(NO));
        assert_eq!(both.count(), 2);
        assert_eq!(NO.count(), 0);
        assert!(KeyCombo::with(Key::KeyA, OPT).met_by(both));
        assert!(!KeyCombo::with(Key::KeyA, CMD).met_by(both));
    }

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: Key) -> Input {
        Input::Key {
            key,
            pressed: false,
            repeat: false,
        }
    }

    #[test]
    fn keys_down_follow_presses_and_releases_and_read_modifiers_by_side() {
        let mut keys = KeysDown::default();
        assert!(keys.follow(&press(Key::AltRight)));
        assert!(!keys.follow(&Input::Text('a')));
        assert_eq!(keys.modifiers(Sides::Either), OPT);
        assert_eq!(
            keys.modifiers(Sides::LeftOnly),
            NO,
            "the right key isn't polled"
        );
        keys.follow(&press(Key::ShiftLeft));
        keys.follow(&press(Key::MetaLeft));
        keys.follow(&press(Key::ControlRight));
        assert_eq!(keys.modifiers(Sides::LeftOnly), SHIFT.union(CMD));
        assert_eq!(
            keys.modifiers(Sides::Either),
            SHIFT.union(CMD).union(OPT).union(Modifiers::CONTROL)
        );
        keys.follow(&release(Key::ShiftLeft));
        assert!(!keys.is_down(Key::ShiftLeft));
        assert_eq!(keys.keys().count(), 3);
        keys.let_go();
        assert_eq!(keys.keys().count(), 0);
    }

    #[test]
    fn modifiers_and_caps_lock_are_modifier_keys_and_letters_are_not() {
        for key in [
            Key::ShiftLeft,
            Key::ShiftRight,
            Key::ControlLeft,
            Key::ControlRight,
            Key::AltLeft,
            Key::AltRight,
            Key::MetaLeft,
            Key::MetaRight,
            Key::CapsLock,
        ] {
            assert!(is_modifier(key), "{key}");
        }
        for key in [Key::KeyA, Key::Enter, Key::Space, Key::Tab] {
            assert!(!is_modifier(key), "{key}");
        }
    }

    #[test]
    fn flight_reads_left_modifiers_and_the_shared_ui_either() {
        assert_eq!(sides(Context::Flight), Sides::LeftOnly);
        assert_eq!(sides(Context::EscortMenu), Sides::LeftOnly);
        assert_eq!(sides(Context::Ui), Sides::Either);
    }
}
