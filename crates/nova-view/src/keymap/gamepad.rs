//! The gamepad's side: the only keymap file that names a
//! [`PadButton`] or a [`PadAxis`], as [`keyboard`](super::keyboard) alone
//! names a `Key`.
//!
//! - [`PadEntry`] is a pad button and the layer button it needs held, as
//!   a `KeyCombo` is a key and its modifiers.
//! - [`PadMap`] binds each command to an entry, to a stick axis (an
//!   [`Kind::Axis`](super::Kind::Axis) command) or to nothing, with the default table that
//!   rdm:phase/key-bindings/phase-7-gamepad-input's findings record for
//!   flight, and rdm:phase/key-bindings/phase-8-gamepad-focus's for the
//!   shared UI context: the same buttons again, in a context no screen
//!   lists with flight's, so the two never clash.
//! - [`Gamepad`] is the pad's device state: the buttons down, each with
//!   the layer its press went down under, what each press meant (as a
//!   hold, and as what its release ends), and the last value of each
//!   stick axis.
//! - The lookup over a pad button's input ([`resolve`], [`held`],
//!   [`meanings_with`], [`bound_to`]) and the flight dead zone
//!   ([`FLIGHT_DEAD_ZONE`], [`dead_zoned`], [`axis_amount`]).
//!
//! The press threshold, where a stick or trigger presses a button, is
//! `nova`'s: its pad module turns axis values into [`PadButton`] presses
//! before they reach a screen.

use std::collections::{BTreeMap, BTreeSet};

use super::bindings::{Change, Clash};
use super::command::{
    Command, Context, EscortMenuCommand, FlightCommand, OutfitterCommand, PauseCommand,
    PlayCommand, RaceCommand, TradeCommand, UiCommand,
};
use super::contexts::{SCREENS, ScreenContexts, router_shadows, shadows, together_on};
use crate::input::{Input, PadAxis, PadButton};

/// The flight dead zone: a stick whose position lies within this of its
/// centre reads none (see [`dead_zoned`]). 0.15 covers the drift of a
/// worn stick on common pads, with some margin (the phase's findings).
pub const FLIGHT_DEAD_ZONE: f32 = 0.15;

/// A stick at (`x`, `y`) through the flight's radial dead zone: (0, 0)
/// within [`FLIGHT_DEAD_ZONE`] of the centre; outside it, the same
/// direction with the length rescaled from the dead zone's edge to 1, so
/// a push just past it reads a little and full deflection reads 1.
/// Radial, so a diagonal push isn't snapped onto an axis.
#[must_use]
pub fn dead_zoned(x: f32, y: f32) -> (f32, f32) {
    let length = x.hypot(y);
    if length <= FLIGHT_DEAD_ZONE {
        return (0.0, 0.0);
    }
    let rescaled = ((length - FLIGHT_DEAD_ZONE) / (1.0 - FLIGHT_DEAD_ZONE)).min(1.0);
    let by = rescaled / length;
    (x * by, y * by)
}

/// A pad button and the layer button it needs held: its gamepad
/// binding, as a `KeyCombo` is a key and its modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PadEntry {
    /// The button pressed.
    pub button: PadButton,
    /// The layer button held as it goes down, or none.
    pub layer: Option<PadButton>,
}

impl PadEntry {
    /// `button` alone.
    #[must_use]
    pub const fn plain(button: PadButton) -> Self {
        Self {
            button,
            layer: None,
        }
    }

    /// `button` pressed with `layer` held.
    #[must_use]
    pub const fn on(layer: PadButton, button: PadButton) -> Self {
        Self {
            button,
            layer: Some(layer),
        }
    }
}

/// How the gamepad binds one command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadBinding {
    /// To a button, under a layer or none.
    Entry(PadEntry),
    /// To a stick axis: an [`Kind::Axis`](super::Kind::Axis) command's.
    Axis(PadAxis),
    /// To nothing.
    Unbound,
}

/// The gamepad's binding table: one [`PadBinding`] per command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadMap {
    /// Every command's binding.
    bindings: BTreeMap<Command, PadBinding>,
}

impl Default for PadMap {
    /// The default table the phases' findings record: the shooter layout's
    /// two flight axes, the d-pad and the buttons for flight's commonest
    /// commands, and two layers, Select and Start, for the rest; and the
    /// shared UI context's plain rows.
    fn default() -> Self {
        Self {
            bindings: Command::all()
                .map(|command| (command, default_pad(command)))
                .collect(),
        }
    }
}

impl PadMap {
    /// How `command` is bound.
    #[must_use]
    pub fn binding(&self, command: Command) -> PadBinding {
        self.bindings
            .get(&command)
            .copied()
            .unwrap_or(PadBinding::Unbound)
    }

    /// The entry `command` is bound to, if a button's.
    #[must_use]
    pub fn entry(&self, command: Command) -> Option<PadEntry> {
        match self.binding(command) {
            PadBinding::Entry(entry) => Some(entry),
            PadBinding::Axis(_) | PadBinding::Unbound => None,
        }
    }

    /// The stick axis `command` is bound to, if an axis.
    #[must_use]
    pub fn axis(&self, command: Command) -> Option<PadAxis> {
        match self.binding(command) {
            PadBinding::Axis(axis) => Some(axis),
            PadBinding::Entry(_) | PadBinding::Unbound => None,
        }
    }

    /// Binds `command` to `binding`.
    pub fn bind(&mut self, command: Command, binding: PadBinding) {
        self.bindings.insert(command, binding);
    }

    /// Leaves `command` unbound.
    pub fn unbind(&mut self, command: Command) {
        self.bindings.insert(command, PadBinding::Unbound);
    }

    /// Every button some entry uses as its layer: a press of one alone
    /// binds nothing.
    #[must_use]
    pub fn layer_buttons(&self) -> BTreeSet<PadButton> {
        self.bindings
            .values()
            .filter_map(|binding| match *binding {
                PadBinding::Entry(PadEntry {
                    layer: Some(layer), ..
                }) => Some(layer),
                _ => None,
            })
            .collect()
    }

    /// The axes flight reads, its turn's and its throttle's.
    fn flight_axes(&self) -> impl Iterator<Item = PadAxis> + '_ {
        [FlightCommand::TurnAxis, FlightCommand::ThrottleAxis]
            .into_iter()
            .filter_map(|command| self.axis(Command::Flight(command)))
    }

    /// The commands a stick-direction `button` means in flight with no
    /// layer held, when flight reads its stick as an axis: Reverse Course
    /// for the throttle's back direction, and none for the bound axes'
    /// other directions. `None` when flight doesn't read `button`'s axis,
    /// so the button means what its entries say.
    fn flight_direction(&self, button: PadButton) -> Option<Vec<Command>> {
        let (axis, sign) = button.stick_direction()?;
        let throttle = self.axis(Command::Flight(FlightCommand::ThrottleAxis));
        if throttle == Some(axis) && sign < 0 {
            return Some(vec![Command::Flight(FlightCommand::ReverseCourse)]);
        }
        self.flight_axes().any(|read| read == axis).then(Vec::new)
    }

    /// Every pair of commands bound to one entry in contexts active
    /// together on some screen, a shadowing context's pairs with the one
    /// it shadows left out. `PadMap::default()` has none.
    #[must_use]
    pub fn clashes(&self) -> Vec<Clash> {
        self.clashes_on(SCREENS)
    }

    /// [`PadMap::clashes`] over `screens`.
    fn clashes_on(&self, screens: &[ScreenContexts]) -> Vec<Clash> {
        let mut by_entry: BTreeMap<PadEntry, Vec<Command>> = BTreeMap::new();
        for command in Command::all() {
            if let Some(entry) = self.entry(command) {
                by_entry.entry(entry).or_default().push(command);
            }
        }
        let mut clashes = Vec::new();
        for commands in by_entry.values() {
            for (at, &first) in commands.iter().enumerate() {
                for &second in &commands[at + 1..] {
                    let (a, b) = (first.context(), second.context());
                    if !router_shadows(first, second)
                        && !router_shadows(second, first)
                        && together_on(screens, a, b)
                        && !shadows(a, b)
                        && !shadows(b, a)
                    {
                        clashes.push(Clash { first, second });
                    }
                }
            }
        }
        clashes
    }
}

/// Every command's default on the pad: the phase's findings' table.
const fn default_pad(command: Command) -> PadBinding {
    match command {
        Command::Flight(command) => default_flight(command),
        Command::EscortMenu(command) => default_escort_menu(command),
        Command::Play(command) => default_play(command),
        Command::Pause(PauseCommand::Pause) => on(PadButton::Select, PadButton::Start),
        Command::Ui(command) => default_ui(command),
        // West held stands in for Option: a press of Buy or Sell (or a
        // racer) asks for a count, as an Option-click does
        // (rdm:phase/key-bindings/phase-9-gamepad-spaceport).
        Command::Trade(TradeCommand::MaxLot)
        | Command::Outfitter(OutfitterCommand::Count)
        | Command::Race(RaceCommand::Ask) => plain(PadButton::West),
        _ => PadBinding::Unbound,
    }
}

/// The shared UI context's defaults (rdm:phase/key-bindings/phase-8-gamepad-focus's
/// findings): the d-pad moves the focus, the shoulders step it in Tab
/// order, South presses the focused control, East goes back, the left
/// stick does what the arrow keys do and the right stick pages. Confirm
/// stays the keyboard's Return: South presses the focused control, and
/// the pad's focus starts on the default button. All are plain, so a
/// held layer never leaks a focus move.
const fn default_ui(command: UiCommand) -> PadBinding {
    use PadButton as P;
    use UiCommand as U;
    match command {
        U::FocusUp => plain(P::DPadUp),
        U::FocusDown => plain(P::DPadDown),
        U::FocusLeft => plain(P::DPadLeft),
        U::FocusRight => plain(P::DPadRight),
        U::PrevFocus => plain(P::LeftShoulder),
        U::Tab => plain(P::RightShoulder),
        U::Activate => plain(P::South),
        U::Back => plain(P::East),
        U::Up => plain(P::LeftStickUp),
        U::Down => plain(P::LeftStickDown),
        U::Left => plain(P::LeftStickLeft),
        U::Right => plain(P::LeftStickRight),
        U::PageUp => plain(P::RightStickUp),
        U::PageDown => plain(P::RightStickDown),
        U::Confirm | U::Home | U::End => PadBinding::Unbound,
    }
}

/// `button` alone.
const fn plain(button: PadButton) -> PadBinding {
    PadBinding::Entry(PadEntry::plain(button))
}

/// `button` with `layer` held.
const fn on(layer: PadButton, button: PadButton) -> PadBinding {
    PadBinding::Entry(PadEntry::on(layer, button))
}

/// Select's layer.
const fn select(button: PadButton) -> PadBinding {
    on(PadButton::Select, button)
}

/// Start's layer.
const fn start(button: PadButton) -> PadBinding {
    on(PadButton::Start, button)
}

/// Flight's defaults: the axes, the plain buttons, then Select's layer
/// and Start's.
const fn default_flight(command: FlightCommand) -> PadBinding {
    use FlightCommand as F;
    use PadButton as P;
    match command {
        F::ThrottleAxis => PadBinding::Axis(PadAxis::LeftY),
        F::TurnAxis => PadBinding::Axis(PadAxis::RightX),
        F::Accelerate => plain(P::DPadUp),
        F::ReverseCourse => plain(P::DPadDown),
        F::RotateLeft => plain(P::DPadLeft),
        F::RotateRight => plain(P::DPadRight),
        F::FirePrimary => plain(P::South),
        F::FireSecondary => plain(P::RightTrigger),
        F::Afterburner => plain(P::LeftTrigger),
        F::TargetSelect => plain(P::RightShoulder),
        F::TargetSelectBack => plain(P::LeftShoulder),
        F::ClosestTarget => plain(P::West),
        F::Autopilot => plain(P::North),
        F::LandDock => plain(P::East),
        F::SelectSecondary => plain(P::RightStick),
        F::Acknowledge => plain(P::LeftStick),
        F::HyperspaceMode => select(P::DPadUp),
        F::HyperSelect => select(P::DPadRight),
        F::NavOff => select(P::DPadDown),
        F::ClearTarget => select(P::DPadLeft),
        F::HyperJump => select(P::South),
        F::Board => select(P::East),
        F::Communicate => select(P::West),
        F::FaceStellar => select(P::North),
        F::TargetEscort => select(P::RightShoulder),
        F::TargetEscortBack => select(P::LeftShoulder),
        F::SelectSecondaryBack => select(P::RightTrigger),
        F::ClosestShip => select(P::LeftTrigger),
        F::HailStellar => select(P::RightStick),
        F::ShowFramerate => select(P::LeftStick),
        F::ZoomIn => select(P::RightStickUp),
        F::ZoomOut => select(P::RightStickDown),
        F::WeaponSafety => select(P::LeftStickLeft),
        F::EngageCloak => select(P::LeftStickRight),
        F::StellarSlot1 => start(P::DPadUp),
        F::StellarSlot2 => start(P::DPadRight),
        F::StellarSlot3 => start(P::DPadDown),
        F::StellarSlot4 => start(P::DPadLeft),
        F::NearestStellar => start(P::LeftStick),
        F::AttackTarget => start(P::South),
        F::DefendMe => start(P::West),
        F::HoldPosition => start(P::East),
        F::Recall => start(P::North),
        F::EscortMenu => start(P::RightShoulder),
        F::RecallToHangar => start(P::LeftShoulder),
        F::JettisonCargo => start(P::RightTrigger),
        F::JettisonNonMission => start(P::LeftTrigger),
        F::Eject => start(P::RightStick),
        F::SelfDestruct => start(P::LeftStickRight),
    }
}

/// The escort menu's groups: Start and the d-pad, then Start and the
/// left stick's click, which shadow the stellar slots while it shows.
const fn default_escort_menu(command: EscortMenuCommand) -> PadBinding {
    match command {
        EscortMenuCommand::Group1 => start(PadButton::DPadUp),
        EscortMenuCommand::Group2 => start(PadButton::DPadRight),
        EscortMenuCommand::Group3 => start(PadButton::DPadDown),
        EscortMenuCommand::Group4 => start(PadButton::DPadLeft),
        EscortMenuCommand::Group5 => start(PadButton::LeftStick),
    }
}

/// The play keys' defaults: on Start's layer, the two stick directions
/// flight doesn't read.
const fn default_play(command: PlayCommand) -> PadBinding {
    match command {
        PlayCommand::GalaxyMap => start(PadButton::RightStickUp),
        PlayCommand::PlayerInfo => start(PadButton::RightStickDown),
        PlayCommand::MissionInfo => start(PadButton::LeftStickLeft),
    }
}

/// One pad button down: the layer its press went down under, and whether
/// it still means anything (a layer released first leaves it meaning
/// nothing until it is pressed again).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Down {
    /// The button.
    button: PadButton,
    /// The layer held as it went down.
    layer: Option<PadButton>,
    /// Whether its layer is still held, or it had none.
    live: bool,
}

/// The gamepad's device state: the buttons down in the order pressed,
/// what each press meant, and the last value of each stick axis.
///
/// A press's meaning is kept twice: as a hold, dropped when its layer
/// goes up first ([`Gamepad::press_of`]), and as what its release ends,
/// kept past its layer and its own release until the button is pressed
/// again ([`Gamepad::release_of`]). So a screen asking after the reader
/// followed a chord's release, its layer let go of first, still reads the
/// chord's command, while nothing holds it. Pads
/// are merged before they get here (`nova`'s pad module), so this is one
/// pad's worth.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Gamepad {
    /// The buttons down, in the order pressed.
    down: Vec<Down>,
    /// What each press meant, while its button is down and its layer
    /// still held.
    presses: BTreeMap<PadButton, Command>,
    /// What the last press of each button meant, kept through its layer's
    /// release and its own until it is pressed again or the pad is let go
    /// of: what its release ends ([`Gamepad::release_of`]).
    meant: BTreeMap<PadButton, Command>,
    /// The last value of each axis.
    axes: BTreeMap<PadAxis, f32>,
}

impl Gamepad {
    /// Follows a pad button going down or up, or an axis moving; anything
    /// else changes nothing. Says whether it was the pad's input.
    ///
    /// A button going down takes the layer already held: of the layer
    /// buttons of `map` down, the one pressed first. A layer going up
    /// leaves every button pressed under it meaning nothing, its press
    /// forgotten as a hold ([`Gamepad::press_of`]), until it is pressed
    /// again; its release still ends that press
    /// ([`Gamepad::release_of`]).
    pub fn follow(&mut self, input: &Input, map: &PadMap) -> bool {
        let layers = map.layer_buttons();
        match *input {
            Input::PadButton {
                button,
                pressed: true,
            } => {
                if !self.is_down(button) {
                    let layer = self.layer_among(button, &layers);
                    self.down.push(Down {
                        button,
                        layer,
                        live: true,
                    });
                }
                true
            }
            Input::PadButton {
                button,
                pressed: false,
            } => {
                self.down.retain(|down| down.button != button);
                self.presses.remove(&button);
                for down in &mut self.down {
                    if down.layer == Some(button) {
                        down.live = false;
                        self.presses.remove(&down.button);
                    }
                }
                true
            }
            Input::PadAxis { axis, value } => {
                self.axes.insert(axis, value);
                true
            }
            _ => false,
        }
    }

    /// The layer a press of `button` goes down under now: the first
    /// pressed of `map`'s layer buttons down, `button` itself aside.
    fn layer_for(&self, button: PadButton, map: &PadMap) -> Option<PadButton> {
        self.layer_among(button, &map.layer_buttons())
    }

    /// [`Gamepad::layer_for`] among `layers`.
    fn layer_among(&self, button: PadButton, layers: &BTreeSet<PadButton>) -> Option<PadButton> {
        self.down
            .iter()
            .map(|down| down.button)
            .find(|&down| down != button && layers.contains(&down))
    }

    /// Whether `button` is down.
    #[must_use]
    pub fn is_down(&self, button: PadButton) -> bool {
        self.down.iter().any(|down| down.button == button)
    }

    /// The buttons down, in the order pressed.
    pub fn buttons(&self) -> impl Iterator<Item = PadButton> + '_ {
        self.down.iter().map(|down| down.button)
    }

    /// The last value of `axis`, none before it moved.
    #[must_use]
    pub fn axis(&self, axis: PadAxis) -> f32 {
        self.axes.get(&axis).copied().unwrap_or(0.0)
    }

    /// Notes that `input`, a pad button's press, meant `command`.
    pub fn note_press(&mut self, input: &Input, command: Option<Command>) {
        if let Input::PadButton {
            button,
            pressed: true,
        } = *input
        {
            for record in [&mut self.presses, &mut self.meant] {
                match command {
                    Some(command) => record.insert(button, command),
                    None => record.remove(&button),
                };
            }
        }
    }

    /// What the press of the pad button `input` presses or releases meant,
    /// if it was noted and still means it.
    #[must_use]
    pub fn press_of(&self, input: &Input) -> Option<Command> {
        let Input::PadButton { button, .. } = *input else {
            return None;
        };
        self.presses.get(&button).copied()
    }

    /// What the press the pad button release `input` ends meant, if it
    /// was noted: its layer let go of first leaves it holding nothing
    /// ([`Gamepad::press_of`]), but its release is still the end of that
    /// press, and stays it once followed, until the button is pressed
    /// again. `None` for any other input.
    #[must_use]
    pub fn release_of(&self, input: &Input) -> Option<Command> {
        let Input::PadButton {
            button,
            pressed: false,
        } = *input
        else {
            return None;
        };
        self.meant.get(&button).copied()
    }

    /// Lets go of every button, forgets every press and centres every
    /// axis.
    pub fn let_go(&mut self) {
        self.down.clear();
        self.presses.clear();
        self.meant.clear();
        self.axes.clear();
    }

    /// Whether a flight axis of `map` lies outside the dead zone.
    #[must_use]
    pub fn flight_axis_pushed(&self, map: &PadMap) -> bool {
        map.flight_axes().any(|axis| {
            let (along, _) = dead_zoned(self.axis(axis), self.axis(axis.across()));
            along != 0.0
        })
    }
}

/// How `input`, a pad button going down or up, changes what is down;
/// `None` for any other input. A pad never repeats.
pub(super) fn change(input: &Input) -> Option<Change> {
    let Input::PadButton { pressed, .. } = *input else {
        return None;
    };
    Some(Change {
        pressed,
        repeat: false,
    })
}

/// A fresh press of the pad button of `input`; `None` for any other
/// input.
pub(super) fn fresh_press(input: &Input) -> Option<Input> {
    let Input::PadButton { button, .. } = *input else {
        return None;
    };
    Some(Input::PadButton {
        button,
        pressed: true,
    })
}

/// Every command `button` pressed under `layer` means in the `active`
/// contexts, in their order:
///
/// 1. A layer button pressed with no layer means nothing.
/// 2. With no layer and flight active, a direction of a stick flight
///    reads as an axis means Reverse Course (the throttle's back
///    direction) or nothing.
/// 3. A shadowing context's commands on the entry, if any.
/// 4. Otherwise every other active context's.
///
/// Unlike a key, a pad button a shadowing context binds under another
/// layer stays the shadowed context's: the escort menu's groups on Start
/// and the d-pad leave the d-pad's own thrust and turn alone.
///
/// An entry matches exactly: a layer held never falls back to the
/// button's plain entry. Two layer buttons pressed together match an
/// entry naming them in either order.
fn meanings(
    map: &PadMap,
    button: PadButton,
    layer: Option<PadButton>,
    active: &[Context],
) -> Vec<Command> {
    let layers = map.layer_buttons();
    if layer.is_none() && layers.contains(&button) {
        return Vec::new();
    }
    if layer.is_none()
        && active.contains(&Context::Flight)
        && let Some(derived) = map.flight_direction(button)
    {
        return derived;
    }
    let pressed = PadEntry { button, layer };
    let met = |entry: PadEntry| {
        entry == pressed
            || (layers.contains(&entry.button)
                && entry.layer == Some(button)
                && layer == Some(entry.button))
    };
    let on = |context: Context| -> Vec<Command> {
        context
            .commands()
            .filter(|&command| map.entry(command).is_some_and(met))
            .collect()
    };
    let shadowing = |context: Context| active.iter().any(|&other| shadows(context, other));
    let first: Vec<Command> = active
        .iter()
        .filter(|&&context| shadowing(context))
        .flat_map(|&context| on(context))
        .collect();
    if !first.is_empty() {
        return first;
    }
    active
        .iter()
        .filter(|&&context| !shadowing(context))
        .flat_map(|&context| on(context))
        .collect()
}

/// The command `input` means in the `active` contexts, with `pad` holding
/// it down already (its layer taken as it went down): for a pad button's
/// press, the first command its entry meets. A release, or any other
/// input, means none.
pub(super) fn resolve(
    map: &PadMap,
    input: &Input,
    pad: &Gamepad,
    active: &[Context],
) -> Option<Command> {
    let Input::PadButton {
        button,
        pressed: true,
    } = *input
    else {
        return None;
    };
    let layer = pad
        .down
        .iter()
        .find(|down| down.button == button)
        .map_or_else(|| pad.layer_for(button, map), |down| down.layer);
    meanings(map, button, layer, active).first().copied()
}

/// Every command the buttons `pad` holds down hold in the `active`
/// contexts: each one a live button means under its layer.
pub(super) fn held(map: &PadMap, pad: &Gamepad, active: &[Context]) -> BTreeSet<Command> {
    pad.down
        .iter()
        .filter(|down| down.live)
        .flat_map(|down| meanings(map, down.button, down.layer, active))
        .collect()
}

/// Every command the pad button of `input` (a press or a release) holds
/// while down, with `pad` holding the rest: under the layer its press
/// took, or would take now. Empty for any other input.
pub(super) fn meanings_with(
    map: &PadMap,
    input: &Input,
    pad: &Gamepad,
    active: &[Context],
) -> Vec<Command> {
    let Input::PadButton { button, .. } = *input else {
        return Vec::new();
    };
    match pad.down.iter().find(|down| down.button == button) {
        Some(down) if !down.live => Vec::new(),
        Some(down) => meanings(map, button, down.layer, active),
        None => meanings(map, button, pad.layer_for(button, map), active),
    }
}

/// Every command of the `active` contexts bound to the pad button of
/// `input`, under any layer, in command order, and Reverse Course for the
/// throttle's back direction while flight is active. Empty for any other
/// input.
pub(super) fn bound_to(map: &PadMap, input: &Input, active: &[Context]) -> Vec<Command> {
    let Input::PadButton { button, .. } = *input else {
        return Vec::new();
    };
    let mut bound: BTreeSet<Command> = active
        .iter()
        .flat_map(|context| context.commands())
        .filter(|&command| {
            map.entry(command)
                .is_some_and(|entry| entry.button == button)
        })
        .collect();
    if active.contains(&Context::Flight)
        && let Some(derived) = map.flight_direction(button)
    {
        bound.extend(derived);
    }
    bound.into_iter().collect()
}

/// The amount the stick axis `map` binds `command` to is pushed, through
/// the flight dead zone ([`dead_zoned`], with the same stick's other
/// axis): -1 to 1, none when `command` is bound to no axis.
#[must_use]
pub fn axis_amount(map: &PadMap, pad: &Gamepad, command: Command) -> f32 {
    map.axis(command).map_or(0.0, |axis| {
        dead_zoned(pad.axis(axis), pad.axis(axis.across())).0
    })
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::super::KeyMap;
    use super::super::command::Kind;
    use super::super::keyboard::Binding;
    use super::*;
    use PadButton as P;
    use UiCommand as U;

    const FLYING: &[Context] = &[Context::Flight, Context::Play, Context::Pause];
    const MENU_SHOWN: &[Context] = &[
        Context::EscortMenu,
        Context::Flight,
        Context::Play,
        Context::Pause,
    ];

    fn press(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: true,
        }
    }

    fn release(button: PadButton) -> Input {
        Input::PadButton {
            button,
            pressed: false,
        }
    }

    fn flight(command: FlightCommand) -> Command {
        Command::Flight(command)
    }

    /// A pad following `inputs` on `map`, and what each press meant.
    struct Pad {
        map: PadMap,
        state: Gamepad,
        active: &'static [Context],
    }

    impl Pad {
        fn new(active: &'static [Context]) -> Self {
            Self {
                map: PadMap::default(),
                state: Gamepad::default(),
                active,
            }
        }

        /// Presses `button`, and says what it meant.
        fn press(&mut self, button: PadButton) -> Option<Command> {
            let input = press(button);
            self.state.follow(&input, &self.map);
            let meant = resolve(&self.map, &input, &self.state, self.active);
            self.state.note_press(&input, meant);
            meant
        }

        fn release(&mut self, button: PadButton) -> Option<Command> {
            let input = release(button);
            let meant = self.state.press_of(&input);
            self.state.follow(&input, &self.map);
            meant
        }

        fn held(&self) -> BTreeSet<Command> {
            held(&self.map, &self.state, self.active)
        }
    }

    #[test]
    fn every_flight_command_the_keyboard_reaches_is_reached_by_the_default_pad_table() {
        let keys = KeyMap::default();
        let pad = PadMap::default();
        for context in [Context::Flight, Context::Play, Context::Pause] {
            for command in context.commands() {
                let keyed = keys.combo(command).is_some()
                    || KeyMap::TWINS.iter().any(|twin| twin.command == command)
                    || matches!(keys.binding(command), Binding::Follows { .. });
                let reached = match command.kind() {
                    Kind::Axis => pad.axis(command).is_some(),
                    Kind::Held | Kind::Pressed => {
                        pad.entry(command).is_some()
                            || command == flight(FlightCommand::ReverseCourse)
                    }
                };
                assert!(reached || !keyed, "{command:?}");
                assert!(reached, "{command:?} is reached");
            }
        }
        assert_eq!(
            pad.entry(Command::Pause(PauseCommand::Pause)),
            Some(PadEntry::on(P::Select, P::Start))
        );
    }

    #[test]
    fn every_entry_of_the_default_table_means_its_command_when_pressed() {
        for command in Context::Flight
            .commands()
            .chain(Context::Play.commands())
            .filter(|command| command.kind() != Kind::Axis)
        {
            let entry = PadMap::default().entry(command).expect("bound");
            let mut pad = Pad::new(FLYING);
            if let Some(layer) = entry.layer {
                assert_eq!(pad.press(layer), None, "{command:?}'s layer alone");
            }
            assert_eq!(pad.press(entry.button), Some(command), "{command:?}");
        }
        let mut pad = Pad::new(MENU_SHOWN);
        pad.press(P::Start);
        assert_eq!(
            pad.press(P::DPadUp),
            Some(Command::EscortMenu(EscortMenuCommand::Group1)),
            "the menu's groups shadow the slots while it shows"
        );
        assert_eq!(
            pad.press(P::LeftStick),
            Some(Command::EscortMenu(EscortMenuCommand::Group5))
        );
    }

    /// The findings' default table, row by row, as
    /// rdm:phase/key-bindings/phase-7-gamepad-input records it and
    /// rdm:phase/key-bindings/phase-8-gamepad-focus adds the UI rows. Every
    /// command not listed is unbound.
    fn findings_table() -> Vec<(Command, PadBinding)> {
        use EscortMenuCommand as E;
        use FlightCommand as F;
        use PadBinding::Axis;
        use PlayCommand as Y;
        let s = |button| on(P::Select, button);
        let st = |button| on(P::Start, button);
        let f = |command, binding| (Command::Flight(command), binding);
        vec![
            f(F::ThrottleAxis, Axis(PadAxis::LeftY)),
            f(F::TurnAxis, Axis(PadAxis::RightX)),
            f(F::Accelerate, plain(P::DPadUp)),
            f(F::ReverseCourse, plain(P::DPadDown)),
            f(F::RotateLeft, plain(P::DPadLeft)),
            f(F::RotateRight, plain(P::DPadRight)),
            f(F::FirePrimary, plain(P::South)),
            f(F::FireSecondary, plain(P::RightTrigger)),
            f(F::Afterburner, plain(P::LeftTrigger)),
            f(F::TargetSelect, plain(P::RightShoulder)),
            f(F::TargetSelectBack, plain(P::LeftShoulder)),
            f(F::ClosestTarget, plain(P::West)),
            f(F::Autopilot, plain(P::North)),
            f(F::LandDock, plain(P::East)),
            f(F::SelectSecondary, plain(P::RightStick)),
            f(F::Acknowledge, plain(P::LeftStick)),
            f(F::HyperspaceMode, s(P::DPadUp)),
            f(F::HyperSelect, s(P::DPadRight)),
            f(F::NavOff, s(P::DPadDown)),
            f(F::ClearTarget, s(P::DPadLeft)),
            f(F::HyperJump, s(P::South)),
            f(F::Board, s(P::East)),
            f(F::Communicate, s(P::West)),
            f(F::FaceStellar, s(P::North)),
            f(F::TargetEscort, s(P::RightShoulder)),
            f(F::TargetEscortBack, s(P::LeftShoulder)),
            f(F::SelectSecondaryBack, s(P::RightTrigger)),
            f(F::ClosestShip, s(P::LeftTrigger)),
            f(F::HailStellar, s(P::RightStick)),
            f(F::ShowFramerate, s(P::LeftStick)),
            f(F::ZoomIn, s(P::RightStickUp)),
            f(F::ZoomOut, s(P::RightStickDown)),
            f(F::WeaponSafety, s(P::LeftStickLeft)),
            f(F::EngageCloak, s(P::LeftStickRight)),
            f(F::StellarSlot1, st(P::DPadUp)),
            f(F::StellarSlot2, st(P::DPadRight)),
            f(F::StellarSlot3, st(P::DPadDown)),
            f(F::StellarSlot4, st(P::DPadLeft)),
            f(F::NearestStellar, st(P::LeftStick)),
            f(F::AttackTarget, st(P::South)),
            f(F::DefendMe, st(P::West)),
            f(F::HoldPosition, st(P::East)),
            f(F::Recall, st(P::North)),
            f(F::EscortMenu, st(P::RightShoulder)),
            f(F::RecallToHangar, st(P::LeftShoulder)),
            f(F::JettisonCargo, st(P::RightTrigger)),
            f(F::JettisonNonMission, st(P::LeftTrigger)),
            f(F::Eject, st(P::RightStick)),
            f(F::SelfDestruct, st(P::LeftStickRight)),
            (Command::Play(Y::GalaxyMap), st(P::RightStickUp)),
            (Command::Play(Y::PlayerInfo), st(P::RightStickDown)),
            (Command::Play(Y::MissionInfo), st(P::LeftStickLeft)),
            (Command::EscortMenu(E::Group1), st(P::DPadUp)),
            (Command::EscortMenu(E::Group2), st(P::DPadRight)),
            (Command::EscortMenu(E::Group3), st(P::DPadDown)),
            (Command::EscortMenu(E::Group4), st(P::DPadLeft)),
            (Command::EscortMenu(E::Group5), st(P::LeftStick)),
            (Command::Pause(PauseCommand::Pause), on(P::Select, P::Start)),
            (ui(U::FocusUp), plain(P::DPadUp)),
            (ui(U::FocusDown), plain(P::DPadDown)),
            (ui(U::FocusLeft), plain(P::DPadLeft)),
            (ui(U::FocusRight), plain(P::DPadRight)),
            (ui(U::PrevFocus), plain(P::LeftShoulder)),
            (ui(U::Tab), plain(P::RightShoulder)),
            (ui(U::Activate), plain(P::South)),
            (ui(U::Back), plain(P::East)),
            (ui(U::Up), plain(P::LeftStickUp)),
            (ui(U::Down), plain(P::LeftStickDown)),
            (ui(U::Left), plain(P::LeftStickLeft)),
            (ui(U::Right), plain(P::LeftStickRight)),
            (ui(U::PageUp), plain(P::RightStickUp)),
            (ui(U::PageDown), plain(P::RightStickDown)),
            // rdm:phase/key-bindings/phase-9-gamepad-spaceport: West held
            // is Option in the trade center, the outfitter and the race.
            (Command::Trade(TradeCommand::MaxLot), plain(P::West)),
            (Command::Outfitter(OutfitterCommand::Count), plain(P::West)),
            (Command::Race(RaceCommand::Ask), plain(P::West)),
        ]
    }

    fn ui(command: UiCommand) -> Command {
        Command::Ui(command)
    }

    #[test]
    fn the_default_pad_table_binds_the_shared_ui_context_as_phase_8_says() {
        let pad = PadMap::default();
        let expected = |command: UiCommand| match command {
            U::FocusUp => plain(P::DPadUp),
            U::FocusDown => plain(P::DPadDown),
            U::FocusLeft => plain(P::DPadLeft),
            U::FocusRight => plain(P::DPadRight),
            U::PrevFocus => plain(P::LeftShoulder),
            U::Tab => plain(P::RightShoulder),
            U::Activate => plain(P::South),
            U::Back => plain(P::East),
            U::Up => plain(P::LeftStickUp),
            U::Down => plain(P::LeftStickDown),
            U::Left => plain(P::LeftStickLeft),
            U::Right => plain(P::LeftStickRight),
            U::PageUp => plain(P::RightStickUp),
            U::PageDown => plain(P::RightStickDown),
            U::Confirm | U::Home | U::End => PadBinding::Unbound,
        };
        for command in Context::Ui.commands() {
            let Command::Ui(ui) = command else {
                unreachable!("the Ui context lists Ui commands")
            };
            assert_eq!(pad.binding(command), expected(ui), "{command:?}");
        }
    }

    #[test]
    fn in_a_dialog_the_pad_reads_as_the_ui_commands() {
        use crate::keymap::{CommandReader, contexts};
        let pressed = |buttons: &[PadButton]| -> Option<Command> {
            let mut reader = CommandReader::new(contexts::DIALOG);
            let mut meant = None;
            for &button in buttons {
                meant = reader.follow(&press(button)).map(|c| c.command);
            }
            meant
        };
        for (button, command) in [
            (P::DPadUp, U::FocusUp),
            (P::DPadDown, U::FocusDown),
            (P::DPadLeft, U::FocusLeft),
            (P::DPadRight, U::FocusRight),
            (P::LeftShoulder, U::PrevFocus),
            (P::RightShoulder, U::Tab),
            (P::South, U::Activate),
            (P::East, U::Back),
            (P::LeftStickUp, U::Up),
            (P::LeftStickDown, U::Down),
            (P::LeftStickLeft, U::Left),
            (P::LeftStickRight, U::Right),
            (P::RightStickUp, U::PageUp),
            (P::RightStickDown, U::PageDown),
        ] {
            assert_eq!(pressed(&[button]), Some(ui(command)), "{button:?}");
        }
        assert_eq!(pressed(&[P::RightStickLeft]), None);
        assert_eq!(pressed(&[P::RightStickRight]), None);
        assert_eq!(pressed(&[P::Select]), None, "a layer alone");
        assert_eq!(pressed(&[P::Start]), None, "a layer alone");
        assert_eq!(
            pressed(&[P::Start, P::DPadUp]),
            None,
            "a held layer never leaks a focus move"
        );
        assert_eq!(pressed(&[P::Select, P::South]), None);
    }

    #[test]
    fn the_default_pad_table_is_the_findings_table_row_by_row() {
        let table: BTreeMap<Command, PadBinding> = findings_table().into_iter().collect();
        assert_eq!(table.len(), findings_table().len(), "no row listed twice");
        let pad = PadMap::default();
        for command in Command::all() {
            let expected = table.get(&command).copied().unwrap_or(PadBinding::Unbound);
            assert_eq!(pad.binding(command), expected, "{command:?}");
        }
    }

    #[test]
    fn the_default_pad_table_is_the_findings_table() {
        let pad = PadMap::default();
        let entries = |context: Context| -> Vec<(Command, PadBinding)> {
            context.commands().map(|c| (c, pad.binding(c))).collect()
        };
        let plain_count = entries(Context::Flight)
            .iter()
            .filter(|(_, b)| matches!(b, PadBinding::Entry(PadEntry { layer: None, .. })))
            .count();
        let on_layer = |layer| {
            [Context::Flight, Context::Play]
                .into_iter()
                .flat_map(entries)
                .filter(|(_, b)| matches!(b, PadBinding::Entry(e) if e.layer == Some(layer)))
                .count()
        };
        assert_eq!(plain_count, 14);
        assert_eq!(on_layer(P::Select), 18);
        assert_eq!(on_layer(P::Start), 18);
        assert_eq!(
            pad.binding(flight(FlightCommand::ThrottleAxis)),
            PadBinding::Axis(PadAxis::LeftY)
        );
        assert_eq!(
            pad.binding(flight(FlightCommand::TurnAxis)),
            PadBinding::Axis(PadAxis::RightX)
        );
        assert_eq!(pad.layer_buttons(), BTreeSet::from([P::Select, P::Start]));
        assert_eq!(
            pad.binding(Command::Ui(UiCommand::Confirm)),
            PadBinding::Unbound,
            "phase 8 binds the UI"
        );
    }

    #[test]
    fn the_default_pad_table_has_no_clash() {
        assert_eq!(PadMap::default().clashes(), vec![]);
        let mut clashing = PadMap::default();
        clashing.bind(
            flight(FlightCommand::LandDock),
            PadBinding::Entry(PadEntry::plain(P::South)),
        );
        assert_eq!(
            clashing.clashes(),
            vec![Clash {
                first: flight(FlightCommand::LandDock),
                second: flight(FlightCommand::FirePrimary),
            }]
        );
    }

    #[test]
    fn a_layered_button_pressed_with_its_layer_up_fires_nothing_layered() {
        let mut pad = Pad::new(FLYING);
        assert_eq!(
            pad.press(P::South),
            Some(flight(FlightCommand::FirePrimary))
        );
        assert_eq!(
            pad.release(P::South),
            Some(flight(FlightCommand::FirePrimary))
        );
        pad.press(P::Select);
        assert_eq!(pad.press(P::South), Some(flight(FlightCommand::HyperJump)));
        pad.release(P::South);
        pad.release(P::Select);
        assert_eq!(
            pad.press(P::South),
            Some(flight(FlightCommand::FirePrimary))
        );
        assert!(!pad.held().contains(&flight(FlightCommand::HyperJump)));
    }

    #[test]
    fn a_layer_button_pressed_alone_fires_nothing() {
        for layer in [P::Select, P::Start] {
            let mut pad = Pad::new(FLYING);
            assert_eq!(pad.press(layer), None, "{layer:?}");
            assert!(pad.held().is_empty(), "{layer:?}");
        }
    }

    #[test]
    fn select_and_start_together_pause_in_either_order() {
        let pause = Some(Command::Pause(PauseCommand::Pause));
        let mut pad = Pad::new(FLYING);
        pad.press(P::Select);
        assert_eq!(pad.press(P::Start), pause);
        let mut pad = Pad::new(FLYING);
        pad.press(P::Start);
        assert_eq!(pad.press(P::Select), pause);
    }

    #[test]
    fn a_held_layer_has_no_plain_fallback() {
        let mut map = PadMap::default();
        map.unbind(flight(FlightCommand::HyperJump));
        let mut pad = Pad::new(FLYING);
        pad.map = map;
        pad.press(P::Select);
        assert_eq!(pad.press(P::South), None, "not Fire Primary");
        assert!(pad.held().is_empty());
    }

    #[test]
    fn with_both_layers_held_the_one_pressed_first_is_the_layer() {
        let mut pad = Pad::new(FLYING);
        pad.press(P::Start);
        pad.press(P::Select);
        assert_eq!(
            pad.press(P::South),
            Some(flight(FlightCommand::AttackTarget))
        );
        let mut pad = Pad::new(FLYING);
        pad.press(P::Select);
        pad.press(P::Start);
        assert_eq!(pad.press(P::South), Some(flight(FlightCommand::HyperJump)));
    }

    #[test]
    fn releasing_the_layer_first_releases_the_layered_command_and_the_button_holds_nothing_after() {
        let mut pad = Pad::new(FLYING);
        pad.press(P::Select);
        assert_eq!(
            pad.press(P::RightStick),
            Some(flight(FlightCommand::HailStellar))
        );
        assert_eq!(
            pad.held(),
            BTreeSet::from([flight(FlightCommand::HailStellar)])
        );
        assert_eq!(pad.release(P::Select), None);
        assert!(pad.held().is_empty(), "nothing held: {:?}", pad.held());
        let still_down = press(P::RightStick);
        assert_eq!(
            meanings_with(&pad.map, &still_down, &pad.state, FLYING),
            vec![]
        );
        assert_eq!(pad.release(P::RightStick), None, "its press forgotten");
        assert_eq!(
            pad.press(P::RightStick),
            Some(flight(FlightCommand::SelectSecondary)),
            "pressed again"
        );
        assert_eq!(
            pad.held(),
            BTreeSet::from([flight(FlightCommand::SelectSecondary)])
        );
    }

    #[test]
    fn a_release_ends_what_its_press_meant_though_its_layer_went_up_first() {
        let mut pad = Pad::new(FLYING);
        let hail = flight(FlightCommand::HailStellar);
        let up = release(P::RightStick);
        pad.press(P::Select);
        pad.press(P::RightStick);
        assert_eq!(pad.state.release_of(&up), Some(hail));
        pad.release(P::Select);
        assert!(pad.held().is_empty(), "the layer let go of drops the hold");
        assert_eq!(pad.state.press_of(&up), None, "it holds nothing");
        assert_eq!(
            pad.state.release_of(&up),
            Some(hail),
            "its release is hail's"
        );
        pad.release(P::RightStick);
        assert_eq!(
            pad.state.release_of(&up),
            Some(hail),
            "still, once the release is followed"
        );
        assert_eq!(pad.state.release_of(&press(P::RightStick)), None, "a press");
        assert_eq!(pad.state.release_of(&release(P::South)), None, "unpressed");
        pad.press(P::RightStick);
        assert_eq!(
            pad.state.release_of(&up),
            Some(flight(FlightCommand::SelectSecondary)),
            "pressed again"
        );
        pad.state.let_go();
        assert_eq!(pad.state.release_of(&up), None, "let go of");
        pad.press(P::Start);
        assert_eq!(
            pad.state.release_of(&release(P::Start)),
            None,
            "meant nothing"
        );
    }

    #[test]
    fn the_throttle_axis_back_direction_holds_reverse_and_letting_it_back_releases_it() {
        let mut pad = Pad::new(FLYING);
        let reverse = flight(FlightCommand::ReverseCourse);
        assert_eq!(pad.press(P::LeftStickDown), Some(reverse));
        assert_eq!(pad.held(), BTreeSet::from([reverse]));
        assert_eq!(pad.release(P::LeftStickDown), Some(reverse));
        assert!(pad.held().is_empty());
        for other in [P::LeftStickUp, P::RightStickLeft, P::RightStickRight] {
            assert_eq!(pad.press(other), None, "{other:?}: flight reads its axis");
            pad.release(other);
        }
        assert_eq!(
            bound_to(&pad.map, &press(P::LeftStickDown), FLYING),
            vec![reverse]
        );
        assert_eq!(
            bound_to(&pad.map, &press(P::LeftStickDown), &[Context::Ui]),
            vec![ui(U::Down)],
            "the UI's own row, not Reverse Course"
        );
        let mut off_flight = Pad::new(&[Context::Play]);
        assert_eq!(off_flight.press(P::LeftStickDown), None);
    }

    #[test]
    fn a_press_holds_only_its_own_entrys_command() {
        let mut pad = Pad::new(FLYING);
        pad.press(P::South);
        assert_eq!(
            pad.held(),
            BTreeSet::from([flight(FlightCommand::FirePrimary)]),
            "not the Select and Start chord's Pause"
        );
        let mut layered = Pad::new(FLYING);
        layered.press(P::Start);
        layered.press(P::South);
        assert_eq!(
            layered.held(),
            BTreeSet::from([flight(FlightCommand::AttackTarget)]),
            "Start's layer meets the chord only with Select"
        );
    }

    #[test]
    fn while_the_escort_menu_shows_the_buttons_it_leaves_keep_their_commands() {
        let mut pad = Pad::new(MENU_SHOWN);
        pad.press(P::Start);
        assert_eq!(
            pad.press(P::South),
            Some(flight(FlightCommand::AttackTarget))
        );
        let mut plain = Pad::new(MENU_SHOWN);
        assert_eq!(
            plain.press(P::DPadUp),
            Some(flight(FlightCommand::Accelerate)),
            "the menu's Start-and-up shadows no plain up"
        );
    }

    #[test]
    fn what_a_button_not_yet_down_means_takes_the_layer_held() {
        let mut pad = Pad::new(FLYING);
        pad.press(P::Select);
        assert_eq!(
            meanings_with(&pad.map, &press(P::South), &pad.state, FLYING),
            vec![flight(FlightCommand::HyperJump)]
        );
        assert_eq!(
            resolve(&pad.map, &press(P::South), &pad.state, FLYING),
            Some(flight(FlightCommand::HyperJump)),
            "resolved before it is followed"
        );
    }

    #[test]
    fn a_plain_entry_on_a_direction_flight_doesnt_read_works_with_no_layer() {
        let mut pad = Pad::new(FLYING);
        pad.map.bind(
            flight(FlightCommand::ZoomIn),
            PadBinding::Entry(PadEntry::plain(P::RightStickUp)),
        );
        assert_eq!(
            pad.press(P::RightStickUp),
            Some(flight(FlightCommand::ZoomIn))
        );
    }

    #[test]
    fn with_a_layer_held_the_throttles_back_direction_is_no_reverse() {
        let mut pad = Pad::new(FLYING);
        pad.press(P::Select);
        assert_eq!(pad.press(P::LeftStickDown), None);
        pad.release(P::LeftStickDown);
        assert_eq!(
            pad.press(P::LeftStickLeft),
            Some(flight(FlightCommand::WeaponSafety)),
            "a direction flight doesn't read is a layered button"
        );
    }

    #[test]
    fn rebinding_throttle_to_righty_moves_reverse_with_it() {
        let mut pad = Pad::new(FLYING);
        pad.map.bind(
            flight(FlightCommand::ThrottleAxis),
            PadBinding::Axis(PadAxis::RightY),
        );
        pad.map.bind(
            flight(FlightCommand::TurnAxis),
            PadBinding::Axis(PadAxis::LeftX),
        );
        assert_eq!(
            pad.press(P::RightStickDown),
            Some(flight(FlightCommand::ReverseCourse))
        );
        assert_eq!(pad.press(P::LeftStickDown), None, "no longer flight's");
    }

    #[test]
    fn inside_the_dead_zone_an_axis_amount_is_zero_and_outside_it_rescales_to_one() {
        assert_eq!(dead_zoned(0.0, 0.0), (0.0, 0.0));
        assert_eq!(dead_zoned(0.1, -0.1), (0.0, 0.0));
        assert_eq!(dead_zoned(FLIGHT_DEAD_ZONE, 0.0), (0.0, 0.0));
        assert_eq!(dead_zoned(1.0, 0.0), (1.0, 0.0));
        assert_eq!(dead_zoned(0.0, -1.0), (0.0, -1.0));
        let (x, y) = dead_zoned(0.0, 0.575);
        assert!(x == 0.0 && (y - 0.5).abs() < 1e-6, "{y}");
        // Diagonally just outside: a little on each axis.
        let (x, y) = dead_zoned(0.12, 0.12);
        assert!(x > 0.0 && x < 0.05 && (x - y).abs() < 1e-6, "{x} {y}");
        // Past full deflection, as a diagonal can read: still 1.
        let (x, y) = dead_zoned(0.9, 0.9);
        assert!((x.hypot(y) - 1.0).abs() < 1e-6, "{x} {y}");
    }

    #[test]
    fn an_axis_amount_reads_the_bound_axis_through_the_dead_zone() {
        let map = PadMap::default();
        let mut pad = Gamepad::default();
        let turn = flight(FlightCommand::TurnAxis);
        let throttle = flight(FlightCommand::ThrottleAxis);
        assert_eq!(axis_amount(&map, &pad, turn), 0.0);
        pad.follow(
            &Input::PadAxis {
                axis: PadAxis::RightX,
                value: -1.0,
            },
            &map,
        );
        pad.follow(
            &Input::PadAxis {
                axis: PadAxis::LeftY,
                value: 0.1,
            },
            &map,
        );
        assert_eq!(axis_amount(&map, &pad, turn), -1.0);
        assert_eq!(axis_amount(&map, &pad, throttle), 0.0, "in the dead zone");
        assert_eq!(
            axis_amount(&map, &pad, flight(FlightCommand::Accelerate)),
            0.0
        );
        assert!(pad.flight_axis_pushed(&map));
        pad.let_go();
        assert_eq!(axis_amount(&map, &pad, turn), 0.0);
        assert!(!pad.flight_axis_pushed(&map));
    }

    #[test]
    fn a_stick_in_the_dead_zone_pushes_no_flight_axis() {
        let map = PadMap::default();
        let mut pad = Gamepad::default();
        for (axis, value) in [(PadAxis::RightX, 0.1), (PadAxis::LeftY, -0.12)] {
            pad.follow(&Input::PadAxis { axis, value }, &map);
        }
        assert!(!pad.flight_axis_pushed(&map));
        pad.follow(
            &Input::PadAxis {
                axis: PadAxis::LeftX,
                value: 0.9,
            },
            &map,
        );
        assert!(
            pad.flight_axis_pushed(&map),
            "radial: the throttle's stick is pushed"
        );
    }

    #[test]
    fn a_pad_change_and_fresh_press_read_only_pad_buttons() {
        assert_eq!(
            change(&release(P::East)),
            Some(Change {
                pressed: false,
                repeat: false
            })
        );
        assert_eq!(fresh_press(&release(P::East)), Some(press(P::East)));
        let axis = Input::PadAxis {
            axis: PadAxis::LeftX,
            value: 0.3,
        };
        assert_eq!(change(&axis), None);
        assert_eq!(fresh_press(&axis), None);
        assert_eq!(
            resolve(&PadMap::default(), &axis, &Gamepad::default(), FLYING),
            None
        );
    }
}
