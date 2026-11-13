//! What of each ship's sprites loads, [`SpriteLoad`], and what it does to
//! a class's display ([`FrameSpec::loaded`]), from
//! `nova_sim_combat::combat::sprite_load`, which re-exports it and keeps
//! what a change of load does to a ship's display state
//! (`ShipFrames::refit`).

use super::ship_frames::FrameSpec;

/// What of each ship's sprites loads (see the module docs). The default
/// loads everything, as the original's preferences are registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteLoad {
    /// The `ShipAnimations` preference: a class's sets all load.
    pub ship_animations: bool,
    /// The `RunningLights` preference: a class's lights sprite loads.
    pub running_lights: bool,
}

impl Default for SpriteLoad {
    fn default() -> Self {
        Self {
            ship_animations: true,
            running_lights: true,
        }
    }
}

impl FrameSpec {
    /// This class's display as `load` loads its sprites (see
    /// `nova_sim_combat::combat::sprite_load`'s module docs): one base set and at most one alt set without the
    /// animations, and no lights sprite without the lights.
    #[must_use]
    pub fn loaded(self, load: SpriteLoad) -> FrameSpec {
        let mut spec = self;
        if !load.ship_animations {
            spec.base_sets = 1;
            spec.alt_sets = spec.alt_sets.min(1);
        }
        if !load.running_lights {
            spec.lights = false;
        }
        spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shown::combat::ship_frames::{ANIMATED, BANKS, FOLDS};

    /// Animations off, lights on.
    const STILL: SpriteLoad = SpriteLoad {
        ship_animations: false,
        running_lights: true,
    };

    /// Animations on, lights off.
    const DARK: SpriteLoad = SpriteLoad {
        ship_animations: true,
        running_lights: false,
    };

    /// A class of `flags`, `sets` base sets and `alt_sets` alt sets, its
    /// `AnimDelay` 0.
    fn class(flags: u16, sets: u16, alt_sets: u16) -> FrameSpec {
        FrameSpec {
            flags,
            base_sets: sets,
            alt_sets,
            ..FrameSpec::NONE
        }
    }

    #[test]
    fn everything_loads_by_default() {
        assert_eq!(
            SpriteLoad::default(),
            SpriteLoad {
                ship_animations: true,
                running_lights: true,
            }
        );
        let spec = FrameSpec {
            lights: true,
            ..class(ANIMATED, 4, 3)
        };
        assert_eq!(spec.loaded(SpriteLoad::default()), spec);
    }

    #[test]
    fn without_the_animations_a_class_has_one_base_set_and_at_most_one_alt_set() {
        let alt = class(ANIMATED, 3, 3).loaded(STILL);
        assert_eq!((alt.base_sets, alt.alt_sets), (1, 1));
        assert_eq!(alt.last_set(), 0);
        assert!(!alt.multi_set());
        let no_alt = class(FOLDS, 6, 0).loaded(STILL);
        assert_eq!((no_alt.base_sets, no_alt.alt_sets), (1, 0), "no alt sprite");
        let lit = FrameSpec {
            lights: true,
            ..class(BANKS, 3, 2)
        };
        assert!(lit.loaded(STILL).lights, "the lights still load");
        assert_eq!(lit.loaded(STILL).flags, BANKS, "its flags kept");
    }

    #[test]
    fn without_the_lights_a_class_has_no_lights_sprite_and_keeps_its_sets() {
        let spec = FrameSpec {
            lights: true,
            ..class(ANIMATED, 4, 3)
        };
        let dark = spec.loaded(DARK);
        assert!(!dark.lights);
        assert_eq!(
            dark,
            FrameSpec {
                lights: false,
                ..spec
            }
        );
    }
}
