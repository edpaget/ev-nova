//! The sounds screens make, as events: what happened, not what to play.
//! The audio side decides which `snd ` each one plays, if any.
//!
//! - [`UiSound`]: the interface's own, a button pressed and released,
//!   the alert as something is refused, the news dialog's beeps, the
//!   course map's beeps, and the beep of the player info refused.
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
//! with the choice they make. The spaceport's screens map their keys to
//! items with no sound (`_PortFilter` @0x4f041, `_BarFilter` @0x47a4b,
//! `_TradeFilter` @0x4e092, `_OutfitFilter` @0x58039, `_ShipyardFilter`
//! @0x60449, `_MissionFilter` @0x9f7a9, `_MissionInfoFilter` @0x9d6e5,
//! `_SingleMissionFilter` @0x98d9a).

pub use nova_sim::{SimSound, SoundId};

/// A sound the interface makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    /// A button was pressed: the pointer went down on it. A key that
    /// activates a button never raises it (see the module docs).
    ButtonDown,
    /// A pressed button popped back up: the pointer was let go, on the
    /// button or off it. A key that activates a button never raises it
    /// (see the module docs).
    ButtonUp,
    /// A key activated a main-menu button: `_PlayMainScreenClickSound`
    /// @0x177af, which plays snd 600, then 601 once 600 has ended.
    ButtonKeyed,
    /// The interface refused what was asked of it: the original's
    /// `SysBeep`.
    Alert,
    /// E opened or closed the escort menu (`_beepSnd[0]`).
    EscortMenuToggled,
    /// E found no escort to command (`_beepSnd[3]`).
    NoEscorts,
    /// The escort menu selected a group (`_beepSnd[2]`).
    EscortGroupSelected,
    /// The bar's news dialog opened: `_DoNewsDialog` plays `_beepSnd[1]`
    /// as it opens (`PlaySound` @0x47f36).
    NewsOpened,
    /// The bar's news dialog closed: `_DoNewsDialog` plays `_beepSnd[2]`
    /// as it closes (@0x4814e).
    NewsClosed,
    /// M opened the course map in flight: `_HandlePlayer` plays
    /// `_beepSnd[2]` before it calls `_DoSystemMap` (`PlaySound`
    /// @0x6a809). The hypergate's map does not raise it.
    MapOpened,
    /// The course map closed in flight, by M or Escape: `_HandlePlayer`
    /// plays `_beepSnd[0]` once `_DoSystemMap` returns (`PlaySound`
    /// @0x6a85a). The hypergate's map does not raise it.
    MapClosed,
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
        let fired = Sound::Combat(CombatSound {
            sound: SoundId(208),
            offset: (-300, 40),
        });
        assert_eq!(
            fired,
            Sound::Combat(CombatSound {
                sound: SoundId(208),
                offset: (-300, 40),
            })
        );
        assert_ne!(
            fired,
            Sound::Combat(CombatSound {
                sound: SoundId(208),
                offset: (0, 0),
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
