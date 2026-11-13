//! How a ship's cloak lets the player's screen show it ([`CloakShown`],
//! [`FADE_FULL`], [`TRANSLUCENCY_STEPS`]) and the world-space rectangle a
//! view shows ([`Area`], [`DEFAULT_PLAY_AREA`]), from
//! `nova_sim_combat::cloak`, which re-exports them and keeps the cloak
//! itself: its fade, its gear and who sees through it.

use nova_sim_base::geometry::Vec2;

/// The fade of a ship fully cloaked, where fading in stops.
pub const FADE_FULL: f32 = 32.0;

/// The steps of a sprite's translucency: a level of this or more is not
/// drawn (`_BlitPixieRLETranslucentDrawProc` @0xbb5b6-0xbb5d8 draws
/// nothing above 0x3f).
pub const TRANSLUCENCY_STEPS: u8 = 0x40;

/// How a ship's cloak lets the player's screen show it
/// (`_HandleShipDisplay` @0x2d169-0x2d648).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CloakShown {
    /// Drawn as it is: no fade (at or below 0, @0x2d169-0x2d16c).
    Opaque,
    /// Fading, at this fade, above 0 and below [`FADE_FULL`]: drawn
    /// jittered and faint, whoever leads it (@0x2d2c0-0x2d648).
    Fading(f32),
    /// Fully faded and drawn translucent, at this level in 64ths: always
    /// below [`TRANSLUCENCY_STEPS`] (@0x2d187-0x2d1cd).
    Translucent(u8),
    /// Fully faded and its hull not drawn (translucency
    /// [`TRANSLUCENCY_STEPS`]); its weapon glow, when lit, still is.
    Hidden,
}

/// A rectangle in world space: x grows right and y down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Area {
    /// Its left edge.
    pub left: f32,
    /// Its top edge.
    pub top: f32,
    /// Its right edge.
    pub right: f32,
    /// Its bottom edge.
    pub bottom: f32,
}

impl Area {
    /// The area `width` x `height` centred on `at`.
    #[must_use]
    pub fn centred(at: Vec2, width: f32, height: f32) -> Self {
        Self {
            left: at.x - width / 2.0,
            top: at.y - height / 2.0,
            right: at.x + width / 2.0,
            bottom: at.y + height / 2.0,
        }
    }

    /// The area moved by `by`.
    #[must_use]
    pub fn offset(self, by: Vec2) -> Self {
        Self {
            left: self.left + by.x,
            top: self.top + by.y,
            right: self.right + by.x,
            bottom: self.bottom + by.y,
        }
    }

    /// Whether this area and `other` strictly overlap, so two that only
    /// touch along an edge do not. A zero-size area (a point or a line)
    /// overlaps one it lies strictly inside.
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
}

/// The play area the session assumes when no view has said what it
/// shows: the original's smallest screen, 800 x 600, less its 194-pixel
/// status bar, centred on the player.
pub const DEFAULT_PLAY_AREA: Area = Area {
    left: -303.0,
    top: -300.0,
    right: 303.0,
    bottom: 300.0,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// The area from (`left`, `top`) to (`right`, `bottom`).
    fn area(left: f32, top: f32, right: f32, bottom: f32) -> Area {
        Area {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn two_areas_overlap_only_strictly() {
        let unit = area(0.0, 0.0, 10.0, 10.0);
        assert!(unit.overlaps(area(5.0, 5.0, 15.0, 15.0)));
        assert!(unit.overlaps(area(-5.0, -5.0, 1.0, 1.0)));
        assert!(unit.overlaps(unit));
        for (touching, side) in [
            (area(10.0, 0.0, 20.0, 10.0), "right"),
            (area(-10.0, 0.0, 0.0, 10.0), "left"),
            (area(0.0, 10.0, 10.0, 20.0), "bottom"),
            (area(0.0, -10.0, 10.0, 0.0), "top"),
        ] {
            assert!(!unit.overlaps(touching), "{side}");
            assert!(!touching.overlaps(unit), "{side}");
        }
        assert!(!unit.overlaps(area(20.0, 20.0, 30.0, 30.0)));
        assert!(!unit.overlaps(area(0.0, 20.0, 10.0, 30.0)), "below");
        assert!(!unit.overlaps(area(20.0, 0.0, 30.0, 10.0)), "beside");
    }

    #[test]
    fn a_point_overlaps_an_area_it_lies_strictly_inside() {
        let unit = area(0.0, 0.0, 10.0, 10.0);
        assert!(area(5.0, 5.0, 5.0, 5.0).overlaps(unit));
        assert!(!area(0.0, 5.0, 0.0, 5.0).overlaps(unit), "on the edge");
        assert!(!area(10.0, 10.0, 10.0, 10.0).overlaps(unit), "a corner");
    }

    #[test]
    fn a_full_fade_is_0x40_translucency_steps() {
        assert_eq!(TRANSLUCENCY_STEPS, 0x40);
    }

    #[test]
    fn an_area_is_centred_and_moved() {
        let area = Area::centred(Vec2::new(10.0, -20.0), 100.0, 60.0);
        assert_eq!(
            area,
            Area {
                left: -40.0,
                top: -50.0,
                right: 60.0,
                bottom: 10.0,
            }
        );
        assert_eq!(
            area.offset(Vec2::new(1.0, 2.0)),
            Area {
                left: -39.0,
                top: -48.0,
                right: 61.0,
                bottom: 12.0,
            }
        );
        assert_eq!(
            DEFAULT_PLAY_AREA,
            Area::centred(Vec2::ZERO, 800.0 - 194.0, 600.0)
        );
    }
}
