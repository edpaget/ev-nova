//! The sounds (`sounds/<id>.md`): one entry per clip, naming the file the
//! engine plays and where it came from, in the asset manifest's style
//! ([`crate::asset`]). Data names a sound only by its id: an outfit's
//! `sound` ([`crate::fleet::Outfit::sound`]) and `game.md`'s `sounds`
//! ([`crate::game::EngineSounds`]).

use schemars::JsonSchema;
use serde::Deserialize;

use crate::asset::Licence;
use crate::id::{Date, MediaPath, Url};

/// A sound (`sounds/<id>.md`): one clip and where it came from.
///
/// The engine plays its `file`, a `.wav` under `media/export/`. A sound
/// has no stand-in but silence, so it always names its file; a
/// placeholder is a clip that stands in for one still to come.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Sound {
    /// Whether it is a stand-in for a clip still to come.
    pub placeholder: bool,
    /// What the engine plays, under `media/export/`: one `.wav`.
    pub file: MediaPath,
    /// The licence it is used under.
    #[serde(default)]
    pub licence: Option<Licence>,
    /// The licence text that came with it, copied next to it.
    #[serde(default)]
    pub licence_file: Option<MediaPath>,
    /// Who made it.
    #[serde(default)]
    pub author: Option<String>,
    /// The page it was downloaded from.
    #[serde(default)]
    pub source_url: Option<Url>,
    /// When it was downloaded.
    #[serde(default)]
    pub retrieved: Option<Date>,
}

crate::model::kinds! {
    Sound => "sound" in "sounds",
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontmatter::parse;
    use crate::model::Kind;

    fn read(yaml: &str) -> Result<Sound, String> {
        parse::<Sound>(&format!("---\n{yaml}---\n")).map(|(t, _)| t)
    }

    #[test]
    fn a_sound_is_the_sound_kind_in_sounds() {
        assert_eq!((Sound::NAME, Sound::DIR), ("sound", "sounds"));
    }

    #[test]
    fn a_sound_reads_its_file_and_provenance() {
        let sound = read(
            "placeholder: false\nfile: media/export/sounds/click.wav\n\
             licence: CC0-1.0\nlicence_file: media/export/sounds/LICENSE.txt\n\
             author: Tester\nsource_url: https://example.com/sounds\nretrieved: 2026-11-11\n",
        )
        .unwrap();
        assert!(!sound.placeholder);
        assert_eq!(sound.file.as_str(), "media/export/sounds/click.wav");
        assert_eq!(sound.licence, Some(Licence::Cc0));
        assert_eq!(
            sound.licence_file.as_ref().map(MediaPath::as_str),
            Some("media/export/sounds/LICENSE.txt")
        );
        assert_eq!(sound.author.as_deref(), Some("Tester"));
        assert!(sound.source_url.is_some());
        assert_eq!(sound.retrieved.unwrap().as_str(), "2026-11-11");
    }

    #[test]
    fn a_placeholder_needs_only_its_file() {
        let sound = read("placeholder: true\nfile: media/export/sounds/hum.wav\n").unwrap();
        assert!(sound.placeholder);
        assert!(sound.licence.is_none() && sound.author.is_none());
    }

    #[test]
    fn a_sound_must_name_its_file_and_whether_it_is_a_placeholder() {
        let err = read("placeholder: true\n").unwrap_err();
        assert!(err.contains("file"), "{err}");
        let err = read("file: media/export/sounds/hum.wav\n").unwrap_err();
        assert!(err.contains("placeholder"), "{err}");
    }

    #[test]
    fn a_sound_names_no_other_word() {
        let err = read("placeholder: true\nfile: media/export/a.wav\nvolume: 3\n").unwrap_err();
        assert!(err.contains("volume"), "{err}");
        let err = read("placeholder: true\nfiles: [media/export/a.wav]\n").unwrap_err();
        assert!(err.contains("files"), "{err}");
    }

    #[test]
    fn a_sound_s_file_is_a_media_path() {
        assert!(read("placeholder: true\nfile: sounds/a.wav\n").is_err());
    }
}
