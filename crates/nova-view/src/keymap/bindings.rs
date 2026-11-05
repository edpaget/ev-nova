//! Every device's binding table, the lookup over them, and the clash
//! query.

use std::collections::{BTreeMap, BTreeSet};

use super::command::{Command, Context};
use super::contexts::{SCREENS, ScreenContexts, shadows, together_on};
use super::keyboard::{self, KeyCombo, KeyMap, KeysDown};
use crate::input::{Input, Key};

/// One binding table per device family. Phase 7 adds the gamepad's
/// beside the keyboard's, and phase 3 saves each on its own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bindings {
    /// The keyboard's table.
    pub keyboard: KeyMap,
}

/// Two commands one combo means, in contexts active together.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clash {
    /// The command declared first.
    pub first: Command,
    /// The command declared later.
    pub second: Command,
}

impl Bindings {
    /// These bindings with `command` bound to `combo` on the keyboard.
    #[must_use]
    pub fn rebound(mut self, command: impl Into<Command>, combo: KeyCombo) -> Self {
        self.keyboard.bind(command.into(), combo);
        self
    }

    /// These bindings with `command` unbound on the keyboard.
    #[must_use]
    pub fn unbound(mut self, command: impl Into<Command>) -> Self {
        self.keyboard.unbind(command.into());
        self
    }

    /// The command `input` means in the `active` contexts, in their
    /// order, with `devices` holding what was down before it: for a key
    /// press or repeat, the most specific command its key and the held
    /// modifiers meet. A release, or any other input, means none here:
    /// [`CommandReader`](super::CommandReader) reports a release as the
    /// command its press meant.
    #[must_use]
    pub fn resolve(&self, input: &Input, devices: &Devices, active: &[Context]) -> Option<Command> {
        let Input::Key {
            key, pressed: true, ..
        } = *input
        else {
            return None;
        };
        let meanings = keyboard::meanings(&self.keyboard, key, &devices.keyboard, active);
        keyboard::most_specific(&self.keyboard, &meanings)
    }

    /// Every command `devices` hold down in the `active` contexts: each
    /// one a key down means with the modifiers held.
    #[must_use]
    pub fn held(&self, devices: &Devices, active: &[Context]) -> BTreeSet<Command> {
        devices
            .keyboard
            .keys()
            .flat_map(|key| keyboard::meanings(&self.keyboard, key, &devices.keyboard, active))
            .collect()
    }

    /// Every pair of commands bound to one combo in contexts active
    /// together on some screen, a shadowing context's pairs with the
    /// context it shadows left out. `KeyMap::default()` has none.
    #[must_use]
    pub fn clashes(&self) -> Vec<Clash> {
        self.clashes_on(SCREENS)
    }

    /// The commands that would clash with `command` were it bound to
    /// `combo`.
    #[must_use]
    pub fn clashes_with(&self, command: impl Into<Command>, combo: KeyCombo) -> Vec<Command> {
        let command = command.into();
        Command::all()
            .filter(|&other| other != command)
            .filter(|&other| self.keyboard.combo(other) == Some(combo))
            .filter(|&other| clash_between(SCREENS, command, other))
            .collect()
    }

    /// [`Bindings::clashes`] over `screens`.
    fn clashes_on(&self, screens: &[ScreenContexts]) -> Vec<Clash> {
        let mut by_combo: BTreeMap<KeyCombo, Vec<Command>> = BTreeMap::new();
        for command in Command::all() {
            if let Some(combo) = self.keyboard.combo(command) {
                by_combo.entry(combo).or_default().push(command);
            }
        }
        let mut clashes = Vec::new();
        for commands in by_combo.values() {
            for (at, &first) in commands.iter().enumerate() {
                for &second in &commands[at + 1..] {
                    if clash_between(screens, first, second) {
                        clashes.push(Clash { first, second });
                    }
                }
            }
        }
        clashes
    }
}

/// Whether two commands on one combo clash over `screens`: their
/// contexts are active together and neither shadows the other.
fn clash_between(screens: &[ScreenContexts], a: Command, b: Command) -> bool {
    let (a, b) = (a.context(), b.context());
    together_on(screens, a, b) && !shadows(a, b) && !shadows(b, a)
}

/// What every device holds down, and what each press meant. Today the
/// keyboard's alone; phase 7 adds the gamepad's beside it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Devices {
    /// The keys down.
    keyboard: KeysDown,
    /// What each key down meant as it went down, so its release lets go
    /// of that, whatever is held by then.
    presses: BTreeMap<Key, Command>,
}

impl Devices {
    /// Follows `input`'s change to what is down. Says whether it was a
    /// device's input (a key, today).
    pub fn follow(&mut self, input: &Input) -> bool {
        if let Input::Key {
            key,
            pressed: false,
            ..
        } = *input
        {
            self.presses.remove(&key);
        }
        self.keyboard.follow(input)
    }

    /// Notes that `input`, a press, meant `command`.
    pub fn note_press(&mut self, input: &Input, command: Option<Command>) {
        if let Input::Key {
            key, pressed: true, ..
        } = *input
        {
            match command {
                Some(command) => self.presses.insert(key, command),
                None => self.presses.remove(&key),
            };
        }
    }

    /// What the press of the key `input` presses, repeats or releases
    /// meant, if it was noted and is still down.
    #[must_use]
    pub fn press_of(&self, input: &Input) -> Option<Command> {
        let Input::Key { key, .. } = *input else {
            return None;
        };
        self.presses.get(&key).copied()
    }

    /// Whether anything is down.
    #[must_use]
    pub fn any_down(&self) -> bool {
        self.keyboard.keys().next().is_some()
    }

    /// The keyboard's keys down.
    #[must_use]
    pub const fn keyboard(&self) -> &KeysDown {
        &self.keyboard
    }

    /// Lets go of everything.
    pub fn let_go(&mut self) {
        self.keyboard.let_go();
        self.presses.clear();
    }
}

/// Whether `input` is a modifier key's (or Caps Lock's) press, repeat or
/// release: an input the "any key" reads pass over.
#[must_use]
pub fn is_modifier_input(input: &Input) -> bool {
    matches!(*input, Input::Key { key, .. } if keyboard::is_modifier(key))
}

#[cfg(test)]
mod tests {
    use super::super::command::{
        EscortMenuCommand, FlightCommand, PauseCommand, PlayCommand, UiCommand,
    };
    use super::super::contexts::{DIALOG, Entry, FLIGHT};
    use super::super::keyboard::Modifiers;
    use super::*;

    fn press(key: Key) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat: false,
        }
    }

    /// What pressing `key` means with `held` down first.
    fn means(bindings: &Bindings, held: &[Key], key: Key, active: &[Context]) -> Option<Command> {
        let mut devices = Devices::default();
        for &down in held {
            devices.follow(&press(down));
        }
        let input = press(key);
        devices.follow(&input);
        bindings.resolve(&input, &devices, active)
    }

    const FLYING: &[Context] = &[Context::Flight, Context::Play, Context::Pause];
    const MENU_SHOWN: &[Context] = &[
        Context::EscortMenu,
        Context::Flight,
        Context::Play,
        Context::Pause,
    ];

    #[allow(clippy::unnecessary_wraps, reason = "compared with an Option")]
    fn flight(command: FlightCommand) -> Option<Command> {
        Some(Command::Flight(command))
    }

    #[test]
    fn a_plain_key_resolves_to_its_command() {
        let bindings = Bindings::default();
        assert_eq!(
            means(&bindings, &[], Key::ArrowUp, FLYING),
            flight(FlightCommand::Accelerate)
        );
        assert_eq!(
            means(&bindings, &[], Key::KeyM, FLYING),
            Some(Command::Play(PlayCommand::GalaxyMap))
        );
        assert_eq!(
            means(&bindings, &[], Key::Escape, FLYING),
            Some(Command::Pause(PauseCommand::Pause))
        );
        assert_eq!(means(&bindings, &[], Key::KeyQ, FLYING), None);
    }

    #[test]
    fn the_combo_needing_the_most_modifiers_wins() {
        let bindings = Bindings::default();
        assert_eq!(
            means(&bindings, &[Key::AltLeft], Key::KeyA, FLYING),
            flight(FlightCommand::FaceStellar)
        );
        assert_eq!(
            means(&bindings, &[Key::ShiftLeft, Key::AltLeft], Key::Tab, FLYING),
            flight(FlightCommand::TargetEscortBack)
        );
        assert_eq!(
            means(&bindings, &[Key::MetaLeft, Key::AltLeft], Key::KeyK, FLYING),
            flight(FlightCommand::JettisonNonMission)
        );
        assert_eq!(
            means(&bindings, &[Key::MetaLeft], Key::Minus, FLYING),
            flight(FlightCommand::SelfDestruct)
        );
        assert_eq!(
            means(&bindings, &[], Key::Minus, FLYING),
            flight(FlightCommand::ZoomOut)
        );
        assert_eq!(
            means(&bindings, &[Key::ShiftLeft], Key::KeyA, FLYING),
            flight(FlightCommand::Autopilot),
            "a modifier the combo doesn't need still meets it"
        );
        assert_eq!(
            means(&bindings, &[], Key::KeyK, FLYING),
            None,
            "K needs Command"
        );
    }

    #[test]
    fn flight_reads_the_left_modifier_and_the_dialogs_either() {
        let bindings = Bindings::default();
        assert_eq!(
            means(&bindings, &[Key::AltRight], Key::KeyA, FLYING),
            flight(FlightCommand::Autopilot),
            "_HandlePlayer polls the left Option's code"
        );
        let shifted_tab =
            Bindings::default().rebound(UiCommand::Tab, KeyCombo::with(Key::Tab, Modifiers::SHIFT));
        assert_eq!(
            means(&shifted_tab, &[Key::ShiftRight], Key::Tab, &[Context::Ui]),
            Some(Command::Ui(UiCommand::Tab)),
            "a dialog reads shiftKey, set by either key"
        );
        assert_eq!(means(&shifted_tab, &[], Key::Tab, &[Context::Ui]), None);
    }

    #[test]
    fn a_key_bound_in_an_inactive_context_resolves_to_nothing() {
        let bindings = Bindings::default();
        assert_eq!(means(&bindings, &[], Key::PageDown, FLYING), None);
        assert_eq!(means(&bindings, &[], Key::KeyL, &[Context::Ui]), None);
    }

    #[test]
    fn the_escort_menu_wins_1_to_5_only_while_it_shows() {
        let bindings = Bindings::default();
        assert_eq!(
            means(&bindings, &[], Key::Digit1, FLYING),
            flight(FlightCommand::StellarSlot1)
        );
        assert_eq!(
            means(&bindings, &[], Key::Digit1, MENU_SHOWN),
            Some(Command::EscortMenu(EscortMenuCommand::Group1))
        );
        assert_eq!(
            means(&bindings, &[Key::MetaLeft], Key::Digit5, MENU_SHOWN),
            Some(Command::EscortMenu(EscortMenuCommand::Group5)),
            "the menu hides Command-5 from the slot block too"
        );
        assert_eq!(
            means(&bindings, &[Key::MetaLeft], Key::Digit5, FLYING),
            flight(FlightCommand::NearestStellar)
        );
        assert_eq!(
            means(&bindings, &[], Key::Digit5, FLYING),
            None,
            "5 alone picks nothing"
        );
    }

    #[test]
    fn a_twin_resolves_to_its_command_and_a_bound_key_beats_it() {
        let bindings = Bindings::default();
        assert_eq!(
            means(&bindings, &[], Key::F1, FLYING),
            flight(FlightCommand::StellarSlot1)
        );
        assert_eq!(
            means(&bindings, &[], Key::F1, MENU_SHOWN),
            flight(FlightCommand::StellarSlot1),
            "F1-F4 always pick a stellar"
        );
        assert_eq!(
            means(&bindings, &[], Key::F5, FLYING),
            flight(FlightCommand::NearestStellar)
        );
        assert_eq!(
            means(&bindings, &[], Key::NumpadAdd, FLYING),
            flight(FlightCommand::ZoomIn)
        );
        assert_eq!(
            means(&bindings, &[], Key::NumpadEnter, &[Context::Ui]),
            Some(Command::Ui(UiCommand::Confirm))
        );
        assert_eq!(means(&bindings, &[], Key::NumpadEnter, FLYING), None);
        let f1_lands = Bindings::default().rebound(FlightCommand::LandDock, KeyCombo::key(Key::F1));
        assert_eq!(
            means(&f1_lands, &[], Key::F1, FLYING),
            flight(FlightCommand::LandDock)
        );
    }

    #[test]
    fn a_twin_stays_when_its_command_is_rebound() {
        let bindings =
            Bindings::default().rebound(FlightCommand::StellarSlot1, KeyCombo::key(Key::KeyQ));
        assert_eq!(
            means(&bindings, &[], Key::KeyQ, FLYING),
            flight(FlightCommand::StellarSlot1)
        );
        assert_eq!(
            means(&bindings, &[], Key::F1, FLYING),
            flight(FlightCommand::StellarSlot1)
        );
        assert_eq!(means(&bindings, &[], Key::Digit1, FLYING), None);
    }

    #[test]
    fn a_variant_follows_its_rebound_base_in_the_lookup() {
        let bindings =
            Bindings::default().rebound(FlightCommand::TargetSelect, KeyCombo::key(Key::KeyT));
        assert_eq!(
            means(&bindings, &[Key::ShiftLeft], Key::KeyT, FLYING),
            flight(FlightCommand::TargetSelectBack)
        );
        assert_eq!(means(&bindings, &[Key::ShiftLeft], Key::Tab, FLYING), None);
    }

    #[test]
    fn a_release_or_other_input_resolves_to_nothing_here() {
        let bindings = Bindings::default();
        let devices = Devices::default();
        let release = Input::Key {
            key: Key::ArrowUp,
            pressed: false,
            repeat: false,
        };
        assert_eq!(bindings.resolve(&release, &devices, FLYING), None);
        assert_eq!(bindings.resolve(&Input::Text('a'), &devices, FLYING), None);
    }

    #[test]
    fn held_holds_every_command_a_key_down_meets() {
        let bindings = Bindings::default();
        let mut devices = Devices::default();
        for key in [Key::AltLeft, Key::KeyA, Key::ArrowUp] {
            devices.follow(&press(key));
        }
        let held = bindings.held(&devices, FLYING);
        assert!(held.contains(&Command::Flight(FlightCommand::FaceStellar)));
        assert!(held.contains(&Command::Flight(FlightCommand::Autopilot)));
        assert!(held.contains(&Command::Flight(FlightCommand::Accelerate)));
        assert_eq!(held.len(), 3);
    }

    #[test]
    fn the_default_map_has_no_clash() {
        assert_eq!(Bindings::default().clashes(), vec![]);
    }

    #[test]
    fn two_commands_on_one_combo_in_contexts_active_together_clash() {
        let bindings =
            Bindings::default().rebound(PlayCommand::GalaxyMap, KeyCombo::key(Key::KeyL));
        assert_eq!(
            bindings.clashes(),
            vec![Clash {
                first: Command::Flight(FlightCommand::LandDock),
                second: Command::Play(PlayCommand::GalaxyMap),
            }]
        );
        assert_eq!(
            Bindings::default().clashes_with(PlayCommand::GalaxyMap, KeyCombo::key(Key::KeyL)),
            vec![Command::Flight(FlightCommand::LandDock)]
        );
    }

    #[test]
    fn commands_of_contexts_never_active_together_do_not_clash() {
        let bindings = Bindings::default().rebound(UiCommand::Confirm, KeyCombo::key(Key::KeyL));
        assert_eq!(bindings.clashes(), vec![]);
        assert_eq!(
            bindings.clashes_with(UiCommand::Back, KeyCombo::key(Key::KeyL)),
            vec![Command::Ui(UiCommand::Confirm)]
        );
    }

    #[test]
    fn a_shadowing_context_does_not_clash_with_the_one_it_shadows() {
        assert_eq!(
            Bindings::default().clashes_with(EscortMenuCommand::Group1, KeyCombo::key(Key::Digit1)),
            vec![]
        );
        let bindings =
            Bindings::default().rebound(EscortMenuCommand::Group1, KeyCombo::key(Key::KeyL));
        assert_eq!(bindings.clashes(), vec![]);
    }

    #[test]
    fn a_clash_query_over_given_screens_reads_their_lists() {
        const BOTH: ScreenContexts = ScreenContexts {
            name: "both",
            entries: &[
                Entry {
                    context: Context::Pause,
                    conditional: false,
                },
                Entry {
                    context: Context::Ui,
                    conditional: false,
                },
            ],
        };
        let bindings = Bindings::default();
        assert_eq!(
            bindings.clashes_on(&[BOTH]),
            vec![Clash {
                first: Command::Pause(PauseCommand::Pause),
                second: Command::Ui(UiCommand::Back),
            }]
        );
        assert_eq!(bindings.clashes_on(&[FLIGHT, DIALOG]), vec![]);
    }

    #[test]
    fn an_unbound_command_answers_nothing() {
        let bindings = Bindings::default().unbound(FlightCommand::WeaponSafety);
        assert_eq!(means(&bindings, &[], Key::KeyS, FLYING), None);
    }

    #[test]
    fn binding_s_to_another_flight_command_takes_it_from_weapon_safety() {
        let s = KeyCombo::key(Key::KeyS);
        let defaults = Bindings::default();
        let losers = defaults.clashes_with(FlightCommand::Board, s);
        assert_eq!(losers, vec![Command::Flight(FlightCommand::WeaponSafety)]);
        // The conflict rule: each command that clashes loses the key.
        let bindings = losers
            .into_iter()
            .fold(defaults, Bindings::unbound)
            .rebound(FlightCommand::Board, s);
        assert_eq!(
            bindings.keyboard.combo(FlightCommand::WeaponSafety.into()),
            None
        );
        assert_eq!(
            means(&bindings, &[], Key::KeyS, FLYING),
            flight(FlightCommand::Board)
        );
        let mut devices = Devices::default();
        devices.follow(&press(Key::KeyS));
        assert!(
            !bindings
                .held(&devices, FLYING)
                .contains(&Command::Flight(FlightCommand::WeaponSafety))
        );
    }

    #[test]
    fn devices_note_what_each_press_meant_until_its_release() {
        let mut devices = Devices::default();
        let down = press(Key::KeyX);
        devices.follow(&down);
        assert!(devices.any_down());
        devices.note_press(&down, Some(Command::Flight(FlightCommand::Eject)));
        let up = Input::Key {
            key: Key::KeyX,
            pressed: false,
            repeat: false,
        };
        assert_eq!(
            devices.press_of(&up),
            Some(Command::Flight(FlightCommand::Eject))
        );
        devices.follow(&up);
        assert_eq!(devices.press_of(&up), None);
        assert!(!devices.any_down());
        devices.follow(&down);
        devices.note_press(&down, Some(Command::Flight(FlightCommand::Eject)));
        devices.note_press(&down, None);
        assert_eq!(devices.press_of(&down), None);
        devices.let_go();
        assert!(!devices.any_down());
        assert!(!devices.keyboard().is_down(Key::KeyX));
        assert_eq!(devices.press_of(&Input::Text('x')), None);
    }

    #[test]
    fn a_modifier_input_is_a_modifier_key_s() {
        assert!(is_modifier_input(&press(Key::ShiftRight)));
        assert!(is_modifier_input(&press(Key::CapsLock)));
        assert!(!is_modifier_input(&press(Key::KeyA)));
        assert!(!is_modifier_input(&Input::Text('a')));
    }
}
