//! The key map's saved form: each command by its stable name
//! ([`Command::name`]), each combo as text (`Meta+Digit5`), and only the
//! bindings that differ from the defaults.
//!
//! A combo's text is its modifiers then its key, joined by `+`: the
//! modifiers by their W3C names, always in the order `Shift`, `Control`,
//! `Alt`, `Meta`, and the key by `Code`'s W3C name (`KeyW`, `F12`), so
//! the file survives a winit upgrade and no name table is ours. No `Code`
//! name holds a `+`, so splitting on it never breaks a key's name.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

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
    use crate::keymap::keyboard::{KeyCombo, KeyMap, Modifiers};
    use crate::keymap::{Command, FlightCommand};

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
