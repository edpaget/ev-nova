//! The pad table's saved form, beside the key map's: each command by its
//! stable name, each binding as text, and only the bindings that differ
//! from the defaults.
//!
//! - An entry is its button's name ([`PadButton::name`]): `south`,
//!   `leftshoulder`, `dpup`, `+lefty` (a stick direction, as SDL writes a
//!   half axis). Under a layer, the layer's name, `+`, then the button's:
//!   `back+south`, `start+dpup`, `back++righty`. Only Select (`back`) and
//!   Start (`start`) are layers ([`PadButton::LAYERS`]). Parsing tries
//!   their prefixes first, so `+lefty` alone is a plain direction.
//! - An axis is its name ([`PadAxis::name`]), with `~` when inverted, as
//!   SDL's mappings mark one: `leftx`, `righty~`.
//! - An unbound command is `""`.
//! - The fixed rows ([`PadMap::is_fixed`]) are never saved, and a saved
//!   entry naming one is dropped.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use super::settle;
use crate::input::{PadAxis, PadButton};
use crate::keymap::command::{Command, Kind};
use crate::keymap::gamepad::{AxisBinding, PadBinding, PadEntry, PadMap, default_pad};

impl fmt::Display for PadEntry {
    /// The entry's saved text: `back+south`, `+lefty`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(layer) = self.layer {
            write!(f, "{}+", layer.name())?;
        }
        f.write_str(self.button.name())
    }
}

impl FromStr for PadEntry {
    type Err = PadTextError;

    /// The entry `text` saves: a layer's prefix and a button, or a
    /// button alone.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        for layer in PadButton::LAYERS {
            let button = text
                .strip_prefix(layer.name())
                .and_then(|rest| rest.strip_prefix('+'))
                .and_then(PadButton::from_name);
            if let Some(button) = button
                && button != layer
            {
                return Ok(Self::on(layer, button));
            }
        }
        PadButton::from_name(text)
            .map(Self::plain)
            .ok_or_else(|| PadTextError(text.to_owned()))
    }
}

/// What marks an inverted axis's text.
const INVERTED: char = '~';

impl fmt::Display for AxisBinding {
    /// The axis's saved text: `leftx`, `righty~`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.axis.name())?;
        if self.inverted {
            write!(f, "{INVERTED}")?;
        }
        Ok(())
    }
}

impl FromStr for AxisBinding {
    type Err = PadTextError;

    /// The axis `text` saves, inverted with a `~` suffix.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (name, inverted) = text
            .strip_suffix(INVERTED)
            .map_or((text, false), |name| (name, true));
        PadAxis::from_name(name)
            .map(|axis| Self { axis, inverted })
            .ok_or_else(|| PadTextError(text.to_owned()))
    }
}

/// Text that names no pad binding: an unknown button, axis or layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadTextError(pub(super) String);

impl fmt::Display for PadTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a pad binding", self.0)
    }
}

impl std::error::Error for PadTextError {}

/// `binding`'s saved text, `""` for none.
pub(super) fn text(binding: PadBinding) -> String {
    match binding {
        PadBinding::Entry(entry) => entry.to_string(),
        PadBinding::Axis(axis) => axis.to_string(),
        PadBinding::Unbound => String::new(),
    }
}

/// The binding `text` saves for `command`, by its kind: an axis's text
/// for an axis command, an entry's for any other, `""` for none.
pub(super) fn parse(command: Command, text: &str) -> Option<PadBinding> {
    if text.is_empty() {
        return Some(PadBinding::Unbound);
    }
    match command.kind() {
        Kind::Axis => text.parse().ok().map(PadBinding::Axis),
        Kind::Held | Kind::Pressed => text.parse().ok().map(PadBinding::Entry),
    }
}

/// Whether `binding` is of `command`'s kind: an axis for an axis
/// command, an entry for any other, or none.
fn fits(command: Command, binding: PadBinding) -> bool {
    match binding {
        PadBinding::Unbound => true,
        PadBinding::Axis(_) => command.kind() == Kind::Axis,
        PadBinding::Entry(_) => command.kind() != Kind::Axis,
    }
}

impl PadMap {
    /// The bindings that differ from the defaults, as the settings file
    /// saves them, the fixed rows left out.
    #[must_use]
    pub fn overrides(&self) -> BTreeMap<Command, PadBinding> {
        Command::all()
            .filter(|&command| !Self::is_fixed(command))
            .map(|command| (command, self.binding(command)))
            .filter(|&(command, binding)| binding != default_pad(command))
            .collect()
    }

    /// The pad table saved `overrides` give: each command they name takes
    /// its binding, and every other command its default. An override is
    /// dropped, its command keeping its default, when its command is a
    /// fixed row, its binding is of the other kind (an entry on an axis
    /// command, or an axis on another), or its entry is a fixed row's in
    /// a context active together ([`PadMap::fixed_holder`]). Then each
    /// clash ([`PadMap::clashes`]) settles by the keyboard's rules
    /// ([`KeyMap::with_overrides`](crate::keymap::KeyMap::with_overrides)):
    /// of two saved commands the later in declaration order is unbound,
    /// and against a defaulted one the defaulted one is.
    pub fn with_overrides(overrides: impl IntoIterator<Item = (Command, PadBinding)>) -> Self {
        let mut map = Self::default();
        let mut named = BTreeSet::new();
        for (command, binding) in overrides {
            let held = match binding {
                PadBinding::Entry(entry) => map.fixed_holder(command, entry).is_some(),
                PadBinding::Axis(_) | PadBinding::Unbound => false,
            };
            if !Self::is_fixed(command) && fits(command, binding) && !held {
                map.bind(command, binding);
                named.insert(command);
            }
        }
        while let Some(loser) = settle(&map.clashes(), |command| named.contains(&command)) {
            map.unbind(loser);
        }
        map
    }

    /// The overrides ([`PadMap::overrides`]) as text: each command's name
    /// and its binding's text, `""` for an unbound one.
    #[must_use]
    pub fn to_table(&self) -> BTreeMap<String, String> {
        self.overrides()
            .into_iter()
            .map(|(command, binding)| (command.name().to_owned(), text(binding)))
            .collect()
    }

    /// The pad table a saved table gives, through
    /// [`PadMap::with_overrides`]: `""` unbinds its command. An entry
    /// whose name no command has, or whose text is no binding of its
    /// command's kind, is dropped, so its command keeps its default.
    pub fn from_table<'a>(entries: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        Self::with_overrides(entries.into_iter().filter_map(|(name, text)| {
            let command = Command::from_name(name)?;
            parse(command, text).map(|binding| (command, binding))
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::input::{PadAxis, PadButton as P};
    use crate::keymap::{
        AxisBinding, Command, FlightCommand, PadBinding, PadEntry, PadMap, PortCommand, UiCommand,
    };

    use super::PadTextError;

    fn flight(command: FlightCommand) -> Command {
        Command::Flight(command)
    }

    fn entry(binding: PadEntry) -> PadBinding {
        PadBinding::Entry(binding)
    }

    #[test]
    fn an_entry_saves_as_its_layer_and_button() {
        for (binding, text) in [
            (PadEntry::on(P::Select, P::South), "back+south"),
            (PadEntry::on(P::Start, P::DPadUp), "start+dpup"),
            (PadEntry::on(P::Select, P::RightStickUp), "back++righty"),
            (PadEntry::on(P::Start, P::LeftStickLeft), "start+-leftx"),
            (PadEntry::on(P::Select, P::Start), "back+start"),
            (PadEntry::plain(P::LeftStickUp), "+lefty"),
            (PadEntry::plain(P::South), "south"),
        ] {
            assert_eq!(binding.to_string(), text);
            assert_eq!(text.parse::<PadEntry>(), Ok(binding), "{text}");
        }
    }

    #[test]
    fn an_axis_saves_bare_and_inverted_with_a_tilde() {
        for (binding, text) in [
            (AxisBinding::plain(PadAxis::LeftX), "leftx"),
            (AxisBinding::inverted(PadAxis::RightY), "righty~"),
        ] {
            assert_eq!(binding.to_string(), text);
            assert_eq!(text.parse::<AxisBinding>(), Ok(binding), "{text}");
        }
    }

    #[test]
    fn text_that_is_no_pad_binding_is_rejected() {
        for text in [
            "a",
            "South",
            "north+south",
            "back+",
            "back+back",
            "back+leftx",
            "leftx",
            "",
        ] {
            assert_eq!(
                text.parse::<PadEntry>(),
                Err(PadTextError(text.to_owned())),
                "{text}"
            );
        }
        for text in ["leftx~~", "~leftx", "+leftx", "south", "LeftX", ""] {
            assert!(text.parse::<AxisBinding>().is_err(), "{text}");
        }
        assert_eq!(
            PadTextError("a".to_owned()).to_string(),
            r#""a" is not a pad binding"#
        );
    }

    #[test]
    fn the_default_pad_map_overrides_nothing() {
        assert!(PadMap::default().overrides().is_empty());
        assert!(PadMap::default().to_table().is_empty());
        assert_eq!(PadMap::with_overrides([]), PadMap::default());
    }

    /// A map that rebinds and unbinds a button, and swaps the sticks
    /// with the throttle inverted, with no clash.
    fn rebound_unbound_swapped_inverted() -> PadMap {
        let mut map = PadMap::default();
        map.bind(
            flight(FlightCommand::Autopilot),
            entry(PadEntry::on(P::Start, P::RightStickLeft)),
        );
        map.unbind(flight(FlightCommand::Afterburner));
        map.bind(
            flight(FlightCommand::TurnAxis),
            PadBinding::Axis(AxisBinding::plain(PadAxis::LeftX)),
        );
        map.bind(
            flight(FlightCommand::ThrottleAxis),
            PadBinding::Axis(AxisBinding::inverted(PadAxis::RightY)),
        );
        map
    }

    #[test]
    fn a_pad_map_s_table_builds_it_again() {
        let map = rebound_unbound_swapped_inverted();
        assert_eq!(map.clashes(), vec![]);
        let table = map.to_table();
        assert_eq!(
            table,
            BTreeMap::from(
                [
                    ("Flight.Afterburner", ""),
                    ("Flight.Autopilot", "start+-rightx"),
                    ("Flight.ThrottleAxis", "righty~"),
                    ("Flight.TurnAxis", "leftx"),
                ]
                .map(|(name, text)| (name.to_owned(), text.to_owned()))
            )
        );
        let loaded = PadMap::from_table(table.iter().map(|(n, t)| (n.as_str(), t.as_str())));
        assert_eq!(loaded, map);
        assert_eq!(PadMap::with_overrides(map.overrides()), map);
    }

    #[test]
    fn a_command_bound_back_to_its_default_is_no_override() {
        let autopilot = flight(FlightCommand::Autopilot);
        let mut map = PadMap::default();
        map.bind(autopilot, entry(PadEntry::plain(P::LeftStick)));
        map.bind(autopilot, entry(PadEntry::plain(P::North)));
        assert!(map.overrides().is_empty());
    }

    #[test]
    fn an_unknown_position_name_leaves_its_command_at_its_default() {
        for text in ["a", "leftx+", "back+a", "select+south"] {
            let map = PadMap::from_table([("Flight.Autopilot", text)]);
            assert_eq!(map, PadMap::default(), "{text}");
        }
        let map = PadMap::from_table([("Flight.NoSuchCommand", "south")]);
        assert_eq!(map, PadMap::default());
        let map = PadMap::from_table([("Flight.TurnAxis", "upx"), ("Flight.Board", "")]);
        let mut expected = PadMap::default();
        expected.unbind(flight(FlightCommand::Board));
        assert_eq!(map, expected, "the rest of the table still loads");
    }

    #[test]
    fn a_fixed_row_is_never_saved_and_never_loaded() {
        let activate = Command::Ui(UiCommand::Activate);
        let mut map = PadMap::default();
        map.bind(activate, entry(PadEntry::plain(P::North)));
        map.unbind(Command::Ui(UiCommand::Back));
        assert!(map.overrides().is_empty());
        assert!(map.to_table().is_empty());
        assert_eq!(
            PadMap::from_table([("Ui.Activate", "north"), ("Ui.Back", "")]),
            PadMap::default()
        );
        assert_eq!(
            PadMap::with_overrides([(activate, PadBinding::Unbound)]),
            PadMap::default()
        );
    }

    #[test]
    fn an_entry_on_a_fixed_rows_button_loads_as_its_default() {
        for text in ["south", "dpdown", "rightshoulder", "+lefty"] {
            assert_eq!(
                PadMap::from_table([("Port.Bar", text)]),
                PadMap::default(),
                "{text}"
            );
        }
        let layered = PadMap::from_table([("Port.Bar", "back+south")]);
        assert_eq!(
            layered.entry(Command::Port(PortCommand::Bar)),
            Some(PadEntry::on(P::Select, P::South)),
            "a layered entry is no fixed row's"
        );
    }

    #[test]
    fn a_kind_mismatch_loads_as_the_default() {
        assert_eq!(
            PadMap::from_table([("Flight.TurnAxis", "south"), ("Flight.Autopilot", "leftx")]),
            PadMap::default()
        );
        assert_eq!(
            PadMap::with_overrides([
                (
                    flight(FlightCommand::TurnAxis),
                    entry(PadEntry::plain(P::South))
                ),
                (
                    flight(FlightCommand::Autopilot),
                    PadBinding::Axis(AxisBinding::plain(PadAxis::LeftX))
                ),
            ]),
            PadMap::default()
        );
    }

    #[test]
    fn two_saved_commands_on_one_entry_keep_it_for_the_one_declared_first() {
        let autopilot = flight(FlightCommand::Autopilot);
        let board = flight(FlightCommand::Board);
        for table in [
            [
                ("Flight.Autopilot", "start+-rightx"),
                ("Flight.Board", "start+-rightx"),
            ],
            [
                ("Flight.Board", "start+-rightx"),
                ("Flight.Autopilot", "start+-rightx"),
            ],
        ] {
            let map = PadMap::from_table(table);
            assert_eq!(
                map.entry(autopilot),
                Some(PadEntry::on(P::Start, P::RightStickLeft)),
                "{table:?}"
            );
            assert_eq!(map.binding(board), PadBinding::Unbound, "{table:?}");
            assert_eq!(map.clashes(), vec![]);
        }
    }

    #[test]
    fn a_saved_entry_on_a_later_default_s_button_keeps_it() {
        let map = PadMap::from_table([("Flight.Autopilot", "south")]);
        assert_eq!(
            map.entry(flight(FlightCommand::Autopilot)),
            Some(PadEntry::plain(P::South)),
            "flight is never active with the shared UI context"
        );
        assert_eq!(
            map.binding(flight(FlightCommand::FirePrimary)),
            PadBinding::Unbound
        );
        let earlier = PadMap::from_table([("Flight.FirePrimary", "north")]);
        assert_eq!(
            earlier.binding(flight(FlightCommand::Autopilot)),
            PadBinding::Unbound,
            "a defaulted command declared later loses too"
        );
    }

    #[test]
    fn two_saved_axes_on_one_axis_keep_it_for_the_one_declared_first() {
        let map = PadMap::from_table([
            ("Flight.TurnAxis", "lefty"),
            ("Flight.ThrottleAxis", "lefty~"),
        ]);
        assert_eq!(
            map.axis_binding(flight(FlightCommand::TurnAxis)),
            Some(AxisBinding::plain(PadAxis::LeftY))
        );
        assert_eq!(
            map.binding(flight(FlightCommand::ThrottleAxis)),
            PadBinding::Unbound
        );
    }
}
