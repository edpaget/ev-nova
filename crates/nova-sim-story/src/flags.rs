//! World flags: the new game's named on/off facts about the galaxy.
//!
//! The new game's data names its flags (`dock_regular`), in snake case so the
//! same name works as an Ink variable. Missions read them in their
//! requirements and set and clear them in their effects (engine-story
//! phase 4), and a conversation reads them as it enters a knot: each
//! boolean `VAR` its story declares is set from them
//! ([`Conversation::open`](crate::dialog::conversation::Conversation::open)).
//! Nothing writes a story's variables back into them.
//!
//! A flag is its name as text; the data's format checks the names when it
//! loads. A flag the set doesn't hold is unset: there is no third state.
//! They are kept in the pilot (`Pilot::flags`)
//! and saved with it. The stock game's scripting uses numbered control bits
//! instead, and never touches these.

use std::collections::BTreeSet;

/// The world flags that are set, by name. A new pilot has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldFlags(BTreeSet<String>);

impl WorldFlags {
    /// Sets the flag `name`.
    pub fn set(&mut self, name: &str) {
        self.0.insert(name.to_owned());
    }

    /// Clears the flag `name`. Clearing a flag that isn't set does nothing.
    pub fn clear(&mut self, name: &str) {
        self.0.remove(name);
    }

    /// Whether the flag `name` is set.
    #[must_use]
    pub fn is_set(&self, name: &str) -> bool {
        self.0.contains(name)
    }

    /// The flags that are set, by name ascending.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }
}

impl FromIterator<String> for WorldFlags {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_set_holds_no_flag() {
        let flags = WorldFlags::default();
        assert!(!flags.is_set("dock_regular"));
        assert_eq!(flags.iter().count(), 0);
    }

    #[test]
    fn a_set_flag_is_set_and_others_are_not() {
        let mut flags = WorldFlags::default();
        flags.set("dock_regular");
        assert!(flags.is_set("dock_regular"));
        assert!(!flags.is_set("verge_open"));
    }

    #[test]
    fn a_cleared_flag_is_unset() {
        let mut flags = WorldFlags::default();
        flags.set("dock_regular");
        flags.set("verge_open");
        flags.clear("dock_regular");
        assert!(!flags.is_set("dock_regular"));
        assert!(flags.is_set("verge_open"));
    }

    #[test]
    fn clearing_an_unset_flag_does_nothing() {
        let mut flags = WorldFlags::default();
        flags.set("verge_open");
        flags.clear("dock_regular");
        assert_eq!(flags.iter().collect::<Vec<_>>(), ["verge_open"]);
    }

    #[test]
    fn flags_are_listed_by_name_ascending_each_once() {
        let mut flags = WorldFlags::default();
        flags.set("verge_open");
        flags.set("dock_regular");
        flags.set("verge_open");
        assert_eq!(
            flags.iter().collect::<Vec<_>>(),
            ["dock_regular", "verge_open"]
        );
    }

    #[test]
    fn flags_collect_from_names() {
        let flags: WorldFlags = ["verge_open".to_owned(), "dock_regular".to_owned()]
            .into_iter()
            .collect();
        assert!(flags.is_set("dock_regular"));
        assert!(flags.is_set("verge_open"));
        assert_eq!(flags.iter().count(), 2);
    }
}
