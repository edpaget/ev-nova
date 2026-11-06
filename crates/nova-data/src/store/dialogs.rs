//! The new game's conversations, kept on the store: each compiled Ink
//! story by its file, and the place each stellar's conversation starts.
//!
//! The new game writes its conversations in Ink (`dialog/<file>.ink`), and
//! the author compiles each to `dialog/<file>.ink.json` with `inklecate`.
//! The loader (`crate::new_game`'s `dialogs`) reads the compiled text and
//! maps each stellar's `dialog` ref onto its `spöb`, then
//! [`GameData::with_dialogs`] sets them and [`GameData::dialogs`] answers
//! them. Nothing here reads Ink: the text is played through the dialog
//! port (`nova_sim_story::dialog`) above.
//!
//! The text is held as `Arc<str>`, so the store stays `Send + Sync` and a
//! reader can share it without copying. The stock game's store holds
//! none.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::GameData;
use crate::wire::id::StellarId;

/// The new game's compiled conversations and where each stellar's starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dialogs {
    /// Each compiled story's text (`inklecate`'s JSON), by its Ink file as
    /// a ref names it (`dialog/<file>.ink`).
    pub compiled: BTreeMap<String, Arc<str>>,
    /// Where each stellar's conversation starts, by its `spöb`. Every
    /// place's file is in [`compiled`](Self::compiled).
    pub stellars: BTreeMap<StellarId, DialogPlace>,
}

impl Dialogs {
    /// The place `stellar`'s conversation starts, with its story's
    /// compiled text, if it has one.
    #[must_use]
    pub fn stellar(&self, stellar: StellarId) -> Option<(&DialogPlace, &Arc<str>)> {
        let place = self.stellars.get(&stellar)?;
        Some((place, self.compiled.get(&place.file)?))
    }
}

/// A place in a compiled story: its file and the knot to enter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogPlace {
    /// The Ink file (`dialog/<file>.ink`), a key of [`Dialogs::compiled`].
    pub file: String,
    /// The knot, or `knot.stitch`, the conversation enters.
    pub knot: String,
}

impl DialogPlace {
    /// The place as a ref writes it: `dialog/<file>.ink#knot`.
    #[must_use]
    pub fn as_ref_text(&self) -> String {
        format!("{}#{}", self.file, self.knot)
    }
}

impl GameData {
    /// The store with `dialogs`, replacing any it had.
    #[must_use]
    pub fn with_dialogs(mut self, dialogs: Dialogs) -> Self {
        self.dialogs = dialogs;
        self
    }

    /// The new game's conversations: none in a stock store (see the module
    /// docs).
    #[must_use]
    pub fn dialogs(&self) -> &Dialogs {
        &self.dialogs
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn dialogs() -> Dialogs {
        Dialogs {
            compiled: BTreeMap::from([("dialog/a.ink".to_owned(), Arc::from("{a}"))]),
            stellars: BTreeMap::from([
                (
                    StellarId(128),
                    DialogPlace {
                        file: "dialog/a.ink".to_owned(),
                        knot: "greet".to_owned(),
                    },
                ),
                (
                    StellarId(129),
                    DialogPlace {
                        file: "dialog/gone.ink".to_owned(),
                        knot: "greet".to_owned(),
                    },
                ),
            ]),
        }
    }

    #[test]
    fn a_stock_store_keeps_no_dialogs() {
        assert_eq!(GameData::default().dialogs(), &Dialogs::default());
        let data = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        assert_eq!(data.dialogs(), &Dialogs::default());
    }

    #[test]
    fn a_store_with_dialogs_answers_them_and_new_ones_replace_them() {
        let data = GameData::default().with_dialogs(dialogs());
        assert_eq!(data.dialogs(), &dialogs());
        let data = data.with_dialogs(Dialogs::default());
        assert_eq!(data.dialogs(), &Dialogs::default());
    }

    #[test]
    fn a_stellar_s_place_comes_with_its_compiled_text() {
        let dialogs = dialogs();
        let (place, text) = dialogs.stellar(StellarId(128)).expect("a conversation");
        assert_eq!(place.knot, "greet");
        assert_eq!(&**text, "{a}");
        assert_eq!(dialogs.stellar(StellarId(130)), None);
        assert_eq!(dialogs.stellar(StellarId(129)), None, "no compiled text");
    }

    #[test]
    fn a_place_writes_as_its_ref() {
        let place = DialogPlace {
            file: "dialog/a.ink".to_owned(),
            knot: "greet.again".to_owned(),
        };
        assert_eq!(place.as_ref_text(), "dialog/a.ink#greet.again");
    }
}
