//! A new game's sounds, kept on the store.
//!
//! A new game plays `.wav` clips (`nova_galaxy_format::sound`), not `snd `
//! resources, so the loader keeps each clip's bytes here by the `snd ` ID
//! it fills (`crate::new_game::sounds`, filled by `crate::new_game::load`):
//! a weapon's sound ID, and each engine cue a `game.md` role names. One
//! clip may fill several IDs. [`GameData::sound_files`] answers them, and
//! the audio side's sound bank (`nova_audio::bank`) plays them, decoding
//! each with [`decode_wav`](crate::sound::decode_wav).
//!
//! A store with sound files plays only them: an ID they don't hold is
//! silent on purpose, not missing. The stock game's store, and
//! [`GameData::default`], hold none: [`GameData::sound_files`] is `None`,
//! and every sound is the `snd ` resource of its ID.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::GameData;
use crate::wire::id::SoundId;

/// A new game's clips, each a WAV file's bytes, by the `snd ` ID it fills.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SoundFiles {
    clips: BTreeMap<SoundId, Arc<[u8]>>,
}

impl SoundFiles {
    /// The clips `clips` holds, by ID.
    #[must_use]
    pub fn new(clips: BTreeMap<SoundId, Arc<[u8]>>) -> Self {
        Self { clips }
    }

    /// The clip at `id`, if one fills it.
    #[must_use]
    pub fn get(&self, id: SoundId) -> Option<&[u8]> {
        self.clips.get(&id).map(|clip| &clip[..])
    }

    /// Every ID a clip fills, ascending.
    pub fn ids(&self) -> impl Iterator<Item = SoundId> + '_ {
        self.clips.keys().copied()
    }
}

impl GameData {
    /// The store with `sounds` as its sound files, replacing any it had
    /// (see the module docs).
    #[must_use]
    pub fn with_sound_files(mut self, sounds: SoundFiles) -> Self {
        self.sound_files = Some(sounds);
        self
    }

    /// The store's sound files: only a new game's store has them (see the
    /// module docs).
    #[must_use]
    pub fn sound_files(&self) -> Option<&SoundFiles> {
        self.sound_files.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::store::fixture::store_of;

    fn clips(entries: &[(i16, &[u8])]) -> SoundFiles {
        SoundFiles::new(
            entries
                .iter()
                .map(|&(id, bytes)| (SoundId(id), Arc::from(bytes)))
                .collect(),
        )
    }

    #[test]
    fn a_stock_store_has_no_sound_files() {
        let decoded = GameData::from_decoded(PathBuf::from("/game"), Vec::new());
        let empty = GameData::default();
        let loaded = store_of(&[]);
        for data in [&decoded, &empty, &loaded] {
            assert_eq!(data.sound_files(), None);
        }
    }

    #[test]
    fn a_store_with_sound_files_answers_each_by_id_and_new_ones_replace_them() {
        let data = GameData::default().with_sound_files(clips(&[(200, b"thump"), (600, b"click")]));
        let files = data.sound_files().expect("sound files");
        assert_eq!(files.get(SoundId(200)), Some(&b"thump"[..]));
        assert_eq!(files.get(SoundId(600)), Some(&b"click"[..]));
        assert_eq!(files.get(SoundId(601)), None, "unfilled");
        assert_eq!(
            files.ids().collect::<Vec<_>>(),
            [SoundId(200), SoundId(600)]
        );
        let data = data.with_sound_files(clips(&[(151, b"chime")]));
        let files = data.sound_files().expect("sound files");
        assert_eq!(files.get(SoundId(200)), None, "replaced");
        assert_eq!(files.get(SoundId(151)), Some(&b"chime"[..]));
        let none = GameData::default().with_sound_files(SoundFiles::default());
        assert_eq!(
            none.sound_files(),
            Some(&SoundFiles::default()),
            "silent, not stock"
        );
    }
}
