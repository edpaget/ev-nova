//! The rotation frame a ship is drawn on and the angle it stands at
//! ([`drawn_frame`], [`frame_angle`]), and a `shän`'s compressions
//! ([`compression`]), from `nova_sim_combat::combat::exits`, which
//! re-exports them and keeps the exit points that turn with them.

use std::num::NonZeroU16;

use nova_sim_base::flight::normalized;
use nova_sim_base::geometry::Vec2;
use nova_sim_catalog::catalog::ShipExits;

use super::ship_frames::FrameTable;

/// The frames in a turn of a `shän` whose `FramesPer` is 0
/// (`_LoadExtendedShipSprites` @0x2274c).
pub const DEFAULT_FRAMES: u16 = 36;

/// A `shän`'s up and down compressions, `raw`'s, each its percent over
/// 100 and any at or below 0 counting as 100
/// (`_LoadExtendedShipSprites` @0x23d45-0x23e07).
#[must_use]
pub fn compression(raw: &ShipExits) -> (Vec2, Vec2) {
    let factor = |pct: i16| {
        if pct > 0 {
            (f64::from(pct) * PERCENT) as f32
        } else {
            1.0
        }
    };
    let compress = |(x, y): (i16, i16)| Vec2::new(factor(x), factor(y));
    (compress(raw.up_compress), compress(raw.down_compress))
}

/// A percent as a factor (0.01 @0xdd098).
const PERCENT: f64 = 0.01;

/// The rotation frame a ship facing `heading` is drawn on, of `frames` a
/// turn: `trunc(heading × frames / 360)`, frame 0 facing up and the frames
/// turning clockwise (`_HandleShipDisplay` @0x2b7a6-0x2b7d0,
/// @0x2bbee-0x2bbff); or, for a class of `shän` `Flags` 0x0100 with its
/// frame-angle `table`, the frame the table picks (@0x2bba2-0x2bbe5,
/// [`FrameTable::frame`]). The view draws it (nova-view's
/// `rotation_frame`), and the exit points turn with it, reading it mod
/// `FramesPer` (`_ModifyShotStartPosition` @0x73c7-0x73d7): it is the
/// rotation alone, whatever set the ship is drawn on.
#[must_use]
pub fn drawn_frame(heading: f32, frames: NonZeroU16, table: Option<&FrameTable>) -> u16 {
    if let Some(table) = table {
        return table.frame(heading);
    }
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

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn frames(n: u16) -> NonZeroU16 {
        NonZeroU16::new(n).expect("non-zero")
    }

    #[test]
    fn a_compression_is_its_percent_over_100_and_none_or_less_counts_as_100() {
        let exits = ShipExits {
            up_compress: (50, 0),
            down_compress: (-20, 150),
            ..ShipExits::default()
        };
        let (up, down) = compression(&exits);
        assert_eq!(up, Vec2::new(0.5, 1.0));
        assert_eq!(down, Vec2::new(1.0, 1.5));
        assert_eq!(
            compression(&ShipExits::default()),
            (Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0))
        );
    }

    #[test]
    fn the_drawn_frame_truncates_the_heading() {
        let at = |heading| drawn_frame(heading, frames(36), None);
        assert_eq!(
            [at(0.0), at(9.9), at(10.0), at(15.0), at(359.9)],
            [0, 0, 1, 1, 35]
        );
        assert_eq!(drawn_frame(-10.0, frames(36), None), 35, "wrapped");
        assert_eq!(drawn_frame(370.0, frames(36), None), 1, "wrapped");
        assert_eq!(drawn_frame(5.7, frames(64), None), 1);
    }

    #[test]
    fn a_frames_angle_is_a_whole_degree() {
        assert_eq!(frame_angle(0, frames(36)), 0.0);
        assert_eq!(frame_angle(9, frames(36)), 90.0);
        assert_eq!(frame_angle(1, frames(64)), 5.0, "5.625, truncated");
        assert_eq!(frame_angle(3, frames(64)), 16.0, "16.875, truncated");
        assert_eq!(frame_angle(1, frames(48)), 7.0, "7.5, truncated");
    }
}
