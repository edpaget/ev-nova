//! The keyboard's side of the key map: its binding table, [`KeyMap`],
//! with the original's defaults and fixed twins, the keys it holds down,
//! [`KeysDown`], what each press meant, [`Keyboard`], and the lookup over
//! a key's input. This is the only place outside the platform adapter
//! and `input` that names a [`Key`] or reads an `Input::Key`.

use std::collections::{BTreeMap, BTreeSet};

use super::bindings::Change;
use super::command::{
    AppCommand, AssignmentCommand, BarCommand, CommCommand, Command, Context, EscortDialogCommand,
    EscortMenuCommand, FlightCommand, HaggleCommand, HireCommand, IntroCommand, MainMenuCommand,
    MainScreenCommand, MapCommand, MissionComputerCommand, OutfitterCommand, PauseCommand,
    PlanetCommCommand, PlayCommand, PlayerInfoCommand, PortCommand, RaceCommand, ShipyardCommand,
    TradeCommand, UiCommand,
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
    /// The fixed key.
    pub key: Key,
    /// The modifiers it needs held, as its context reads them; more held
    /// still meet it. Only a variant's twin needs any: `\` with Shift
    /// beside Shift-Tab on the map.
    pub modifiers: Modifiers,
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
    pub const TWINS: [Twin; 24] = [
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
        // `_PlanetCommFilter` reads B beside O for Offer Bribe and T beside
        // D for Demand Tribute (@0x911a9).
        twin(
            Context::PlanetComm,
            Key::KeyB,
            Command::PlanetComm(PlanetCommCommand::OfferBribe),
        ),
        twin(
            Context::PlanetComm,
            Key::KeyT,
            Command::PlanetComm(PlanetCommCommand::DemandTribute),
        ),
        // `_PortFilter` reads F beside R for Recharge (0x03, 0x0f) and T
        // beside C for the Trade Center (0x11, 0x08), @0x4f209-0x4f257.
        twin(
            Context::Port,
            Key::KeyF,
            Command::Port(PortCommand::Recharge),
        ),
        twin(
            Context::Port,
            Key::KeyT,
            Command::Port(PortCommand::TradeCenter),
        ),
        // `_BarFilter` reads E beside H for Hire Escort (14, 4), R beside
        // G for Gamble (0x0f, 0x05, @0x47b1c-0x47b31) and W beside N for
        // Holovid (0x0d, 0x2d, @0x47b33-0x47b48).
        twin(Context::Bar, Key::KeyE, Command::Bar(BarCommand::Hire)),
        twin(Context::Bar, Key::KeyR, Command::Bar(BarCommand::Gamble)),
        twin(Context::Bar, Key::KeyW, Command::Bar(BarCommand::News)),
        // The right Option, where a screen reads the `optionKey` bit
        // (`_DoTradeDialog` @0x5e211, `_DoOutfitDialog` @0x5c0c0), which
        // either Option sets.
        twin(
            Context::Trade,
            Key::AltRight,
            Command::Trade(TradeCommand::MaxLot),
        ),
        twin(
            Context::Outfitter,
            Key::AltRight,
            Command::Outfitter(OutfitterCommand::Count),
        ),
        // `_MapFilter` reads the keypad's `+` beside `=` (@0x14271-0x14293),
        // S beside F for Find (@0x144b9-0x144d6), and `\` beside Tab for
        // the cycle, back with the `shiftKey` bit as Tab's is (@0x13ee4):
        // so `\` with Shift is Shift-Tab's twin, listed first to win.
        twin(Context::Map, Key::NumpadAdd, map(MapCommand::ZoomIn)),
        twin(Context::Map, Key::KeyS, map(MapCommand::Find)),
        Twin {
            context: Context::Map,
            key: Key::Backslash,
            modifiers: Modifiers::SHIFT,
            command: map(MapCommand::TabBack),
        },
        twin(Context::Map, Key::Backslash, Command::Ui(UiCommand::Tab)),
        // The system view pans on WASD beside the arrows.
        twin(Context::SystemView, Key::KeyA, Command::Ui(UiCommand::Left)),
        twin(
            Context::SystemView,
            Key::KeyD,
            Command::Ui(UiCommand::Right),
        ),
        twin(Context::SystemView, Key::KeyW, Command::Ui(UiCommand::Up)),
        twin(Context::SystemView, Key::KeyS, Command::Ui(UiCommand::Down)),
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

/// A [`Twin`] that needs no modifier, for the const table.
const fn twin(context: Context, key: Key, command: Command) -> Twin {
    Twin {
        context,
        key,
        modifiers: Modifiers::NONE,
        command,
    }
}

/// A flight command, for the const tables.
const fn flight(command: FlightCommand) -> Command {
    Command::Flight(command)
}

/// A map command, for the const tables.
const fn map(command: MapCommand) -> Command {
    Command::Map(command)
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
pub(super) const fn default_binding(command: Command) -> Binding {
    match command {
        Command::Flight(command) => default_flight(command),
        Command::EscortMenu(command) => default_escort_menu(command),
        Command::Play(command) => default_play(command),
        Command::Pause(PauseCommand::Pause) => bare(Key::Escape),
        Command::Ui(command) => default_ui(command),
        Command::Comm(CommCommand::Close) => bare(Key::KeyE),
        Command::Haggle(command) => default_haggle(command),
        Command::PlanetComm(command) => default_planet_comm(command),
        Command::EscortDialog(command) => default_escort_dialog(command),
        Command::Assignment(command) => default_assignment(command),
        Command::PlayerInfo(PlayerInfoCommand::TabBack) => Binding::Follows {
            base: Command::Ui(UiCommand::Tab),
            plus: Modifiers::SHIFT,
        },
        Command::Port(command) => default_port(command),
        Command::Trade(command) => default_trade(command),
        Command::Outfitter(command) => default_outfitter(command),
        Command::Shipyard(ShipyardCommand::Buy) => bare(Key::KeyB),
        Command::Bar(command) => default_bar(command),
        Command::Hire(HireCommand::Hire) => bare(Key::KeyH),
        Command::MissionComputer(command) => default_mission_computer(command),
        Command::Race(RaceCommand::Ask) => bare(Key::AltLeft),
        Command::Map(command) => default_map(command),
        Command::Intro(command) => default_intro(command),
        Command::SystemView(command) => match command {},
        Command::App(AppCommand::DoubleTime) => bare(Key::CapsLock),
        Command::MainScreen(command) => default_main_screen(command),
        Command::MainMenu(command) => default_main_menu(command),
    }
}

/// The main screen's defaults: F, I and P.
const fn default_main_screen(command: MainScreenCommand) -> Binding {
    match command {
        MainScreenCommand::Fly => bare(Key::KeyF),
        MainScreenCommand::About => bare(Key::KeyI),
        MainScreenCommand::Preferences => bare(Key::KeyP),
    }
}

/// The main menu's defaults: A, X, and the S, L and E keys it polls.
const fn default_main_menu(command: MainMenuCommand) -> Binding {
    match command {
        MainMenuCommand::About => bare(Key::KeyA),
        MainMenuCommand::Text => bare(Key::KeyX),
        MainMenuCommand::OpenGuardS => bare(Key::KeyS),
        MainMenuCommand::OpenGuardL => bare(Key::KeyL),
        MainMenuCommand::EnterShip => bare(Key::KeyE),
    }
}

/// The galaxy map's defaults (`_MapFilter`): B, C, F, `-` and `=`, the
/// shared Tab with Shift, and the left Shift (`_DoSystemMap`).
const fn default_map(command: MapCommand) -> Binding {
    match command {
        MapCommand::Borders => bare(Key::KeyB),
        MapCommand::ClearRoute => bare(Key::KeyC),
        MapCommand::Find => bare(Key::KeyF),
        MapCommand::ZoomOut => bare(Key::Minus),
        MapCommand::ZoomIn => bare(Key::Equal),
        MapCommand::TabBack => Binding::Follows {
            base: Command::Ui(UiCommand::Tab),
            plus: Modifiers::SHIFT,
        },
        MapCommand::PlotCourse => bare(Key::ShiftLeft),
    }
}

/// The intro's defaults: Return and Space (`_ShowIntroScreen`).
const fn default_intro(command: IntroCommand) -> Binding {
    match command {
        IntroCommand::Confirm => bare(Key::Enter),
        IntroCommand::Activate => bare(Key::Space),
    }
}

/// The spaceport's defaults, each its button's initial (`_PortFilter`
/// @0x4f209-0x4f257).
const fn default_port(command: PortCommand) -> Binding {
    match command {
        PortCommand::Recharge => bare(Key::KeyR),
        PortCommand::TradeCenter => bare(Key::KeyC),
        PortCommand::Outfitter => bare(Key::KeyO),
        PortCommand::Shipyard => bare(Key::KeyS),
        PortCommand::MissionBbs => bare(Key::KeyN),
        PortCommand::Bar => bare(Key::KeyB),
    }
}

/// The Trade Center's defaults: B, S and the left Option (the shared Tab
/// with Shift, its `TabBack`, follows the Tab).
const fn default_trade(command: TradeCommand) -> Binding {
    match command {
        TradeCommand::Buy => bare(Key::KeyB),
        TradeCommand::Sell => bare(Key::KeyS),
        TradeCommand::MaxLot => bare(Key::AltLeft),
        TradeCommand::TabBack => Binding::Follows {
            base: Command::Ui(UiCommand::Tab),
            plus: Modifiers::SHIFT,
        },
    }
}

/// The outfitter's defaults: B, S and the left Option.
const fn default_outfitter(command: OutfitterCommand) -> Binding {
    match command {
        OutfitterCommand::Buy => bare(Key::KeyB),
        OutfitterCommand::Sell => bare(Key::KeyS),
        OutfitterCommand::Count => bare(Key::AltLeft),
    }
}

/// The bar's defaults: H, G and N (`_BarFilter`).
const fn default_bar(command: BarCommand) -> Binding {
    match command {
        BarCommand::Hire => bare(Key::KeyH),
        BarCommand::Gamble => bare(Key::KeyG),
        BarCommand::News => bare(Key::KeyN),
    }
}

/// The mission computer's defaults: L and A (`_MissionFilter`).
const fn default_mission_computer(command: MissionComputerCommand) -> Binding {
    match command {
        MissionComputerCommand::Leave => bare(Key::KeyL),
        MissionComputerCommand::Accept => bare(Key::KeyA),
    }
}

/// The haggle dialog's defaults: A and L (`_HaggleFilter` @0x91d9d).
const fn default_haggle(command: HaggleCommand) -> Binding {
    match command {
        HaggleCommand::Accept => bare(Key::KeyA),
        HaggleCommand::Lower => bare(Key::KeyL),
    }
}

/// The planet comm dialog's defaults, each its button's initial
/// (`_PlanetCommFilter` @0x911a9).
const fn default_planet_comm(command: PlanetCommCommand) -> Binding {
    match command {
        PlanetCommCommand::Close => bare(Key::KeyE),
        PlanetCommCommand::Greetings => bare(Key::KeyG),
        PlanetCommCommand::OfferBribe => bare(Key::KeyO),
        PlanetCommCommand::DemandTribute => bare(Key::KeyD),
        PlanetCommCommand::Release => bare(Key::KeyR),
    }
}

/// The escort dialog's defaults (`_EscortFilter` @0x950e9).
const fn default_escort_dialog(command: EscortDialogCommand) -> Binding {
    match command {
        EscortDialogCommand::Close => bare(Key::KeyC),
        EscortDialogCommand::Release => bare(Key::KeyR),
        EscortDialogCommand::Upgrade => bare(Key::KeyU),
        EscortDialogCommand::Sell => bare(Key::KeyS),
    }
}

/// The captured ship's assignment's defaults: M and E
/// (`_CapturedAssignmentFilter` @0x51782).
const fn default_assignment(command: AssignmentCommand) -> Binding {
    match command {
        AssignmentCommand::MyShip => bare(Key::KeyM),
        AssignmentCommand::Escort => bare(Key::KeyE),
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
        Context::Flight
        | Context::EscortMenu
        | Context::Play
        | Context::Pause
        | Context::Race
        | Context::Intro
        | Context::App => Sides::LeftOnly,
        Context::Ui
        | Context::Comm
        | Context::Haggle
        | Context::PlanetComm
        | Context::EscortDialog
        | Context::Assignment
        | Context::PlayerInfo
        | Context::Port
        | Context::Trade
        | Context::Outfitter
        | Context::Shipyard
        | Context::Bar
        | Context::Hire
        | Context::MissionComputer
        | Context::Map
        | Context::SystemView
        | Context::MainScreen
        | Context::MainMenu => Sides::Either,
    }
}

/// Every command `key` means with the modifiers `keys` holds, over the
/// `active` contexts in their order: the commands of [`matched`].
fn meanings(map: &KeyMap, key: Key, keys: &KeysDown, active: &[Context]) -> Vec<Command> {
    matched(map, key, keys, active)
        .into_iter()
        .map(|(command, _)| command)
        .collect()
}

/// Every command `key` means with the modifiers `keys` holds, over the
/// `active` contexts in their order, each with the modifiers what it
/// matched through needs (its combo's, or its twin's):
///
/// 1. A shadowing context's commands on the key ([`shadows`]), if any is
///    met: it wins its keys.
/// 2. Otherwise every other active context's commands met on the key,
///    leaving out a shadowed context's commands on a key its shadowing
///    context binds.
/// 3. Otherwise the fixed twins of the active contexts on the key whose
///    modifiers are held: a bound command beats a twin.
///
/// Every command returned is held while the key stays down; a press
/// means the most specific of them ([`most_specific`]).
fn matched(
    map: &KeyMap,
    key: Key,
    keys: &KeysDown,
    active: &[Context],
) -> Vec<(Command, Modifiers)> {
    let met = |context: Context| -> Vec<(Command, Modifiers)> {
        let held = keys.modifiers(sides(context));
        context
            .commands()
            .filter_map(|command| {
                map.combo(command)
                    .filter(|combo| combo.key == key && combo.met_by(held))
                    .map(|combo| (command, combo.modifiers))
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
    let first: Vec<(Command, Modifiers)> = active
        .iter()
        .filter(|&&context| shadowing(context))
        .flat_map(|&context| met(context))
        .collect();
    if !first.is_empty() {
        return first;
    }
    let bound: Vec<(Command, Modifiers)> = active
        .iter()
        .filter(|&&context| !shadowing(context) && !shadowed_here(context))
        .flat_map(|&context| met(context))
        .collect();
    if !bound.is_empty() {
        return bound;
    }
    KeyMap::TWINS
        .iter()
        .filter(|twin| {
            twin.key == key
                && active.contains(&twin.context)
                && keys.modifiers(sides(twin.context)).contains(twin.modifiers)
        })
        .map(|twin| (twin.command, twin.modifiers))
        .collect()
}

/// The command a press means among `matched`, which [`matched`] lists in
/// the active contexts' order: the one whose match needs the most
/// modifiers, so Option-A beats A and Shift-`\` the map's Shift-Tab twin
/// beats its Tab twin, and the first of those.
fn most_specific(matched: &[(Command, Modifiers)]) -> Option<Command> {
    matched
        .iter()
        .rev()
        .max_by_key(|(_, needs)| needs.count())
        .map(|&(command, _)| command)
}

/// The command `input` means in the `active` contexts, with `keys` down
/// before it: for a key press or repeat, the most specific command its
/// key and the held modifiers meet. A release, or any other input, means
/// none.
pub(super) fn resolve(
    map: &KeyMap,
    input: &Input,
    keys: &KeysDown,
    active: &[Context],
) -> Option<Command> {
    let Input::Key {
        key, pressed: true, ..
    } = *input
    else {
        return None;
    };
    most_specific(&matched(map, key, keys, active))
}

/// Every command the keys down hold in the `active` contexts: each one a
/// key down means with the modifiers held.
pub(super) fn held(map: &KeyMap, keys: &KeysDown, active: &[Context]) -> BTreeSet<Command> {
    keys.keys()
        .flat_map(|key| meanings(map, key, keys, active))
        .collect()
}

/// Every command the key of `input` (a press, a repeat or a release)
/// holds while down, with `keys` down beside it. Empty for any other
/// input.
pub(super) fn meanings_with(
    map: &KeyMap,
    input: &Input,
    keys: &KeysDown,
    active: &[Context],
) -> Vec<Command> {
    let Input::Key { key, .. } = *input else {
        return Vec::new();
    };
    let mut keys = keys.clone();
    keys.down.insert(key);
    meanings(map, key, &keys, active)
}

/// Every command of the `active` contexts bound to the key of `input`
/// (a press, a repeat or a release), whatever modifiers its combo needs,
/// then the active contexts' fixed twins on it, each in command order.
/// Empty for any other input.
pub(super) fn bound_to(map: &KeyMap, input: &Input, active: &[Context]) -> Vec<Command> {
    let Input::Key { key, .. } = *input else {
        return Vec::new();
    };
    let bound: BTreeSet<Command> = active
        .iter()
        .flat_map(|context| context.commands())
        .filter(|&command| map.combo(command).is_some_and(|combo| combo.key == key))
        .collect();
    let mut found: Vec<Command> = bound.into_iter().collect();
    for twin in &KeyMap::TWINS {
        if twin.key == key && active.contains(&twin.context) && !found.contains(&twin.command) {
            found.push(twin.command);
        }
    }
    found
}

/// Whether the key of `input` means `command` as one of its fixed twins
/// ([`KeyMap::TWINS`]), not through its binding. False for any other
/// input.
pub(super) fn is_twin(map: &KeyMap, input: &Input, command: Command) -> bool {
    let Input::Key { key, .. } = *input else {
        return false;
    };
    let bound = map.combo(command).is_some_and(|combo| combo.key == key);
    !bound
        && KeyMap::TWINS
            .iter()
            .any(|twin| twin.key == key && twin.command == command)
}

/// Whether the key of `input` is a modifier key the `active` contexts
/// read ([`sides`]). False for any other input.
pub(super) fn reads_modifier(input: &Input, active: &[Context]) -> bool {
    let Input::Key { key, .. } = *input else {
        return false;
    };
    KeysDown::from_iter([key]).modifier_down(active)
}

/// Whether `input` is a modifier key's (or Caps Lock's) press, repeat or
/// release: an input the "any key" reads pass over.
#[must_use]
pub fn is_modifier_input(input: &Input) -> bool {
    matches!(*input, Input::Key { key, .. } if is_modifier(key))
}

/// How `input`, a key going down or up, changes what is down; `None`
/// for any other input.
pub(super) fn change(input: &Input) -> Option<Change> {
    let Input::Key {
        pressed, repeat, ..
    } = *input
    else {
        return None;
    };
    Some(Change { pressed, repeat })
}

/// A fresh press of the key of `input` (a press, a repeat or a release);
/// `None` for any other input.
pub(super) fn fresh_press(input: &Input) -> Option<Input> {
    let Input::Key { key, .. } = *input else {
        return None;
    };
    Some(Input::Key {
        key,
        pressed: true,
        repeat: false,
    })
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

    /// A fresh press of each key down, in order: what a screen follows to
    /// take these keys into its own reader.
    pub fn presses(&self) -> impl Iterator<Item = Input> + '_ {
        self.keys().map(|key| Input::Key {
            key,
            pressed: true,
            repeat: false,
        })
    }

    /// Whether the key of `input`, a press, repeat or release, is down;
    /// false for any other input.
    #[must_use]
    pub fn holds(&self, input: &Input) -> bool {
        matches!(*input, Input::Key { key, .. } if self.is_down(key))
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

    /// Whether a modifier key is down, as any of the `active` contexts
    /// reads the modifier keys ([`sides`]).
    #[must_use]
    pub fn modifier_down(&self, active: &[Context]) -> bool {
        active
            .iter()
            .any(|&context| self.modifiers(sides(context)) != Modifiers::NONE)
    }

    /// Lets go of every key.
    pub fn let_go(&mut self) {
        self.down.clear();
    }
}

impl FromIterator<Key> for KeysDown {
    /// The keys `keys` down.
    fn from_iter<I: IntoIterator<Item = Key>>(keys: I) -> Self {
        Self {
            down: keys.into_iter().collect(),
        }
    }
}

/// The keyboard's whole device state: the keys down, and what each press
/// meant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keyboard {
    /// The keys down.
    down: KeysDown,
    /// What each key down meant as it went down, so its release lets go
    /// of that, whatever is held by then.
    presses: BTreeMap<Key, Command>,
}

impl Keyboard {
    /// Follows a key going down or up, forgetting a released key's press;
    /// anything else changes nothing. Says whether it was a key.
    pub fn follow(&mut self, input: &Input) -> bool {
        if let Input::Key {
            key,
            pressed: false,
            ..
        } = *input
        {
            self.presses.remove(&key);
        }
        self.down.follow(input)
    }

    /// Notes that `input`, a key's press, meant `command`.
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

    /// The keys down.
    #[must_use]
    pub const fn down(&self) -> &KeysDown {
        &self.down
    }

    /// Lets go of every key and forgets every press.
    pub fn let_go(&mut self) {
        self.down.let_go();
        self.presses.clear();
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
    fn the_comm_dialog_s_default_is_e() {
        assert_eq!(
            defaults(Context::Comm),
            vec![row(CommCommand::Close, Key::KeyE, NO)]
        );
    }

    #[test]
    fn the_haggle_dialog_s_defaults_are_a_and_l() {
        assert_eq!(
            defaults(Context::Haggle),
            vec![
                row(HaggleCommand::Accept, Key::KeyA, NO),
                row(HaggleCommand::Lower, Key::KeyL, NO),
            ]
        );
    }

    #[test]
    fn the_planet_comm_dialog_s_defaults_are_its_buttons_initials() {
        use PlanetCommCommand as P;
        assert_eq!(
            defaults(Context::PlanetComm),
            vec![
                row(P::Close, Key::KeyE, NO),
                row(P::Greetings, Key::KeyG, NO),
                row(P::OfferBribe, Key::KeyO, NO),
                row(P::DemandTribute, Key::KeyD, NO),
                row(P::Release, Key::KeyR, NO),
            ]
        );
    }

    #[test]
    fn the_escort_dialog_s_defaults_are_c_r_u_and_s() {
        use EscortDialogCommand as E;
        assert_eq!(
            defaults(Context::EscortDialog),
            vec![
                row(E::Close, Key::KeyC, NO),
                row(E::Release, Key::KeyR, NO),
                row(E::Upgrade, Key::KeyU, NO),
                row(E::Sell, Key::KeyS, NO),
            ]
        );
    }

    #[test]
    fn the_assignment_s_defaults_are_m_and_e() {
        assert_eq!(
            defaults(Context::Assignment),
            vec![
                row(AssignmentCommand::MyShip, Key::KeyM, NO),
                row(AssignmentCommand::Escort, Key::KeyE, NO),
            ]
        );
    }

    #[test]
    fn the_player_info_s_default_is_shift_tab_following_the_shared_tab() {
        assert_eq!(
            defaults(Context::PlayerInfo),
            vec![row(PlayerInfoCommand::TabBack, Key::Tab, SHIFT)]
        );
        let mut map = KeyMap::default();
        map.bind(UiCommand::Tab.into(), KeyCombo::key(Key::KeyQ));
        assert_eq!(
            combo(&map, PlayerInfoCommand::TabBack),
            Some(KeyCombo::with(Key::KeyQ, SHIFT))
        );
    }

    #[test]
    fn the_spaceport_s_defaults_are_its_buttons_initials() {
        use PortCommand as P;
        assert_eq!(
            defaults(Context::Port),
            vec![
                row(P::Recharge, Key::KeyR, NO),
                row(P::TradeCenter, Key::KeyC, NO),
                row(P::Outfitter, Key::KeyO, NO),
                row(P::Shipyard, Key::KeyS, NO),
                row(P::MissionBbs, Key::KeyN, NO),
                row(P::Bar, Key::KeyB, NO),
            ]
        );
    }

    #[test]
    fn the_trade_center_s_defaults_are_b_s_option_and_shift_tab() {
        use TradeCommand as T;
        assert_eq!(
            defaults(Context::Trade),
            vec![
                row(T::Buy, Key::KeyB, NO),
                row(T::Sell, Key::KeyS, NO),
                row(T::MaxLot, Key::AltLeft, NO),
                row(T::TabBack, Key::Tab, SHIFT),
            ]
        );
        let mut map = KeyMap::default();
        map.bind(UiCommand::Tab.into(), KeyCombo::key(Key::KeyQ));
        assert_eq!(
            combo(&map, TradeCommand::TabBack),
            Some(KeyCombo::with(Key::KeyQ, SHIFT)),
            "Shift-Tab follows the shared Tab"
        );
    }

    #[test]
    fn the_outfitter_s_defaults_are_b_s_and_option() {
        use OutfitterCommand as O;
        assert_eq!(
            defaults(Context::Outfitter),
            vec![
                row(O::Buy, Key::KeyB, NO),
                row(O::Sell, Key::KeyS, NO),
                row(O::Count, Key::AltLeft, NO),
            ]
        );
    }

    #[test]
    fn the_shipyard_s_default_is_b() {
        assert_eq!(
            defaults(Context::Shipyard),
            vec![row(ShipyardCommand::Buy, Key::KeyB, NO)]
        );
    }

    #[test]
    fn the_bar_s_defaults_are_h_g_and_n() {
        use BarCommand as B;
        assert_eq!(
            defaults(Context::Bar),
            vec![
                row(B::Hire, Key::KeyH, NO),
                row(B::Gamble, Key::KeyG, NO),
                row(B::News, Key::KeyN, NO),
            ]
        );
    }

    #[test]
    fn the_hire_screen_s_default_is_h() {
        assert_eq!(
            defaults(Context::Hire),
            vec![row(HireCommand::Hire, Key::KeyH, NO)]
        );
    }

    #[test]
    fn the_mission_computer_s_defaults_are_l_and_a() {
        assert_eq!(
            defaults(Context::MissionComputer),
            vec![
                row(MissionComputerCommand::Leave, Key::KeyL, NO),
                row(MissionComputerCommand::Accept, Key::KeyA, NO),
            ]
        );
    }

    #[test]
    fn the_race_s_default_is_the_left_option() {
        assert_eq!(
            defaults(Context::Race),
            vec![row(RaceCommand::Ask, Key::AltLeft, NO)]
        );
    }

    #[test]
    fn the_map_s_defaults_are_b_c_f_minus_equal_shift_tab_and_the_left_shift() {
        use MapCommand as M;
        assert_eq!(
            defaults(Context::Map),
            vec![
                row(M::Borders, Key::KeyB, NO),
                row(M::ClearRoute, Key::KeyC, NO),
                row(M::Find, Key::KeyF, NO),
                row(M::ZoomOut, Key::Minus, NO),
                row(M::ZoomIn, Key::Equal, NO),
                row(M::TabBack, Key::Tab, SHIFT),
                row(M::PlotCourse, Key::ShiftLeft, NO),
            ]
        );
        let mut map = KeyMap::default();
        map.bind(UiCommand::Tab.into(), KeyCombo::key(Key::KeyQ));
        assert_eq!(
            combo(&map, MapCommand::TabBack),
            Some(KeyCombo::with(Key::KeyQ, SHIFT)),
            "Shift-Tab follows the shared Tab"
        );
    }

    #[test]
    fn the_intro_s_defaults_are_return_and_space() {
        assert_eq!(
            defaults(Context::Intro),
            vec![
                row(IntroCommand::Confirm, Key::Enter, NO),
                row(IntroCommand::Activate, Key::Space, NO),
            ]
        );
    }

    #[test]
    fn the_system_view_has_no_command_of_its_own() {
        assert_eq!(defaults(Context::SystemView), vec![]);
    }

    #[test]
    fn the_app_s_default_is_caps_lock() {
        assert_eq!(
            defaults(Context::App),
            vec![row(AppCommand::DoubleTime, Key::CapsLock, NO)]
        );
    }

    #[test]
    fn the_main_screen_s_defaults_are_f_i_and_p() {
        assert_eq!(
            defaults(Context::MainScreen),
            vec![
                row(MainScreenCommand::Fly, Key::KeyF, NO),
                row(MainScreenCommand::About, Key::KeyI, NO),
                row(MainScreenCommand::Preferences, Key::KeyP, NO),
            ]
        );
    }

    #[test]
    fn the_main_menu_s_defaults_are_a_x_and_the_s_l_and_e_keys() {
        assert_eq!(
            defaults(Context::MainMenu),
            vec![
                row(MainMenuCommand::About, Key::KeyA, NO),
                row(MainMenuCommand::Text, Key::KeyX, NO),
                row(MainMenuCommand::OpenGuardS, Key::KeyS, NO),
                row(MainMenuCommand::OpenGuardL, Key::KeyL, NO),
                row(MainMenuCommand::EnterShip, Key::KeyE, NO),
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
        assert_eq!(
            both.union(SHIFT),
            both,
            "a flag set on both sides stays set"
        );
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
    fn keys_down_hand_back_a_press_of_each_key_and_say_whether_an_input_s_is_down() {
        let keys: KeysDown = [Key::KeyJ, Key::ArrowUp].into_iter().collect();
        assert_eq!(
            keys.presses().collect::<Vec<_>>(),
            vec![press(Key::KeyJ), press(Key::ArrowUp)],
            "in the keys' order"
        );
        assert!(keys.holds(&press(Key::KeyJ)));
        assert!(keys.holds(&release(Key::ArrowUp)));
        assert!(!keys.holds(&press(Key::KeyB)));
        assert!(!keys.holds(&Input::Text('j')));
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
    fn a_modifier_input_is_a_modifier_key_s() {
        assert!(is_modifier_input(&press(Key::ShiftRight)));
        assert!(is_modifier_input(&press(Key::CapsLock)));
        assert!(!is_modifier_input(&press(Key::KeyA)));
        assert!(!is_modifier_input(&Input::Text('a')));
    }

    #[test]
    fn flight_reads_left_modifiers_and_the_shared_ui_either() {
        assert_eq!(sides(Context::Flight), Sides::LeftOnly);
        assert_eq!(sides(Context::EscortMenu), Sides::LeftOnly);
        assert_eq!(sides(Context::Ui), Sides::Either);
        assert_eq!(sides(Context::PlayerInfo), Sides::Either);
        assert_eq!(sides(Context::Trade), Sides::Either);
        assert_eq!(
            sides(Context::Race),
            Sides::LeftOnly,
            "_DoRaceDialog polls the left Option's code"
        );
        assert_eq!(
            sides(Context::Map),
            Sides::Either,
            "_MapFilter reads shiftKey"
        );
        assert_eq!(
            sides(Context::Intro),
            Sides::LeftOnly,
            "_ShowIntroScreen polls key codes"
        );
        assert_eq!(sides(Context::SystemView), Sides::Either);
    }
}
