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
//! @0x22ac3-0x22b1e), is taken as on, its default.
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
//!   with a direction its timer (+0xc8e4, the bank's own field) grows by a
//!   frame, and past `AnimDelay` it zeroes and the set steps, stopping
//!   at `BaseSets` − 1 going up or 0 going down, where the direction goes
//!   back to 0; with none the timer zeroes. Then a ship of 0x0080
//!   ([`UNFOLDS_TO_FIRE`]) with no direction up, short of its last set,
//!   refolds once [`REFOLD_TICKS`] Mac ticks have passed since it last
//!   fired (+0xa8, @0x2b986-0x2b9e3).
//! - **0x0004** ([`KEY_CARRIED`], @0x2ba0b-0x2ba42): set 1 while its
//!   class's `KeyCarried` ship is aboard, else 0.
//! - **0x0008, animated** ([`ANIMATED`], @0x2ba47-0x2bb5c): the animation
//!   timer (+0xc8e0) grows by a frame, unless the ship is disabled and of
//!   0x0010 ([`STILL_WHILE_DISABLED`]); past `AnimDelay` it loses
//!   `AnimDelay` while above it (or zeroes, with an `AnimDelay` of 0 or
//!   less) and the anim set (+0xc8f6) steps, round to 0 at `BaseSets`.
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
//! **The alt layer** (@0x2c97d-0x2cb11), drawn for a class with an alt
//! sprite unless the ship is disabled and of 0x0020
//! ([`ALT_HIDDEN_WHILE_DISABLED`], @0x2b6b7-0x2b705): the animation timer
//! grows by a frame again, unless still as above (@0x2c9aa-0x2c9d6).
//! Unless the set came by the 0x0008 path, past `AnimDelay` the timer
//! zeroes and the alt set (+0xc8f4) steps; by that path, it steps when the
//! anim set did. Either way round to 0 at `AltSetCount`. It is drawn at
//! alt set × `FramesPer` + the rotation ([`FrameSpec::index`]). So a
//! 0x0008 ship of more than one set with an alt sprite moves its timer
//! twice a frame, as the rulebook's
//! [`RuleKey::ShipAnimationDoubleStep`](crate::RuleKey::ShipAnimationDoubleStep)
//! says by the engine; by the other reading, once. No stock ship does:
//! the Thunderforge (380), the one with an alt sprite, has one base set.
//!
//! The frame's `_gSpeedMult` is 1.0, the engine's fixed step. No sound
//! plays in any of it.
//!
//! **Ordering.** Each ship's display steps after its flight and its fire
//! in the same tick, before the frame is drawn, as `_HandleShipDisplay`
//! runs after `_HandlePlayer` and each `_HandleShip`; firing reads the
//! fold set the last display left, as the original's AI fires before the
//! display. No boundary the player sees falls between them.

use std::num::NonZeroU16;

use nova_sim_base::blink::Blink;

use super::armament::Fold;
use super::exits::{DEFAULT_FRAMES, compression, drawn_frame};
use super::ship_lights::ShipLights;
use crate::catalog::{ShipDisplay, ShipExits};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;
use crate::trig::{accel, intercept_angle};

/// `Flags` 0x0001: the ship banks as it turns, on sets 1 and 2.
pub const BANKS: u16 = 0x0001;
/// `Flags` 0x0002: the ship folds through its sets.
pub const FOLDS: u16 = 0x0002;
/// `Flags` 0x0004: set 1 while its class's `KeyCarried` ship is aboard.
pub const KEY_CARRIED: u16 = 0x0004;
/// `Flags` 0x0008: the ship cycles through its sets.
pub const ANIMATED: u16 = 0x0008;
/// `Flags` 0x0010: its animation stops while it is disabled.
pub const STILL_WHILE_DISABLED: u16 = 0x0010;
/// `Flags` 0x0020: its alt layer is hidden while it is disabled.
pub const ALT_HIDDEN_WHILE_DISABLED: u16 = 0x0020;
/// `Flags` 0x0040: its running lights go out while it is disabled
/// (@0x2c56c-0x2c593).
pub const LIGHTS_OUT_WHILE_DISABLED: u16 = 0x0040;
/// `Flags` 0x0080: a folding ship unfolds to set 0 to fire, and refolds
/// after.
pub const UNFOLDS_TO_FIRE: u16 = 0x0080;
/// `Flags` 0x0100: its rotations are picked by a frame-angle table.
pub const ANGLE_TABLE: u16 = 0x0100;

/// The Mac ticks after its last shot a ship of [`UNFOLDS_TO_FIRE`] waits
/// to refold (`addl $0x2d` @0x2b9cb).
pub const REFOLD_TICKS: u32 = 45;

/// How far a timer moves a frame: `_gSpeedMult` at the engine's fixed
/// step.
pub const SPEED_MULT: f32 = 1.0;

/// The magnitude of the push each frame-angle table entry turns
/// (`_Accel(a, 100)`, 100.0 @0x1eff2).
const TABLE_REACH: f32 = 100.0;

/// The degrees in a turn (360.0 @0xdd4d4).
const TURN: f32 = 360.0;

/// The angle up to which an entry takes the up compression (90.0
/// @0xdd4d8).
const UP_TO: f32 = 90.0;

/// The angle past which an entry takes the up compression again (270.0
/// @0xdd4dc).
const UP_FROM: f32 = 270.0;

/// A class's frame-angle table, `shän` `Flags` 0x0100: the screen angle
/// each rotation frame shows, as `_InitSprites` builds it
/// (@0x1ef4b-0x1f0b9). Entry i is `_CalcInterceptAngle` from the origin
/// to `_Accel(trunc(i × (360 / FramesPer)), 100)`, squashed by the up
/// compression for an angle of 90 or less or above 270, by the down one
/// otherwise. No stock `shän` sets 0x0100.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameTable {
    frames: NonZeroU16,
    up: Vec2,
    down: Vec2,
}

impl FrameTable {
    /// The table of `frames` a turn, its up and down compressions `up`
    /// and `down`.
    #[must_use]
    pub fn new(frames: NonZeroU16, up: Vec2, down: Vec2) -> Self {
        Self { frames, up, down }
    }

    /// The frames in a turn.
    #[must_use]
    pub fn frames(&self) -> NonZeroU16 {
        self.frames
    }

    /// Entry `i`: the screen angle rotation frame `i` shows, in whole
    /// degrees.
    #[must_use]
    pub fn angle(&self, i: u16) -> i16 {
        let step = TURN / f32::from(self.frames.get());
        let a = f32::from(i) * step;
        // A whole degree, as `cvttss2si` gives it (@0x1effa).
        #[allow(clippy::cast_possible_truncation)]
        let push = accel(a.trunc() as i16, TABLE_REACH);
        let factor = if a <= UP_TO || a > UP_FROM {
            self.up
        } else {
            self.down
        };
        intercept_angle(Vec2::ZERO, Vec2::new(push.x * factor.x, push.y * factor.y))
    }

    /// The rotation a ship facing `heading` is drawn on: from the last
    /// frame down, the first whose entry `heading` is above, or the last
    /// frame when it is above none (@0x2bbb5-0x2bbe5).
    #[must_use]
    pub fn frame(&self, heading: f32) -> u16 {
        let last = self.frames.get() - 1;
        (0..=last)
            .rev()
            .find(|&i| heading > f32::from(self.angle(i)))
            .unwrap_or(last)
    }
}

/// A ship class's display: what picks the frames it is drawn on (see the
/// module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameSpec {
    /// Its `shän` `Flags`.
    pub flags: u16,
    /// Its `BaseSets`: `BaseSetCount`, at least 1.
    pub base_sets: u16,
    /// Its `AnimDelay`, in frames.
    pub anim_delay: i16,
    /// Its alt sets: its `AltSetCount` when its alt sprite loads, else
    /// none.
    pub alt_sets: u16,
    /// Its `FramesPer`, 36 for 0.
    pub frames: NonZeroU16,
    /// Its frame-angle table, with `Flags` 0x0100.
    pub table: Option<FrameTable>,
    /// Its `BlinkMode` and `BlinkValA`-`D`, raw.
    pub blink: Blink,
    /// Whether it has a lights sprite (`LightImageID` above 0), without
    /// which no lights block runs (see
    /// [`ship_lights`](super::ship_lights)).
    pub lights: bool,
}

impl Default for FrameSpec {
    fn default() -> Self {
        Self::NONE
    }
}

impl FrameSpec {
    /// A class with no `shän` display: one set of 36 frames, no flags.
    pub const NONE: Self = Self {
        flags: 0,
        base_sets: 1,
        anim_delay: 0,
        alt_sets: 0,
        frames: NonZeroU16::new(DEFAULT_FRAMES).expect("36"),
        table: None,
        blink: Blink::STEADY,
        lights: false,
    };

    /// The display `display` and `exits` give (see the module docs); none
    /// without a `shän`. A `FramesPer` below 0 reads as 36.
    #[must_use]
    pub fn new(display: Option<&ShipDisplay>, exits: Option<&ShipExits>) -> Self {
        let Some(display) = display else {
            return Self::NONE;
        };
        let frames = exits
            .and_then(|exits| u16::try_from(exits.frames).ok())
            .and_then(NonZeroU16::new)
            .unwrap_or(Self::NONE.frames);
        let table = (display.flags & ANGLE_TABLE != 0).then(|| {
            let (up, down) = exits.map_or((Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0)), compression);
            FrameTable::new(frames, up, down)
        });
        let alt_sets = if display.alt_image && display.alt_sets > 0 {
            u16::try_from(display.alt_sets).unwrap_or(0)
        } else {
            0
        };
        Self {
            flags: display.flags,
            base_sets: u16::try_from(display.base_sets.max(1)).unwrap_or(1),
            anim_delay: display.anim_delay,
            alt_sets,
            frames,
            table,
            blink: display.blink,
            lights: display.lights_image,
        }
    }

    /// Whether its `Flags` have `flag`.
    #[must_use]
    pub fn has(&self, flag: u16) -> bool {
        self.flags & flag != 0
    }

    /// Whether its hull's sprite holds more frames than a turn's: more
    /// than one base set.
    #[must_use]
    pub fn multi_set(&self) -> bool {
        self.base_sets > 1
    }

    /// Whether its set comes by the 0x0008 path: of more than one set,
    /// animated and of none of the flags before it in the chain.
    #[must_use]
    pub fn anim_path(&self) -> bool {
        self.multi_set()
            && self.has(ANIMATED)
            && !self.has(BANKS)
            && !self.has(FOLDS)
            && !self.has(KEY_CARRIED)
    }

    /// Its `BaseSets` when it unfolds to fire, `Flags` 0x0002 and 0x0080:
    /// what the fold-before-fire gates read (see
    /// [`Armament::fire_pick`](super::armament::Armament::fire_pick)).
    #[must_use]
    pub fn unfolds_to_fire(&self) -> Option<u16> {
        (self.has(FOLDS) && self.has(UNFOLDS_TO_FIRE)).then_some(self.base_sets)
    }

    /// The last of its base sets: `BaseSets` − 1, which a spawn folds to.
    #[must_use]
    pub fn last_set(&self) -> u16 {
        self.base_sets - 1
    }

    /// The rotation a ship facing `heading` is drawn on: its table's, or
    /// `trunc(heading × FramesPer / 360)` ([`drawn_frame`]).
    #[must_use]
    pub fn rotation(&self, heading: f32) -> u16 {
        drawn_frame(heading, self.frames, self.table.as_ref())
    }

    /// The frame of set `set` a ship facing `heading` is drawn on: set ×
    /// `FramesPer` + its rotation (@0x2bc06-0x2bc2e, @0x2ca79-0x2ca91).
    #[must_use]
    pub fn index(&self, set: u16, heading: f32) -> u16 {
        set.wrapping_mul(self.frames.get())
            .wrapping_add(self.rotation(heading))
    }

    /// The frame of a shield layer of `frames` frames, the hull drawn on
    /// set `set` facing `heading`: the hull's frame when the layer has
    /// more frames than a turn, else its rotation when the layer has more
    /// than one, else 0 (@0x2bcd0-0x2bd1f), each mod its frames.
    #[must_use]
    pub fn shield_frame(&self, set: u16, heading: f32, frames: NonZeroU16) -> u16 {
        let frame = if frames.get() > self.frames.get() {
            self.index(set, heading)
        } else if frames.get() > 1 {
            self.rotation(heading)
        } else {
            0
        };
        frame % frames.get()
    }
}

/// The rulebook readings a ship's display runs by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayRules {
    /// How often its alt block moves the animation timer, as
    /// [`RuleKey::ShipAnimationDoubleStep`](crate::RuleKey::ShipAnimationDoubleStep)
    /// says: again by the engine, not again after the set block moved it
    /// by the other reading.
    pub double_step: RuleSource,
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
}

/// The sets a ship is drawn on this frame, and its lights' level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    /// The hull's set, which its glow, lights, weapon and shield layers
    /// follow.
    pub set: u16,
    /// The alt layer's set, while the alt layer is drawn.
    pub alt_set: Option<u16>,
    /// The level its lights layer is drawn at, if drawn: its lights
    /// block's ([`ShipFrames::light`]), which [`ShipFrames::step`] leaves
    /// to its caller.
    pub lights: Option<u8>,
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
            self.fold(spec, fold);
            fold.set
        } else if spec.has(KEY_CARRIED) {
            u16::from(moment.key_aboard)
        } else if spec.has(ANIMATED) {
            animated = true;
            stepped = self.animate(spec, still);
            u16::try_from(self.anim_set).unwrap_or(0)
        } else {
            0
        };
        let alt_drawn =
            spec.alt_sets > 0 && !(spec.has(ALT_HIDDEN_WHILE_DISABLED) && moment.disabled);
        let alt_set = alt_drawn.then(|| {
            let again = !animated || moment.rules.double_step == RuleSource::Engine;
            if !still && again {
                self.anim_timer += SPEED_MULT;
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
        });
        Drawn {
            set,
            alt_set,
            lights: None,
        }
    }

    /// The 0x0002 block (@0x2b894-0x2b9f1): `fold`'s set steps toward its
    /// direction on the timer, then a ship of 0x0080 refolds once due.
    fn fold(&mut self, spec: &FrameSpec, fold: &mut Fold) {
        let delay = f32::from(spec.anim_delay);
        let last = i32::from(spec.base_sets) - 1;
        if fold.dir == 0 {
            self.bank = 0.0;
        } else {
            self.bank += SPEED_MULT;
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

    /// The 0x0008 block's timer and anim set (@0x2ba4b-0x2bb55), still
    /// when `still`: whether the anim set stepped.
    fn animate(&mut self, spec: &FrameSpec, still: bool) -> bool {
        if !still {
            self.anim_timer += SPEED_MULT;
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

    fn frames(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

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

    #[test]
    fn a_classes_display_comes_from_its_shän() {
        let display = ShipDisplay {
            flags: 0x00C2,
            base_sets: 6,
            anim_delay: 5,
            alt_sets: 3,
            alt_image: false,
            blink: Blink {
                mode: 1,
                a: 4,
                b: 1,
                c: 2,
                d: 20,
            },
            lights_image: true,
        };
        let exits = ShipExits {
            frames: 0,
            ..ShipExits::default()
        };
        let spec = FrameSpec::new(Some(&display), Some(&exits));
        assert_eq!(spec.flags, 0x00C2);
        assert_eq!(spec.base_sets, 6);
        assert_eq!(spec.anim_delay, 5);
        assert_eq!(spec.alt_sets, 0, "no alt image: no alt sprite");
        assert_eq!(spec.frames.get(), 36, "0 reads as 36");
        assert_eq!(spec.table, None);
        assert_eq!((spec.blink, spec.lights), (display.blink, true));
        assert_eq!(spec.unfolds_to_fire(), Some(6));
        assert_eq!(spec.last_set(), 5);
        let alt = ShipDisplay {
            alt_image: true,
            base_sets: 0,
            ..display
        };
        let spec = FrameSpec::new(
            Some(&alt),
            Some(&ShipExits {
                frames: 64,
                ..exits
            }),
        );
        assert_eq!(
            (spec.alt_sets, spec.base_sets, spec.frames.get()),
            (3, 1, 64)
        );
        let none = ShipDisplay { alt_sets: 0, ..alt };
        assert_eq!(FrameSpec::new(Some(&none), None).alt_sets, 0, "no alt sets");
        assert_eq!(FrameSpec::new(None, Some(&exits)), FrameSpec::NONE);
        assert_eq!(
            (FrameSpec::NONE.blink, FrameSpec::NONE.lights),
            (Blink::STEADY, false),
            "no shän: no lights sprite"
        );
        assert_eq!(
            spec_of(0x0002).unfolds_to_fire(),
            None,
            "folds, never to fire"
        );
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
    fn the_anim_path_is_an_animated_multi_set_ship_of_no_earlier_flag() {
        assert!(spec(ANIMATED, 2, 0).anim_path());
        assert!(!spec(ANIMATED, 1, 0).anim_path(), "one set");
        for earlier in [BANKS, FOLDS, KEY_CARRIED] {
            assert!(!spec(ANIMATED | earlier, 3, 0).anim_path(), "{earlier:#x}");
        }
        assert!(!spec(0, 3, 0).anim_path());
    }

    #[test]
    fn a_ship_of_no_set_flag_draws_set_0() {
        let spec = spec(LIGHTS_OUT_WHILE_DISABLED, 3, 0);
        let mut ship = ShipFrames::default();
        assert_eq!(ship.step(&spec, &mut Fold::default(), MOMENT).set, 0);
    }

    /// The alt sets `ship` draws over `n` frames of `spec`.
    fn alts(ship: &mut ShipFrames, spec: &FrameSpec, n: usize, moment: Moment) -> Vec<Option<u16>> {
        let mut fold = Fold::default();
        (0..n)
            .map(|_| ship.step(spec, &mut fold, moment).alt_set)
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
                (drawn.set, drawn.alt_set)
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
                (drawn.set, drawn.alt_set)
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

    #[test]
    fn the_alt_layer_hides_while_a_0x0020_ship_is_disabled_and_stops_for_0x0010() {
        let hidden = FrameSpec {
            alt_sets: 4,
            ..spec(ALT_HIDDEN_WHILE_DISABLED, 1, 0)
        };
        let mut ship = ShipFrames::default();
        assert_eq!(alts(&mut ship, &hidden, 2, disabled()), [None, None]);
        assert_eq!(ship.anim_timer, 0.0, "a hidden layer moves no timer");
        assert_eq!(alts(&mut ship, &hidden, 2, MOMENT), [Some(1), Some(2)]);
        let still = FrameSpec {
            alt_sets: 4,
            ..spec(STILL_WHILE_DISABLED, 1, 0)
        };
        let mut ship = ShipFrames::default();
        assert_eq!(alts(&mut ship, &still, 2, disabled()), [Some(0), Some(0)]);
        assert_eq!(FrameSpec::NONE.alt_sets, 0);
        assert_eq!(
            alts(&mut ship, &FrameSpec::NONE, 1, MOMENT),
            [None],
            "no alt sprite"
        );
    }

    #[test]
    fn the_frame_is_the_set_times_frames_per_plus_the_rotation() {
        let spec = spec(0, 3, 0);
        assert_eq!(spec.index(0, 95.0), 9);
        assert_eq!(spec.index(2, 95.0), 72 + 9);
        assert_eq!(spec.rotation(359.9), 35);
    }

    #[test]
    fn the_shield_layer_takes_the_full_frame_the_rotation_or_0_by_its_frames() {
        let spec = spec(BANKS, 3, 0);
        assert_eq!(
            spec.shield_frame(2, 95.0, frames(108)),
            81,
            "more than a turn"
        );
        assert_eq!(spec.shield_frame(2, 95.0, frames(37)), 81 % 37);
        assert_eq!(
            spec.shield_frame(2, 95.0, frames(36)),
            9,
            "a turn: its rotation"
        );
        assert_eq!(
            spec.shield_frame(2, 95.0, frames(4)),
            1,
            "the rotation mod 4"
        );
        assert_eq!(spec.shield_frame(2, 95.0, frames(1)), 0);
    }

    #[test]
    fn a_frame_angle_table_turns_each_frame_by_its_compression() {
        let even = FrameTable::new(frames(4), Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0));
        assert_eq!(
            (0..4).map(|i| even.angle(i)).collect::<Vec<_>>(),
            [0, 90, 180, 270],
            "uncompressed, each frame's own angle"
        );
        // The diagonals a degree short, as the original's arctangent table
        // truncates.
        let even = FrameTable::new(frames(8), Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0));
        let angles = |table: &FrameTable| (0..8).map(|i| table.angle(i)).collect::<Vec<_>>();
        assert_eq!(angles(&even), [0, 44, 90, 134, 180, 224, 270, 314]);
        // Squashed vertically below the horizon (90 < a <= 270), a frame
        // 45 degrees down shows nearer the horizontal.
        let squashed = FrameTable::new(frames(8), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.5));
        assert_eq!(angles(&squashed), [0, 44, 90, 116, 180, 244, 270, 314]);
        // Squashed above it, 90 and 270 themselves among the up frames.
        let wide = FrameTable::new(frames(8), Vec2::new(1.0, 0.5), Vec2::new(1.0, 1.0));
        assert_eq!(angles(&wide), [0, 64, 90, 134, 180, 224, 270, 296]);
        // Squashed across above it, a diagonal shows nearer the vertical.
        let narrow = FrameTable::new(frames(8), Vec2::new(0.5, 1.0), Vec2::new(1.0, 1.0));
        assert_eq!(angles(&narrow)[1], 26);
        assert_eq!(angles(&narrow)[7], 334);
    }

    #[test]
    fn the_table_picks_the_highest_frame_the_heading_is_above() {
        let even = FrameTable::new(frames(4), Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0));
        assert_eq!(even.frame(45.0), 0);
        assert_eq!(even.frame(90.0), 0, "on an entry: below it");
        assert_eq!(even.frame(90.5), 1);
        assert_eq!(even.frame(271.0), 3);
        assert_eq!(even.frame(0.0), 3, "above none: the last");
        let tabled = FrameSpec {
            table: Some(even),
            frames: frames(4),
            ..spec(ANGLE_TABLE, 2, 0)
        };
        assert_eq!(tabled.rotation(90.5), 1);
        assert_eq!(tabled.index(1, 90.5), 5);
    }

    #[test]
    fn a_table_class_builds_its_table_from_its_compression() {
        let display = ShipDisplay {
            flags: ANGLE_TABLE,
            base_sets: 1,
            ..ShipDisplay::default()
        };
        let exits = ShipExits {
            frames: 8,
            up_compress: (100, 100),
            down_compress: (100, 50),
            ..ShipExits::default()
        };
        let spec = FrameSpec::new(Some(&display), Some(&exits));
        let table = spec.table.expect("a table");
        assert_eq!(
            table,
            FrameTable::new(frames(8), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.5))
        );
        assert_eq!(table.frames(), frames(8));
        let plain = ShipDisplay {
            flags: 0,
            ..display
        };
        assert_eq!(FrameSpec::new(Some(&plain), Some(&exits)).table, None);
        let bare = FrameSpec::new(Some(&display), None).table.expect("a table");
        assert_eq!(
            bare,
            FrameTable::new(frames(36), Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0))
        );
    }
}
