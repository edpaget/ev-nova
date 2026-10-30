//! What each [`UiSound`] plays: a [`SoundCue`], the `snd ` and priority of
//! the original's `PlaySound` call, decided where the call is ported
//! ([`UiSound::cue`]). Re-exported from [`sound`](crate::sound).

use nova_sim::sound::BEEP;

use crate::sound::{SoundCue, SoundId, UiSound};

/// The main screen's four sounds, 600 to 603: its button clicks, "Menu
/// button down" and "Menu button up" (the resources' names, and the
/// community resource guide's), and its row slides, "Menu button start"
/// and "Menu button end" (the stock resources' names). `_LoadMainScreen`
/// loads them and `_DisposeMainScreenSounds` @0x16cae halts and frees them
/// all as play is entered ([`UiSound::MainScreenSoundsHalted`]).
pub const MAIN_SCREEN_SOUNDS: [SoundId; 4] =
    [SoundId(600), SoundId(601), SoundId(602), SoundId(603)];

/// A main-menu button's click: `_PlayMainScreenClickSound` @0x177d9,
/// @0x17812, and `_TrackMainScreenButtons` @0x19687, @0x19974, @0x19ac4,
/// @0x19c38, all at 10.
const BUTTON_PRIORITY: u16 = 10;

/// A main-screen slide's start or end: `_MainScreenAnimIdle` @0x188e6,
/// @0x1891c, at 50, above the buttons' 10, so a click's 600 doesn't cut
/// them off.
const ROW_SLIDE_PRIORITY: u16 = 50;

impl UiSound {
    /// What the sound plays: the `snd ` and priority of the original's
    /// call that makes it. `None` for the events that play no one `snd `:
    /// the keyed click, which plays two in turn (the audio side's
    /// [`UiSound::ButtonDown`] then [`UiSound::ButtonUp`]); the alert, the
    /// system's `SysBeep` (as the Text Input dialog refuses a name too long,
    /// `_EVTextInputDialog` @0x554a9-0x554e7), which no stock `snd ` holds;
    /// the halts; and the switch-out, which stops the music.
    ///
    /// Every variant has its arm, with no wildcard: a new sound decides
    /// here what it plays.
    #[must_use]
    #[allow(clippy::match_same_arms)] // each arm is its own call sites, with their addresses
    pub const fn cue(self) -> Option<SoundCue> {
        let cue = match self {
            Self::ButtonDown => SoundCue::new(MAIN_SCREEN_SOUNDS[0], BUTTON_PRIORITY),
            Self::ButtonUp => SoundCue::new(MAIN_SCREEN_SOUNDS[1], BUTTON_PRIORITY),
            Self::ButtonKeyed
            | Self::Alert
            | Self::PortAmbientHalted(_)
            | Self::IntroPictureHalted(_)
            | Self::MainScreenSoundsHalted
            | Self::SwitchedOut => return None,
            // `_MainScreenAnimIdle` as a slide steps from its first frame
            // (@0x188fc) and as it reaches its count (@0x18932).
            Self::RowSlideStart => SoundCue::new(MAIN_SCREEN_SOUNDS[2], ROW_SLIDE_PRIORITY),
            Self::RowSlideEnd => SoundCue::new(MAIN_SCREEN_SOUNDS[3], ROW_SLIDE_PRIORITY),
            // `_MainScreenChoice(3)` as the main screen's E finds no game
            // in progress, or a dying pilot (`PlaySound`
            // @0x1aa89-0x1aaac), at 10.
            Self::EnterRefused => SoundCue::new(BEEP[3], 10),
            // `_HandlePlayer` as E opens the escort menu
            // (@0x6de8c-0x6deb4) and closes it (@0x6e02b-0x6e04d), at 1.
            Self::EscortMenuToggled => SoundCue::new(BEEP[0], 1),
            // With "You don't have any escorts." (`_HandlePlayer`
            // @0x6de08-0x6de87), at 1.
            Self::NoEscorts => SoundCue::new(BEEP[3], 1),
            // `_DoShipyardDialog`'s item 1 with no ship to buy or hire, or
            // one it cannot (`PlaySound` @0x5f238-0x5f256), at 1.
            Self::BuyRefused => SoundCue::new(BEEP[3], 1),
            // As keys 1-5 select a group (`_HandlePlayer`
            // @0x6e126-0x6e149), at 1.
            Self::EscortGroupSelected => SoundCue::new(BEEP[2], 1),
            // `_DoNewsDialog` as it opens (`PlaySound` @0x47f36) and
            // `_DoMissionInfoDialog` too (@0x9e1d5), at 1.
            Self::NewsOpened | Self::MissionInfoOpened => SoundCue::new(BEEP[1], 1),
            // `_DoNewsDialog` as it closes (@0x4814e) and
            // `_DoMissionInfoDialog` too (@0x9e791), at 1.
            Self::NewsClosed | Self::MissionInfoClosed => SoundCue::new(BEEP[2], 1),
            // `_HandlePlayer` as `-` or `=` zooms the floating map
            // (@0x7103a, @0x710bc), at 1.
            Self::FloatingMapZoomed => SoundCue::new(BEEP[0], 1),
            // "Beep3": `_HandlePlayer` as the map key's press passes every
            // gate, just before it calls `_DoSystemMap` (`PlaySound`
            // @0x6a7e4-0x6a809), at 1. A refused press plays nothing
            // (every gate jumps past it, to @0x6a89f).
            Self::MapOpened => SoundCue::new(BEEP[2], 1),
            // "Beep1": `_HandlePlayer` with no condition once `_DoSystemMap`
            // returns (@0x6a842-0x6a85a), however the map closed, at 1.
            // The hypergate's map plays neither: `_PlayerEnterHypergate`
            // calls `_DoSystemMap` (@0x6385b) with no `PlaySound` before
            // or after it.
            Self::MapClosed => SoundCue::new(BEEP[0], 1),
            // "Beep5": `_MapFilter`'s `PlaySound(.., 1, ..)` first as Space
            // goes down, on every map (@0x140fe-0x14119).
            Self::MapCentred => SoundCue::new(BEEP[4], 1),
            // "Beep5" as `_DoSystemSearch` selects the system found, and
            // "Beep4" when it finds none: its `PlaySound(.., 1, ..)`
            // @0x120b6 and @0x120d8, played @0x120f3. Neither plays for an
            // empty text (@0x11e26-0x11e5d).
            Self::MapSystemFound => SoundCue::new(BEEP[4], 1),
            Self::MapSystemNotFound => SoundCue::new(BEEP[3], 1),
            // `_planetSnd`: the planet param block `_LoadSounds` fills with
            // 0x7d00 (@0x1c1b8), played by `ST_PlaySoundParam` from
            // `_PortFilter` (@0x4f0e8, @0x4f106) and the landscape's press
            // (@0x5fe59). `_PlanetSoundCallback`'s replay of a looped one
            // as that voice ends plays at 10 (@0x55f9c; rdm task
            // `refused-and-port-ambient-priorities-by-site`). The audio
            // side plays a looped one unless it is still playing.
            Self::PortAmbient { sound, .. } => SoundCue::new(sound, 32_000),
            // `_ShowIntroScreen`'s `_PlaySound(.., 0xa, ..)` @0x16ebb, once
            // it has cancelled the music.
            Self::IntroPicture(sound) => SoundCue::new(sound, 10),
        };
        Some(cue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(sound: i16, priority: u16) -> SoundCue {
        SoundCue::new(SoundId(sound), priority)
    }

    /// One row for each variant, each with the original's global and call
    /// site. Rust can't count an enum's variants, so a new variant could
    /// miss its row here: the exhaustive [`UiSound::cue`] is what makes its
    /// author decide what it plays, and its row is CLAUDE.md's "sounds
    /// come with the function".
    #[test]
    #[allow(clippy::too_many_lines)] // one row for each of the interface's sounds
    fn each_ui_sound_plays_its_snd_at_its_priority() {
        let ambient = |until_stopped| UiSound::PortAmbient {
            sound: SoundId(10_032),
            until_stopped,
        };
        let pinned = [
            (
                UiSound::ButtonDown,
                Some(cue(600, 10)),
                "Menu button down @0x19687",
            ),
            (
                UiSound::ButtonUp,
                Some(cue(601, 10)),
                "Menu button up @0x19c38",
            ),
            (
                UiSound::ButtonKeyed,
                None,
                "600, then 601 once 600 has ended",
            ),
            (
                UiSound::RowSlideStart,
                Some(cue(602, 50)),
                "Menu button start @0x188fc",
            ),
            (
                UiSound::RowSlideEnd,
                Some(cue(603, 50)),
                "Menu button end @0x18932",
            ),
            (
                UiSound::EnterRefused,
                Some(cue(153, 10)),
                "_beepSnd[3] @0x1aaac",
            ),
            (UiSound::Alert, None, "the system's SysBeep"),
            (
                UiSound::EscortMenuToggled,
                Some(cue(150, 1)),
                "_beepSnd[0] @0x6deb4",
            ),
            (
                UiSound::NoEscorts,
                Some(cue(153, 1)),
                "_beepSnd[3] @0x6de82",
            ),
            (
                UiSound::BuyRefused,
                Some(cue(153, 1)),
                "_beepSnd[3] @0x5f256",
            ),
            (
                UiSound::EscortGroupSelected,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x6e149",
            ),
            (
                UiSound::NewsOpened,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x47f36",
            ),
            (
                UiSound::NewsClosed,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x4814e",
            ),
            (
                UiSound::MissionInfoOpened,
                Some(cue(151, 1)),
                "_beepSnd[1] @0x9e1d5",
            ),
            (
                UiSound::MissionInfoClosed,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x9e791",
            ),
            (
                UiSound::FloatingMapZoomed,
                Some(cue(150, 1)),
                "_beepSnd[0] @0x7103a",
            ),
            (
                UiSound::MapOpened,
                Some(cue(152, 1)),
                "_beepSnd[2] @0x6a809",
            ),
            (
                UiSound::MapClosed,
                Some(cue(150, 1)),
                "_beepSnd[0] @0x6a85a",
            ),
            (
                UiSound::MapCentred,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x14119",
            ),
            (
                UiSound::MapSystemFound,
                Some(cue(154, 1)),
                "_beepSnd[4] @0x120f3",
            ),
            (
                UiSound::MapSystemNotFound,
                Some(cue(153, 1)),
                "_beepSnd[3] @0x120f3",
            ),
            (
                ambient(false),
                Some(cue(10_032, 32_000)),
                "_planetSnd @0x4f106",
            ),
            (
                ambient(true),
                Some(cue(10_032, 32_000)),
                "_planetSnd @0x4f0e8",
            ),
            (UiSound::PortAmbientHalted(SoundId(10_032)), None, "a halt"),
            (
                UiSound::IntroPicture(SoundId(30_003)),
                Some(cue(30_003, 10)),
                "@0x16ebb",
            ),
            (UiSound::IntroPictureHalted(SoundId(30_003)), None, "a halt"),
            (UiSound::MainScreenSoundsHalted, None, "a halt"),
            (UiSound::SwitchedOut, None, "the music's stop"),
        ];
        for (sound, cue, source) in pinned {
            assert_eq!(sound.cue(), cue, "{sound:?}: {source}");
        }
    }

    #[test]
    fn the_main_screens_sounds_are_its_clicks_and_slides() {
        let cued = [
            UiSound::ButtonDown,
            UiSound::ButtonUp,
            UiSound::RowSlideStart,
            UiSound::RowSlideEnd,
        ]
        .map(|sound| sound.cue().map(|cue| cue.sound));
        assert_eq!(MAIN_SCREEN_SOUNDS.map(Some), cued);
    }
}
