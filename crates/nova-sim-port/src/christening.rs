//! Christening a new pilot's ship, as `_DoNewPilot` (0x18b0a-0x193e9)
//! asks for its name once the pilot-name dialog is confirmed.
//!
//! # The prompt
//!
//! The prompt ([`Christening::prompt`]) is `STR#` 2002 #121 ([`PROMPT_LIST`],
//! [`PROMPT_INDEX`]; stock "Now, please christen your brand-new",
//! `_StrcatIndCString` @0x18cca), a space (@0x18cee), the `LongName` of the
//! first `chär`'s ship (`_shipyardLongNames`, @0x18cf4-0x18d10) and ": "
//! (@0x18d2b). `_StrcatIndCString` appends nothing for a missing entry, so
//! a data file without #121 asks " <LongName>: ".
//!
//! Its default ([`Christening::default`]) is `STR#` 128 #(7 + `_Rand(3)`)
//! ([`DEFAULT_NAMES`], [`FIRST_DEFAULT`], [`DEFAULTS`], @0x18d3c-0x18d5b):
//! stock "Ring of Glory", "Snowy Owl" or "Cardinal Virtue". One roll of
//! three is drawn. A missing list or entry gives an empty default, as
//! `GetIndString` does.
//!
//! # The dialog and after
//!
//! `_DoNewPilot` runs the two through `_EVTextInputDialog` (`DLOG` 3001,
//! @0x18d6f) with a most of 0x40, the shipyard's
//! [`SHIP_NAME_CHARS`](crate::shipyard::SHIP_NAME_CHARS). Cancel returns
//! at once (@0x18d76), before `_ResetPlayer` (@0x18dd4) and
//! `_SavePilotFile` (@0x1935f): the new pilot is abandoned and nothing is
//! saved. The name confirmed goes through `_CullNameString` into
//! `_shipName` (@0x19189-0x191ae; [`christen`]), culled but not trimmed.
//! The `chär`'s `OnStart` (`_ActivateCharResource` @0x19345) and the first
//! save both come after it, so a `T` set operator in `OnStart` renames the
//! christened ship, and the save keeps whichever name is left.
//!
//! The christening makes no sound of its own: the Text Input dialog's
//! clicks and its overlong `SysBeep` are its only ones.

use crate::catalog::{CommCatalog, ShipId, ShipRecord};
use crate::chance::Chance;
use crate::pilot::Pilot;
use crate::shipyard::cull_name;

/// The `STR#` the prompt's opening words come from.
pub const PROMPT_LIST: i16 = 2002;

/// The prompt's entry in [`PROMPT_LIST`], 1-based.
pub const PROMPT_INDEX: usize = 121;

/// The `STR#` the default names come from.
pub const DEFAULT_NAMES: i16 = 128;

/// The first default name's entry in [`DEFAULT_NAMES`], 1-based.
pub const FIRST_DEFAULT: usize = 7;

/// How many default names there are to roll among.
pub const DEFAULTS: u16 = 3;

/// The christening prompt for a new pilot's ship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Christening {
    /// The prompt's text: `STR#` 2002 #121, a space, the class's
    /// `LongName`, then ": ".
    pub prompt: String,
    /// The name the prompt opens with: one of `STR#` 128 #7-#9.
    pub default: String,
}

/// The christening prompt for a ship of class `ship`, whose record is
/// among `ships` (an empty `LongName` when none is), with its default
/// rolled on `chance`, as the module says.
pub fn christening(
    strings: &impl CommCatalog,
    ships: &[ShipRecord],
    ship: ShipId,
    chance: &mut dyn Chance,
) -> Christening {
    let long_name = ships
        .iter()
        .find(|record| record.id == ship)
        .map_or("", |record| record.long_name.as_str());
    let opening = entry(strings, PROMPT_LIST, PROMPT_INDEX);
    let pick = usize::from(chance.roll(DEFAULTS));
    Christening {
        prompt: format!("{opening} {long_name}: "),
        default: entry(strings, DEFAULT_NAMES, FIRST_DEFAULT + pick),
    }
}

/// `pilot` with its ship christened `typed`, culled as the original keeps
/// a ship's name ([`cull_name`]); an empty name stays empty.
#[must_use]
pub fn christen(pilot: Pilot, typed: &str) -> Pilot {
    pilot.with_ship_name(cull_name(typed))
}

/// Entry `index` (1-based) of `STR#` `list`; empty when either is missing.
pub(crate) fn entry(strings: &impl CommCatalog, list: i16, index: usize) -> String {
    strings
        .string_list(list)
        .into_iter()
        .nth(index - 1)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use nova_sim_base::fixture::{FAST, Scripted};
    use nova_sim_catalog::fixture::{catalog, ship};

    /// String lists by ID, canned.
    #[derive(Default)]
    struct Lists(BTreeMap<i16, Vec<String>>);

    impl Lists {
        fn with(mut self, id: i16, strings: &[&str]) -> Self {
            self.0
                .insert(id, strings.iter().map(|&s| s.to_owned()).collect());
            self
        }
    }

    impl CommCatalog for Lists {
        fn string_list(&self, id: i16) -> Vec<String> {
            self.0.get(&id).cloned().unwrap_or_default()
        }
    }

    /// `STR#` 2002 with #121 and `STR#` 128 with #7-#9.
    fn stock() -> Lists {
        let mut prompts = vec![""; 120];
        prompts.push("Now, please christen your brand-new");
        let mut names = vec!["Shane Merrol", "Cade Connelly", "Goroth Obarskyr"];
        names.extend(["Hunter", "Hawkeye", "Maverick"]);
        names.extend(["Ring of Glory", "Snowy Owl", "Cardinal Virtue"]);
        Lists::default().with(2002, &prompts).with(128, &names)
    }

    fn kestrel() -> Vec<ShipRecord> {
        let mut record = ship(128, FAST);
        record.long_name = "Kestrel".to_owned();
        vec![ship(129, FAST), record]
    }

    #[test]
    fn the_prompt_is_str_2002_121_a_space_the_long_name_and_a_colon() {
        let asked = christening(&stock(), &kestrel(), ShipId(128), &mut Scripted::default());
        assert_eq!(
            asked.prompt,
            "Now, please christen your brand-new Kestrel: "
        );
    }

    #[test]
    fn a_missing_prompt_entry_leaves_the_space_the_long_name_and_the_colon() {
        let asked = christening(
            &Lists::default(),
            &kestrel(),
            ShipId(128),
            &mut Scripted::default(),
        );
        assert_eq!(asked.prompt, " Kestrel: ");
        let short = Lists::default().with(2002, &["only one"]);
        let asked = christening(&short, &kestrel(), ShipId(128), &mut Scripted::default());
        assert_eq!(asked.prompt, " Kestrel: ");
    }

    #[test]
    fn a_class_with_no_record_reads_an_empty_long_name() {
        let asked = christening(&stock(), &kestrel(), ShipId(200), &mut Scripted::default());
        assert_eq!(asked.prompt, "Now, please christen your brand-new : ");
    }

    #[test]
    fn the_default_is_str_128_7_plus_one_roll_of_three() {
        for (roll, name) in [
            (0, "Ring of Glory"),
            (1, "Snowy Owl"),
            (2, "Cardinal Virtue"),
        ] {
            let mut chance = Scripted::rolling(&[roll]);
            let asked = christening(&stock(), &kestrel(), ShipId(128), &mut chance);
            assert_eq!(asked.default, name, "roll {roll}");
            assert_eq!(chance.sides_asked, [3], "one roll of three");
        }
    }

    #[test]
    fn a_missing_default_entry_or_list_is_empty() {
        let short = Lists::default().with(128, &["a", "b", "c", "d", "e", "f", "g", "h"]);
        let asked = christening(
            &short,
            &kestrel(),
            ShipId(128),
            &mut Scripted::rolling(&[2]),
        );
        assert_eq!(asked.default, "", "#9 is past the end");
        let asked = christening(
            &short,
            &kestrel(),
            ShipId(128),
            &mut Scripted::rolling(&[1]),
        );
        assert_eq!(asked.default, "h");
        let asked = christening(
            &Lists::default(),
            &kestrel(),
            ShipId(128),
            &mut Scripted::default(),
        );
        assert_eq!(asked.default, "");
    }

    #[test]
    fn christening_culls_a_leading_the_and_keeps_the_rest() {
        let pilot = Pilot::new(&catalog(), "Ada").expect("starts");
        for (typed, kept) in [
            ("The Raven", "Raven"),
            ("tHe Raven", "Raven"),
            ("Theory", "Theory"),
            ("", ""),
            ("the", "the"),
            ("  Raven ", "  Raven "),
        ] {
            let christened = christen(pilot.clone(), typed);
            assert_eq!(christened.ship_name(), Some(kept), "{typed:?}");
            assert_eq!(christened.name(), "Ada");
        }
    }
}
