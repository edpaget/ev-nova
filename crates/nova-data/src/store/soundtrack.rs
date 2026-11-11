//! A new game's music, kept on the store.
//!
//! The new game names its one track in `game.md`'s `music`, a sound whose
//! file is an `.mp3` (`crate::new_game::music`, filled by
//! `crate::new_game::load`). The loader keeps the file's bytes here, and
//! start-up hands them to the audio side, which streams them where the
//! stock game streams `Nova Music.mp3`.
//!
//! The stock game's store, and [`GameData::default`], hold none: the
//! stock track is the loose file beside the data, read by
//! [`crate::music`].

use std::sync::Arc;

use super::GameData;

impl GameData {
    /// The store with `track`, an MP3 file's bytes, as its soundtrack,
    /// replacing any it had (see the module docs).
    #[must_use]
    pub fn with_soundtrack(mut self, track: Arc<[u8]>) -> Self {
        self.soundtrack = Some(track);
        self
    }

    /// The store's soundtrack: only a new game's store that names its
    /// music has one (see the module docs).
    #[must_use]
    pub fn soundtrack(&self) -> Option<&[u8]> {
        self.soundtrack.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    #[test]
    fn a_stock_store_has_no_soundtrack() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.soundtrack(), None);
        }
    }

    #[test]
    fn a_store_with_a_soundtrack_gives_its_bytes_and_a_new_one_replaces_it() {
        let data = GameData::default().with_soundtrack(Arc::from(&b"theme"[..]));
        assert_eq!(data.soundtrack(), Some(&b"theme"[..]));
        let data = data.with_soundtrack(Arc::from(&b"other"[..]));
        assert_eq!(data.soundtrack(), Some(&b"other"[..]));
    }
}
