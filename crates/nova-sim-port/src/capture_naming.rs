//! Naming a captured ship the player takes over, as `_DoPlunderDialog`
//! asks once `_DoCapturedShipAssignmentDialog` returns Use As My Ship
//! (@0x943a3-0x94596).
//!
//! # The gate
//!
//! The rename is reached only when the **player's** class has `Crew` > 0
//! (ship 0's class, `shïp` +0x9f2, @0x9437a-0x94394) and the assignment
//! dialog returns Use As My Ship (nonzero, @0x943a8). A player of `Crew` 0,
//! or Use As Escort, goes to the escort path (@0x9459b) with no rename. The
//! assignment dialog's own button beep (`SimSound::CaptureAssigned`,
//! @0x5b56b / @0x5b66b) comes before the prompt opens.
//!
//! # The prompt
//!
//! The prompt ([`CaptureNaming::prompt`]) is `STR#` 2002 #119 alone
//! ([`PROMPT_LIST`], [`PROMPT_INDEX`]; stock "Now rename this captured
//! ship:", @0x943bc-0x943cf), with no class name. A missing entry gives an
//! empty prompt, as `GetIndString` does.
//!
//! Its default ([`CaptureNaming::default`]) is the **captured** class's
//! `Name` (`shïp` +0x6c, `_pstrcpy` @0x943d4-0x943fd), a space and three
//! digits, each `Rand(9)` + '1' (@0x94402-0x9449a): the shipyard's
//! [`numbered_name`]. The dialog is `_EVTextInputDialog` (@0x944ab) with a
//! most of 0x40 (@0x94476), [`SHIP_NAME_CHARS`](crate::shipyard::SHIP_NAME_CHARS).
//! The branch makes no `PlaySound` of its own: the Text Input dialog's
//! clicks and its overlong beep are its only sounds.
//!
//! The Pascal length byte of the default (the `Name`'s length + 4, in a
//! byte) would wrap past 251 characters. That is an integer width, so it is
//! not replicated.
//!
//! # OK and Cancel
//!
//! OK copies the typed text into `_shipName` by `_pstrcpy` alone
//! (@0x944ce-0x944df), **with no `_CullNameString`** (the christening
//! @0x1919d and the shipyard @0x5ed74 cull), so a leading "The " stays.
//! Then `_DoShipCapture(target, 0)` (@0x94515) takes the ship over: its
//! `OnRetire` and `OnCapture` run after the name, so a `T` in them renames
//! the ship again, and a capture that finds no slot for the old ship
//! (#305) leaves the typed name on the player's old ship.
//!
//! Cancel says `STR#` 2002 #122 (stock "You decided not to capture this
//! ship after all.", @0x9453c-0x9458f) as an ordinary message line for 240
//! ticks and leaves by the plunder dialog's exit (@0x936ec → @0x936fc).
//! Nothing is taken over and nothing is saved; the ship stays disabled and
//! boarded, and the prize is lost.

use crate::catalog::{CommCatalog, ShipRecord};
use crate::chance::Chance;
use crate::shipyard::numbered_name;
use crate::string_entry::entry;

/// The `STR#` the prompt comes from.
pub const PROMPT_LIST: i16 = 2002;

/// The prompt's entry in [`PROMPT_LIST`], 1-based.
pub const PROMPT_INDEX: usize = 119;

/// How many digits the default's number has.
pub const DEFAULT_DIGITS: usize = 3;

/// The prompt for renaming a captured ship the player takes over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureNaming {
    /// The prompt's text: `STR#` 2002 #119 alone (empty when missing).
    pub prompt: String,
    /// The name the prompt opens with: the captured class's `Name`, a
    /// space and three digits from 1 to 9.
    pub default: String,
}

/// The rename prompt for a captured ship of class `record`, its text read
/// from `strings` and its default's digits drawn on `chance`, as the module
/// says.
pub fn capture_naming(
    strings: &(impl CommCatalog + ?Sized),
    record: &ShipRecord,
    chance: &mut dyn Chance,
) -> CaptureNaming {
    CaptureNaming {
        prompt: entry(strings, PROMPT_LIST, PROMPT_INDEX),
        default: numbered_name(&record.name, DEFAULT_DIGITS, chance),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use nova_sim_base::fixture::{FAST, Scripted};
    use nova_sim_catalog::fixture::ship;

    /// String lists by ID, canned.
    #[derive(Default)]
    struct Lists(BTreeMap<i16, Vec<String>>);

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.0.get(&id).cloned().unwrap_or_default()
        }
    }

    /// `STR#` 2002 with #119 and its neighbours.
    fn stock() -> Lists {
        let mut prompts: Vec<String> = (1..=118).map(|n| format!("entry {n}")).collect();
        prompts.push("Now rename this captured ship:".to_owned());
        prompts.push("Now name your new".to_owned());
        Lists(BTreeMap::from([(2002, prompts)]))
    }

    fn shuttle() -> ShipRecord {
        let mut record = ship(128, FAST);
        record.name = "Shuttle".to_owned();
        record.long_name = "Light Shuttle".to_owned();
        record
    }

    #[test]
    fn the_prompt_is_str_2002_119_alone() {
        let asked = capture_naming(&stock(), &shuttle(), &mut Scripted::default());
        assert_eq!(asked.prompt, "Now rename this captured ship:");
    }

    #[test]
    fn a_missing_prompt_entry_gives_an_empty_prompt() {
        let asked = capture_naming(&Lists::default(), &shuttle(), &mut Scripted::default());
        assert_eq!(asked.prompt, "");
    }

    #[test]
    fn the_default_is_the_captured_class_name_and_three_digits_of_one_to_nine() {
        let mut chance = Scripted::rolling(&[0, 1, 8]);
        let asked = capture_naming(&stock(), &shuttle(), &mut chance);
        assert_eq!(asked.default, "Shuttle 129");
        assert_eq!(chance.sides_asked, [9, 9, 9]);
    }
}
