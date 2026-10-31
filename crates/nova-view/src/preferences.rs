//! The player's preferences as the Preferences dialog shows them, which a
//! screen reports through [`Screen::take_prefs`](crate::Screen::take_prefs).

use nova_sim::combat::SpriteLoad;

use crate::scaling::Scaling;
use crate::sound::SoundPrefs;
use crate::video_mode::FullscreenKind;

/// The player's preferences, as the Preferences dialog shows them: the
/// sound preferences, whether a hyperspace jump plays its white fades,
/// whether a `dësc`'s movie plays, whether shots trail and smoke, whether
/// ships show their weapon glow, animate, glow and show their running
/// lights, how the game runs fullscreen, and how it is enlarged onto the
/// window.
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
    /// Whether ships bank, fold and animate through their sets, the
    /// original's Ship Animations preference (`ShipAnimations`, prefs
    /// `*0x3b501c + 0xe`, `_LoadPrefs` @0xcee57-0xcee93): clear, the
    /// sprite load gives each class one base set and at most one alt set
    /// (`nova_sim::combat::sprite_load`). The original reads it only at
    /// launch.
    pub ship_animations: bool,
    /// Whether ships draw their engine glow, the original's Engine Glows
    /// preference (`EngineGlows`, prefs `*0x3b501c + 0xf`, `_LoadPrefs`
    /// @0xcee96-0xceed2): clear, no `GlowImageID` sprite loads
    /// (`_LoadExtendedShipSprites` @0x22e07-0x22e25). The original reads
    /// it only at launch.
    pub engine_glows: bool,
    /// Whether ships draw their running lights, the original's Running
    /// Lights preference (`RunningLights`, prefs `*0x3b501c + 0x11`,
    /// `_LoadPrefs` @0xcef14-0xcef50): clear, no `LightImageID` sprite
    /// loads (@0x230d7-0x230f5) and no ship's lights step. The original
    /// reads it only at launch.
    pub running_lights: bool,
    /// Whether each star of the starfield gets its own parallax factor,
    /// the original's Parallax Starfield preference (`_settings` +0xb,
    /// stored by `_LoadPrefs` @0xced11 and the dialog's `accept:`
    /// @0xcff03): set, `_RandomizeStars` draws each star's factor as
    /// `Rand(35)` x 0.01 (@0x0b2c4-0x0b2f0); clear, every factor is 0
    /// (@0x0b2f2-0x0b2fa). It applies at the next scattering.
    pub parallax_starfield: bool,
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
    /// as YES (@0xce96a-0xce993); `ShipAnimations`, `EngineGlows` and
    /// `RunningLights` as YES, each a box bound with no transformer, so
    /// checked means on; and `ParallaxStarfield` on when the key is
    /// missing (@0xcecee). Fullscreen borderless, and Scaling native.
    fn default() -> Self {
        Self {
            sound: SoundPrefs::default(),
            hyperspace_effects: true,
            quicktime_movies: true,
            smoke_trails: true,
            weapon_effects: true,
            ship_animations: true,
            engine_glows: true,
            running_lights: true,
            parallax_starfield: true,
            fullscreen: FullscreenKind::Borderless,
            scaling: Scaling::Native,
        }
    }
}

/// The preferences the original reads only at launch, which
/// `settingsChange:` compares with their launch values to show its
/// relaunch notice (@0xd0b90-0xd0c40): Ship Animations (+0x30 against
/// prefs +0xe), Engine Glows (+0x34, +0xf), Running Lights (+0x38, +0x11)
/// and Weapon Effects. Its windowed byte, the fifth, has no box here: its
/// row holds the display picker, which switches at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // One field per on-off preference.
pub struct LaunchPrefs {
    /// [`Prefs::ship_animations`].
    pub ship_animations: bool,
    /// [`Prefs::engine_glows`].
    pub engine_glows: bool,
    /// [`Prefs::weapon_effects`].
    pub weapon_effects: bool,
    /// [`Prefs::running_lights`].
    pub running_lights: bool,
}

impl LaunchPrefs {
    /// The four of `prefs`.
    #[must_use]
    pub fn of(prefs: &Prefs) -> Self {
        Self {
            ship_animations: prefs.ship_animations,
            engine_glows: prefs.engine_glows,
            weapon_effects: prefs.weapon_effects,
            running_lights: prefs.running_lights,
        }
    }

    /// What of each ship's sprites the session loads by them: its sets by
    /// the animations, its lights by the lights.
    #[must_use]
    pub fn sprite_load(self) -> SpriteLoad {
        SpriteLoad {
            ship_animations: self.ship_animations,
            running_lights: self.running_lights,
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
                ship_animations: true,
                engine_glows: true,
                running_lights: true,
                parallax_starfield: true,
                fullscreen: FullscreenKind::Borderless,
                scaling: Scaling::Native,
            }
        );
    }

    #[test]
    fn the_launch_prefs_are_the_four_that_wait_for_the_next_launch() {
        let prefs = Prefs {
            ship_animations: false,
            engine_glows: true,
            weapon_effects: false,
            running_lights: true,
            smoke_trails: false,
            ..Prefs::default()
        };
        assert_eq!(
            LaunchPrefs::of(&prefs),
            LaunchPrefs {
                ship_animations: false,
                engine_glows: true,
                weapon_effects: false,
                running_lights: true,
            }
        );
        let flipped = Prefs {
            ship_animations: true,
            engine_glows: false,
            weapon_effects: true,
            running_lights: false,
            ..prefs
        };
        assert_eq!(
            LaunchPrefs::of(&flipped),
            LaunchPrefs {
                ship_animations: true,
                engine_glows: false,
                weapon_effects: true,
                running_lights: false,
            }
        );
        assert_eq!(
            LaunchPrefs::of(&Prefs::default()),
            LaunchPrefs {
                ship_animations: true,
                engine_glows: true,
                weapon_effects: true,
                running_lights: true,
            },
            "all on by default"
        );
    }

    #[test]
    fn the_sprite_load_takes_the_animations_and_the_lights() {
        for (ship_animations, running_lights) in
            [(true, true), (false, true), (true, false), (false, false)]
        {
            let launch = LaunchPrefs {
                ship_animations,
                running_lights,
                engine_glows: !ship_animations,
                weapon_effects: !running_lights,
            };
            assert_eq!(
                launch.sprite_load(),
                SpriteLoad {
                    ship_animations,
                    running_lights,
                }
            );
        }
    }
}
