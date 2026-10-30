//! What of each ship's sprites loads: the `ShipAnimations` and
//! `RunningLights` preferences, as `_LoadExtendedShipSprites` reads them
//! in the `EV Nova` executable, and what each does to a class's display
//! ([`FrameSpec::loaded`]).
//!
//! - **`ShipAnimations`** (prefs +0xe, registered YES; `_LoadPrefs`
//!   @0xcee57-0xcee93). Off, the sprite load sets `BaseSets` to 1
//!   (@0x22ac3-0x22ade) and, for a class whose alt sprite loads,
//!   `AltSetCount` to 1 (@0x22b04-0x22b1e), so its alt sprite holds one
//!   set (@0x22c09-0x22c35); a class whose alt sprite doesn't load keeps
//!   none. With one base set [`ShipFrames::step`](super::ship_frames::ShipFrames::step)
//!   draws set 0 and moves no set timer (@0x2b837), so no ship shows a
//!   bank, fold or 0x0008 animation. Its alt layer steps 0 to 0. Its
//!   [`FrameSpec::last_set`] is 0, so a spawn folds to 0, and
//!   [`FrameSpec::unfolds_to_fire`] gives 1, which no NPC's
//!   fold-before-fire gate fires on. The bank flag and a banking ship's
//!   +2 glow are keyed on its `Flags`, not its sets, so they still run,
//!   as `_HandleShip`'s do.
//! - **`RunningLights`** (prefs +0x11, registered YES; `_LoadPrefs`
//!   @0xcef14-0xcef50). Off, no `LightImageID` master loads
//!   (@0x230d7-0x230f5), so [`FrameSpec::lights`] is false and
//!   [`ShipFrames::light`](super::ship_frames::ShipFrames::light) skips the
//!   whole lights block, blink and lights-out alike (@0x2b681-0x2b6a2,
//!   @0x2c2dd-0x2c2e4): the lights state stands still.
//!
//! **The sheets.** Off, the original loads only `FramesPer` frames of
//! each master (@0x2277b-0x22790). The view draws from the full sheets
//! all the same: with every set at 0, every frame it computes (hull, alt,
//! glow, lights, weapon and shield) is already below `FramesPer`, or is
//! the same index mod the layer's own frames, so the frames drawn are the
//! same.
//!
//! The original reads both preferences once, in `_InitEVNova`
//! (@0x4677b, @0x467d2, @0x47371), and `settingsChange:` shows its
//! relaunch notice while either differs from its launch value
//! (@0xd0b90-0xd0c40). When a change applies is the caller's choice. No
//! sound plays in any of it.

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
    /// This class's display as `load` loads its sprites (see the module
    /// docs): one base set and at most one alt set without the
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
    use nova_sim_base::blink::Blink;
    use nova_sim_base::reserves::Gauge;
    use nova_sim_catalog::fixture::weapon;

    use super::*;
    use crate::chance::NeverFires;
    use crate::combat::aim::Aim;
    use crate::combat::armament::{Armament, Fold, LauncherRounds, Shooter};
    use crate::combat::hull::Condition;
    use crate::combat::ship_frames::{
        ANIMATED, Alt, BANKS, DisplayRules, FOLDS, KEY_CARRIED, Moment, ShipFrames, UNFOLDS_TO_FIRE,
    };
    use crate::combat::weapon::WeaponSpec;
    use crate::rulebook::RuleSource;

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

    const MOMENT: Moment = Moment {
        disabled: false,
        key_aboard: false,
        rules: DisplayRules::ENGINE,
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

    /// The sets a ship of `spec` draws over `n` frames in `moment`, its
    /// display starting as `frames` and its fold as `fold`.
    fn sets(spec: &FrameSpec, mut frames: ShipFrames, mut fold: Fold, moment: Moment) -> Vec<u16> {
        (0..40)
            .map(|_| frames.step(spec, &mut fold, moment).set)
            .collect()
    }

    #[test]
    fn without_the_animations_no_ship_banks_folds_or_animates() {
        let still = vec![0; 40];
        for flag in [-1, 1] {
            let frames = ShipFrames {
                flag,
                ..ShipFrames::default()
            };
            let spec = class(BANKS, 3, 0).loaded(STILL);
            assert_eq!(
                sets(&spec, frames, Fold::default(), MOMENT),
                still,
                "banked {flag}"
            );
        }
        let folding = Fold {
            dir: 1,
            ..Fold::default()
        };
        let spec = class(FOLDS, 4, 0).loaded(STILL);
        assert_eq!(
            sets(&spec, ShipFrames::default(), folding, MOMENT),
            still,
            "folding"
        );
        let keyed = Moment {
            key_aboard: true,
            ..MOMENT
        };
        let spec = class(KEY_CARRIED, 2, 0).loaded(STILL);
        assert_eq!(
            sets(&spec, ShipFrames::default(), Fold::default(), keyed),
            still,
            "its key aboard"
        );
        let spec = class(ANIMATED, 3, 0).loaded(STILL);
        assert_eq!(
            sets(&spec, ShipFrames::default(), Fold::default(), MOMENT),
            still,
            "animated"
        );
        assert_ne!(
            sets(
                &class(ANIMATED, 3, 0),
                ShipFrames::default(),
                Fold::default(),
                MOMENT
            ),
            still,
            "animated, loaded whole"
        );
    }

    #[test]
    fn without_the_animations_the_alt_layer_shows_its_one_set() {
        let spec = class(0, 1, 3).loaded(STILL);
        let mut frames = ShipFrames::default();
        let mut fold = Fold::default();
        for _ in 0..20 {
            assert_eq!(frames.step(&spec, &mut fold, MOMENT).alt, Alt::Drawn(0));
        }
        let whole = class(0, 1, 3);
        let alts: Vec<Alt> = (0..3)
            .map(|_| frames.step(&whole, &mut fold, MOMENT).alt)
            .collect();
        assert_eq!(alts, [Alt::Drawn(1), Alt::Drawn(2), Alt::Drawn(0)]);
    }

    /// Whether an NPC of `spec`, its fold set at `set`, fires a blaster.
    fn npc_fires(spec: &FrameSpec, set: u16) -> bool {
        let blaster = weapon(128);
        let mut armament = Armament::new([(WeaponSpec::new(&blaster), 1)]).with_fold(Fold {
            set,
            dir: 0,
            since_fire: 0,
        });
        let shooter = Shooter {
            npc: true,
            folds: spec.unfolds_to_fire(),
            ..Shooter::PLAYER
        };
        armament
            .fire_pick(
                blaster.id,
                Condition::Intact,
                shooter,
                &mut LauncherRounds::default(),
                &mut Gauge::full(0.0),
                &mut NeverFires,
                RuleSource::Engine,
                &mut |_| Some(Aim::along(0.0, None)),
                None,
            )
            .is_some()
    }

    #[test]
    fn without_the_animations_an_npc_that_unfolds_to_fire_fires_at_once() {
        let whole = class(FOLDS | UNFOLDS_TO_FIRE, 6, 0);
        assert!(
            !npc_fires(&whole, whole.last_set()),
            "loaded whole, it spawns folded and turns to unfold"
        );
        let spec = whole.loaded(STILL);
        assert!(npc_fires(&spec, spec.last_set()), "spawned folded at 0");
        assert!(
            npc_fires(&spec, whole.last_set()),
            "a fold set left from its six sets"
        );
    }

    /// The Shuttle's square wave, `shän` 128.
    const SHUTTLE: Blink = Blink {
        mode: 1,
        a: 4,
        b: 1,
        c: 2,
        d: 20,
    };

    #[test]
    fn without_the_lights_no_ship_draws_lights_and_its_lights_stand_still() {
        let spec = FrameSpec {
            blink: SHUTTLE,
            lights: true,
            ..FrameSpec::NONE
        };
        let mut lit = ShipFrames::default();
        let levels: Vec<Option<u8>> = (0..30).map(|_| lit.light(&spec, &MOMENT, |_| 0)).collect();
        assert!(
            levels.iter().any(Option::is_some),
            "loaded whole, it blinks"
        );
        assert_ne!(lit.lights, ShipFrames::default().lights, "and steps");
        let dark = spec.loaded(DARK);
        let mut frames = ShipFrames::default();
        let before = frames.lights;
        for _ in 0..30 {
            assert_eq!(frames.light(&dark, &MOMENT, |_| 0), None);
        }
        assert_eq!(frames.lights, before, "never stepped");
    }
}
