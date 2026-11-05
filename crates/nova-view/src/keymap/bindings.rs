//! Every device's binding table, the lookup over them, and the clash
//! query.

use std::collections::{BTreeMap, BTreeSet};

use super::command::{Command, Context};
use super::contexts::{SCREENS, ScreenContexts, router_shadows, shadows, together_on};
use super::gamepad::{self, Gamepad, PadMap};
use super::keyboard::{self, KeyCombo, KeyMap, Keyboard, KeysDown};
use crate::input::Input;

/// One binding table per device family: the keyboard's and the
/// gamepad's. The settings file saves each family's table on its own
/// ([`KeyMap::to_table`]); the gamepad's is not saved until phase 12
/// (rdm:phase/key-bindings/phase-12-gamepad-rebind-glyphs).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bindings {
    /// The keyboard's table.
    pub keyboard: KeyMap,
    /// The gamepad's table.
    pub gamepad: PadMap,
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
    /// modifiers meet; for a pad button's press, the command its entry
    /// under the layer held meets. A release, or any other input, means
    /// none here: [`CommandReader`](super::CommandReader) reports a
    /// release as the command its press meant.
    #[must_use]
    pub fn resolve(&self, input: &Input, devices: &Devices, active: &[Context]) -> Option<Command> {
        match input {
            Input::PadButton { .. } => {
                gamepad::resolve(&self.gamepad, input, &devices.gamepad, active)
            }
            _ => keyboard::resolve(&self.keyboard, input, devices.keyboard.down(), active),
        }
    }

    /// Every command `devices` hold down in the `active` contexts: each
    /// one a key down means with the modifiers held, and each one a pad
    /// button down means under its layer.
    #[must_use]
    pub fn held(&self, devices: &Devices, active: &[Context]) -> BTreeSet<Command> {
        let mut held = keyboard::held(&self.keyboard, devices.keyboard.down(), active);
        held.extend(gamepad::held(&self.gamepad, &devices.gamepad, active));
        held
    }

    /// Every command the device input of `input` (a press, a repeat or a
    /// release) holds while it is down, with what `devices` hold beside
    /// it, in the `active` contexts. Empty for any other input.
    #[must_use]
    pub fn meanings(&self, input: &Input, devices: &Devices, active: &[Context]) -> Vec<Command> {
        match input {
            Input::PadButton { .. } => {
                gamepad::meanings_with(&self.gamepad, input, &devices.gamepad, active)
            }
            _ => keyboard::meanings_with(&self.keyboard, input, devices.keyboard.down(), active),
        }
    }

    /// Every command of the `active` contexts bound to the device input
    /// of `input`, whatever else its binding needs held, then the active
    /// contexts' fixed twins on it (a key's). Empty for any other input.
    #[must_use]
    pub fn bound_to(&self, input: &Input, active: &[Context]) -> Vec<Command> {
        match input {
            Input::PadButton { .. } => gamepad::bound_to(&self.gamepad, input, active),
            _ => keyboard::bound_to(&self.keyboard, input, active),
        }
    }

    /// How far the stick axis `command` is bound to is pushed, through
    /// the flight dead zone, with `devices` as they are: -1 to 1, none
    /// for a command bound to no axis.
    #[must_use]
    pub fn axis(&self, command: Command, devices: &Devices) -> f32 {
        gamepad::axis_amount(&self.gamepad, &devices.gamepad, command)
    }

    /// Whether a stick axis flight reads lies outside the flight dead
    /// zone, with `devices` as they are.
    #[must_use]
    pub fn flight_axis_pushed(&self, devices: &Devices) -> bool {
        devices.gamepad.flight_axis_pushed(&self.gamepad)
    }

    /// Every pair of commands bound to one pad entry in contexts active
    /// together on some screen ([`PadMap::clashes`]), kept apart from the
    /// keyboard's ([`Bindings::clashes`]), so a key's rebind never
    /// unbinds a pad entry. `PadMap::default()` has none.
    #[must_use]
    pub fn pad_clashes(&self) -> Vec<Clash> {
        self.gamepad.clashes()
    }

    /// Whether the device input of `input` means `command` as one of its
    /// fixed twins, not through its binding.
    #[must_use]
    pub fn is_twin(&self, input: &Input, command: Command) -> bool {
        keyboard::is_twin(&self.keyboard, input, command)
    }

    /// Every pair of commands bound to one combo in contexts active
    /// together on some screen, a shadowing context's pairs with the
    /// context it shadows left out. `KeyMap::default()` has none.
    #[must_use]
    pub fn clashes(&self) -> Vec<Clash> {
        self.clashes_on(SCREENS)
    }

    /// The commands that would clash with `command` were it bound to
    /// `combo`, in command order: worked out on the bindings the rebind
    /// would make, so a command that follows `command`
    /// ([`Binding::Follows`](super::Binding::Follows)) and moves with it
    /// counts too. Each is the other side of a [`Bindings::clashes`] pair
    /// that involves `command` or one of its followers. A follower that
    /// clashes with `command` itself is named, and of two followers that
    /// clash, the later: with Target Select on Shift-T, its Shift variant
    /// means what it does, as Shift-Option-T its Option variant does.
    #[must_use]
    pub fn clashes_with(&self, command: impl Into<Command>, combo: KeyCombo) -> Vec<Command> {
        let command = command.into();
        self.clone().rebound(command, combo).losers_to(command)
    }

    /// Binds `command` to `combo` on the keyboard, then unbinds, one at a
    /// time and first in command order, each command that clashes with it
    /// or with a command following it, until none does: so `command`, and
    /// each follower that keeps its key, is never unbound. A variant whose
    /// base is unbound so loses its key too, still following its base.
    /// Returns, in command order, every command this left unbound that
    /// was bound before, those variants included: the keys screen marks
    /// them.
    pub fn assign(&mut self, command: impl Into<Command>, combo: KeyCombo) -> Vec<Command> {
        let command = command.into();
        let before = self.clone();
        self.keyboard.bind(command, combo);
        while let Some(&loser) = self.losers_to(command).first() {
            self.keyboard.unbind(loser);
        }
        Command::all()
            .filter(|&other| {
                before.keyboard.combo(other).is_some() && self.keyboard.combo(other).is_none()
            })
            .collect()
    }

    /// The commands that clash with `command`, or with one of its
    /// followers, as these bindings stand, in command order.
    fn losers_to(&self, command: Command) -> Vec<Command> {
        let mut moved = self.keyboard.followers(command);
        moved.push(command);
        let mut losers = BTreeSet::new();
        for Clash { first, second } in self.clashes() {
            match (moved.contains(&first), moved.contains(&second)) {
                (true, false) => {
                    losers.insert(second);
                }
                (false, true) => {
                    losers.insert(first);
                }
                (true, true) => {
                    losers.insert(if second == command { first } else { second });
                }
                (false, false) => {}
            }
        }
        losers.into_iter().collect()
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
/// contexts are active together, neither shadows the other, and neither
/// command is the router's deliberate shadow of the other.
fn clash_between(screens: &[ScreenContexts], a: Command, b: Command) -> bool {
    if router_shadows(a, b) || router_shadows(b, a) {
        return false;
    }
    let (a, b) = (a.context(), b.context());
    together_on(screens, a, b) && !shadows(a, b) && !shadows(b, a)
}

/// What every device holds down, and what each press meant: the
/// keyboard's and the gamepad's, with the pad's stick axes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Devices {
    /// The keyboard's state.
    keyboard: Keyboard,
    /// The gamepad's state.
    gamepad: Gamepad,
}

/// A device's input going down or up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    /// Down (`true`) or up.
    pub pressed: bool,
    /// Whether the press is the OS repeating a held input; never set on a
    /// release.
    pub repeat: bool,
}

impl Devices {
    /// How `input` changes what is down, if it is a device's button (a
    /// key or a pad button).
    #[must_use]
    pub fn change(input: &Input) -> Option<Change> {
        keyboard::change(input).or_else(|| gamepad::change(input))
    }

    /// A fresh press of the device input `input` presses, repeats or
    /// releases; `None` for any other input.
    #[must_use]
    pub fn fresh_press(input: &Input) -> Option<Input> {
        keyboard::fresh_press(input).or_else(|| gamepad::fresh_press(input))
    }

    /// Whether `input` is a modifier the `active` contexts read: a
    /// modifier key whose side they poll.
    #[must_use]
    pub fn reads_modifier(input: &Input, active: &[Context]) -> bool {
        keyboard::reads_modifier(input, active)
    }

    /// Follows `input`'s change to what is down, or a stick axis's move,
    /// a pad press taking its layer from `bindings`' pad table. Says
    /// whether it was a device's input.
    pub fn follow(&mut self, input: &Input, bindings: &Bindings) -> bool {
        self.keyboard.follow(input) || self.gamepad.follow(input, &bindings.gamepad)
    }

    /// Notes that `input`, a press, meant `command`.
    pub fn note_press(&mut self, input: &Input, command: Option<Command>) {
        self.keyboard.note_press(input, command);
        self.gamepad.note_press(input, command);
    }

    /// What the press of the device input `input` presses, repeats or
    /// releases meant, if it was noted and is still down.
    #[must_use]
    pub fn press_of(&self, input: &Input) -> Option<Command> {
        self.keyboard
            .press_of(input)
            .or_else(|| self.gamepad.press_of(input))
    }

    /// What the press a pad button release `input` ends meant, its layer
    /// let go of first or not, and once followed too
    /// ([`Gamepad::release_of`]).
    /// `None` for a key, whose release [`Devices::press_of`] answers
    /// while it is down, or any other input.
    #[must_use]
    pub fn release_of(&self, input: &Input) -> Option<Command> {
        self.gamepad.release_of(input)
    }

    /// Whether anything is down: a key or a pad button.
    #[must_use]
    pub fn any_down(&self) -> bool {
        self.keyboard.down().keys().next().is_some() || self.gamepad.buttons().next().is_some()
    }

    /// The gamepad's state.
    #[must_use]
    pub const fn gamepad(&self) -> &Gamepad {
        &self.gamepad
    }

    /// Whether a modifier is down, as any of the `active` contexts reads
    /// the modifiers.
    #[must_use]
    pub fn modifier_down(&self, active: &[Context]) -> bool {
        self.keyboard.down().modifier_down(active)
    }

    /// The keyboard's keys down.
    #[must_use]
    pub const fn keyboard(&self) -> &KeysDown {
        self.keyboard.down()
    }

    /// Lets go of everything, the pad's axes centred.
    pub fn let_go(&mut self) {
        self.keyboard.let_go();
        self.gamepad.let_go();
    }
}

#[cfg(test)]
mod tests {
    use super::super::command::{
        AppCommand, BarCommand, EscortMenuCommand, FlightCommand, HaggleCommand, IntroCommand,
        Kind, MainScreenCommand, MapCommand, MissionComputerCommand, OutfitterCommand,
        PauseCommand, PlanetCommCommand, PlayCommand, PlayerInfoCommand, PortCommand, RaceCommand,
        TradeCommand, UiCommand,
    };
    use super::super::contexts::{DIALOG, Entry, FLIGHT};
    use super::super::keyboard::Modifiers;
    use super::*;
    use crate::input::Key;

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
            devices.follow(&press(down), &Bindings::default());
        }
        let input = press(key);
        devices.follow(&input, &Bindings::default());
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
    fn the_escort_menu_hides_a_key_it_binds_even_when_its_modifiers_are_not_held() {
        let bindings = Bindings::default().rebound(
            EscortMenuCommand::Group1,
            KeyCombo::with(Key::Digit1, Modifiers::SHIFT),
        );
        assert_eq!(
            means(&bindings, &[], Key::Digit1, MENU_SHOWN),
            None,
            "flight's slot 1 is shadowed on 1 while the menu shows"
        );
        assert_eq!(
            means(&bindings, &[Key::ShiftLeft], Key::Digit1, MENU_SHOWN),
            Some(Command::EscortMenu(EscortMenuCommand::Group1))
        );
        assert_eq!(
            means(&bindings, &[], Key::Digit1, FLYING),
            flight(FlightCommand::StellarSlot1)
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
            devices.follow(&press(key), &Bindings::default());
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
        // The mission computer's Leave, on L beside the play keys too, moved
        // off it, so flight's L is the one left.
        let base =
            Bindings::default().rebound(MissionComputerCommand::Leave, KeyCombo::key(Key::KeyQ));
        let bindings = base
            .clone()
            .rebound(PlayCommand::GalaxyMap, KeyCombo::key(Key::KeyL));
        assert_eq!(
            bindings.clashes(),
            vec![Clash {
                first: Command::Flight(FlightCommand::LandDock),
                second: Command::Play(PlayCommand::GalaxyMap),
            }]
        );
        assert_eq!(
            base.clashes_with(PlayCommand::GalaxyMap, KeyCombo::key(Key::KeyL)),
            vec![Command::Flight(FlightCommand::LandDock)]
        );
    }

    #[test]
    fn double_time_s_key_clashes_with_every_screen_s_commands() {
        let caps = KeyCombo::key(Key::CapsLock);
        let found = Bindings::default().clashes_with(FlightCommand::Accelerate, caps);
        assert_eq!(found, vec![Command::App(AppCommand::DoubleTime)]);
        let found = Bindings::default().clashes_with(UiCommand::Confirm, caps);
        assert_eq!(found, vec![Command::App(AppCommand::DoubleTime)]);
        let found = Bindings::default().clashes_with(MainScreenCommand::Fly, caps);
        assert_eq!(found, vec![Command::App(AppCommand::DoubleTime)]);
    }

    #[test]
    fn commands_of_contexts_never_active_together_do_not_clash() {
        // F13, which no screen binds, so only flight's L could meet it.
        let bindings = Bindings::default()
            .rebound(FlightCommand::LandDock, KeyCombo::key(Key::F13))
            .rebound(UiCommand::Confirm, KeyCombo::key(Key::F13));
        assert_eq!(bindings.clashes(), vec![]);
        assert_eq!(
            bindings.clashes_with(UiCommand::Back, KeyCombo::key(Key::F13)),
            vec![Command::Ui(UiCommand::Confirm)]
        );
    }

    #[test]
    fn a_dialog_s_letter_clashes_with_a_shared_ui_command_on_its_key() {
        // Q, which no other screen beside the shared UI context binds (L
        // is the mission computer's Leave too).
        let q = KeyCombo::key(Key::KeyQ);
        let bindings = Bindings::default()
            .rebound(HaggleCommand::Lower, q)
            .rebound(UiCommand::Confirm, q);
        assert_eq!(
            bindings.clashes(),
            vec![Clash {
                first: Command::Ui(UiCommand::Confirm),
                second: Command::Haggle(HaggleCommand::Lower),
            }]
        );
    }

    #[test]
    fn a_spaceport_command_rebound_to_return_clashes_with_confirm() {
        let enter = KeyCombo::key(Key::Enter);
        let bindings = Bindings::default().rebound(PortCommand::Bar, enter);
        assert_eq!(
            bindings.clashes(),
            vec![Clash {
                first: Command::Ui(UiCommand::Confirm),
                second: Command::Port(PortCommand::Bar),
            }]
        );
        assert_eq!(
            Bindings::default().clashes_with(PortCommand::Bar, enter),
            vec![Command::Ui(UiCommand::Confirm)]
        );
    }

    #[test]
    fn the_spaceport_s_and_the_bar_s_second_letters_are_twins_of_their_buttons() {
        let bindings = Bindings::default();
        let port = &[Context::Port, Context::Play, Context::Ui];
        let bar = &[Context::Bar, Context::Play, Context::Ui];
        for (key, command) in [
            (Key::KeyF, Command::Port(PortCommand::Recharge)),
            (Key::KeyT, Command::Port(PortCommand::TradeCenter)),
        ] {
            assert_eq!(means(&bindings, &[], key, port), Some(command), "{key}");
        }
        for (key, command) in [
            (Key::KeyE, Command::Bar(BarCommand::Hire)),
            (Key::KeyR, Command::Bar(BarCommand::Gamble)),
            (Key::KeyW, Command::Bar(BarCommand::News)),
        ] {
            assert_eq!(means(&bindings, &[], key, bar), Some(command), "{key}");
        }
        assert_eq!(
            means(&bindings, &[], Key::KeyE, port),
            None,
            "the bar's twins answer only in the bar"
        );
    }

    #[test]
    fn either_option_holds_the_trade_center_s_and_the_outfitter_s_and_the_left_one_the_race_s() {
        let bindings = Bindings::default();
        let held = |key: Key, active: &[Context]| {
            let mut devices = Devices::default();
            devices.follow(&press(key), &Bindings::default());
            bindings.held(&devices, active)
        };
        let trade = &[Context::Trade, Context::Play, Context::Ui];
        let outfitter = &[Context::Outfitter, Context::Play, Context::Ui];
        let race = &[Context::Race, Context::Ui];
        for key in [Key::AltLeft, Key::AltRight] {
            assert!(held(key, trade).contains(&Command::Trade(TradeCommand::MaxLot)));
            assert!(held(key, outfitter).contains(&Command::Outfitter(OutfitterCommand::Count)));
        }
        assert!(held(Key::AltLeft, race).contains(&Command::Race(RaceCommand::Ask)));
        assert!(held(Key::AltRight, race).is_empty());
    }

    #[test]
    fn the_planet_comm_s_second_letters_are_twins_of_its_buttons() {
        let bindings = Bindings::default();
        let planet_comm = &[Context::PlanetComm, Context::Ui];
        assert_eq!(
            means(&bindings, &[], Key::KeyB, planet_comm),
            Some(Command::PlanetComm(PlanetCommCommand::OfferBribe))
        );
        assert_eq!(
            means(&bindings, &[], Key::KeyT, planet_comm),
            Some(Command::PlanetComm(PlanetCommCommand::DemandTribute))
        );
        let rebound = bindings.rebound(PlanetCommCommand::OfferBribe, KeyCombo::key(Key::KeyQ));
        assert_eq!(
            means(&rebound, &[], Key::KeyB, planet_comm),
            Some(Command::PlanetComm(PlanetCommCommand::OfferBribe)),
            "the twin stays put"
        );
        assert_eq!(means(&rebound, &[], Key::KeyO, planet_comm), None);
    }

    #[test]
    fn shift_tab_on_the_player_info_takes_tab_back_and_tab_alone_the_shared_tab() {
        let bindings = Bindings::default();
        let player_info = &[Context::PlayerInfo, Context::Play, Context::Ui];
        assert_eq!(
            means(&bindings, &[Key::ShiftRight], Key::Tab, player_info),
            Some(Command::PlayerInfo(PlayerInfoCommand::TabBack))
        );
        assert_eq!(
            means(&bindings, &[], Key::Tab, player_info),
            Some(Command::Ui(UiCommand::Tab))
        );
    }

    #[test]
    fn the_map_s_twins_are_s_the_keypad_s_plus_and_backslash_back_with_shift() {
        let bindings = Bindings::default();
        let course = &[Context::Map, Context::Play, Context::Ui];
        assert_eq!(
            means(&bindings, &[], Key::KeyS, course),
            Some(Command::Map(MapCommand::Find))
        );
        assert_eq!(
            means(&bindings, &[], Key::NumpadAdd, course),
            Some(Command::Map(MapCommand::ZoomIn))
        );
        assert_eq!(
            means(&bindings, &[], Key::Backslash, course),
            Some(Command::Ui(UiCommand::Tab))
        );
        for shift in [Key::ShiftLeft, Key::ShiftRight] {
            assert_eq!(
                means(&bindings, &[shift], Key::Backslash, course),
                Some(Command::Map(MapCommand::TabBack)),
                "{shift}"
            );
            assert_eq!(
                means(&bindings, &[shift], Key::Tab, course),
                Some(Command::Map(MapCommand::TabBack)),
                "{shift}"
            );
        }
        let tab_on_q = bindings.rebound(UiCommand::Tab, KeyCombo::key(Key::KeyQ));
        assert_eq!(
            means(&tab_on_q, &[Key::ShiftLeft], Key::Backslash, course),
            Some(Command::Map(MapCommand::TabBack)),
            "the twin stays put"
        );
        assert_eq!(
            means(&tab_on_q, &[Key::ShiftLeft], Key::KeyQ, course),
            Some(Command::Map(MapCommand::TabBack))
        );
    }

    #[test]
    fn a_twin_ranks_by_its_own_modifiers_not_its_command_s_binding() {
        let bindings = Bindings::default()
            .rebound(UiCommand::Tab, KeyCombo::with(Key::Tab, Modifiers::CONTROL))
            .rebound(MapCommand::TabBack, KeyCombo::key(Key::KeyQ));
        let course = &[Context::Map, Context::Play, Context::Ui];
        assert_eq!(
            means(&bindings, &[Key::ShiftLeft], Key::Backslash, course),
            Some(Command::Map(MapCommand::TabBack)),
            "Shift-\\ is Shift-Tab's twin, needing Shift, whatever Tab is bound to"
        );
        assert_eq!(
            means(&bindings, &[], Key::Backslash, course),
            Some(Command::Ui(UiCommand::Tab))
        );
    }

    #[test]
    fn the_map_s_left_shift_alone_is_held_to_plot_the_course() {
        let bindings = Bindings::default();
        let course = &[Context::Map, Context::Play, Context::Ui];
        let held = |key: Key| {
            let mut devices = Devices::default();
            devices.follow(&press(key), &Bindings::default());
            bindings.held(&devices, course)
        };
        assert!(held(Key::ShiftLeft).contains(&Command::Map(MapCommand::PlotCourse)));
        assert!(held(Key::ShiftRight).is_empty());
    }

    #[test]
    fn wasd_are_the_system_view_s_twins_of_the_arrows() {
        let bindings = Bindings::default();
        let system = &[Context::SystemView, Context::Ui];
        for (key, command) in [
            (Key::KeyW, UiCommand::Up),
            (Key::KeyA, UiCommand::Left),
            (Key::KeyS, UiCommand::Down),
            (Key::KeyD, UiCommand::Right),
        ] {
            assert_eq!(
                means(&bindings, &[], key, system),
                Some(Command::Ui(command)),
                "{key}"
            );
            assert_eq!(means(&bindings, &[], key, &[Context::Ui]), None, "{key}");
        }
    }

    #[test]
    fn the_intro_reads_return_and_space_and_pause_but_not_the_keypad_s_enter() {
        let bindings = Bindings::default();
        let intro = &[Context::Intro, Context::Pause];
        assert_eq!(
            means(&bindings, &[], Key::Enter, intro),
            Some(Command::Intro(IntroCommand::Confirm))
        );
        assert_eq!(
            means(&bindings, &[], Key::Space, intro),
            Some(Command::Intro(IntroCommand::Activate))
        );
        assert_eq!(
            means(&bindings, &[], Key::Escape, intro),
            Some(Command::Pause(PauseCommand::Pause))
        );
        assert_eq!(means(&bindings, &[], Key::NumpadEnter, intro), None);
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
    fn the_router_s_fly_shadows_the_galaxy_viewer_s_find_by_design() {
        let f = KeyCombo::key(Key::KeyF);
        assert_eq!(
            Bindings::default().clashes_with(MapCommand::Find, f),
            vec![],
            "the router reads F first; the viewer finds through S"
        );
        let q = KeyCombo::key(Key::KeyQ);
        let both = Bindings::default()
            .rebound(MainScreenCommand::Fly, q)
            .rebound(MapCommand::Find, q);
        assert_eq!(both.clashes(), vec![], "wherever the two go");
    }

    #[test]
    fn a_map_command_on_a_key_the_router_reads_on_the_galaxy_viewer_clashes() {
        let f = KeyCombo::key(Key::KeyF);
        let bindings = Bindings::default().rebound(MapCommand::Borders, f);
        assert_eq!(
            bindings.clashes(),
            vec![
                Clash {
                    first: Command::Map(MapCommand::Borders),
                    second: Command::Map(MapCommand::Find),
                },
                Clash {
                    first: Command::Map(MapCommand::Borders),
                    second: Command::MainScreen(MainScreenCommand::Fly),
                },
            ]
        );
        assert_eq!(
            Bindings::default().clashes_with(MapCommand::ZoomIn, f),
            vec![
                Command::Map(MapCommand::Find),
                Command::MainScreen(MainScreenCommand::Fly),
            ],
            "not flight's Attack Target, which no map reads"
        );
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
        devices.follow(&press(Key::KeyS), &Bindings::default());
        assert!(
            !bindings
                .held(&devices, FLYING)
                .contains(&Command::Flight(FlightCommand::WeaponSafety))
        );
    }

    #[test]
    fn a_rebind_s_followers_clash_too() {
        // ClosestTarget on W moves ClosestShip to Option-W, which is
        // SelectSecondaryBack's key.
        let w = KeyCombo::key(Key::KeyW);
        assert_eq!(
            Bindings::default().clashes_with(FlightCommand::ClosestTarget, w),
            vec![
                Command::Flight(FlightCommand::SelectSecondary),
                Command::Flight(FlightCommand::SelectSecondaryBack),
            ]
        );
    }

    #[test]
    fn a_rebind_onto_its_own_variant_s_combo_clashes_with_the_variant() {
        // Shift-T moves TargetSelectBack onto Shift-T itself, and
        // TargetEscortBack onto TargetEscort's Shift-Option-T: of two
        // followers, the later loses.
        let shift_t = KeyCombo::with(Key::KeyT, Modifiers::SHIFT);
        assert_eq!(
            Bindings::default().clashes_with(FlightCommand::TargetSelect, shift_t),
            vec![
                Command::Flight(FlightCommand::TargetSelectBack),
                Command::Flight(FlightCommand::TargetEscortBack),
            ]
        );
    }

    #[test]
    fn a_clash_the_rebind_does_not_touch_is_not_reported() {
        let q = KeyCombo::key(Key::KeyQ);
        let clashing = Bindings::default()
            .rebound(HaggleCommand::Lower, q)
            .rebound(UiCommand::Confirm, q);
        assert_eq!(
            clashing.clashes_with(FlightCommand::Board, KeyCombo::key(Key::F13)),
            vec![]
        );
    }

    fn combo(bindings: &Bindings, command: impl Into<Command>) -> Option<KeyCombo> {
        bindings.keyboard.combo(command.into())
    }

    #[test]
    fn assigning_a_spaceport_command_escape_unbinds_back() {
        let mut bindings = Bindings::default();
        let escape = KeyCombo::key(Key::Escape);
        let unbound = bindings.assign(PortCommand::Bar, escape);
        assert_eq!(unbound, vec![Command::Ui(UiCommand::Back)]);
        assert_eq!(combo(&bindings, PortCommand::Bar), Some(escape));
        assert_eq!(combo(&bindings, UiCommand::Back), None);
        assert_eq!(
            combo(&bindings, PauseCommand::Pause),
            Some(escape),
            "Pause is never active with the spaceport"
        );
    }

    #[test]
    fn assigning_a_flight_command_a_spaceport_key_leaves_both() {
        let mut bindings = Bindings::default();
        let o = KeyCombo::key(Key::KeyO);
        assert_eq!(bindings.assign(FlightCommand::Board, o), vec![]);
        assert_eq!(combo(&bindings, FlightCommand::Board), Some(o));
        assert_eq!(combo(&bindings, PortCommand::Outfitter), Some(o));
    }

    #[test]
    fn assigning_target_select_shift_t_unbinds_its_own_shift_variant() {
        let mut bindings = Bindings::default();
        let shift_t = KeyCombo::with(Key::KeyT, Modifiers::SHIFT);
        assert_eq!(
            bindings.assign(FlightCommand::TargetSelect, shift_t),
            vec![
                Command::Flight(FlightCommand::TargetSelectBack),
                Command::Flight(FlightCommand::TargetEscortBack),
            ]
        );
        assert_eq!(combo(&bindings, FlightCommand::TargetSelect), Some(shift_t));
        assert_eq!(combo(&bindings, FlightCommand::TargetSelectBack), None);
        assert_eq!(
            combo(&bindings, FlightCommand::TargetEscort),
            Some(KeyCombo::with(
                Key::KeyT,
                Modifiers::SHIFT.union(Modifiers::OPTION)
            )),
            "the Option variant follows onto Shift-Option-T and keeps it"
        );
        assert_eq!(combo(&bindings, FlightCommand::TargetEscortBack), None);
        assert_eq!(bindings.clashes(), vec![]);
    }

    #[test]
    fn assigning_closest_target_w_unbinds_select_secondary_and_its_variant_with_it() {
        let mut bindings = Bindings::default();
        let unbound = bindings.assign(FlightCommand::ClosestTarget, KeyCombo::key(Key::KeyW));
        assert_eq!(
            unbound,
            vec![
                Command::Flight(FlightCommand::SelectSecondary),
                Command::Flight(FlightCommand::SelectSecondaryBack),
            ]
        );
        assert_eq!(
            bindings
                .keyboard
                .binding(FlightCommand::SelectSecondaryBack.into()),
            KeyMap::default().binding(FlightCommand::SelectSecondaryBack.into()),
            "the variant still follows its base, which lost its key"
        );
        assert_eq!(
            combo(&bindings, FlightCommand::ClosestShip),
            Some(KeyCombo::with(Key::KeyW, Modifiers::OPTION))
        );
    }

    #[test]
    fn assigning_reports_only_commands_it_unbound() {
        let mut bindings = Bindings::default().unbound(UiCommand::Back);
        assert_eq!(
            bindings.assign(PortCommand::Bar, KeyCombo::key(Key::Escape)),
            vec![],
            "Back was unbound already"
        );
        let mut bindings = Bindings::default().unbound(PortCommand::Bar);
        assert_eq!(
            bindings.assign(PortCommand::Bar, KeyCombo::key(Key::F13)),
            vec![],
            "the command itself goes from unbound to bound"
        );
    }

    #[test]
    fn no_assign_from_the_defaults_leaves_a_clash() {
        let combos = [
            KeyCombo::key(Key::KeyW),
            KeyCombo::key(Key::Escape),
            KeyCombo::key(Key::Enter),
            KeyCombo::key(Key::Tab),
            KeyCombo::with(Key::KeyT, Modifiers::SHIFT),
            KeyCombo::with(Key::KeyK, Modifiers::COMMAND),
            KeyCombo::key(Key::CapsLock),
            KeyCombo::key(Key::Digit1),
            KeyCombo::key(Key::KeyF),
        ];
        for command in Command::all().filter(|c| c.kind() != Kind::Axis) {
            for combo in combos {
                let mut bindings = Bindings::default();
                let unbound = bindings.assign(command, combo);
                assert_eq!(bindings.clashes(), vec![], "{command:?} on {combo:?}");
                assert_eq!(bindings.keyboard.combo(command), Some(combo), "{command:?}");
                assert!(!unbound.contains(&command), "{command:?}");
            }
        }
    }

    #[test]
    fn the_pad_s_clashes_are_its_table_s_own() {
        assert_eq!(Bindings::default().pad_clashes(), vec![]);
        let mut bindings = Bindings::default();
        bindings.gamepad.bind(
            Command::Flight(FlightCommand::LandDock),
            super::super::PadBinding::Entry(super::super::PadEntry::plain(
                crate::input::PadButton::South,
            )),
        );
        assert_eq!(
            bindings.pad_clashes(),
            bindings.gamepad.clashes(),
            "the pad table's"
        );
        assert_eq!(bindings.pad_clashes().len(), 1);
        assert_eq!(bindings.clashes(), vec![], "the keyboard's are apart");
    }

    #[test]
    fn devices_note_what_each_press_meant_until_its_release() {
        let mut devices = Devices::default();
        let down = press(Key::KeyX);
        devices.follow(&down, &Bindings::default());
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
        devices.follow(&up, &Bindings::default());
        assert_eq!(devices.press_of(&up), None);
        assert!(!devices.any_down());
        devices.follow(&down, &Bindings::default());
        devices.note_press(&down, Some(Command::Flight(FlightCommand::Eject)));
        devices.note_press(&down, None);
        assert_eq!(devices.press_of(&down), None);
        devices.let_go();
        assert!(!devices.any_down());
        assert!(!devices.keyboard().is_down(Key::KeyX));
        assert_eq!(devices.press_of(&Input::Text('x')), None);
    }
}
