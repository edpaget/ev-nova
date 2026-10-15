//! The sounds screens make, as events: what happened, not what to play.
//! The audio side decides which `snd ` each one plays, if any.
//!
//! - [`UiSound`]: the interface's own, a button pressed and released,
//!   the alert as something is refused, the news dialog's beeps, and the
//!   beep of the player info refused.
//! - [`SimSound`]: the flight session's, re-exported from `nova_sim`.
//! - [`CombatSound`]: a fight's, a `snd ` heard from where it happened.
//! - [`Sound`]: any of them, as a screen reports it through
//!   [`Screen::take_sounds`](crate::Screen::take_sounds).
//! - [`SoundPrefs`]: the player's sound preferences as the Preferences
//!   dialog shows them, part of the [`Prefs`](crate::Prefs) a screen
//!   reports through [`Screen::take_prefs`](crate::Screen::take_prefs).

pub use nova_sim::{SimSound, SoundId};

/// A sound the interface makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSound {
    /// A button was pressed: the pointer went down on it.
    ButtonDown,
    /// A pressed button popped back up: the pointer was let go, on the
    /// button or off it.
    ButtonUp,
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
    /// The player info key was refused in flight, during a jump:
    /// `_HandlePlayer` plays `_beepSnd[3]` (`PlaySound` @0x7039a-0x703bd).
    PlayerInfoRefused,
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

/// The loudest volume level; levels run from 0 (silent) to this.
pub const MAX_LEVEL: u8 = 7;

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
    /// (0-7) blinks it at 1 or less (`_HandlePlayer` @0x68d04-0x68d15);
    /// here sound off stands for its 0.
    #[must_use]
    pub fn alert_quiet(self) -> bool {
        !self.sound || self.effects_level <= 1
    }
}

impl Default for SoundPrefs {
    /// Sound and music on, both at the loudest level.
    fn default() -> Self {
        Self {
            sound: true,
            music: true,
            effects_level: MAX_LEVEL,
            music_level: MAX_LEVEL,
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
    fn the_default_is_everything_on_at_the_loudest_level() {
        assert_eq!(MAX_LEVEL, 7);
        assert_eq!(
            SoundPrefs::default(),
            SoundPrefs {
                sound: true,
                music: true,
                effects_level: 7,
                music_level: 7,
            }
        );
    }
}
