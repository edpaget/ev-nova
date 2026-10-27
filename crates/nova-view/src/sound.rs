//! The sounds screens make, as events: what happened, not what to play.
//! The audio side decides which `snd ` each one plays, if any.
//!
//! - [`UiSound`]: the interface's own, a main-menu button pressed and
//!   released, the main screen's rows sliding in, its sounds halted as
//!   play is entered, an intro picture's sound, the alert as something is refused, the news dialog's
//!   beeps, the course map's beeps (opening, closing, Space and Find),
//!   and the beep of the player info
//!   refused.
//! - [`SimSound`]: the flight session's, re-exported from `nova_sim`.
//! - [`CombatSound`]: a fight's, a `snd ` heard from where it happened.
//! - [`Sound`]: any of them, as a screen reports it through
//!   [`Screen::take_sounds`](crate::Screen::take_sounds).
//! - [`SoundPrefs`]: the player's sound preferences as the Preferences
//!   dialog shows them, part of the [`Prefs`](crate::Prefs) a screen
//!   reports through [`Screen::take_prefs`](crate::Screen::take_prefs).
//!
//! **A key that activates a button makes no sound**, but on the main
//! menu. Return, Enter, Escape, Space on a focused button, or a dialog's
//! or the spaceport's own letter keys raise neither [`UiSound::ButtonDown`]
//! nor [`UiSound::ButtonUp`], as in the original: `_TextDialogFilter` maps Return, Enter and Escape straight
//! to an item (@0x51fca-0x51fef) with no hilite, flash or sound;
//! `_CommFilter`, `_PlanetCommFilter`, `_PlunderFilter`, `_ShipDescFilter`,
//! `_PlayerInfoFilter` and `_MyModalDialog` call no sound routine; the
//! dialogs run with the Toolbox's standard filter (`_DoNewPilotNameDialog`,
//! `_DoQuantityDialog`, `_EVTextInputDialog`, `_EVNoteAlert`) only flash
//! the button; and the Cocoa Preferences' `accept:` @0xcfe3a and
//! `cancel:` @0xcff7c play nothing. The original's button clicks, snd 600
//! and 601, belong to its main screen alone: `_TrackMainScreenButtons`
//! @0x195b9 plays them under the pointer, and `_PlayMainScreenClickSound`
//! @0x177af plays both, one after the other, for `_MainScreenKey`'s keys.
//! So the main menu's shortcut keys raise [`UiSound::ButtonKeyed`], once,
//! and their choice waits for the click to play. The spaceport's screens map their keys to
//! items with no sound (`_PortFilter` @0x4f041, `_BarFilter` @0x47a4b,
//! `_TradeFilter` @0x4e092, `_OutfitFilter` @0x58039, `_ShipyardFilter`
//! @0x60449, `_MissionFilter` @0x9f7a9, `_MissionInfoFilter` @0x9d6e5,
//! `_SingleMissionFilter` @0x98d9a).
//!
//! **A pointer click makes no sound either**, but on the main menu. Of the
//! original's button trackers only `_TrackMainScreenButtons` @0x195b9
//! plays a sound. The rest call no sound routine, so their buttons are
//! silent under the pointer: the in-game dialogs' `_TrackTextButtons`
//! @0x26bb0 (text dialogs, descriptions and the About box),
//! `_TrackOkButton` @0x26d7a, `_TrackCommDialogButtons` @0x28eff,
//! `_TrackPlanetCommDialogButtons` @0x28b28, `_TrackHaggleButtons`
//! @0x27670, `_TrackPlunderButtons` @0x2732a, `_TrackCapAssignButtons`
//! @0x291c7, `_TrackPlayerInfoButtons` @0x27c4d, `_TrackRaceButtons`
//! @0x2689d and `_TrackMapButtons` @0x2aafa, and the spaceport's
//! `_TrackPlanetButtons` @0x2996f, `_TrackBarButtons` @0x26fd5,
//! `_TrackEscortButtons` @0x27987, `_TrackTradeButtons` @0x294cd,
//! `_TrackOutfitButtons` @0x2a029, `_TrackShipyardButtons` @0x2a54e,
//! `_TrackMissionButtons` @0x287f5, `_TrackMissionInfoButtons` @0x2850e
//! and `_TrackSingleMissionButtons` @0x28188. Open Pilot is Navigation
//! Services (`_MyOpenDocument` @0x731c5), which plays nothing either.

pub use nova_sim::{SimSound, SoundId};

/// A sound the interface makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    /// A main-menu button was pressed: the pointer went down on it
    /// (@0x19687), or onto it with the press held, from off every button
    /// or straight from another (@0x19974), in
    /// `_TrackMainScreenButtons` @0x195b9. Only the main menu's buttons
    /// raise it; a key that activates a button never does (see the module
    /// docs).
    ButtonDown,
    /// A pressed main-menu button popped back up: the held press was
    /// dragged off every button (@0x19ac4), or let go of over a button,
    /// choosing it (@0x19c38), in `_TrackMainScreenButtons` @0x195b9; a
    /// press let go of off every button is silent. Only the main menu's
    /// buttons raise it; a key that activates a button never does (see
    /// the module docs).
    ButtonUp,
    /// A key activated a main-menu button: `_PlayMainScreenClickSound`
    /// @0x177af, which plays snd 600, then 601 once 600 has ended. The
    /// key's choice waits for that click: the main menu acts on it only
    /// once the audio side says 600 has ended
    /// ([`Screen::keyed_click_ended`](crate::Screen::keyed_click_ended)).
    /// E clicks on its release, not its press.
    ButtonKeyed,
    /// A main-screen slide stepped from its first frame, a row of buttons
    /// starting to slide in: `_MainScreenAnimIdle` plays snd 602, "Menu
    /// button start", at priority 50 (@0x188e6-0x188fc).
    RowSlideStart,
    /// A main-screen slide reached its count, its row of buttons shown:
    /// `_MainScreenAnimIdle` plays snd 603, "Menu button end", at
    /// priority 50 (@0x1891c-0x18932). A click that finishes the slides
    /// at once plays none (`_MainScreenMouse` @0x1ae3a-0x1ae99).
    RowSlideEnd,
    /// Enter Ship was chosen on the main screen with no pilot loaded, or
    /// a dying one: `_MainScreenChoice(3)` @0x1aa89-0x1aaac plays
    /// `_beepSnd[3]`. E raises it after its click.
    EnterRefused,
    /// The interface refused what was asked of it: the original's
    /// `SysBeep`.
    Alert,
    /// E opened or closed the escort menu (`_beepSnd[0]`).
    EscortMenuToggled,
    /// E found no escort to command (`_beepSnd[3]`).
    NoEscorts,
    /// B in the shipyard, or H on the hire screen, asked for a ship with
    /// none selected, or one that cannot be bought or hired:
    /// `_DoShipyardDialog`'s item 1 plays `_beepSnd[3]` (`snd ` 153) at
    /// priority 1 (@0x5e9bb-0x5e9d6 → @0x5f22a/@0x5f231, `PlaySound`
    /// @0x5f238-0x5f256), in both modes. Buy Ship and Hire Escort are
    /// greyed then, and a greyed button is never pressed
    /// (`_TrackShipyardButtons` @0x2a63c-0x2a64b), so only the key beeps.
    BuyRefused,
    /// The escort menu selected a group (`_beepSnd[2]`).
    EscortGroupSelected,
    /// The bar's news dialog opened: `_DoNewsDialog` plays `_beepSnd[1]`
    /// as it opens (`PlaySound` @0x47f36).
    NewsOpened,
    /// The bar's news dialog closed: `_DoNewsDialog` plays `_beepSnd[2]`
    /// as it closes (@0x4814e).
    NewsClosed,
    /// The mission info screen opened: `_DoMissionInfoDialog` plays
    /// `_beepSnd[1]` (`snd ` 151) as it opens (@0x9e1d5), before its
    /// dialog is loaded.
    MissionInfoOpened,
    /// The mission info screen closed: `_DoMissionInfoDialog` plays
    /// `_beepSnd[2]` (`snd ` 152) as it closes (@0x9e791), after its
    /// `_RefreshSAT`; without its dialog it returns before it (@0x9e20b).
    MissionInfoClosed,
    /// A zoom key zoomed the floating map: `_HandlePlayer` plays
    /// `_beepSnd[0]` (@0x7103a, @0x710bc).
    FloatingMapZoomed,
    /// M opened the course map in flight: `_HandlePlayer` plays
    /// `_beepSnd[2]` before it calls `_DoSystemMap` (`PlaySound`
    /// @0x6a809). The hypergate's map does not raise it.
    MapOpened,
    /// The course map closed in flight, by M or Escape: `_HandlePlayer`
    /// plays `_beepSnd[0]` once `_DoSystemMap` returns (`PlaySound`
    /// @0x6a85a). The hypergate's map does not raise it.
    MapClosed,
    /// Space centred a map on the current system: `_MapFilter` plays
    /// `_beepSnd[4]` (`snd ` 154) first, on key-down, on every map
    /// (`PlaySound` @0x14119).
    MapCentred,
    /// The map's Find found a system: `_DoSystemSearch` plays
    /// `_beepSnd[4]` (`snd ` 154) as it selects it (@0x120c6-0x120f3).
    MapSystemFound,
    /// The map's Find found no system: `_DoSystemSearch` plays
    /// `_beepSnd[3]` (`snd ` 153) (@0x120d0-0x120f3). An empty text plays
    /// nothing.
    MapSystemNotFound,
    /// The stellar's ambient sound, its `CustSndID` (128 and up), plays
    /// while the spaceport is frontmost: `_PortFilter` @0x4f041 plays
    /// `_planetSnd` (@0x4f0e8, @0x4f106), and so does a press on the
    /// landscape (`_DoPortDialog`'s item 5, @0x5fe2a-0x5fe5e). The
    /// spaceport decides when (see [`spaceport::ambient`]).
    ///
    /// [`spaceport::ambient`]: crate::spaceport::ambient
    PortAmbient {
        /// The `snd `.
        sound: SoundId,
        /// Whether it plays only if it is not still playing, the
        /// original's `ST_IsSoundPlaying` check for a stellar whose
        /// `Flags2` has 0x0010 (@0x4f0cd-0x4f0de): a looped ambient
        /// sound. Otherwise it plays over whatever is playing.
        until_stopped: bool,
    },
    /// The spaceport closed: its ambient sound stops, as `_DoPortDialog`
    /// halts `_planetSnd` as the port closes (`ST_HaltSound`
    /// @0x601d2-0x601eb), whatever closed it.
    PortAmbientHalted(SoundId),
    /// A picture of the intro screen started showing, and the data has
    /// its `snd `: `_ShowIntroScreen` cancels the music, then plays snd
    /// 30003 at priority 10 (`_CancelMusicThread` @0x16eae, `_PlaySound`
    /// @0x16eb3-0x16ed1). See `menu::intro`.
    IntroPicture(SoundId),
    /// An intro picture ended: its `snd ` is halted (@0x17064), whatever
    /// the sound setting.
    IntroPictureHalted(SoundId),
    /// Play is being entered: the main screen's four sounds, its button
    /// clicks and row slides, are halted, as `_DisposeMainScreenSounds`
    /// @0x16cae (called @0x19e91) halts and frees snd 600-603, cutting
    /// off Enter Ship's own button-up click. Whatever the sound setting.
    MainScreenSoundsHalted,
}

/// A sound a fight makes: a weapon firing or an explosion, heard from
/// where it happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatSound {
    /// The `snd `.
    pub sound: SoundId,
    /// Where it happened from the player, in whole pixels, x right and y
    /// down: how far away it is heard.
    pub offset: (i32, i32),
    /// The original's priority for it, `_PlaySoundDistance`'s third
    /// argument: how much it matters when the voices are full.
    pub priority: u16,
    /// Whether it is skipped while its `snd ` plays on any voice, as the
    /// original's looped weapons are (`ST_IsSoundPlaying`); named after
    /// [`UiSound::PortAmbient`]'s field.
    pub until_stopped: bool,
}

/// A sound a screen reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// From the flight session.
    Sim(SimSound),
    /// From the interface.
    Ui(UiSound),
    /// From a fight in flight.
    Combat(CombatSound),
}

/// The loudest volume level; levels run from 0 (silent) to this. The
/// original's Preferences stepper (`Keys.nib`) has `NSMaxValue` 8, and
/// `-[EVPrefsDialogController setStatusForVolume:]` @0xd0059 labels all
/// nine levels, 0 "No Bloody Noise" to 8 "Hoo Boy!".
pub const MAX_LEVEL: u8 = 8;

/// The level a player starts at, the original's "Normal": `_LoadPrefs`
/// defaults a missing `Volume` to 4 (@0xcef6c-0xcef8f), as does the
/// stepper's `NSValue` in `Keys.nib`.
pub const DEFAULT_LEVEL: u8 = 4;

/// The player's sound preferences, as the Preferences dialog shows them:
/// sound effects and music on or off, and each one's volume level, 0
/// through [`MAX_LEVEL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoundPrefs {
    /// Whether sound effects play.
    pub sound: bool,
    /// Whether music plays.
    pub music: bool,
    /// The sound effects' volume level.
    pub effects_level: u8,
    /// The music's volume level.
    pub music_level: u8,
}

impl SoundPrefs {
    /// Whether the effects are quiet enough for the Red Alert to blink its
    /// icon: off, or at level 1 or less. The original's one `Volume` level
    /// (0-8) blinks it at 1 or less (`_HandlePlayer` @0x68d04-0x68d15);
    /// here sound off stands for its 0.
    #[must_use]
    pub fn alert_quiet(self) -> bool {
        !self.sound || self.effects_level <= 1
    }
}

impl Default for SoundPrefs {
    /// Sound and music on, both at [`DEFAULT_LEVEL`].
    fn default() -> Self {
        Self {
            sound: true,
            music: true,
            effects_level: DEFAULT_LEVEL,
            music_level: DEFAULT_LEVEL,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_alert_is_quiet_with_sound_off_or_at_level_1_or_less() {
        let at = |sound, effects_level| SoundPrefs {
            sound,
            effects_level,
            ..SoundPrefs::default()
        };
        assert!(at(false, 7).alert_quiet(), "sound off");
        assert!(at(true, 0).alert_quiet());
        assert!(at(true, 1).alert_quiet());
        assert!(!at(true, 2).alert_quiet());
        assert!(!at(true, 7).alert_quiet());
        assert!(!SoundPrefs::default().alert_quiet());
    }

    #[test]
    fn a_combat_sound_is_its_snd_heard_from_where_it_happened() {
        let heard = CombatSound {
            sound: SoundId(208),
            offset: (-300, 40),
            priority: 4,
            until_stopped: true,
        };
        let fired = Sound::Combat(heard);
        assert_eq!(fired, Sound::Combat(heard));
        assert_ne!(
            fired,
            Sound::Combat(CombatSound {
                offset: (0, 0),
                ..heard
            })
        );
        assert_ne!(
            fired,
            Sound::Combat(CombatSound {
                priority: 6,
                ..heard
            })
        );
    }

    #[test]
    fn the_default_is_everything_on_at_the_normal_level() {
        assert_eq!(MAX_LEVEL, 8);
        assert_eq!(DEFAULT_LEVEL, 4);
        assert_eq!(
            SoundPrefs::default(),
            SoundPrefs {
                sound: true,
                music: true,
                effects_level: 4,
                music_level: 4,
            }
        );
    }
}
