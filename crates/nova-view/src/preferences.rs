//! The player's preferences as the Preferences dialog shows them, which a
//! screen reports through [`Screen::take_prefs`](crate::Screen::take_prefs).

use crate::scaling::Scaling;
use crate::sound::SoundPrefs;
use crate::video_mode::FullscreenKind;

/// The player's preferences, as the Preferences dialog shows them: the
/// sound preferences, whether a hyperspace jump plays its white fades,
/// whether a `dësc`'s movie plays, whether shots trail and smoke, whether
/// ships show their weapon glow, how the game runs fullscreen, and how it
/// is enlarged onto the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // One field per on-off preference.
pub struct Prefs {
    /// Sound effects and music, on or off, and their volumes.
    pub sound: SoundPrefs,
    /// Whether hyperspace jumps fade to white and back, the original's
    /// Hyperspace Effects preference.
    pub hyperspace_effects: bool,
    /// Whether a `dësc`'s QuickTime movie plays, the original's QuickTime
    /// Movies preference (`*0x3b501c + 7`, set meaning off: `_PlayQTMovie`
    /// @0x5597c).
    pub quicktime_movies: bool,
    /// Whether shots leave their particle trails and smoke, the original's
    /// Smoke Trails preference. The original keeps it inverted, as an
    /// "off" switch: `SmokeTrails` (prefs `*0x3b501c + 9`) set skips
    /// `_HandleShot`'s particle trail (@0x35d64-0x35d6d) and `_SpawnSmoke`
    /// (@0x3f93d-0x3f951), and its `Keys.nib` box binds it through
    /// `NSNegateBoolean`, so a checked box (this field true) stores NO.
    pub smoke_trails: bool,
    /// Whether ships show their weapon glow, the original's Weapon Effects
    /// preference (`WeaponEffects`, prefs `*0x3b501c + 0x10`): set, the
    /// ships' `shän` `WeapImageID` sprites load (`_LoadExtendedShipSprites`
    /// @0x2343c-0x2344b). The original reads it only at launch.
    pub weapon_effects: bool,
    /// How the game runs fullscreen: borderless, or in an exclusive
    /// display mode the dialog's picker chose. Not the original's: its
    /// dialog has no such control.
    pub fullscreen: FullscreenKind,
    /// The Scaling, as the dialog shows it: the one running, a per-frame
    /// upscaler or a load-time enlargement ([`Scaling::sites`]). Not the
    /// original's: its dialog has no such control.
    pub scaling: Scaling,
}

impl Default for Prefs {
    /// The default sound preferences, with the hyperspace effects, the
    /// movies, the smoke trails and the weapon effects on, as a fresh
    /// install of the original has them: it registers `HyperspaceEffects`
    /// and `QuickTimeMovies` as NO (`_LoadPrefs` @0xce8ac, @0xce8ee), and
    /// a clear byte means on; `SmokeTrails` as NO (@0xce949-0xce972),
    /// which `Keys.nib` negates, so it too means on; and `WeaponEffects`
    /// as YES (@0xce96a-0xce993). Fullscreen borderless, and Scaling
    /// native.
    fn default() -> Self {
        Self {
            sound: SoundPrefs::default(),
            hyperspace_effects: true,
            quicktime_movies: true,
            smoke_trails: true,
            weapon_effects: true,
            fullscreen: FullscreenKind::Borderless,
            scaling: Scaling::Native,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_default_sound_with_the_hyperspace_effects_and_movies_on() {
        assert_eq!(
            Prefs::default(),
            Prefs {
                sound: SoundPrefs::default(),
                hyperspace_effects: true,
                quicktime_movies: true,
                smoke_trails: true,
                weapon_effects: true,
                fullscreen: FullscreenKind::Borderless,
                scaling: Scaling::Native,
            }
        );
    }
}
