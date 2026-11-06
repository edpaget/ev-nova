//! Each stellar's conversation, as the session opens it: the compiled
//! story and the knot a landed pilot's conversation enters.
//!
//! The new game names a conversation on a stellar (`dialog:` in its
//! file); the store keeps the compiled Ink by file and each stellar's
//! place (`nova_data::Dialogs`). The catalog serves them joined, by
//! stellar, through [`PilotCatalog::stellar_dialogs`], whose default is
//! none: the stock game has no stellar conversations. The session plays
//! them through the dialog port (`nova_sim_story::dialog`).
//!
//! [`PilotCatalog::stellar_dialogs`]: crate::catalog::PilotCatalog::stellar_dialogs

use std::collections::BTreeMap;
use std::sync::Arc;

use nova_data::GameData;

use crate::catalog::StellarId;

/// Every stellar's conversation, by stellar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StellarDialogs {
    stellars: BTreeMap<StellarId, StellarDialog>,
}

/// One stellar's conversation: its story, compiled, and the knot it
/// enters.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StellarDialog {
    compiled: Arc<str>,
    knot: String,
}

impl StellarDialogs {
    /// These conversations, with `stellar`'s the story compiled to
    /// `compiled`, entered at `knot` (a knot or `knot.stitch`).
    #[must_use]
    pub fn with(mut self, stellar: StellarId, compiled: Arc<str>, knot: &str) -> Self {
        self.stellars.insert(
            stellar,
            StellarDialog {
                compiled,
                knot: knot.to_owned(),
            },
        );
        self
    }

    /// `stellar`'s conversation, as its compiled story and the knot it
    /// enters, or `None` when it has none.
    #[must_use]
    pub fn place(&self, stellar: StellarId) -> Option<(&str, &str)> {
        let dialog = self.stellars.get(&stellar)?;
        Some((&dialog.compiled, &dialog.knot))
    }
}

/// The store's stellar conversations, each joined with its story's text.
pub(crate) fn stellar_dialogs(data: &GameData) -> StellarDialogs {
    let dialogs = data.dialogs();
    dialogs
        .stellars
        .keys()
        .filter_map(|&stellar| {
            let (place, compiled) = dialogs.stellar(stellar)?;
            Some((StellarId(stellar.0), Arc::clone(compiled), &place.knot))
        })
        .fold(
            StellarDialogs::default(),
            |all, (stellar, compiled, knot)| all.with(stellar, compiled, knot),
        )
}

#[cfg(test)]
mod tests {
    use nova_data::{DialogPlace, Dialogs};

    use super::*;
    use crate::catalog::PilotCatalog;
    use crate::fixture::{FakePilotCatalog, catalog};

    fn data() -> GameData {
        let place = |file: &str, knot: &str| DialogPlace {
            file: file.to_owned(),
            knot: knot.to_owned(),
        };
        GameData::default().with_dialogs(Dialogs {
            compiled: BTreeMap::from([("dialog/a.ink".to_owned(), Arc::from("{a}"))]),
            stellars: BTreeMap::from([
                (nova_data::StellarId(128), place("dialog/a.ink", "greet")),
                (
                    nova_data::StellarId(129),
                    place("dialog/a.ink", "ledger.open"),
                ),
                (nova_data::StellarId(130), place("dialog/gone.ink", "greet")),
            ]),
        })
    }

    #[test]
    fn game_data_serves_each_stellar_s_story_and_knot() {
        let dialogs = data().stellar_dialogs();
        assert_eq!(dialogs.place(StellarId(128)), Some(("{a}", "greet")));
        assert_eq!(dialogs.place(StellarId(129)), Some(("{a}", "ledger.open")));
    }

    #[test]
    fn a_stellar_without_one_or_without_its_story_serves_none() {
        let dialogs = data().stellar_dialogs();
        assert_eq!(dialogs.place(StellarId(131)), None);
        assert_eq!(dialogs.place(StellarId(130)), None);
        assert_eq!(
            GameData::default().stellar_dialogs(),
            StellarDialogs::default()
        );
    }

    #[test]
    fn the_fake_serves_none_by_default_and_what_it_was_given() {
        assert_eq!(catalog().stellar_dialogs(), StellarDialogs::default());
        let given = StellarDialogs::default().with(StellarId(128), Arc::from("{a}"), "greet");
        let fake = FakePilotCatalog {
            stellar_dialogs: given.clone(),
            ..catalog()
        };
        assert_eq!(fake.stellar_dialogs(), given);
    }

    #[test]
    fn a_later_with_replaces_the_stellar_s_conversation() {
        let dialogs = StellarDialogs::default()
            .with(StellarId(128), Arc::from("{a}"), "one")
            .with(StellarId(128), Arc::from("{b}"), "two");
        assert_eq!(dialogs.place(StellarId(128)), Some(("{b}", "two")));
    }
}
