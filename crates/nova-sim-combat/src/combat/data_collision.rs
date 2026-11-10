//! The shape a ship is struck by, as the rulebook's
//! [`RuleKey::DataCollision`](crate::RuleKey::DataCollision) says
//! ([`CollisionShape`]).
//!
//! Under either reading this governs every test of whether a *ship is
//! struck*:
//!
//! - a shot's contact, placed and swept ([`contact`](super::contact));
//! - a proximity fuse, its `radius + prox` and its fuse width
//!   ([`projectile`](super::projectile));
//! - a beam's cone, read from the target's sprite width, and its swept
//!   reading ([`beam`](super::beam));
//! - point defence's view of its target, which shares the
//!   [`Target`](super::projectile::Target);
//! - a deadly stellar's disc test
//!   ([`stellar_hazards`](super::stellar_hazards)).
//!
//! The two readings:
//!
//! - By the engine ([`CollisionShape::Sprite`]), a ship is struck by its
//!   sprite: its rect placed at its frame's size, tested by
//!   `_SWRadiusCollision` @0xabc88 when 32 or fewer pixels wide and by its
//!   RLE mask (`_SWRLECollision` @0x37045) when wider
//!   (`_ShipCollisionHandler` @0x3700f-0x3704a), else the swept disc of
//!   its [`hit_radius`](super::hull::HullSpec::hit_radius), a third of its
//!   frame width (from `_HandleBeams`' 0.66 × width), which the fuse and a
//!   deadly stellar's disc read too.
//! - By the other reading ([`CollisionShape::DataDisc`]), the new game's,
//!   a ship whose catalog data gives a collision radius
//!   ([`HullSpec::collision_radius`]) is struck as a disc of that radius
//!   `R`: its target radius is `R`, its sprite width for the beam's cone
//!   and the fuse `2R` (its diameter), it skips the placed rect and mask
//!   and goes straight to the swept disc, and a deadly stellar skips its
//!   mask and tests the disc of `R` ([`CollisionShape::disc`],
//!   [`CollisionShape::extent`]). The shape doesn't turn with the hull, so
//!   it holds under either reading of
//!   [`RuleKey::ContinuousHeadings`](crate::RuleKey::ContinuousHeadings).
//!
//! A ship without a collision radius, every stock ship among them, keeps
//! the engine's reading under both, so turning the key on over stock data
//! changes nothing.
//!
//! Left alone under either reading, as none is a ship being struck, and
//! for the new game's ships each already reads the data's beam and length
//! through the `shän`: the board reach
//! ([`HullSpec::board_reach`](super::hull::HullSpec::board_reach)),
//! formation spacing, a cargo box's drop and its scoop's reach, the cloak's
//! on-screen test, target brackets, a fighter's launch width, and asteroid
//! and stellar contacts, which no ship body is in. None of this plays a
//! sound.

use crate::rulebook::{RuleKey, RuleSource, Rulebook};

use super::beam;
use super::defence::Candidate;
use super::hull::HullSpec;
use super::projectile::Target;
use super::{Combat, Fighter};

/// The shape a ship is struck by (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollisionShape {
    /// The original's: its sprite.
    #[default]
    Sprite,
    /// The new game's: its data's collision disc, where it has one.
    DataDisc,
}

impl From<RuleSource> for CollisionShape {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::Sprite,
            RuleSource::Bible => Self::DataDisc,
        }
    }
}

/// How big a target a ship is, as a shot, a fuse and a beam read it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetExtent {
    /// The radius of the swept disc a shot, a fuse or a beam's swept
    /// reading strikes it by, in pixels.
    pub radius: f32,
    /// The sprite width the beam's cone and the fuse read, in pixels.
    pub sprite_x: f32,
    /// Whether it has a sheet the beam's cone reads.
    pub sheet: bool,
}

impl CollisionShape {
    /// The radius of the disc `hull` is struck as: its data's collision
    /// radius under [`CollisionShape::DataDisc`] when it has one; none
    /// otherwise, when it is struck by its sprite.
    #[must_use]
    pub fn disc(self, hull: &HullSpec) -> Option<f32> {
        match self {
            Self::Sprite => None,
            Self::DataDisc => hull.collision_radius,
        }
    }

    /// How big a target `hull` is: by its [`disc`](Self::disc) `R`, a
    /// radius of `R`, a sprite width of `2R` and a sheet; by its sprite,
    /// its hit radius, its sprite width (or
    /// [`beam::NO_SPRITE_WIDTH`] with none) and whether it has one.
    #[must_use]
    pub fn extent(self, hull: &HullSpec) -> TargetExtent {
        match self.disc(hull) {
            Some(radius) => TargetExtent {
                radius,
                sprite_x: 2.0 * radius,
                sheet: true,
            },
            None => TargetExtent {
                radius: hull.hit_radius,
                sprite_x: hull.sprite_size.unwrap_or(beam::NO_SPRITE_WIDTH),
                sheet: hull.sprite_size.is_some(),
            },
        }
    }
}

impl Combat {
    /// Has the fight read its hulls' three rules as `rules` says: how a
    /// hull turns,
    /// [`RuleKey::ContinuousHeadings`](crate::RuleKey::ContinuousHeadings)
    /// ([`Combat::set_continuous_headings`]); the shape a ship is struck
    /// by, [`RuleKey::DataCollision`](crate::RuleKey::DataCollision)
    /// ([`Combat::set_data_collision`]); and where its shots start,
    /// [`RuleKey::MountedMuzzles`](crate::RuleKey::MountedMuzzles)
    /// ([`Combat::set_mounted_muzzles`]).
    pub fn set_hull_rules(&mut self, rules: &Rulebook) {
        self.set_continuous_headings(rules.source_for(RuleKey::ContinuousHeadings));
        self.set_data_collision(rules.source_for(RuleKey::DataCollision));
        self.set_mounted_muzzles(rules.source_for(RuleKey::MountedMuzzles));
    }

    /// `fighters` as shots, beams and aims see them, each by the shape the
    /// fight strikes it as ([`Fighter::as_target`]): every list of the
    /// fight's targets is read through this, or through the lineup, which
    /// takes the shape with the hit readings.
    pub(super) fn targets(&self, fighters: &[Fighter]) -> Vec<Target> {
        fighters
            .iter()
            .map(|fighter| fighter.as_target(self.collision_shape))
            .collect()
    }

    /// `fighters` as point defence weighs them as quarries, each by the
    /// shape the fight strikes it as ([`Fighter::as_candidate`]).
    pub(super) fn candidates(&self, fighters: &[Fighter]) -> Vec<Candidate> {
        fighters
            .iter()
            .map(|fighter| fighter.as_candidate(self.collision_shape))
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// A hull like the sample's light freighter: 26 x 40, radius 20.
    fn freighter() -> HullSpec {
        HullSpec {
            hit_radius: 26.0 * super::super::hull::HIT_RADIUS_PER_SIZE,
            sprite_size: Some(26.0),
            sprite_height: Some(40.0),
            collision_radius: Some(20.0),
            ..HullSpec::default()
        }
    }

    fn without_radius(hull: &HullSpec) -> HullSpec {
        HullSpec {
            collision_radius: None,
            ..*hull
        }
    }

    #[test]
    fn the_engine_reads_the_sprite_and_the_other_reading_the_data_disc() {
        assert_eq!(
            CollisionShape::from(RuleSource::Engine),
            CollisionShape::Sprite
        );
        assert_eq!(
            CollisionShape::from(RuleSource::Bible),
            CollisionShape::DataDisc
        );
        assert_eq!(CollisionShape::default(), CollisionShape::Sprite);
    }

    #[test]
    fn only_the_data_disc_reading_of_a_hull_with_a_radius_gives_a_disc() {
        assert_eq!(CollisionShape::DataDisc.disc(&freighter()), Some(20.0));
        assert_eq!(CollisionShape::Sprite.disc(&freighter()), None);
        assert_eq!(
            CollisionShape::DataDisc.disc(&without_radius(&freighter())),
            None
        );
        assert_eq!(CollisionShape::DataDisc.disc(&HullSpec::default()), None);
        assert_eq!(CollisionShape::Sprite.disc(&HullSpec::default()), None);
    }

    #[test]
    fn by_its_disc_a_ship_is_a_target_of_its_radius_and_diameter_with_a_sheet() {
        assert_eq!(
            CollisionShape::DataDisc.extent(&freighter()),
            TargetExtent {
                radius: 20.0,
                sprite_x: 40.0,
                sheet: true,
            }
        );
    }

    #[test]
    fn by_its_sprite_a_ship_is_a_target_of_its_hit_radius_and_sprite_width() {
        let hull = freighter();
        let sprite = TargetExtent {
            radius: hull.hit_radius,
            sprite_x: 26.0,
            sheet: true,
        };
        assert_eq!(CollisionShape::Sprite.extent(&hull), sprite);
        assert_eq!(
            CollisionShape::DataDisc.extent(&without_radius(&hull)),
            sprite,
            "without a radius, the sprite under both"
        );
    }

    #[test]
    fn a_hull_with_no_sprite_and_no_radius_is_the_placeholder_target_under_both() {
        let placeholder = TargetExtent {
            radius: HullSpec::default().hit_radius,
            sprite_x: beam::NO_SPRITE_WIDTH,
            sheet: false,
        };
        for shape in [CollisionShape::Sprite, CollisionShape::DataDisc] {
            assert_eq!(shape.extent(&HullSpec::default()), placeholder, "{shape:?}");
        }
    }

    #[test]
    fn a_hull_with_a_radius_but_no_sprite_gains_a_sheet_only_by_its_disc() {
        let disc_only = HullSpec {
            collision_radius: Some(8.0),
            ..HullSpec::default()
        };
        assert_eq!(
            CollisionShape::DataDisc.extent(&disc_only),
            TargetExtent {
                radius: 8.0,
                sprite_x: 16.0,
                sheet: true,
            }
        );
        assert!(!CollisionShape::Sprite.extent(&disc_only).sheet);
    }
}
