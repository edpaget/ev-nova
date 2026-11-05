//! The key map's saved form: each command by its stable name
//! ([`Command::name`]), each combo as text (`Meta+Digit5`), and only the
//! bindings that differ from the defaults.
//!
//! A combo's text is its modifiers then its key, joined by `+`: the
//! modifiers by their W3C names, always in the order `Shift`, `Control`,
//! `Alt`, `Meta`, and the key by `Code`'s W3C name (`KeyW`, `F12`), so
//! the file survives a winit upgrade and no name table is ours. No `Code`
//! name holds a `+`, so splitting on it never breaks a key's name.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use super::bindings::Bindings;
use super::command::Command;
use super::keyboard::{Binding, KeyCombo, KeyMap, Modifiers, default_binding};
use crate::input::Key;

impl KeyMap {
    /// The bindings that differ from the defaults, as the settings file
    /// saves them: each rebound command's combo, and `None` for each
    /// unbound one. A variant still following its base is no override,
    /// even when its base is rebound.
    #[must_use]
    pub fn overrides(&self) -> BTreeMap<Command, Option<KeyCombo>> {
        Command::all()
            .filter_map(|command| match self.binding(command) {
                binding if binding == default_binding(command) => None,
                Binding::Combo(combo) => Some((command, Some(combo))),
                Binding::Unbound => Some((command, None)),
                Binding::Follows { .. } => None,
            })
            .collect()
    }

    /// The key map saved `overrides` give: each command they name takes
    /// its combo, or is unbound for `None`, and every other command its
    /// default ([`KeyMap::from_entries`]). Then each clash
    /// ([`Bindings::clashes`]) is settled by unbinding one command, until
    /// none is left:
    ///
    /// - A clash between two *chosen* commands settles first, the later
    ///   one in `Command`'s declaration order unbound, as phase 4's keys
    ///   screen does. A command is chosen when the overrides name it, or
    ///   when it follows a base they name: a rebound base carries its
    ///   variant, and that is the player's choice too.
    /// - Then a chosen command's clash with one that kept its default
    ///   unbinds the defaulted one: the player's choice wins, as when a
    ///   later build gives a saved key to another command by default.
    /// - Two defaulted commands never clash (`KeyMap::default()` has no
    ///   clash), but if they did, the later one would be unbound.
    ///
    /// The clashes are found again after every unbind, so a command never
    /// loses its key over a clash an earlier unbind already settled.
    pub fn with_overrides(
        overrides: impl IntoIterator<Item = (Command, Option<KeyCombo>)>,
    ) -> Self {
        let overrides: Vec<_> = overrides.into_iter().collect();
        let named: BTreeSet<Command> = overrides.iter().map(|&(command, _)| command).collect();
        let mut bindings = Bindings {
            keyboard: Self::from_entries(overrides),
        };
        while let Some(loser) = next_to_unbind(&bindings, &named) {
            bindings.keyboard.unbind(loser);
        }
        bindings.keyboard
    }

    /// The overrides ([`KeyMap::overrides`]) as text: each command's name
    /// and its combo's text, `""` for an unbound one.
    #[must_use]
    pub fn to_table(&self) -> BTreeMap<String, String> {
        self.overrides()
            .into_iter()
            .map(|(command, combo)| {
                let text = combo.map(|combo| combo.to_string()).unwrap_or_default();
                (command.name().to_owned(), text)
            })
            .collect()
    }

    /// The key map a saved table gives, through
    /// [`KeyMap::with_overrides`]: `""` unbinds its command. An entry
    /// whose name no command has, or whose text is no combo, is dropped,
    /// so its command keeps its default.
    pub fn from_table<'a>(entries: impl IntoIterator<Item = (&'a str, &'a str)>) -> Self {
        Self::with_overrides(entries.into_iter().filter_map(|(name, text)| {
            let command = Command::from_name(name)?;
            if text.is_empty() {
                Some((command, None))
            } else {
                text.parse().ok().map(|combo| (command, Some(combo)))
            }
        }))
    }
}

/// Whether `command` is the player's choice in `map`: `named` names it,
/// or it follows a base `named` names.
fn chosen(map: &KeyMap, named: &BTreeSet<Command>, command: Command) -> bool {
    named.contains(&command)
        || matches!(map.binding(command), Binding::Follows { base, .. } if named.contains(&base))
}

/// The command settling the next of `bindings`' clashes unbinds, if any
/// is left ([`KeyMap::with_overrides`]' rules). Each one unbinds a bound
/// command, so the settling ends.
fn next_to_unbind(bindings: &Bindings, named: &BTreeSet<Command>) -> Option<Command> {
    let chosen = |command| chosen(&bindings.keyboard, named, command);
    let clashes = bindings.clashes();
    let clash = clashes
        .iter()
        .find(|clash| chosen(clash.first) && chosen(clash.second))
        .or_else(|| clashes.first())?;
    // The defaulted one loses to a chosen one; otherwise the later one.
    Some(if chosen(clash.second) && !chosen(clash.first) {
        clash.first
    } else {
        clash.second
    })
}

#[cfg(test)]
mod pinned;

/// Each modifier flag with its W3C name, in the order a combo's text
/// lists them.
const MODIFIER_NAMES: [(Modifiers, &str); 4] = [
    (Modifiers::SHIFT, "Shift"),
    (Modifiers::CONTROL, "Control"),
    (Modifiers::OPTION, "Alt"),
    (Modifiers::COMMAND, "Meta"),
];

impl fmt::Display for KeyCombo {
    /// The combo's saved text: `Meta+Digit5`, `KeyW`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, name) in MODIFIER_NAMES {
            if self.modifiers.contains(flag) {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", self.key)
    }
}

/// Text that names no combo: an unknown key or modifier, a modifier named
/// twice, or no key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboTextError(String);

impl fmt::Display for ComboTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a key combo", self.0)
    }
}

impl std::error::Error for ComboTextError {}

impl FromStr for KeyCombo {
    type Err = ComboTextError;

    /// The combo `text` saves: its modifiers in any order, then its key.
    /// `Unidentified` is no key.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || ComboTextError(text.to_owned());
        let mut parts = text.split('+');
        let key_name = parts.next_back().ok_or_else(error)?;
        let key = match Key::from_str(key_name) {
            Ok(Key::Unidentified) | Err(_) => return Err(error()),
            Ok(key) => key,
        };
        let mut modifiers = Modifiers::NONE;
        for part in parts {
            let (flag, _) = MODIFIER_NAMES
                .into_iter()
                .find(|&(_, name)| name == part)
                .ok_or_else(error)?;
            if modifiers.contains(flag) {
                return Err(error());
            }
            modifiers = modifiers.union(flag);
        }
        Ok(Self::with(key, modifiers))
    }
}

#[cfg(test)]
mod tests {
    use crate::input::Key;
    use std::collections::BTreeSet;

    use super::chosen;
    use crate::keymap::keyboard::{Binding, KeyCombo, KeyMap, Modifiers};
    use crate::keymap::{
        Bindings, Command, EscortMenuCommand, FlightCommand, PlayCommand, PortCommand,
    };

    fn flight(command: FlightCommand) -> Command {
        Command::Flight(command)
    }

    #[test]
    fn the_default_map_overrides_nothing() {
        assert!(KeyMap::default().overrides().is_empty());
    }

    #[test]
    fn the_overrides_are_the_rebound_and_unbound_commands() {
        let shift_w = KeyCombo::with(Key::KeyW, Modifiers::SHIFT);
        let mut map = KeyMap::default();
        map.bind(flight(FlightCommand::Accelerate), shift_w);
        map.unbind(flight(FlightCommand::Afterburner));
        assert_eq!(
            map.overrides().into_iter().collect::<Vec<_>>(),
            vec![
                (flight(FlightCommand::Accelerate), Some(shift_w)),
                (flight(FlightCommand::Afterburner), None),
            ]
        );
    }

    #[test]
    fn a_rebound_base_does_not_list_the_variant_that_follows_it() {
        let mut map = KeyMap::default();
        map.bind(
            flight(FlightCommand::TargetSelect),
            KeyCombo::key(Key::KeyT),
        );
        assert_eq!(
            map.overrides().into_iter().collect::<Vec<_>>(),
            vec![(
                flight(FlightCommand::TargetSelect),
                Some(KeyCombo::key(Key::KeyT))
            )]
        );
    }

    #[test]
    fn a_command_bound_back_to_its_default_is_no_override() {
        let accelerate = flight(FlightCommand::Accelerate);
        let mut map = KeyMap::default();
        map.bind(accelerate, KeyCombo::key(Key::KeyW));
        map.bind(accelerate, KeyCombo::key(Key::ArrowUp));
        assert!(map.overrides().is_empty());
    }

    /// A map that rebinds and unbinds, with no clash.
    fn rebound_and_unbound() -> KeyMap {
        let mut map = KeyMap::default();
        map.bind(
            flight(FlightCommand::Accelerate),
            KeyCombo::with(Key::KeyG, Modifiers::SHIFT),
        );
        map.bind(
            flight(FlightCommand::TargetSelect),
            KeyCombo::key(Key::KeyQ),
        );
        map.unbind(flight(FlightCommand::Afterburner));
        map
    }

    fn clash_free(map: &KeyMap) -> bool {
        Bindings {
            keyboard: map.clone(),
        }
        .clashes()
        .is_empty()
    }

    #[test]
    fn a_map_s_overrides_build_it_again() {
        let map = rebound_and_unbound();
        assert!(clash_free(&map));
        assert_eq!(KeyMap::with_overrides(map.overrides()), map);
    }

    #[test]
    fn no_overrides_build_the_default_map() {
        assert_eq!(KeyMap::with_overrides([]), KeyMap::default());
    }

    #[test]
    fn two_saved_commands_on_one_key_keep_it_for_the_one_declared_first() {
        let q = KeyCombo::key(Key::KeyQ);
        let autopilot = flight(FlightCommand::Autopilot);
        let board = flight(FlightCommand::Board);
        for entries in [
            [(autopilot, Some(q)), (board, Some(q))],
            [(board, Some(q)), (autopilot, Some(q))],
        ] {
            let map = KeyMap::with_overrides(entries);
            assert_eq!(map.combo(autopilot), Some(q), "{entries:?}");
            assert_eq!(map.binding(board), Binding::Unbound, "{entries:?}");
            assert!(clash_free(&map));
        }
    }

    #[test]
    fn a_saved_command_on_a_later_default_s_key_keeps_it() {
        let z = KeyCombo::key(Key::KeyZ);
        let map = KeyMap::with_overrides([(flight(FlightCommand::Accelerate), Some(z))]);
        assert_eq!(map.combo(flight(FlightCommand::Accelerate)), Some(z));
        assert_eq!(
            map.binding(flight(FlightCommand::Afterburner)),
            Binding::Unbound
        );
    }

    #[test]
    fn a_saved_command_on_an_earlier_default_s_key_keeps_it() {
        let up = KeyCombo::key(Key::ArrowUp);
        let map = KeyMap::with_overrides([(flight(FlightCommand::Afterburner), Some(up))]);
        assert_eq!(map.combo(flight(FlightCommand::Afterburner)), Some(up));
        assert_eq!(
            map.binding(flight(FlightCommand::Accelerate)),
            Binding::Unbound
        );
    }

    #[test]
    fn commands_in_contexts_never_active_together_share_a_key() {
        let o = KeyCombo::key(Key::KeyO);
        let map = KeyMap::with_overrides([(flight(FlightCommand::Board), Some(o))]);
        assert_eq!(map.combo(flight(FlightCommand::Board)), Some(o));
        assert_eq!(map.combo(Command::Port(PortCommand::Outfitter)), Some(o));
    }

    #[test]
    fn a_clash_an_earlier_unbind_settled_unbinds_nothing_more() {
        // Play is active with both Flight and Port, which are never active
        // together. Unbinding the Play command settles its clash with the
        // Port command's default too, so the Port command keeps its key.
        let o = KeyCombo::key(Key::KeyO);
        let autopilot = flight(FlightCommand::Autopilot);
        let galaxy_map = Command::Play(PlayCommand::GalaxyMap);
        let outfitter = Command::Port(PortCommand::Outfitter);
        assert_eq!(KeyMap::default().combo(outfitter), Some(o));
        let map = KeyMap::with_overrides([(autopilot, Some(o)), (galaxy_map, Some(o))]);
        assert_eq!(map.combo(autopilot), Some(o));
        assert_eq!(map.binding(galaxy_map), Binding::Unbound);
        assert_eq!(map.combo(outfitter), Some(o));
    }

    #[test]
    fn a_clash_of_two_saved_commands_settles_before_one_with_a_default() {
        // The escort menu's group shadows flight, so it never clashes with
        // Land; the map key clashes with both. Settling the two saved ones
        // first unbinds the map key, and Land keeps its default.
        let l = KeyCombo::key(Key::KeyL);
        let land = flight(FlightCommand::LandDock);
        let group = Command::EscortMenu(EscortMenuCommand::Group1);
        let galaxy_map = Command::Play(PlayCommand::GalaxyMap);
        assert_eq!(KeyMap::default().combo(land), Some(l));
        let map = KeyMap::with_overrides([(group, Some(l)), (galaxy_map, Some(l))]);
        assert_eq!(map.combo(group), Some(l));
        assert_eq!(map.binding(galaxy_map), Binding::Unbound);
        assert_eq!(map.combo(land), Some(l));
        assert!(clash_free(&map));
    }

    #[test]
    fn a_saved_base_carries_its_variant_onto_a_defaulted_combo() {
        // Communicate on Command-K takes Jettison Cargo's key, and its
        // Option variant, Hail Stellar, takes Jettison Non-Mission's.
        let meta_k = KeyCombo::with(Key::KeyK, Modifiers::COMMAND);
        let alt_meta_k = KeyCombo::with(Key::KeyK, Modifiers::COMMAND.union(Modifiers::OPTION));
        let jettison_non_mission = flight(FlightCommand::JettisonNonMission);
        assert_eq!(
            KeyMap::default().combo(jettison_non_mission),
            Some(alt_meta_k)
        );
        let map = KeyMap::with_overrides([(flight(FlightCommand::Communicate), Some(meta_k))]);
        assert_eq!(
            map.combo(flight(FlightCommand::HailStellar)),
            Some(alt_meta_k)
        );
        assert_eq!(map.combo(jettison_non_mission), None);
        assert!(clash_free(&map));
    }

    #[test]
    fn a_variant_whose_base_was_saved_is_chosen() {
        let named = BTreeSet::from([flight(FlightCommand::TargetSelect)]);
        let map = KeyMap::default();
        assert!(chosen(&map, &named, flight(FlightCommand::TargetSelect)));
        assert!(chosen(
            &map,
            &named,
            flight(FlightCommand::TargetSelectBack)
        ));
        assert!(!chosen(&map, &named, flight(FlightCommand::ClosestShip)));
        assert!(!chosen(&map, &named, flight(FlightCommand::Accelerate)));
    }

    #[test]
    fn a_command_settling_unbinds_saves_as_unbound() {
        let z = KeyCombo::key(Key::KeyZ);
        let map = KeyMap::with_overrides([(flight(FlightCommand::Accelerate), Some(z))]);
        assert_eq!(
            map.overrides().into_iter().collect::<Vec<_>>(),
            vec![
                (flight(FlightCommand::Accelerate), Some(z)),
                (flight(FlightCommand::Afterburner), None),
            ]
        );
    }

    #[test]
    fn the_table_names_each_override_with_its_text() {
        let table = rebound_and_unbound().to_table();
        assert_eq!(
            table.into_iter().collect::<Vec<_>>(),
            vec![
                ("Flight.Accelerate".to_owned(), "Shift+KeyG".to_owned()),
                ("Flight.Afterburner".to_owned(), String::new()),
                ("Flight.TargetSelect".to_owned(), "KeyQ".to_owned()),
            ]
        );
        assert!(KeyMap::default().to_table().is_empty());
    }

    #[test]
    fn a_map_s_table_builds_it_again() {
        let map = rebound_and_unbound();
        let table = map.to_table();
        assert_eq!(
            KeyMap::from_table(
                table
                    .iter()
                    .map(|(name, text)| (name.as_str(), text.as_str()))
            ),
            map
        );
    }

    #[test]
    fn an_empty_text_loads_unbound() {
        let map = KeyMap::from_table([("Flight.Board", "")]);
        assert_eq!(map.binding(flight(FlightCommand::Board)), Binding::Unbound);
    }

    #[test]
    fn an_unknown_name_or_key_leaves_its_command_at_its_default() {
        let map = KeyMap::from_table([
            ("Flight.Nope", "KeyQ"),
            ("Flight.Accelerate", "KeyQQ"),
            ("Flight.Board", "KeyN"),
        ]);
        let mut expected = KeyMap::default();
        expected.bind(flight(FlightCommand::Board), KeyCombo::key(Key::KeyN));
        assert_eq!(map, expected);
    }

    #[test]
    fn a_table_s_clashes_settle_as_its_overrides_do() {
        let map = KeyMap::from_table([("Flight.Accelerate", "KeyZ")]);
        assert_eq!(
            map,
            KeyMap::with_overrides([(
                flight(FlightCommand::Accelerate),
                Some(KeyCombo::key(Key::KeyZ))
            )])
        );
    }

    #[test]
    fn a_combo_shows_as_its_modifiers_then_its_key() {
        assert_eq!(KeyCombo::key(Key::KeyW).to_string(), "KeyW");
        assert_eq!(
            KeyCombo::with(Key::Digit5, Modifiers::COMMAND).to_string(),
            "Meta+Digit5"
        );
    }

    #[test]
    fn every_set_of_modifiers_round_trips_in_a_fixed_order() {
        let flags = [
            Modifiers::SHIFT,
            Modifiers::CONTROL,
            Modifiers::OPTION,
            Modifiers::COMMAND,
        ];
        for bits in 0..16_u8 {
            let modifiers = flags
                .iter()
                .enumerate()
                .filter(|&(at, _)| bits & (1 << at) != 0)
                .fold(Modifiers::NONE, |set, (_, &flag)| set.union(flag));
            let combo = KeyCombo::with(Key::KeyK, modifiers);
            let text = combo.to_string();
            assert_eq!(text.parse::<KeyCombo>(), Ok(combo), "{text}");
        }
        assert_eq!(
            KeyCombo::with(
                Key::KeyK,
                Modifiers::COMMAND
                    .union(Modifiers::OPTION)
                    .union(Modifiers::CONTROL)
                    .union(Modifiers::SHIFT)
            )
            .to_string(),
            "Shift+Control+Alt+Meta+KeyK"
        );
    }

    #[test]
    fn modifiers_parse_in_any_order() {
        assert_eq!(
            "Meta+Alt+KeyK".parse::<KeyCombo>(),
            Ok(KeyCombo::with(
                Key::KeyK,
                Modifiers::OPTION.union(Modifiers::COMMAND)
            ))
        );
        assert_eq!(
            "Control+Shift+Tab".parse::<KeyCombo>(),
            Ok(KeyCombo::with(
                Key::Tab,
                Modifiers::SHIFT.union(Modifiers::CONTROL)
            ))
        );
    }

    #[test]
    fn text_that_is_no_combo_is_rejected() {
        for text in [
            "",
            "Meta+",
            "+KeyW",
            "Hyper+KeyW",
            "KeyQQ",
            "Unidentified",
            "Shift",
            "Meta",
            "keyw",
            "Shift+Shift+KeyW",
        ] {
            assert!(text.parse::<KeyCombo>().is_err(), "{text:?}");
        }
    }

    #[test]
    fn a_rejected_text_is_named_in_the_error() {
        let error = "Hyper+KeyW".parse::<KeyCombo>().expect_err("no combo");
        assert_eq!(error.to_string(), r#""Hyper+KeyW" is not a key combo"#);
    }

    #[test]
    fn a_modifier_key_can_be_the_key_itself() {
        assert_eq!(
            "ControlLeft".parse::<KeyCombo>(),
            Ok(KeyCombo::key(Key::ControlLeft))
        );
        assert_eq!(
            "Alt+ShiftLeft".parse::<KeyCombo>(),
            Ok(KeyCombo::with(Key::ShiftLeft, Modifiers::OPTION))
        );
    }
}
