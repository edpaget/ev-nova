//! Which frames of its sheets a ship is drawn on: the set its `shän`
//! `Flags` pick and the alt layer's, as `_HandleShipDisplay`
//! (@0x2b514-0x2dfa0) picks them each frame in the `EV Nova` executable.
//!
//! A class's display ([`FrameSpec`]) comes from its `shän`: its `Flags`,
//! its `BaseSets` (class +0xa00, `BaseSetCount` at least 1), its
//! `AnimDelay` (+0x9fe), its `AltSetCount` (+0x9fc, its alt sprite loaded
//! only with an `AltImageID` above 0 and an `AltSetCount` above 0) and its
//! `FramesPer` (+0xa06, 0 read as 36), as `_LoadExtendedShipSprites`
//! loads them (@0x22735-0x22b1e). The `ShipAnimations` preference, which
//! sets `BaseSets` and `AltSetCount` to 1 when off (prefs +0xe,
//! @0x22ac3-0x22b1e), and `RunningLights`, which drops the lights sprite,
//! apply on top of that ([`FrameSpec::loaded`], see
//! [`sprite_load`](super::sprite_load)).
//!
//! **The set** ([`ShipFrames::step`], @0x2b7d8-0x2bc2e). A hull whose
//! sprite holds no more frames than `FramesPer` draws set 0 and moves no
//! timer (@0x2b837). Every stock hull's sheet holds exactly `BaseSets` ×
//! `FramesPer` frames, so that is a hull of one set. Otherwise the
//! `Flags` pick it, the first that holds:
//!
//! - **0x0001, banking** ([`BANKS`], @0x2b85a-0x2b88f): set 1 while its
//!   bank flag (+0xc8f8) is below 0, set 2 above, set 0 at 0. The flag is
//!   the flight's (`nova_sim::traffic::ship_display` and the session's).
//! - **0x0002, folding** ([`FOLDS`], @0x2b894-0x2b9f1): the fold set
//!   (+0xc8fc, [`Armament::fold`](super::armament::Armament::fold)) steps
//!   toward its direction (+0xc8fa):
//!   with a direction either way its timer (+0xc8e4, the bank's own
//!   field) gains the frame's gain (@0x2b8a6, @0x2b914), and past
//!   `AnimDelay` (a float compare) it zeroes and the set steps, stopping
//!   at `BaseSets` − 1 going up or 0 going down, where the direction goes
//!   back to 0; with none the timer zeroes, unscaled (@0x2b97c). Then a ship of 0x0080
//!   ([`UNFOLDS_TO_FIRE`]) with no direction up, short of its last set,
//!   refolds once [`REFOLD_TICKS`] Mac ticks have passed since it last
//!   fired (+0xa8, @0x2b986-0x2b9e3).
//! - **0x0004** ([`KEY_CARRIED`], @0x2ba0b-0x2ba42): set 1 while its
//!   class's `KeyCarried` ship is aboard, else 0.
//! - **0x0008, animated** ([`ANIMATED`], @0x2ba47-0x2bb5c): the animation
//!   timer (+0xc8e0) gains the frame's gain (@0x2ba64), unless the ship is
//!   disabled and of 0x0010 ([`STILL_WHILE_DISABLED`]); past `AnimDelay`
//!   it loses `AnimDelay`, unscaled (@0x2bac1), while above it (or zeroes, with an `AnimDelay` of 0 or
//!   less) and the anim set (+0xc8f6) steps, round to 0 at `BaseSets`.
//!   A set past its last, rolled on another class's `BaseSets` at its
//!   spawn, is drawn mod `BaseSets`, as `_SWSetCurrentFrameIndex` takes a
//!   frame mod the sprite's frames (@0xa90d4-0xa90ee).
//! - none: set 0.
//!
//! **The frame.** Drawn at set × `FramesPer` + the rotation
//! ([`FrameSpec::index`]): the rotation is `trunc(heading × FramesPer /
//! 360)`, or, with `Flags` 0x0100 ([`ANGLE_TABLE`]), its frame-angle
//! table's ([`FrameTable`], @0x2bb94-0x2bc06). The glow, lights and
//! weapon layers take that index, each mod its own frames
//! (`_SWSetCurrentFrameIndex` @0xa90d4-0xa90ee); the shield layer by its
//! own rule ([`FrameSpec::shield_frame`]).
//!
//! **The alt layer** (@0x2c97d-0x2cb11), for a class with an alt sprite
//! ([`Alt`]): the animation timer
//! gains the frame's gain again (@0x2c9c1), unless still as above
//! (@0x2c9aa-0x2c9d6). Unless the set came by the 0x0008 path, past
//! `AnimDelay` the timer zeroes, unscaled (@0x2ca08), and the alt set
//! (+0xc8f4) steps; by that path, it steps when the
//! anim set did. Either way round to 0 at `AltSetCount`. It is drawn at
//! alt set × `FramesPer` + the rotation ([`FrameSpec::index`]). So a
//! 0x0008 ship of more than one set with an alt sprite moves its timer
//! twice a frame (two gains), as the rulebook's
//! [`RuleKey::ShipAnimationDoubleStep`](crate::RuleKey::ShipAnimationDoubleStep)
//! says by the engine; by the other reading, once. No stock ship does:
//! the Thunderforge (380), the one with an alt sprite, has one base set.
//!
//! A disabled ship of 0x0020 ([`ALT_HIDDEN_WHILE_DISABLED`]) skips the
//! whole alt block without hiding the sprite (@0x2b6b7-0x2b705; only the
//! no-alt-master path hides it, @0x2b6e8-0x2b700): no timer step, no set
//! step, no frame, tint or place (@0x2cecb-0x2cf5a). So by
//! [`RuleKey::DisabledAltHeld`](crate::RuleKey::DisabledAltHeld)'s engine
//! reading its alt layer is held ([`Alt::Held`]) at the frame, tint and
//! screen place it was last drawn at, which the view keeps; by the other
//! it is hidden ([`Alt::None`]). Either way its set and timer resume from
//! where they stopped on repair. No stock `shän` of 0x0020 has an alt
//! image.
//!
//! **The gain.** Each timer takes the frame's gain, `_gSpeedMult`
//! ([`Moment::gain`]), as a float field gains it: the sum in doubles,
//! narrowed (`SpeedMult::add_times`). So below full gain a timer steps its
//! set later, and at the settled gain (one ulp below 1.0) the narrowing
//! makes each frame a whole one, as at full. The compares against
//! `AnimDelay`, the losses and the zeroes are unscaled. No sound plays in
//! any of it.
//!
//! **The lights** ([`ShipFrames::light`]): the lights block, after the
//! alt layer's, steps the ship's lights state ([`ShipFrames::lights`]) and
//! puts it out (see [`ship_lights`](super::ship_lights)); its callers fill
//! [`Drawn::lights`] with it.
//!
//! **Ordering.** Each ship's display steps after its flight and its fire
//! in the same tick, before the frame is drawn, as `_HandleShipDisplay`
//! runs after `_HandlePlayer` and each `_HandleShip`; firing reads the
//! fold set the last display left, as the original's AI fires before the
//! display. No boundary the player sees falls between them.

use super::armament::Fold;
use super::ship_lights::ShipLights;
use crate::rulebook::RuleSource;
use crate::speed_mult::SpeedMult;

pub use nova_sim_state::shown::combat::ship_frames::{
    ALT_HIDDEN_WHILE_DISABLED, ANGLE_TABLE, ANIMATED, Alt, BANKS, Drawn, FOLDS, FrameSpec,
    FrameTable, KEY_CARRIED, LIGHTS_OUT_WHILE_DISABLED, STILL_WHILE_DISABLED, UNFOLDS_TO_FIRE,
};

/// The Mac ticks after its last shot a ship of [`UNFOLDS_TO_FIRE`] waits
/// to refold (`addl $0x2d` @0x2b9cb).
pub const REFOLD_TICKS: u32 = 45;

/// The rulebook readings a ship's display runs by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayRules {
    /// How often its alt block moves the animation timer, as
    /// [`RuleKey::ShipAnimationDoubleStep`](crate::RuleKey::ShipAnimationDoubleStep)
    /// says: again by the engine, not again after the set block moved it
    /// by the other reading.
    pub double_step: RuleSource,
    /// Whether a disabled 0x0020 ship's alt layer is held as last drawn, as
    /// [`RuleKey::DisabledAltHeld`](crate::RuleKey::DisabledAltHeld) says:
    /// by the engine, or hidden by the other reading.
    pub alt_held: RuleSource,
    /// Whether lights out writes the intensity 0 into its lights state, as
    /// [`RuleKey::LightsOutZeroesBlink`](crate::RuleKey::LightsOutZeroesBlink)
    /// says: by the engine, or only hides the layer by the other reading
    /// (see [`ship_lights`](super::ship_lights)).
    pub lights_zeroed: RuleSource,
}

impl DisplayRules {
    /// Every reading the engine's.
    pub const ENGINE: Self = Self {
        double_step: RuleSource::Engine,
        alt_held: RuleSource::Engine,
        lights_zeroed: RuleSource::Engine,
    };
}

/// What a ship's display reads this frame beside its own state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moment {
    /// Whether it is disabled (`_IsDisabled`, @0x2b61c).
    pub disabled: bool,
    /// Whether its class's `KeyCarried` ship is aboard
    /// (`_KeyCarriedShipTypeOnboard`, @0x2ba12).
    pub key_aboard: bool,
    /// The readings it runs by.
    pub rules: DisplayRules,
    /// The frame's gain at its display, `_gSpeedMult` (see the module
    /// docs' "The gain"): the fold timer gains it (@0x2b8a6, @0x2b914),
    /// the animation timer in the set block (@0x2ba64) and the alt block
    /// (@0x2c9c1), and the lights' timer and step (@0x2c3fe, @0x2c4a5,
    /// @0x2c523; see [`ship_lights`](super::ship_lights)).
    pub gain: SpeedMult,
}

/// A ship's display state, each field the original's (see the module
/// docs). The fold set, its direction and the last shot's stamp are its
/// [`Armament`](super::armament::Armament)'s ([`Fold`]), which firing
/// reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShipFrames {
    /// The bank, and a folding ship's fold timer, +0xc8e4: one field, as
    /// the original shares it.
    pub bank: f32,
    /// The bank flag, +0xc8f8: -1 banked left, 1 right, 0 level.
    pub flag: i8,
    /// The animation timer, +0xc8e0.
    pub anim_timer: f32,
    /// The anim set, +0xc8f6.
    pub anim_set: i16,
    /// The alt set, +0xc8f4.
    pub alt_set: i16,
    /// Its running lights' state ([`ShipLights`]).
    pub lights: ShipLights,
}

impl ShipFrames {
    /// One frame of `_HandleShipDisplay`'s set and alt blocks for a ship
    /// of class `spec`, its fold `fold`, in the `moment` (see the module
    /// docs): the sets it is drawn on.
    pub fn step(&mut self, spec: &FrameSpec, fold: &mut Fold, moment: Moment) -> Drawn {
        let still = spec.has(STILL_WHILE_DISABLED) && moment.disabled;
        let mut stepped = false;
        let mut animated = false;
        let set = if !spec.multi_set() {
            0
        } else if spec.has(BANKS) {
            match self.flag {
                ..0 => 1,
                0 => 0,
                1.. => 2,
            }
        } else if spec.has(FOLDS) {
            self.fold(spec, fold, moment.gain);
            fold.set
        } else if spec.has(KEY_CARRIED) {
            u16::from(moment.key_aboard)
        } else if spec.has(ANIMATED) {
            animated = true;
            stepped = self.animate(spec, still, moment.gain);
            // A set rolled on another class's `BaseSets` may lie past its
            // own: `_SWSetCurrentFrameIndex` takes the frame mod the
            // sprite's, `BaseSets` x `FramesPer` (@0xa90d4-0xa90ee).
            u16::try_from(self.anim_set).unwrap_or(0) % spec.base_sets
        } else {
            0
        };
        let alt = if spec.alt_sets == 0 {
            Alt::None
        } else if spec.has(ALT_HIDDEN_WHILE_DISABLED) && moment.disabled {
            match moment.rules.alt_held {
                RuleSource::Engine => Alt::Held,
                RuleSource::Bible => Alt::None,
            }
        } else {
            Alt::Drawn(self.alt_step(spec, moment, still, animated, stepped))
        };
        Drawn {
            set,
            alt,
            lights: None,
        }
    }

    /// The alt block (@0x2c97d-0x2cb11) of a ship of class `spec` in the
    /// `moment`, still when `still`, its set by the 0x0008 path when
    /// `animated`, that path's set having `stepped`: the alt set drawn.
    fn alt_step(
        &mut self,
        spec: &FrameSpec,
        moment: Moment,
        still: bool,
        animated: bool,
        stepped: bool,
    ) -> u16 {
        let again = !animated || moment.rules.double_step == RuleSource::Engine;
        if !still && again {
            self.anim_timer = moment.gain.add_times(self.anim_timer, 1.0);
        }
        let step_alt = if animated {
            stepped
        } else if self.anim_timer > f32::from(spec.anim_delay) {
            self.anim_timer = 0.0;
            true
        } else {
            false
        };
        if step_alt {
            self.alt_set += 1;
            if i32::from(self.alt_set) >= i32::from(spec.alt_sets) {
                self.alt_set = 0;
            }
        }
        u16::try_from(self.alt_set).unwrap_or(0)
    }

    /// The 0x0002 block (@0x2b894-0x2b9f1): `fold`'s set steps toward its
    /// direction on the timer, which gains `gain`, then a ship of 0x0080
    /// refolds once due.
    fn fold(&mut self, spec: &FrameSpec, fold: &mut Fold, gain: SpeedMult) {
        let delay = f32::from(spec.anim_delay);
        let last = i32::from(spec.base_sets) - 1;
        if fold.dir == 0 {
            self.bank = 0.0;
        } else {
            self.bank = gain.add_times(self.bank, 1.0);
            if self.bank > delay {
                self.bank = 0.0;
                let set = i32::from(fold.set) + i32::from(fold.dir.signum());
                if fold.dir > 0 && set > last {
                    fold.set = u16::try_from(last).unwrap_or(0);
                    fold.dir = 0;
                } else if fold.dir < 0 && set <= 0 {
                    fold.set = 0;
                    fold.dir = 0;
                } else {
                    fold.set = u16::try_from(set).unwrap_or(0);
                }
            }
        }
        if spec.has(UNFOLDS_TO_FIRE)
            && fold.dir <= 0
            && i32::from(fold.set) < last
            && fold.since_fire >= REFOLD_TICKS
        {
            fold.dir = 1;
        }
    }

    /// The 0x0008 block's timer, gaining `gain`, and anim set
    /// (@0x2ba4b-0x2bb55), still when `still`: whether the anim set
    /// stepped.
    fn animate(&mut self, spec: &FrameSpec, still: bool, gain: SpeedMult) -> bool {
        if !still {
            self.anim_timer = gain.add_times(self.anim_timer, 1.0);
        }
        let delay = f32::from(spec.anim_delay);
        if self.anim_timer <= delay {
            return false;
        }
        if spec.anim_delay > 0 {
            while self.anim_timer > delay {
                self.anim_timer -= delay;
            }
        } else {
            self.anim_timer = 0.0;
        }
        self.anim_set = self.anim_set.max(0) + 1;
        if i32::from(self.anim_set) >= i32::from(spec.base_sets) {
            self.anim_set = 0;
        }
        true
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::catalog::ShipDisplay;

    /// A class of `flags`, `sets` base sets, `AnimDelay` `delay`, 36
    /// frames a turn.
    fn spec(flags: u16, sets: u16, delay: i16) -> FrameSpec {
        FrameSpec {
            flags,
            base_sets: sets,
            anim_delay: delay,
            ..FrameSpec::NONE
        }
    }

    const MOMENT: Moment = Moment {
        disabled: false,
        key_aboard: false,
        rules: DisplayRules::ENGINE,
        gain: SpeedMult::FULL,
    };

    /// The moment, the double step read by `reading`.
    fn double_step(reading: RuleSource) -> Moment {
        Moment {
            rules: DisplayRules {
                double_step: reading,
                ..DisplayRules::ENGINE
            },
            ..MOMENT
        }
    }

    fn disabled() -> Moment {
        Moment {
            disabled: true,
            ..MOMENT
        }
    }

    /// The sets `frames` draws over `n` frames of `spec`, its fold starting
    /// at `fold`.
    fn run(frames: &mut ShipFrames, spec: &FrameSpec, fold: &mut Fold, n: usize) -> Vec<u16> {
        (0..n)
            .map(|_| frames.step(spec, fold, MOMENT).set)
            .collect()
    }

    fn spec_of(flags: u16) -> FrameSpec {
        FrameSpec::new(
            Some(&ShipDisplay {
                flags,
                base_sets: 2,
                ..ShipDisplay::default()
            }),
            None,
        )
    }

    #[test]
    fn a_banking_ship_draws_set_1_banked_left_and_2_banked_right() {
        let spec = spec(BANKS, 3, 0);
        let mut ship = ShipFrames::default();
        let mut fold = Fold::default();
        for (flag, set) in [(-1, 1), (0, 0), (1, 2)] {
            ship.flag = flag;
            assert_eq!(ship.step(&spec, &mut fold, MOMENT).set, set, "flag {flag}");
        }
    }

    #[test]
    fn a_single_set_ship_draws_set_0_and_moves_no_timer() {
        for flags in [BANKS, FOLDS, KEY_CARRIED, ANIMATED] {
            let spec = spec(flags, 1, 0);
            let mut ship = ShipFrames {
                flag: 1,
                ..ShipFrames::default()
            };
            let mut fold = Fold {
                dir: 1,
                ..Fold::default()
            };
            let drawn = ship.step(
                &spec,
                &mut fold,
                Moment {
                    key_aboard: true,
                    ..MOMENT
                },
            );
            assert_eq!(drawn, Drawn::default(), "{flags:#x}");
            assert_eq!((ship.bank, ship.anim_timer), (0.0, 0.0), "{flags:#x}");
            assert_eq!((fold.set, fold.dir), (0, 1), "{flags:#x}");
        }
    }

    #[test]
    fn a_folding_ship_steps_a_set_each_anim_delay_to_its_last_then_stops() {
        let spec = spec(FOLDS, 4, 2);
        let mut ship = ShipFrames::default();
        let mut fold = Fold {
            dir: 1,
            ..Fold::default()
        };
        assert_eq!(
            run(&mut ship, &spec, &mut fold, 11),
            [0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3],
            "past 2 every third frame"
        );
        assert_eq!((fold.set, fold.dir), (3, 1), "at the last, still folding");
        assert_eq!(run(&mut ship, &spec, &mut fold, 1), [3]);
        assert_eq!(
            (fold.set, fold.dir),
            (3, 0),
            "a step past: held, no direction"
        );
        assert_eq!(run(&mut ship, &spec, &mut fold, 3), [3, 3, 3]);
        assert_eq!(ship.bank, 0.0, "no direction zeroes the timer");
    }

    /// The moment at the frame's gain `gain`.
    fn at(gain: f64) -> Moment {
        Moment {
            gain: SpeedMult::new(gain),
            ..MOMENT
        }
    }

    /// The sets `frames` draws over `n` frames of `spec` in `moment`, its
    /// fold starting at `fold`.
    fn run_in(
        frames: &mut ShipFrames,
        spec: &FrameSpec,
        fold: &mut Fold,
        n: usize,
        moment: Moment,
    ) -> Vec<u16> {
        (0..n)
            .map(|_| frames.step(spec, fold, moment).set)
            .collect()
    }

    #[test]
    fn a_folding_ships_timer_gains_the_frames_gain_either_way() {
        let spec = spec(FOLDS, 4, 2);
        let mut ship = ShipFrames::default();
        let mut up = Fold {
            dir: 1,
            ..Fold::default()
        };
        assert_eq!(
            run_in(&mut ship, &spec, &mut up, 5, at(0.5)),
            [0, 0, 0, 0, 1],
            "2.5 > 2 on the 5th frame"
        );
        assert_eq!(ship.bank, 0.0, "past it, zeroed");
        let mut ship = ShipFrames::default();
        let mut down = Fold {
            set: 3,
            dir: -1,
            ..Fold::default()
        };
        assert_eq!(
            run_in(&mut ship, &spec, &mut down, 5, at(0.5)),
            [3, 3, 3, 3, 2],
            "folding down too"
        );
        let mut ship = ShipFrames::default();
        let mut up = Fold {
            dir: 1,
            ..Fold::default()
        };
        assert_eq!(
            run_in(&mut ship, &spec, &mut up, 6, at(0.0)),
            [0; 6],
            "at no gain it never steps"
        );
        assert_eq!(ship.bank, 0.0);
    }

    #[test]
    fn a_folding_ship_of_no_direction_zeroes_its_timer_at_any_gain() {
        let spec = spec(FOLDS, 4, 2);
        for gain in [0.5, 0.0] {
            let mut ship = ShipFrames {
                bank: 1.5,
                ..ShipFrames::default()
            };
            let mut fold = Fold::default();
            ship.step(&spec, &mut fold, at(gain));
            assert_eq!(ship.bank, 0.0, "{gain}");
        }
    }

    #[test]
    fn an_animated_ships_timer_gains_the_frames_gain_and_loses_a_whole_delay() {
        let every = spec(ANIMATED, 3, 0);
        let spec = spec(ANIMATED, 3, 1);
        let mut ship = ShipFrames::default();
        let mut fold = Fold::default();
        assert_eq!(
            run_in(&mut ship, &spec, &mut fold, 3, at(0.5)),
            [0, 0, 1],
            "1.5 > 1 on the 3rd frame"
        );
        assert_eq!(ship.anim_timer, 0.5, "it loses 1, unscaled");
        assert_eq!(
            run_in(&mut ship, &spec, &mut fold, 2, at(0.5)),
            [1, 2],
            "the next 2 frames later"
        );
        // An `AnimDelay` of 0 steps every frame at any gain above none.
        let mut ship = ShipFrames::default();
        assert_eq!(run_in(&mut ship, &every, &mut fold, 3, at(0.5)), [1, 2, 0]);
        let mut ship = ShipFrames::default();
        assert_eq!(
            run_in(&mut ship, &every, &mut fold, 3, at(0.0)),
            [0, 0, 0],
            "at no gain its timer stays 0, not above 0"
        );
        assert_eq!(ship.anim_timer, 0.0);
    }

    #[test]
    fn a_still_ship_gains_nothing_at_any_gain() {
        let spec = spec(ANIMATED | STILL_WHILE_DISABLED, 3, 1);
        for gain in [0.5, 1.0] {
            let mut ship = ShipFrames::default();
            let moment = Moment {
                disabled: true,
                ..at(gain)
            };
            ship.step(&spec, &mut Fold::default(), moment);
            assert_eq!(ship.anim_timer, 0.0, "{gain}");
        }
    }

    #[test]
    fn the_alt_layers_own_timer_gains_the_frames_gain_and_zeroes_past_its_delay() {
        let spec = FrameSpec {
            alt_sets: 6,
            ..spec(0x0058, 1, 1)
        };
        let mut ship = ShipFrames::default();
        assert_eq!(
            alts(&mut ship, &spec, 6, at(0.5)),
            [Some(0), Some(0), Some(1), Some(1), Some(1), Some(2)],
            "1.5 > 1 on the 3rd frame, then zeroed"
        );
        let mut ship = ShipFrames::default();
        alts(&mut ship, &spec, 3, at(0.5));
        assert_eq!(ship.anim_timer, 0.0, "zeroed, unscaled");
    }

    #[test]
    fn an_animated_ships_alt_block_gains_the_gain_again_by_the_engine() {
        let spec = FrameSpec {
            alt_sets: 4,
            ..spec(ANIMATED, 3, 100)
        };
        for (reading, gained) in [(RuleSource::Engine, 1.0), (RuleSource::Bible, 0.5)] {
            let mut ship = ShipFrames::default();
            let moment = Moment {
                gain: SpeedMult::new(0.5),
                ..double_step(reading)
            };
            ship.step(&spec, &mut Fold::default(), moment);
            assert_eq!(ship.anim_timer, gained, "{reading:?}");
        }
    }

    #[test]
    fn at_the_settled_gain_each_timer_steps_on_the_frames_it_does_at_full() {
        let settled = Moment {
            gain: SpeedMult::SETTLED,
            ..MOMENT
        };
        for spec in [spec(FOLDS, 4, 2), spec(ANIMATED, 3, 2)] {
            let (mut full, mut near) = (ShipFrames::default(), ShipFrames::default());
            let up = Fold {
                dir: 1,
                ..Fold::default()
            };
            let (mut a, mut b) = (up, up);
            assert_eq!(
                run_in(&mut near, &spec, &mut b, 11, settled),
                run_in(&mut full, &spec, &mut a, 11, MOMENT),
                "{:#x}",
                spec.flags
            );
        }
        let alt = FrameSpec {
            alt_sets: 6,
            ..spec(0x0058, 1, 5)
        };
        let (mut full, mut near) = (ShipFrames::default(), ShipFrames::default());
        assert_eq!(
            alts(&mut near, &alt, 12, settled),
            alts(&mut full, &alt, 12, MOMENT)
        );
    }

    #[test]
    fn a_folding_ship_unfolds_to_set_0() {
        let spec = spec(FOLDS, 4, 0);
        let mut ship = ShipFrames::default();
        let mut fold = Fold {
            set: 3,
            dir: -1,
            ..Fold::default()
        };
        assert_eq!(run(&mut ship, &spec, &mut fold, 4), [2, 1, 0, 0]);
        assert_eq!((fold.set, fold.dir), (0, 0));
    }

    #[test]
    fn an_unfolded_miner_refolds_only_45_mac_ticks_after_its_last_shot() {
        let spec = spec(FOLDS | UNFOLDS_TO_FIRE, 6, 5);
        let mut ship = ShipFrames::default();
        let mut fold = Fold {
            since_fire: REFOLD_TICKS - 1,
            ..Fold::default()
        };
        assert_eq!(run(&mut ship, &spec, &mut fold, 1), [0]);
        assert_eq!(fold.dir, 0, "44 Mac ticks: not yet");
        fold.since_fire = REFOLD_TICKS;
        assert_eq!(run(&mut ship, &spec, &mut fold, 1), [0]);
        assert_eq!(fold.dir, 1, "45: refolding");
        assert_eq!(run(&mut ship, &spec, &mut fold, 6), [0, 0, 0, 0, 0, 1]);
        // Folded, it stays so.
        fold.set = 5;
        fold.dir = 0;
        assert_eq!(run(&mut ship, &spec, &mut fold, 1), [5]);
        assert_eq!(fold.dir, 0, "at its last set");
        // Unfolding from its last set, it steps down first.
        fold.dir = -1;
        assert_eq!(run(&mut ship, &spec, &mut fold, 5), [5; 5]);
        assert_eq!(fold.dir, -1, "the refold waits for a set short of the last");
        assert_eq!(run(&mut ship, &spec, &mut fold, 1), [4]);
        assert_eq!(fold.dir, 1, "then turns back up in the same frame");
        // A folding ship of no 0x0080 never refolds.
        let plain = spec_of(FOLDS);
        let mut fold = Fold {
            since_fire: u32::MAX,
            ..Fold::default()
        };
        ship.step(&plain, &mut fold, MOMENT);
        assert_eq!(fold.dir, 0);
    }

    #[test]
    fn a_banking_and_folding_ship_never_folds() {
        let spec = spec(BANKS | FOLDS | UNFOLDS_TO_FIRE, 3, 0);
        let mut ship = ShipFrames {
            bank: 7.0,
            ..ShipFrames::default()
        };
        let mut fold = Fold {
            dir: 1,
            since_fire: u32::MAX,
            ..Fold::default()
        };
        assert_eq!(run(&mut ship, &spec, &mut fold, 3), [0, 0, 0]);
        assert_eq!((fold.set, fold.dir, ship.bank), (0, 1, 7.0));
    }

    #[test]
    fn an_animated_ship_cycles_its_sets_every_anim_delay() {
        let spec = spec(ANIMATED, 3, 2);
        let mut ship = ShipFrames::default();
        let mut fold = Fold::default();
        assert_eq!(
            run(&mut ship, &spec, &mut fold, 9),
            [0, 0, 1, 1, 2, 2, 0, 0, 1],
            "past 2 it loses 2: a step every second frame"
        );
        assert_eq!(ship.anim_timer, 1.0);
        // Past it by a whole delay, it loses it down to the delay itself.
        ship.anim_timer = 3.0;
        ship.step(&spec, &mut fold, MOMENT);
        assert_eq!(ship.anim_timer, 2.0, "4 less 2, not less 4");
        // A negative anim set reads 0 before its step.
        ship.anim_set = -4;
        ship.anim_timer = 2.0;
        assert_eq!(ship.step(&spec, &mut fold, MOMENT).set, 1);
    }

    #[test]
    fn an_anim_set_past_the_last_is_drawn_mod_base_sets() {
        let spec = spec(ANIMATED, 3, 10);
        let mut ship = ShipFrames {
            anim_set: 5,
            ..ShipFrames::default()
        };
        let mut fold = Fold::default();
        assert_eq!(ship.step(&spec, &mut fold, MOMENT).set, 2, "5 mod 3");
        assert_eq!(ship.anim_set, 5, "held until it steps");
    }

    #[test]
    fn an_animation_with_no_anim_delay_steps_every_frame() {
        for delay in [0, -3] {
            let spec = spec(ANIMATED, 3, delay);
            let mut ship = ShipFrames::default();
            let mut fold = Fold::default();
            assert_eq!(run(&mut ship, &spec, &mut fold, 4), [1, 2, 0, 1], "{delay}");
            assert_eq!(ship.anim_timer, 0.0);
        }
    }

    #[test]
    fn a_disabled_0x0010_ship_stops_its_animation() {
        let plain = spec(ANIMATED, 3, 0);
        let spec = spec(ANIMATED | STILL_WHILE_DISABLED, 3, 0);
        let mut ship = ShipFrames::default();
        let mut fold = Fold::default();
        for _ in 0..3 {
            assert_eq!(ship.step(&spec, &mut fold, disabled()).set, 0);
        }
        assert_eq!(ship.step(&plain, &mut fold, disabled()).set, 1, "no 0x0010");
    }

    #[test]
    fn a_key_carried_ship_draws_set_1_with_its_key_aboard() {
        let spec = spec(KEY_CARRIED, 2, 0);
        let mut ship = ShipFrames::default();
        let mut fold = Fold::default();
        assert_eq!(ship.step(&spec, &mut fold, MOMENT).set, 0);
        let aboard = Moment {
            key_aboard: true,
            ..MOMENT
        };
        assert_eq!(ship.step(&spec, &mut fold, aboard).set, 1);
    }

    #[test]
    fn a_ship_of_no_set_flag_draws_set_0() {
        let spec = spec(LIGHTS_OUT_WHILE_DISABLED, 3, 0);
        let mut ship = ShipFrames::default();
        assert_eq!(ship.step(&spec, &mut Fold::default(), MOMENT).set, 0);
    }

    /// The alt set `drawn` draws its alt layer on, if drawn on one.
    fn alt_set(drawn: Drawn) -> Option<u16> {
        match drawn.alt {
            Alt::Drawn(set) => Some(set),
            Alt::Held | Alt::None => None,
        }
    }

    /// The alt sets `ship` draws over `n` frames of `spec`.
    fn alts(ship: &mut ShipFrames, spec: &FrameSpec, n: usize, moment: Moment) -> Vec<Option<u16>> {
        alt_layers(ship, spec, n, moment)
            .into_iter()
            .map(|alt| match alt {
                Alt::Drawn(set) => Some(set),
                Alt::Held | Alt::None => None,
            })
            .collect()
    }

    /// What `ship`'s alt layer shows over `n` frames of `spec`.
    fn alt_layers(ship: &mut ShipFrames, spec: &FrameSpec, n: usize, moment: Moment) -> Vec<Alt> {
        let mut fold = Fold::default();
        (0..n)
            .map(|_| ship.step(spec, &mut fold, moment).alt)
            .collect()
    }

    #[test]
    fn the_alt_layer_steps_on_its_own_timer_once_a_frame_off_the_0x0008_path() {
        // The Thunderforge: 0x58, one base set, `AnimDelay` 5, 6 alt sets.
        let spec = FrameSpec {
            alt_sets: 6,
            ..spec(0x0058, 1, 5)
        };
        let mut ship = ShipFrames::default();
        let drawn = alts(&mut ship, &spec, 12, MOMENT);
        let expected: Vec<Option<u16>> = [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 2]
            .into_iter()
            .map(Some)
            .collect();
        assert_eq!(drawn, expected, "every AnimDelay + 1 frames");
        for reading in [RuleSource::Engine, RuleSource::Bible] {
            let mut ship = ShipFrames::default();
            let moment = double_step(reading);
            assert_eq!(alts(&mut ship, &spec, 12, moment), expected, "{reading:?}");
        }
        // Round to 0 at its alt sets.
        ship.alt_set = 5;
        ship.anim_timer = 5.0;
        assert_eq!(alts(&mut ship, &spec, 1, MOMENT), [Some(0)]);
    }

    #[test]
    fn an_animated_ships_alt_layer_steps_with_its_sets_and_doubles_its_pace_by_the_engine() {
        let spec = FrameSpec {
            alt_sets: 4,
            ..spec(ANIMATED, 3, 3)
        };
        let mut fold = Fold::default();
        let mut ship = ShipFrames::default();
        let engine: Vec<(u16, Option<u16>)> = (0..6)
            .map(|_| {
                let drawn = ship.step(&spec, &mut fold, MOMENT);
                (drawn.set, alt_set(drawn))
            })
            .collect();
        assert_eq!(
            engine,
            [
                (0, Some(0)),
                (0, Some(0)),
                (1, Some(1)),
                (2, Some(2)),
                (2, Some(2)),
                (0, Some(3))
            ],
            "the timer moves 2 a frame: past 3 it loses 3, a step 3 in 5 frames"
        );
        let mut ship = ShipFrames::default();
        let bible = double_step(RuleSource::Bible);
        let once: Vec<(u16, Option<u16>)> = (0..8)
            .map(|_| {
                let drawn = ship.step(&spec, &mut fold, bible);
                (drawn.set, alt_set(drawn))
            })
            .collect();
        assert_eq!(
            once,
            [
                (0, Some(0)),
                (0, Some(0)),
                (0, Some(0)),
                (1, Some(1)),
                (1, Some(1)),
                (1, Some(1)),
                (2, Some(2)),
                (2, Some(2))
            ],
            "once a frame: a step every third"
        );
    }

    /// A class of 0x0020 with four alt sets and `AnimDelay` 0.
    fn alt_held() -> FrameSpec {
        FrameSpec {
            alt_sets: 4,
            ..spec(ALT_HIDDEN_WHILE_DISABLED, 1, 0)
        }
    }

    /// The moment, disabled, the held alt read by `reading`.
    fn disabled_reading(reading: RuleSource) -> Moment {
        Moment {
            rules: DisplayRules {
                alt_held: reading,
                ..DisplayRules::ENGINE
            },
            ..disabled()
        }
    }

    #[test]
    fn a_disabled_0x0020_ship_holds_its_alt_set_and_timer() {
        let held = alt_held();
        let mut ship = ShipFrames::default();
        assert_eq!(alts(&mut ship, &held, 2, MOMENT), [Some(1), Some(2)]);
        let before = ship;
        assert_eq!(
            alt_layers(&mut ship, &held, 3, disabled()),
            [Alt::Held; 3],
            "held as last drawn"
        );
        assert_eq!(ship, before, "no timer or set step");
    }

    #[test]
    fn by_the_other_reading_a_disabled_0x0020_ships_alt_layer_is_hidden() {
        let held = alt_held();
        let mut ship = ShipFrames::default();
        let bible = disabled_reading(RuleSource::Bible);
        assert_eq!(alt_layers(&mut ship, &held, 2, bible), [Alt::None; 2]);
        assert_eq!(ship.anim_timer, 0.0, "a hidden layer moves no timer");
        assert_eq!(
            alt_layers(&mut ship, &held, 1, disabled_reading(RuleSource::Engine)),
            [Alt::Held]
        );
    }

    #[test]
    fn a_repaired_ship_resumes_its_alt_from_where_it_was_held() {
        let held = FrameSpec {
            anim_delay: 2,
            ..alt_held()
        };
        let mut ship = ShipFrames::default();
        assert_eq!(
            alts(&mut ship, &held, 4, MOMENT),
            [Some(0), Some(0), Some(1), Some(1)]
        );
        assert_eq!(ship.anim_timer, 1.0);
        alt_layers(&mut ship, &held, 5, disabled());
        assert_eq!(
            alts(&mut ship, &held, 3, MOMENT),
            [Some(1), Some(2), Some(2)],
            "a frame from its step, as before it was disabled"
        );
    }

    #[test]
    fn a_disabled_0x0010_ships_alt_layer_is_drawn_still() {
        let still = FrameSpec {
            alt_sets: 4,
            ..spec(STILL_WHILE_DISABLED, 1, 0)
        };
        let mut ship = ShipFrames::default();
        assert_eq!(alts(&mut ship, &still, 2, disabled()), [Some(0), Some(0)]);
        assert_eq!(FrameSpec::NONE.alt_sets, 0);
        assert_eq!(
            alt_layers(&mut ship, &FrameSpec::NONE, 1, MOMENT),
            [Alt::None],
            "no alt sprite"
        );
        assert_eq!(
            alt_layers(&mut ship, &FrameSpec::NONE, 1, disabled()),
            [Alt::None],
            "no alt sprite, disabled"
        );
    }
}
