//! Weapon exit points: where on its ship a shot or beam leaves from, as
//! `_ModifyShotStartPosition` (@0x7348-0x74d2), `_ModifyShotStartPosition2`
//! (@0x706e-0x7253) and `_SelectClosestShotStartPosition` (@0x7254-0x7347)
//! work them out.
//!
//! - A ship class's `shän` gives four sets of four exit points, one set for
//!   each `ExitType` ([`ExitKind`]), in pixels from its centre with the
//!   ship facing up: +x to its right and +y ahead
//!   ([`ExitPoints::offset`]). A weapon of `ExitType` 0-3 fires from that
//!   set; any other, -1 included, from the centre.
//! - An exit point turns with the frame the ship was drawn on, not its
//!   heading: the frame's angle, `trunc(frame × 360 / FramesPer)` in whole
//!   degrees (@0x739e-0x73f5; [`drawn_frame`], [`frame_angle`]). The turn
//!   reads the original's sine table (`_BuildTrigTable` @0x25828), so a
//!   point turned to 180 lands where the original puts it.
//! - The turned point is then squashed by the `shän`'s up or down
//!   compression, each its percent over 100, any at or below 0 counting as
//!   100 (`_LoadExtendedShipSprites` @0x23d45-0x23e07). By the engine,
//!   which compression is chosen by whether the turned point lies up the
//!   screen (y < 0) or not (@0x717d-0x71f2); by the Bible, by the heading
//!   ([`Compression`], the rulebook's
//!   [`RuleKey::ExitCompression`](crate::RuleKey::ExitCompression)).
//!   Last, its z moves it up the screen, unscaled (@0x71f2-0x7228).
//! - Each ship keeps a cursor for each `ExitType` ([`ExitCursors`],
//!   +0xc8fe-+0xc905): each shot of the type leaves from the cursor's
//!   point and moves it on to the next, 0 to 3 and round again
//!   (@0x747a-0x74a3). A weapon of `Flags3` 0x0010
//!   ([`NEAREST_EXIT`](super::weapon::NEAREST_EXIT)) with a target leaves
//!   from the point nearest the target instead, the first of equals, and
//!   the cursor moves on from there (@0x73d9-0x742c).
//! - A beam takes its exit as `_SpawnBeam` does (@0x450e4-0x4523d,
//!   [`ExitCursors::beam_exit`]): the nearest by the ship's heading, not
//!   its drawn frame, with the cursor moved on from where it was, not from
//!   the nearest. It then starts at that exit on every tick's drawn frame
//!   ([`Beam`](super::beam::Beam)).
//!
//! The cursors start at 0, as the ship table is fresh zeroed memory
//! (`_InitArrays` @0x1b7a8) that nothing else writes, and stay 0-3, so
//! the original's `Rand(4)` for a cursor out of range (@0x7437-0x7451,
//! @0x450ff-0x45116, @0x451c2-0x451d9) can't come up. They live on the
//! ship's [`Armament`](super::armament::Armament), which a new ship builds
//! afresh: the original keeps a slot's cursors from ship to ship and
//! through the player's ship change (weapons-fidelity phase 18). A beam of
//! `ExitType` outside 0-3, which the original lets step its fold frame
//! (@0x450fa), fires from the centre and steps nothing here (phase 18).
//! `_SpawnBeam`'s given exit (@0x450c7-0x450df) is never given by any of
//! its callers. A negative `FramesPer`, which the original divides by,
//! gives a hull no exits; 0 reads as 36 (@0x2274c). None of this plays a
//! sound.

use std::num::NonZeroU16;

use crate::catalog::ShipExits;
use crate::flight::normalized;
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

/// The frames in a turn of a `shän` whose `FramesPer` is 0
/// (`_LoadExtendedShipSprites` @0x2274c).
pub const DEFAULT_FRAMES: u16 = 36;

/// The exits of each kind.
const EXITS: usize = 4;

/// Which of its ship's four sets of exit points a weapon fires from, by
/// its `ExitType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitKind {
    /// 0: `GunPosX/Y`.
    Gun,
    /// 1: `TurretPosX/Y`.
    Turret,
    /// 2: `GuidedPosX/Y`.
    Guided,
    /// 3: `BeamPosX/Y`.
    Beam,
}

impl ExitKind {
    /// The kind of `ExitType` `raw`; none, the centre, for any but 0-3
    /// (@0x7390-0x7398).
    #[must_use]
    pub fn from_raw(raw: i16) -> Option<Self> {
        match raw {
            0 => Some(Self::Gun),
            1 => Some(Self::Turret),
            2 => Some(Self::Guided),
            3 => Some(Self::Beam),
            _ => None,
        }
    }

    /// Its place among the `shän`'s sets, 0 to 3.
    fn index(self) -> usize {
        match self {
            Self::Gun => 0,
            Self::Turret => 1,
            Self::Guided => 2,
            Self::Beam => 3,
        }
    }
}

/// Which compression an exit point takes, as the rulebook's
/// [`RuleKey::ExitCompression`](crate::RuleKey::ExitCompression) says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Compression {
    /// The original's: the up compression when the turned point lies up
    /// the screen (y below 0), else the down one (@0x717d-0x71f2).
    #[default]
    ByOffset,
    /// The Bible's: the up compression when the ship faces 0-90 or
    /// 270-359, else the down one.
    ByHeading,
}

impl From<RuleSource> for Compression {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::ByOffset,
            RuleSource::Bible => Self::ByHeading,
        }
    }
}

/// A ship class's exit points, in the simulation's units (see the module
/// docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExitPoints {
    /// Each kind's points, in whole pixels, +x right and +y ahead.
    points: [[(i16, i16); EXITS]; EXITS],
    /// Each point's z, in whole pixels up the screen.
    z: [[i16; EXITS]; EXITS],
    /// The up compression's x and y factors.
    up: Vec2,
    /// The down compression's.
    down: Vec2,
    /// The frames in a turn.
    frames: NonZeroU16,
}

impl ExitPoints {
    /// The exit points `raw` gives: none for a negative `FramesPer`, and
    /// 36 frames for 0.
    #[must_use]
    pub fn new(raw: &ShipExits) -> Option<Self> {
        let frames = match raw.frames {
            0 => DEFAULT_FRAMES,
            frames => u16::try_from(frames).ok()?,
        };
        let factor = |pct: i16| {
            if pct > 0 {
                (f64::from(pct) * PERCENT) as f32
            } else {
                1.0
            }
        };
        let compress = |(x, y): (i16, i16)| Vec2::new(factor(x), factor(y));
        Some(Self {
            points: raw.points,
            z: raw.z,
            up: compress(raw.up_compress),
            down: compress(raw.down_compress),
            frames: NonZeroU16::new(frames)?,
        })
    }

    /// The frames in a turn: its `FramesPer`, 36 for 0.
    #[must_use]
    pub fn frames(&self) -> NonZeroU16 {
        self.frames
    }

    /// Where exit `index` of `kind` stands off the ship's centre with the
    /// ship drawn at `angle`, a whole degree, its compression chosen as
    /// `compression` says: `_ModifyShotStartPosition2`. An `index` past 3
    /// stands at the centre (@0x70b6-0x70c5).
    #[must_use]
    pub fn offset(
        &self,
        angle: f32,
        kind: ExitKind,
        index: usize,
        compression: Compression,
    ) -> Vec2 {
        let (Some(&(x, y)), Some(&z)) = (
            self.points[kind.index()].get(index),
            self.z[kind.index()].get(index),
        ) else {
            return Vec2::ZERO;
        };
        let point = Vec2::new(f32::from(x), f32::from(y));
        // `_Accel(angle, Y)` then `_Accel((angle + 90) mod 360, X)`, each
        // `v.x += m·sin`, `v.y −= m·cos` from the table (@0x7124, @0x7178).
        let (sin, cos) = table_trig(angle);
        let (sin_right, cos_right) = table_trig(angle + 90.0);
        let turned = Vec2::new(
            point.y * sin + point.x * sin_right,
            -(point.y * cos) - point.x * cos_right,
        );
        let up = match compression {
            Compression::ByOffset => turned.y < 0.0,
            Compression::ByHeading => angle <= 90.0 || angle >= 270.0,
        };
        let factor = if up { self.up } else { self.down };
        Vec2::new(turned.x * factor.x, turned.y * factor.y - f32::from(z))
    }

    /// Which of `kind`'s exits, with the ship at `from` drawn at `angle`,
    /// is nearest `toward`, the first of equals:
    /// `_SelectClosestShotStartPosition`.
    #[must_use]
    pub fn nearest(
        &self,
        from: Vec2,
        angle: f32,
        kind: ExitKind,
        toward: Vec2,
        compression: Compression,
    ) -> usize {
        let mut nearest = 0;
        let mut best = f32::INFINITY;
        for index in 0..EXITS {
            let off = from + self.offset(angle, kind, index, compression) - toward;
            let distance = off.x * off.x + off.y * off.y;
            if distance < best {
                best = distance;
                nearest = index;
            }
        }
        nearest
    }

    /// The side exit `index` of `kind` stands on: 1 to the right, -1 to
    /// the left and 0 on the centre line, by its x (`_SpawnShot`
    /// @0x3ec2d-0x3ec95). A broadside shot flies out that way.
    #[must_use]
    pub fn side(&self, kind: ExitKind, index: usize) -> f32 {
        let x = self.points[kind.index()].get(index).map_or(0, |&(x, _)| x);
        f32::from(x.signum())
    }
}

/// A percent as a factor (0.01 @0xdd098).
const PERCENT: f64 = 0.01;

/// The degrees-to-radians factor `_BuildTrigTable` builds its table with
/// (0.01745329 @0xdd4e0, a little short of pi / 180).
const TABLE_RADIANS_PER_DEGREE: f64 = 0.017_453_29;

/// The sine and cosine of the whole degree `angle` as the original's table
/// holds them (`_BuildTrigTable` @0x2583b-0x25891): the degree times
/// [`TABLE_RADIANS_PER_DEGREE`] rounded to a float, its sine and cosine
/// in doubles, each rounded to a float. `_TableSin` and `_TableCos` wrap
/// the degree into 0-359 first (@0x2590d-0x25926).
fn table_trig(angle: f32) -> (f32, f32) {
    let degree = normalized(angle.trunc()).trunc();
    let radians = f64::from((f64::from(degree) * TABLE_RADIANS_PER_DEGREE) as f32);
    (radians.sin() as f32, radians.cos() as f32)
}

/// The rotation frame a ship facing `heading` is drawn on, of `frames` a
/// turn: `trunc(heading × frames / 360)`, frame 0 facing up and the frames
/// turning clockwise (`_HandleShipDisplay` @0x2b7a6-0x2b7d0,
/// @0x2bbee-0x2bbff). The view draws it (nova-view's `rotation_frame`),
/// and the exit points turn with it.
///
/// The `shän` `Flags` 0x0100 table that replaces it (@0x2bba2-0x2bbe5),
/// which no stock `shän` sets, is ship-handling-fidelity phase 10's.
#[must_use]
pub fn drawn_frame(heading: f32, frames: NonZeroU16) -> u16 {
    let count = frames.get();
    let per_degree = f32::from(count) / 360.0;
    let frame = (normalized(heading) * per_degree).trunc() as u16;
    // Just short of a full turn can round up to `count`: frame 0 again.
    frame % count
}

/// The angle, in whole degrees, at which a ship's exit points stand on
/// rotation `frame` of `frames` a turn: `trunc(frame × (360 / frames))` in
/// doubles (@0x739e-0x73f5, `_HandleBeams` @0x303bb-0x3042e).
#[must_use]
pub fn frame_angle(frame: u16, frames: NonZeroU16) -> f32 {
    let count = frames.get();
    let step = 360.0 / f64::from(count);
    (f64::from(frame % count) * step).trunc() as f32
}

/// A ship's cursor for each `ExitType` (+0xc8fe-+0xc905): the exit its
/// next shot or beam of the type leaves from, 0 to 3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExitCursors([u8; EXITS]);

impl ExitCursors {
    /// The exit of `kind` the next shot leaves from, in turn.
    #[must_use]
    pub fn next(&self, kind: ExitKind) -> usize {
        usize::from(self.0[kind.index()])
    }

    /// Moves `kind`'s cursor on from `index`, round from 3 to 0.
    fn step_from(&mut self, kind: ExitKind, index: usize) {
        self.0[kind.index()] = ((index + 1) % EXITS) as u8;
    }

    /// Where a shot of `kind` leaves from, of a ship at `from` drawn at
    /// `angle` with `exits`, and the exit it takes: `_ModifyShotStartPosition`
    /// (@0x739e-0x74c4). With `toward`, the target of a weapon of `Flags3`
    /// 0x0010, the exit nearest it; otherwise the cursor's. Either way the
    /// cursor moves on from the exit taken.
    pub fn shot_start(
        &mut self,
        exits: &ExitPoints,
        kind: ExitKind,
        from: Vec2,
        angle: f32,
        toward: Option<Vec2>,
        compression: Compression,
    ) -> (Vec2, usize) {
        let index = match toward {
            Some(toward) => exits.nearest(from, angle, kind, toward, compression),
            None => self.next(kind),
        };
        self.step_from(kind, index);
        (from + exits.offset(angle, kind, index, compression), index)
    }

    /// The exit a beam of `kind` takes, of a ship at `from` facing
    /// `heading`, with `exits`: `_SpawnBeam` (@0x450e4-0x4523d). With
    /// `toward`, the target of a weapon of `Flags3` 0x0010 aimed at a ship,
    /// the exit nearest it with the ship turned to its heading's whole
    /// degree (@0x4519e); otherwise the cursor's. Either way the cursor
    /// moves on from where it was, not from a nearest exit.
    pub fn beam_exit(
        &mut self,
        exits: &ExitPoints,
        kind: ExitKind,
        from: Vec2,
        heading: f32,
        toward: Option<Vec2>,
        compression: Compression,
    ) -> usize {
        let cursor = self.next(kind);
        let index = toward.map_or(cursor, |toward| {
            exits.nearest(from, heading.trunc(), kind, toward, compression)
        });
        self.step_from(kind, cursor);
        index
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn frames(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-4
    }

    /// A 36-frame ship with no compression and no z, whose guns are at
    /// `guns`.
    fn raw(guns: [(i16, i16); 4]) -> ShipExits {
        ShipExits {
            points: [guns, [(0, 0); 4], [(0, 0); 4], [(0, 0); 4]],
            up_compress: (100, 100),
            down_compress: (100, 100),
            frames: 36,
            ..ShipExits::default()
        }
    }

    fn exits(raw: &ShipExits) -> ExitPoints {
        ExitPoints::new(raw).expect("exits")
    }

    #[test]
    fn only_exit_types_0_to_3_name_a_kind() {
        assert_eq!(ExitKind::from_raw(0), Some(ExitKind::Gun));
        assert_eq!(ExitKind::from_raw(1), Some(ExitKind::Turret));
        assert_eq!(ExitKind::from_raw(2), Some(ExitKind::Guided));
        assert_eq!(ExitKind::from_raw(3), Some(ExitKind::Beam));
        for raw in [-1, 4, 99, i16::MIN] {
            assert_eq!(ExitKind::from_raw(raw), None, "{raw}");
        }
    }

    #[test]
    fn the_drawn_frame_truncates_the_heading() {
        let at = |heading| drawn_frame(heading, frames(36));
        assert_eq!(
            [at(0.0), at(9.9), at(10.0), at(15.0), at(359.9)],
            [0, 0, 1, 1, 35]
        );
        assert_eq!(drawn_frame(-10.0, frames(36)), 35, "wrapped");
        assert_eq!(drawn_frame(370.0, frames(36)), 1, "wrapped");
        assert_eq!(drawn_frame(5.7, frames(64)), 1);
    }

    #[test]
    fn a_frames_angle_is_a_whole_degree() {
        assert_eq!(frame_angle(0, frames(36)), 0.0);
        assert_eq!(frame_angle(9, frames(36)), 90.0);
        assert_eq!(frame_angle(1, frames(64)), 5.0, "5.625, truncated");
        assert_eq!(frame_angle(3, frames(64)), 16.0, "16.875, truncated");
        assert_eq!(frame_angle(1, frames(48)), 7.0, "7.5, truncated");
    }

    #[test]
    fn an_exit_point_turns_with_the_drawn_frame() {
        let gun = exits(&raw([(3, 10); 4]));
        let at = |angle| gun.offset(angle, ExitKind::Gun, 0, Compression::ByOffset);
        assert!(close(at(0.0), Vec2::new(3.0, -10.0)), "{:?}", at(0.0));
        assert!(close(at(90.0), Vec2::new(10.0, 3.0)), "{:?}", at(90.0));
        assert!(close(at(180.0), Vec2::new(-3.0, 10.0)), "{:?}", at(180.0));
        assert!(close(at(270.0), Vec2::new(-10.0, -3.0)), "{:?}", at(270.0));
    }

    /// Guns ahead, behind and to the right; up compression (1, 0.5) and
    /// down (0.5, 1).
    fn squashed() -> ExitPoints {
        exits(&ShipExits {
            up_compress: (100, 50),
            down_compress: (50, 100),
            ..raw([(0, 10), (0, -10), (10, 0), (0, 0)])
        })
    }

    #[test]
    fn compression_follows_the_rotated_offset_up_or_down_the_screen() {
        let at =
            |angle, index| squashed().offset(angle, ExitKind::Gun, index, Compression::ByOffset);
        assert!(
            close(at(0.0, 0), Vec2::new(0.0, -5.0)),
            "ahead, up the screen"
        );
        assert!(close(at(0.0, 1), Vec2::new(0.0, 10.0)), "behind, down it");
        assert!(
            close(at(180.0, 0), Vec2::new(0.0, 10.0)),
            "ahead, turned down it"
        );
        assert!(
            close(at(180.0, 1), Vec2::new(0.0, -5.0)),
            "behind, turned up it"
        );
    }

    #[test]
    fn the_original_trig_table_decides_a_screen_y_of_almost_0() {
        // In `_BuildTrigTable`'s table cos 90 is just above 0 and cos 270
        // just below, so a side exit facing up is just up the screen, and
        // one turned to 180 just down it.
        let at = |angle| squashed().offset(angle, ExitKind::Gun, 2, Compression::ByOffset);
        assert!(close(at(0.0), Vec2::new(10.0, 0.0)), "{:?}", at(0.0));
        assert!(at(0.0).y < 0.0);
        assert!(close(at(180.0), Vec2::new(-5.0, 0.0)), "{:?}", at(180.0));
        assert!(at(180.0).y > 0.0);
    }

    #[test]
    fn a_point_on_the_centre_takes_the_down_compression_and_its_z() {
        let mut raw = raw([(0, 0); 4]);
        raw.z[0] = [4, 0, 0, 0];
        let at = exits(&raw).offset(0.0, ExitKind::Gun, 0, Compression::ByOffset);
        assert_eq!(at, Vec2::new(0.0, -4.0));
    }

    #[test]
    fn by_the_bible_compression_follows_the_heading() {
        let at =
            |angle, index| squashed().offset(angle, ExitKind::Gun, index, Compression::ByHeading);
        assert!(close(at(0.0, 1), Vec2::new(0.0, 5.0)), "behind, facing up");
        assert!(close(at(90.0, 1), Vec2::new(-10.0, 0.0)), "90 is up");
        assert!(close(at(180.0, 0), Vec2::new(0.0, 10.0)), "180 is down");
        assert!(close(at(270.0, 0), Vec2::new(-10.0, 0.0)), "270 is up");
        let down = at(91.0, 2);
        let rotated = Vec2::new(
            10.0 * 91.0_f32.to_radians().cos(),
            10.0 * 91.0_f32.to_radians().sin(),
        );
        assert!(
            close(down, Vec2::new(rotated.x * 0.5, rotated.y)),
            "91 is down: {down:?}"
        );
    }

    #[test]
    fn z_moves_the_point_up_the_screen_after_compression() {
        let mut raw = raw([(0, 10); 4]);
        raw.up_compress = (100, 50);
        raw.z[0] = [2, -2, 0, 0];
        let gun = exits(&raw);
        let at = |index| gun.offset(0.0, ExitKind::Gun, index, Compression::ByOffset);
        assert!(close(at(0), Vec2::new(0.0, -7.0)), "{:?}", at(0));
        assert!(close(at(1), Vec2::new(0.0, -3.0)), "{:?}", at(1));
    }

    #[test]
    fn each_kind_reads_its_own_points_and_z() {
        let mut raw = raw([(0, 0); 4]);
        raw.points[3] = [(0, 0), (30, 0), (0, 0), (-30, 0)];
        raw.z[3] = [25, 0, -25, 0];
        let beam = exits(&raw);
        let at = |index| beam.offset(0.0, ExitKind::Beam, index, Compression::ByOffset);
        assert!(close(at(0), Vec2::new(0.0, -25.0)));
        assert!(close(at(1), Vec2::new(30.0, 0.0)));
        assert!(close(at(2), Vec2::new(0.0, 25.0)));
        assert!(close(at(3), Vec2::new(-30.0, 0.0)));
        let gun = beam.offset(0.0, ExitKind::Gun, 1, Compression::ByOffset);
        assert_eq!(gun, Vec2::ZERO);
    }

    #[test]
    fn a_compression_of_zero_or_less_counts_as_100_percent() {
        for (pct, factor) in [(0, 1.0), (-5, 1.0), (71, 0.71), (130, 1.3)] {
            let gun = exits(&ShipExits {
                up_compress: (pct, pct),
                ..raw([(0, 10); 4])
            });
            let at = gun.offset(0.0, ExitKind::Gun, 0, Compression::ByOffset);
            assert!(close(at, Vec2::new(0.0, -10.0 * factor)), "{pct}: {at:?}");
        }
    }

    #[test]
    fn a_frame_count_of_0_reads_as_36_and_one_below_0_has_no_exits() {
        let zero = exits(&ShipExits {
            frames: 0,
            ..raw([(0, 10); 4])
        });
        assert_eq!(zero.frames(), frames(36));
        assert_eq!(
            ExitPoints::new(&ShipExits {
                frames: -1,
                ..raw([(0, 10); 4])
            }),
            None
        );
        assert_eq!(
            exits(&ShipExits {
                frames: 64,
                ..raw([(0, 10); 4])
            })
            .frames(),
            frames(64)
        );
    }

    #[test]
    fn shots_take_their_exit_types_points_in_turn() {
        let gun = exits(&raw([(1, 0), (2, 0), (3, 0), (4, 0)]));
        let mut cursors = ExitCursors::default();
        let from = Vec2::new(100.0, 50.0);
        let fire = |cursors: &mut ExitCursors| {
            cursors.shot_start(&gun, ExitKind::Gun, from, 0.0, None, Compression::ByOffset)
        };
        for (shot, x) in [(0, 101.0), (1, 102.0), (2, 103.0), (3, 104.0), (0, 101.0)] {
            let (start, index) = fire(&mut cursors);
            assert_eq!(index, shot);
            assert!(close(start, Vec2::new(x, 50.0)), "{shot}: {start:?}");
        }
        assert_eq!(cursors.next(ExitKind::Gun), 1);
        assert_eq!(cursors.next(ExitKind::Turret), 0, "untouched");
    }

    #[test]
    fn the_nearest_exit_to_the_target_fires_and_the_turn_goes_on_after_it() {
        let gun = exits(&raw([(-10, 0), (0, 10), (10, 0), (10, 0)]));
        let mut cursors = ExitCursors::default();
        let start = |cursors: &mut ExitCursors, toward| {
            cursors.shot_start(
                &gun,
                ExitKind::Gun,
                Vec2::ZERO,
                0.0,
                toward,
                Compression::ByOffset,
            )
        };
        let (at, index) = start(&mut cursors, Some(Vec2::new(50.0, 0.0)));
        assert_eq!(index, 2, "the first of the two nearest");
        assert!(close(at, Vec2::new(10.0, 0.0)));
        assert_eq!(cursors.next(ExitKind::Gun), 3);
        assert_eq!(start(&mut cursors, None).1, 3, "with no target, in turn");
        assert_eq!(start(&mut cursors, Some(Vec2::new(0.0, -50.0))).1, 1);
        assert_eq!(cursors.next(ExitKind::Gun), 2);
    }

    #[test]
    fn the_nearest_exit_is_measured_from_where_the_ship_is() {
        let gun = exits(&raw([(-10, 0), (10, 0), (0, 0), (0, 0)]));
        assert_eq!(
            gun.nearest(
                Vec2::new(100.0, 0.0),
                0.0,
                ExitKind::Gun,
                Vec2::new(95.0, 0.0),
                Compression::ByOffset
            ),
            0
        );
        assert_eq!(
            gun.nearest(
                Vec2::new(100.0, 0.0),
                180.0,
                ExitKind::Gun,
                Vec2::new(95.0, 0.0),
                Compression::ByOffset
            ),
            1,
            "turned"
        );
    }

    #[test]
    fn a_beam_takes_the_exit_nearest_by_the_ships_heading_and_steps_its_cursor_without_setting_it()
    {
        let mut raw = raw([(0, 0); 4]);
        raw.points[3] = [(0, 0), (30, 0), (0, 0), (-30, 0)];
        raw.z[3] = [25, 0, -25, 0];
        let beam = exits(&raw);
        let mut cursors = ExitCursors::default();
        let exit = |cursors: &mut ExitCursors, heading, toward| {
            cursors.beam_exit(
                &beam,
                ExitKind::Beam,
                Vec2::ZERO,
                heading,
                toward,
                Compression::ByOffset,
            )
        };
        assert_eq!(
            exit(&mut cursors, 0.0, Some(Vec2::new(100.0, 0.0))),
            1,
            "starboard"
        );
        assert_eq!(
            cursors.next(ExitKind::Beam),
            1,
            "stepped from 0, not set to 1 + 1"
        );
        assert_eq!(
            exit(&mut cursors, 0.0, Some(Vec2::new(-100.0, 0.0))),
            3,
            "port"
        );
        assert_eq!(cursors.next(ExitKind::Beam), 2);
        assert_eq!(
            exit(&mut cursors, 180.9, Some(Vec2::new(-100.0, 0.0))),
            1,
            "turned about, by the heading's whole degree"
        );
        assert_eq!(exit(&mut cursors, 0.0, None), 3, "with no target, in turn");
        assert_eq!(cursors.next(ExitKind::Beam), 0);
    }

    #[test]
    fn the_side_is_the_sign_of_the_exits_x() {
        let gun = exits(&raw([(3, 10), (-3, 10), (0, 10), (0, 0)]));
        assert_eq!(gun.side(ExitKind::Gun, 0), 1.0);
        assert_eq!(gun.side(ExitKind::Gun, 1), -1.0);
        assert_eq!(gun.side(ExitKind::Gun, 2), 0.0);
    }
}
