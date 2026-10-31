//! The player's wrap: `_WrapPlayerPosition` (@0x67efa-0x6838f).
//!
//! A ship past [`WRAP_EDGE`] from the system's centre on an axis is carried
//! back [`WRAP_SHIFT`] on that axis, and what stands near it comes along.
//! [`PlayerWrap`] holds the one rule every table's carry shares, so each
//! owner of a table only calls it.
//!
//! The delta (`_wrapDelta` @0x316984), from the player's +0x18/+0x1c, each
//! axis on its own and both strict: above 15000.0 (@0x67f21, y @0x67f5c)
//! −25000.0, below −15000.0 (@0x67f34, y @0x67f73) +25000.0, exactly ±15000.0
//! or inside it 0. Neither axis out and the routine returns (@0x67f86).
//!
//! The test (@0x67fab-0x67fe8), against the player as it stood before the
//! wrap: `cvttss2si` of slot − player on each axis, `abs`, `cmpl $0x3a97`
//! (14999) `setle`, so a slot moves when |dx| and |dy| are both under
//! 15000.0. The whole delta is added, 0 on an axis that didn't wrap.

use crate::geometry::Vec2;

/// How far out on an axis the player may stand: past it, it wraps (floats
/// @0xdd950, @0xdd954), and what stands this far from it stays behind.
pub const WRAP_EDGE: f32 = 15000.0;

/// How far a wrap carries the player back on the axis it left by
/// (`0xc6c35000` @0x67f23, `0x46c35000` @0x67f3d).
pub const WRAP_SHIFT: f32 = 25000.0;

/// One wrap of the player: where it stood and how far it was carried.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerWrap {
    /// The player's position before the wrap, which [`carries`] measures
    /// from.
    ///
    /// [`carries`]: Self::carries
    pub from: Vec2,
    /// What the wrap adds: ∓[`WRAP_SHIFT`] on an axis it left by, 0 on the
    /// other.
    pub delta: Vec2,
}

impl PlayerWrap {
    /// The wrap of a player standing at `player`, or `None` while it stands
    /// inside (or exactly on) the edge on both axes.
    #[must_use]
    pub fn of(player: Vec2) -> Option<Self> {
        let delta = Vec2::new(axis_delta(player.x), axis_delta(player.y));
        (delta != Vec2::ZERO).then_some(Self {
            from: player,
            delta,
        })
    }

    /// Whether what stands at `at` comes along: within the edge of where
    /// the player stood on both axes.
    #[must_use]
    pub fn carries(&self, at: Vec2) -> bool {
        (at.x - self.from.x).abs() < WRAP_EDGE && (at.y - self.from.y).abs() < WRAP_EDGE
    }

    /// `at` with the whole delta added, 0 on an axis that didn't wrap.
    #[must_use]
    pub fn carried(&self, at: Vec2) -> Vec2 {
        at + self.delta
    }

    /// Moves `at` if the wrap [`carries`](Self::carries) it. Says whether
    /// it did.
    pub fn carry(&self, at: &mut Vec2) -> bool {
        let moves = self.carries(*at);
        if moves {
            *at = self.carried(*at);
        }
        moves
    }
}

/// What one axis of the wrap adds, for a player at `at` on it.
fn axis_delta(at: f32) -> f32 {
    if at > WRAP_EDGE {
        -WRAP_SHIFT
    } else if at < -WRAP_EDGE {
        WRAP_SHIFT
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrap_at(player: Vec2) -> PlayerWrap {
        PlayerWrap::of(player).expect("the player is past the edge")
    }

    #[test]
    fn inside_the_edge_and_on_it_there_is_no_wrap() {
        for at in [
            Vec2::ZERO,
            Vec2::new(WRAP_EDGE, 0.0),
            Vec2::new(-WRAP_EDGE, 0.0),
            Vec2::new(0.0, WRAP_EDGE),
            Vec2::new(0.0, -WRAP_EDGE),
            Vec2::new(WRAP_EDGE, -WRAP_EDGE),
        ] {
            assert_eq!(PlayerWrap::of(at), None, "{at:?}");
        }
    }

    #[test]
    fn past_an_x_edge_it_shifts_x_back_and_leaves_y() {
        let east = Vec2::new(15000.5, 40.0);
        assert_eq!(wrap_at(east).delta, Vec2::new(-WRAP_SHIFT, 0.0));
        let west = Vec2::new(-15000.5, 40.0);
        assert_eq!(wrap_at(west).delta, Vec2::new(WRAP_SHIFT, 0.0));
    }

    #[test]
    fn past_a_y_edge_it_shifts_y_back_and_leaves_x() {
        let south = Vec2::new(40.0, 15000.5);
        assert_eq!(wrap_at(south).delta, Vec2::new(0.0, -WRAP_SHIFT));
        let north = Vec2::new(40.0, -15000.5);
        assert_eq!(wrap_at(north).delta, Vec2::new(0.0, WRAP_SHIFT));
    }

    #[test]
    fn past_both_edges_it_shifts_each_axis() {
        let corner = Vec2::new(15000.5, -15000.5);
        assert_eq!(wrap_at(corner).delta, Vec2::new(-WRAP_SHIFT, WRAP_SHIFT));
    }

    #[test]
    fn the_wrap_remembers_where_the_player_stood() {
        let at = Vec2::new(15000.5, 40.0);
        assert_eq!(wrap_at(at).from, at);
    }

    #[test]
    fn it_carries_what_stands_under_the_edge_on_both_axes_of_the_player() {
        let from = Vec2::new(15010.0, 300.0);
        let wrap = wrap_at(from);
        for offset in [
            Vec2::ZERO,
            Vec2::new(14999.9, 14999.9),
            Vec2::new(-14999.9, 14999.9),
            Vec2::new(14999.9, -14999.9),
            Vec2::new(-14999.9, -14999.9),
        ] {
            assert!(wrap.carries(from + offset), "{offset:?}");
        }
    }

    #[test]
    fn it_leaves_what_stands_at_the_edge_on_either_axis_of_the_player() {
        let from = Vec2::new(15010.0, 300.0);
        let wrap = wrap_at(from);
        for offset in [
            Vec2::new(WRAP_EDGE, 0.0),
            Vec2::new(-WRAP_EDGE, 0.0),
            Vec2::new(0.0, WRAP_EDGE),
            Vec2::new(0.0, -WRAP_EDGE),
        ] {
            assert!(!wrap.carries(from + offset), "{offset:?}");
        }
    }

    #[test]
    fn it_adds_the_whole_delta() {
        let wrap = wrap_at(Vec2::new(15010.0, -15010.0));
        assert_eq!(
            wrap.carried(Vec2::new(10.0, 20.0)),
            Vec2::new(10.0 - WRAP_SHIFT, 20.0 + WRAP_SHIFT)
        );
    }

    #[test]
    fn carry_moves_only_what_the_wrap_carries() {
        let wrap = wrap_at(Vec2::new(15010.0, 0.0));
        let mut near = Vec2::new(14000.0, 50.0);
        assert!(wrap.carry(&mut near));
        assert_eq!(near, Vec2::new(14000.0 - WRAP_SHIFT, 50.0));
        let mut far = Vec2::new(10.0, 50.0);
        assert!(!wrap.carry(&mut far));
        assert_eq!(far, Vec2::new(10.0, 50.0));
    }
}
