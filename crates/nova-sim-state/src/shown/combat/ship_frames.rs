//! A ship class's display, the frames of its sheets it is drawn on:
//! [`FrameSpec`] with its frame-angle [`FrameTable`], the `shän` `Flags`
//! that pick its set, and what a frame draws ([`Drawn`], [`Alt`]), from
//! `nova_sim_combat::combat::ship_frames`, which re-exports them and keeps
//! the display state that steps them each frame (`ShipFrames`).

use std::num::NonZeroU16;

use nova_sim_base::blink::Blink;
use nova_sim_base::geometry::Vec2;
use nova_sim_base::trig::{accel, intercept_angle};
use nova_sim_catalog::catalog::{ShipDisplay, ShipExits};

use super::exits::{DEFAULT_FRAMES, compression, drawn_frame};
use super::hull_turn::{HullTurn, hull_angle};

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

/// `Flags` 0x0020: its alt layer stops while it is disabled, held as it
/// was last drawn (see `nova_sim_combat::combat::ship_frames`'s module
/// docs).
pub const ALT_HIDDEN_WHILE_DISABLED: u16 = 0x0020;

/// `Flags` 0x0040: its running lights go out while it is disabled
/// (@0x2c56c-0x2c593).
pub const LIGHTS_OUT_WHILE_DISABLED: u16 = 0x0040;

/// `Flags` 0x0080: a folding ship unfolds to set 0 to fire, and refolds
/// after.
pub const UNFOLDS_TO_FIRE: u16 = 0x0080;

/// `Flags` 0x0100: its rotations are picked by a frame-angle table.
pub const ANGLE_TABLE: u16 = 0x0100;

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

/// A ship class's display: what picks the frames it is drawn on (see
/// `nova_sim_combat::combat::ship_frames`'s module docs).
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
    /// `nova_sim_combat::combat::ship_lights`).
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

    /// The display `display` and `exits` give (see
    /// `nova_sim_combat::combat::ship_frames`'s module docs); none
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
    /// `nova_sim_combat::combat::armament::Armament::fire_pick`).
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

    /// The angle its hull stands at facing `heading`, as `turn` says: by
    /// frame its [`rotation`](Self::rotation)'s angle, continuously the
    /// heading itself ([`hull_angle`]).
    #[must_use]
    pub fn hull_angle(&self, heading: f32, turn: HullTurn) -> f32 {
        hull_angle(heading, self.frames, self.table.as_ref(), turn)
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

/// What a ship's alt layer shows this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alt {
    /// Drawn on this alt set.
    Drawn(u16),
    /// Left as it was last drawn: its frame, tint and screen place (see
    /// `nova_sim_combat::combat::ship_frames`'s module docs).
    Held,
    /// Not drawn: no alt sprite, or hidden by the other reading of
    /// [`RuleKey::DisabledAltHeld`](crate::RuleKey::DisabledAltHeld).
    #[default]
    None,
}

/// The sets a ship is drawn on this frame, and its lights' level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Drawn {
    /// The hull's set, which its glow, lights, weapon and shield layers
    /// follow.
    pub set: u16,
    /// What its alt layer shows.
    pub alt: Alt,
    /// The level its lights layer is drawn at, if drawn: its lights
    /// block's (`nova_sim_combat::combat::ship_frames::ShipFrames::light`), which `nova_sim_combat::combat::ship_frames::ShipFrames::step` leaves
    /// to its caller.
    pub lights: Option<u8>,
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
    fn a_specs_hull_angle_follows_its_table_by_frame_and_the_heading_continuously() {
        let table = FrameTable::new(frames(8), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.5));
        let tabled = FrameSpec {
            frames: frames(8),
            table: Some(table),
            ..FrameSpec::NONE
        };
        let plain = FrameSpec {
            frames: frames(64),
            ..FrameSpec::NONE
        };
        for heading in [0.0, 7.5, 125.0, 359.9] {
            for turn in [HullTurn::ByFrame, HullTurn::Continuous] {
                assert_eq!(
                    tabled.hull_angle(heading, turn),
                    hull_angle(heading, frames(8), Some(&table), turn),
                    "{heading} {turn:?}"
                );
                assert_eq!(
                    plain.hull_angle(heading, turn),
                    hull_angle(heading, frames(64), None, turn),
                    "{heading} {turn:?}"
                );
            }
        }
        assert_ne!(
            tabled.hull_angle(125.0, HullTurn::ByFrame),
            hull_angle(125.0, frames(8), None, HullTurn::ByFrame),
            "the table picks the frame"
        );
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
