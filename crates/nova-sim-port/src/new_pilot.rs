//! The New Pilot dialog's name and nickname, as `_DoNewPilot`
//! (0x18b0a-0x193e9) opens and keeps them.
//!
//! # The defaults
//!
//! Before the dialog opens, the Full Name field's default is `STR#` 128
//! #(1 + `_Rand(3)`) and the Nickname field's #(4 + `_Rand(3)`)
//! ([`NAME_LIST`], [`FIRST_NAME`], [`FIRST_NICKNAME`], [`DEFAULTS`],
//! @0x18b44-0x18ba7): stock "Shane Merrol", "Cade Connelly" or "Goroth
//! Obarskyr", and "Hunter", "Hawkeye" or "Maverick" ([`name_defaults`]).
//! One roll of three is drawn for each. A missing list or entry gives an
//! empty default, as `GetIndString` does.
//!
//! # The names kept
//!
//! Once the dialog is confirmed, the name and then the nickname go through
//! `_CullNameString`, and then the name and the nickname through
//! `_TrimResourceName` (@0x18bd6-0x18c03; [`kept_name`]). All of it runs
//! before `_DoesPilotExist` (@0x72def-0x72e6a), so the check reads the name
//! kept.
//!
//! # A name already saved
//!
//! When a pilot file of that name exists, `_DoNewPilot` asks `STR#` 140 #9
//! in `_EVYesNoDialog` (@0x18c08-0x18c63). No leaves at once (@0x193df),
//! with nothing changed; Yes goes on to the christening (@0x18c68), and
//! the new pilot replaces the file only when `_SavePilotFile` (@0x1935f)
//! writes it. The question itself is the app's (`nova::app`'s menu), by
//! `RuleKey::NewPilotNameTaken`.
//!
//! None of these branches makes a sound of its own.

use crate::catalog::CommCatalog;
use crate::chance::Chance;
use crate::shipyard::cull_name;
use crate::string_entry::entry;
use nova_sim_catalog::trim_resource_name;

/// The `STR#` the default name and nickname come from.
pub const NAME_LIST: i16 = 128;

/// The first default name's entry in [`NAME_LIST`], 1-based.
pub const FIRST_NAME: usize = 1;

/// The first default nickname's entry in [`NAME_LIST`], 1-based.
pub const FIRST_NICKNAME: usize = 4;

/// How many defaults there are to roll among, for each field.
pub const DEFAULTS: u16 = 3;

/// The two fields the New Pilot dialog opens with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NameDefaults {
    /// The Full Name: one of `STR#` 128 #1-#3.
    pub name: String,
    /// The Nickname: one of `STR#` 128 #4-#6.
    pub nickname: String,
}

/// The New Pilot dialog's defaults, each rolled on `chance`, as the module
/// says.
pub fn name_defaults(strings: &impl CommCatalog, chance: &mut dyn Chance) -> NameDefaults {
    let name = usize::from(chance.roll(DEFAULTS));
    let nickname = usize::from(chance.roll(DEFAULTS));
    NameDefaults {
        name: entry(strings, NAME_LIST, FIRST_NAME + name),
        nickname: entry(strings, NAME_LIST, FIRST_NICKNAME + nickname),
    }
}

/// A name or nickname `typed` into the New Pilot dialog as the original
/// keeps it: culled ([`cull_name`]), then trimmed ([`trim_resource_name`]),
/// in that order (@0x18bd6-0x18c03).
#[must_use]
pub fn kept_name(typed: &str) -> String {
    trim_resource_name(&cull_name(typed)).to_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use nova_sim_base::fixture::Scripted;

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

    fn stock() -> Lists {
        Lists::default().with(
            128,
            &[
                "Shane Merrol",
                "Cade Connelly",
                "Goroth Obarskyr",
                "Hunter",
                "Hawkeye",
                "Maverick",
            ],
        )
    }

    #[test]
    fn each_default_is_one_roll_of_three_into_its_own_entries() {
        let mut chance = Scripted::rolling(&[0, 2]);
        let defaults = name_defaults(&stock(), &mut chance);
        assert_eq!(
            defaults,
            NameDefaults {
                name: "Shane Merrol".to_owned(),
                nickname: "Maverick".to_owned(),
            }
        );
        let mut chance = Scripted::rolling(&[2, 0]);
        let defaults = name_defaults(&stock(), &mut chance);
        assert_eq!(defaults.name, "Goroth Obarskyr");
        assert_eq!(defaults.nickname, "Hunter");
    }

    #[test]
    fn each_default_rolls_among_three() {
        let mut chance = Scripted::rolling(&[1, 1]);
        name_defaults(&stock(), &mut chance);
        assert_eq!(chance.sides_asked, [3, 3]);
    }

    #[test]
    fn a_missing_list_or_entry_gives_an_empty_default() {
        let mut chance = Scripted::rolling(&[0, 0]);
        assert_eq!(
            name_defaults(&Lists::default(), &mut chance),
            NameDefaults::default()
        );
        let short = Lists::default().with(128, &["A", "B", "C", "Hunter"]);
        let mut chance = Scripted::rolling(&[1, 1]);
        let defaults = name_defaults(&short, &mut chance);
        assert_eq!(defaults.name, "B");
        assert_eq!(defaults.nickname, "");
    }

    #[test]
    fn a_leading_the_in_any_case_is_culled() {
        assert_eq!(kept_name("The Ace"), "Ace");
        assert_eq!(kept_name("THE ace"), "ace");
        assert_eq!(kept_name("Theodore"), "Theodore");
        assert_eq!(kept_name("the"), "the");
    }

    #[test]
    fn the_name_is_trimmed_as_a_resource_name() {
        assert_eq!(kept_name("Ace;x"), "Ace");
        assert_eq!(kept_name(" Ada "), " Ada ");
    }

    #[test]
    fn the_name_is_culled_before_it_is_trimmed() {
        // Trimming first would leave "the", which culling then keeps.
        assert_eq!(kept_name("the ;x"), "");
    }
}
