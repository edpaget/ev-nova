//! Gravity: a stellar whose `spöb` `Gravity` is not none pulls (or, below
//! none, pushes) every ship in its system, as `_HandleGravity`
//! (@0x39bc3-0x39ccc) and `_ApplyGravity` (@0x7c5a-0x7d07) do in the
//! original (`EV Nova.app`, i386).
//!
//! - **The pull** ([`pull`], `_ApplyGravity`): the ship's offset from the
//!   stellar is scaled by [`DISTANCE_SCALE`] on each axis (the `f32`
//!   difference times the double 0.01 @0xdd098, back to `f32`), its square
//!   is floored at [`MIN_DISTANCE_SQUARED`] (`maxss` against 30.0
//!   @0xdd084), and `_Accel` pushes the velocity by `Gravity` × the speed
//!   multiplier over that square along the bearing from the ship to the
//!   stellar (`_CalcInterceptAngle(ship, stellar)`). The pull is constant
//!   within √30 × 100 ≈ 548 pixels and falls with the inverse square
//!   beyond. `_Accel` caps nothing: the player's velocity is clamped to
//!   its box on its next move, and an NPC's is not clamped, so gravity can
//!   carry an NPC past its top speed.
//! - **Who resists** ([`resists_gravity`], `_ShipResistsGravity`
//!   @0x7b10-0x7bbd): a class of `Flags3` [`STELLAR_IMMUNE_HULL`]
//!   (0x0020, class +0x9ec), an inertialess ship (`_ShipIsInertialess`),
//!   and the player carrying an `oütf` of `ModType`
//!   [`GRAVITY_RESISTANCE`] (41) in any of its four mod slots, whatever
//!   its `ModVal`. An NPC's outfits never count. The original caches the
//!   player's outfit scan (`_playerResistsGravity` 0x231766, reset by
//!   `_ResetPlayerPrecalcedValues` @0xc420); the engine's stats are worked
//!   out afresh, so it needs no cache.
//! - **The afterburner** ([`GravityAfterburner`],
//!   [`RuleKey::GravityAfterburner`]): `_gravitySourcesPresent`
//!   (*0x3b5568), which `_HandleGravity` sets for each stellar of
//!   `Gravity` not none (@0x39c20-0x39c27), keeps the afterburner from
//!   widening the player's thrust top, its box and its knockback's cap to
//!   1.8 times its top speed. `_DoPlayGameWork` clears it at the head of
//!   every pass (@0x43e06-0x43e11), before `_HandlePlayer`, and
//!   `_HandleGravity` runs after the shots (@0x44509), so by the original
//!   only an advance-pass beam's knockback on the player
//!   (`_HandleBeams` @0x316a1, after the gravity) ever sees it set.
//!
//! None of these routines plays a sound.

use crate::flight::Flyer;
use crate::geometry::Vec2;
#[cfg(doc)]
use crate::rulebook::RuleKey;
use crate::rulebook::RuleSource;
use crate::trig::{accel, intercept_angle};

/// What each axis of a ship's offset from a stellar is scaled by before
/// it is squared (the double 0.01 @0xdd098, @0x7c69).
pub const DISTANCE_SCALE: f64 = 0.01;

/// The least the scaled distance's square may be (the float 30.0
/// @0xdd084, `maxss` @0x7cda): within √30 × 100 pixels the pull is
/// constant.
pub const MIN_DISTANCE_SQUARED: f32 = 30.0;

/// The `shïp` `Flags3` bit of a class that neither gravity nor a deadly
/// stellar touches (`testb $0x20` on class +0x9ec, `_ShipResistsGravity`
/// @0x7b10 and `_ShipResistsDeadlyStellars` @0x7bbe; loaded from `shïp`
/// 0x726 @0x7a20b).
pub const STELLAR_IMMUNE_HULL: u16 = 0x0020;

/// The `oütf` `ModType` of the gravity resistance: carried by the player
/// at all, whatever its `ModVal`, it keeps gravity off it (`cmpw $0x29`
/// over each of an outfit's four `ModType`s, `_ShipResistsGravity`).
pub const GRAVITY_RESISTANCE: i16 = 41;

/// The engine's speed multiplier, `_gSpeedMult` (*0x3b511c), which
/// `_HandleGravity` multiplies each `Gravity` by: 1.0 at the engine's
/// tick.
pub const SPEED_MULT: f64 = 1.0;

/// The velocity a stellar of `gravity` at `stellar` adds to a ship at
/// `ship` this pass, under the speed multiplier `speed_mult`
/// (`_ApplyGravity`, see the module docs). A negative `gravity` pushes the
/// ship away; none adds nothing.
#[must_use]
pub fn pull(ship: Vec2, stellar: Vec2, gravity: i16, speed_mult: f64) -> Vec2 {
    // `Gravity` × `_gSpeedMult` in doubles, passed on as a float.
    #[allow(clippy::cast_possible_truncation)]
    let magnitude = (f64::from(gravity) * speed_mult) as f32;
    // Each axis's difference in floats, scaled in doubles, back to floats.
    #[allow(clippy::cast_possible_truncation)]
    let scaled = |from: f32, to: f32| (f64::from(from - to) * DISTANCE_SCALE) as f32;
    let dx = scaled(ship.x, stellar.x);
    let dy = scaled(ship.y, stellar.y);
    let squared = dx.mul_add(dx, dy * dy).max(MIN_DISTANCE_SQUARED);
    accel(intercept_angle(ship, stellar), magnitude / squared)
}

/// Whether gravity leaves a ship alone (`_ShipResistsGravity`, see the
/// module docs): its class is [`STELLAR_IMMUNE_HULL`] (`immune_hull`), it
/// flies inertialess (`inertialess`), or it is the player (`flyer`)
/// carrying a [`GRAVITY_RESISTANCE`] (`outfit`). An NPC's outfit never
/// counts.
#[must_use]
pub fn resists_gravity(immune_hull: bool, inertialess: bool, outfit: bool, flyer: Flyer) -> bool {
    immune_hull || inertialess || (outfit && flyer == Flyer::Player)
}

/// Which of the afterburner's widenings a gravity source in the system
/// keeps at the plain top speed, by [`RuleKey::GravityAfterburner`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GravityAfterburner {
    /// The original's: only an advance-pass beam tick's knockback on the
    /// player, the one reader that runs after `_HandleGravity` sets
    /// `_gravitySourcesPresent` (see the module docs).
    #[default]
    BeamKnockback,
    /// Every widening: the thrust's top, the box and every knockback, as
    /// the original's flag reads where it is tested.
    Everywhere,
}

impl From<RuleSource> for GravityAfterburner {
    /// The engine's reading is the original's; the other, every widening.
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::BeamKnockback,
            RuleSource::Bible => Self::Everywhere,
        }
    }
}

/// Whether the afterburner widens the player's knockback caps: for a
/// hit, a blast or a shockwave (`shots`), and for a beam tick (`beams`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KnockbackCaps {
    /// A hit's, a blast's and the shockwave's push.
    pub shots: bool,
    /// A beam tick's push.
    pub beams: bool,
}

impl GravityAfterburner {
    /// Whether the afterburner, `on`, widens the thrust's top to 1.8 times
    /// the top speed (`_HandlePlayer` @0x6cc4d), a gravity source being
    /// present as `gravity` says.
    #[must_use]
    pub fn thrust_widens(self, on: bool, gravity: bool) -> bool {
        on && !(gravity && self == Self::Everywhere)
    }

    /// The gravity the afterburner's box write sees (`_HandlePlayer`
    /// @0x6f97e): none by the original, whose flag is always clear there;
    /// `gravity` by the other reading.
    #[must_use]
    pub fn box_gravity(self, gravity: bool) -> bool {
        gravity && self == Self::Everywhere
    }

    /// Which pushes on the player the afterburner, `on`, widens the cap
    /// of (`_DamageShip` @0x3ab19-0x3ab68), a gravity source being present
    /// as `gravity` says: a beam's only without one, by either reading;
    /// any other push whatever the gravity by the original, and only
    /// without one by the other reading.
    #[must_use]
    pub fn knockback(self, on: bool, gravity: bool) -> KnockbackCaps {
        KnockbackCaps {
            shots: self.thrust_widens(on, gravity),
            beams: on && !gravity,
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn near(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < 1e-6
    }

    #[test]
    fn a_ship_1000_east_of_a_gravity_50_stellar_is_pulled_west_by_a_half() {
        let ship = Vec2::new(1000.0, 0.0);
        let pulled = pull(ship, Vec2::ZERO, 50, SPEED_MULT);
        let expected = accel(intercept_angle(ship, Vec2::ZERO), 0.5);
        assert!(near(pulled, expected), "{pulled:?}, {expected:?}");
        assert!(pulled.x < -0.49 && pulled.y.abs() < 1e-6, "{pulled:?}");
    }

    #[test]
    fn the_pull_points_from_the_ship_toward_the_stellar_on_either_axis() {
        let at = Vec2::new(300.0, -200.0);
        let below = pull(at + Vec2::new(0.0, 2000.0), at, 40, SPEED_MULT);
        assert!(below.y < 0.0 && below.x.abs() < 1e-6, "up: {below:?}");
        let left = pull(at + Vec2::new(-2000.0, 0.0), at, 40, SPEED_MULT);
        assert!(left.x > 0.0 && left.y.abs() < 1e-6, "right: {left:?}");
    }

    #[test]
    fn halving_the_distance_quadruples_the_pull() {
        let far = pull(Vec2::new(0.0, 2000.0), Vec2::ZERO, 30, SPEED_MULT);
        let near_ = pull(Vec2::new(0.0, 1000.0), Vec2::ZERO, 30, SPEED_MULT);
        assert!((near_.length() - 4.0 * far.length()).abs() < 1e-6);
        assert!((far.length() - 30.0 / 400.0).abs() < 1e-6, "{far:?}");
    }

    #[test]
    fn within_548_pixels_the_pull_is_that_at_a_square_of_30() {
        let at = |x: f32| pull(Vec2::new(x, 0.0), Vec2::ZERO, 60, SPEED_MULT);
        assert_eq!(at(100.0).length(), at(500.0).length());
        assert!((at(500.0).length() - 2.0).abs() < 1e-6, "{:?}", at(500.0));
        assert!(at(560.0).length() < 2.0, "beyond, it falls");
        // On the diagonal too: (380, 380) squares to 28.88 once scaled.
        let diagonal = pull(Vec2::new(380.0, 380.0), Vec2::ZERO, 60, SPEED_MULT);
        assert!((diagonal.length() - 2.0).abs() < 1e-5, "{diagonal:?}");
    }

    #[test]
    fn a_negative_gravity_pushes_away() {
        let pushed = pull(Vec2::new(1000.0, 0.0), Vec2::ZERO, -50, SPEED_MULT);
        assert!(pushed.x > 0.49, "{pushed:?}");
    }

    #[test]
    fn no_gravity_adds_nothing() {
        assert_eq!(
            pull(Vec2::new(1000.0, 7.0), Vec2::ZERO, 0, SPEED_MULT),
            Vec2::ZERO
        );
    }

    #[test]
    fn the_speed_multiplier_scales_the_pull() {
        let ship = Vec2::new(0.0, 3000.0);
        let plain = pull(ship, Vec2::ZERO, 90, 1.0);
        let doubled = pull(ship, Vec2::ZERO, 90, 2.0);
        assert!(near(doubled, plain * 2.0), "{doubled:?}, {plain:?}");
    }

    #[test]
    fn the_offset_is_scaled_as_a_float_difference() {
        // 0.01 of the difference, not of each place: a ship and a stellar
        // far from the centre pull as two near it.
        let a = pull(Vec2::new(11000.0, 0.0), Vec2::new(10000.0, 0.0), 50, 1.0);
        let b = pull(Vec2::new(1000.0, 0.0), Vec2::ZERO, 50, 1.0);
        assert!(near(a, b), "{a:?}, {b:?}");
    }

    #[test]
    fn a_stellar_immune_hull_or_an_inertialess_ship_resists_and_the_outfit_only_the_players() {
        for flyer in [Flyer::Player, Flyer::Npc] {
            assert!(resists_gravity(true, false, false, flyer));
            assert!(resists_gravity(false, true, false, flyer));
            assert!(!resists_gravity(false, false, false, flyer));
        }
        assert!(resists_gravity(false, false, true, Flyer::Player));
        assert!(!resists_gravity(false, false, true, Flyer::Npc));
        assert_eq!((STELLAR_IMMUNE_HULL, GRAVITY_RESISTANCE), (0x0020, 41));
    }

    #[test]
    fn the_rules_reading_follows_its_source() {
        assert_eq!(
            GravityAfterburner::from(RuleSource::Engine),
            GravityAfterburner::BeamKnockback
        );
        assert_eq!(
            GravityAfterburner::from(RuleSource::Bible),
            GravityAfterburner::Everywhere
        );
        assert_eq!(
            GravityAfterburner::default(),
            GravityAfterburner::BeamKnockback
        );
    }

    #[test]
    fn by_the_original_gravity_keeps_only_the_beams_cap_plain() {
        let rule = GravityAfterburner::BeamKnockback;
        assert!(rule.thrust_widens(true, true));
        assert!(rule.thrust_widens(true, false));
        assert!(!rule.thrust_widens(false, false));
        assert!(!rule.box_gravity(true));
        assert!(!rule.box_gravity(false));
        let caps = |on, gravity| rule.knockback(on, gravity);
        assert_eq!(
            caps(true, true),
            KnockbackCaps {
                shots: true,
                beams: false
            }
        );
        assert_eq!(
            caps(true, false),
            KnockbackCaps {
                shots: true,
                beams: true
            }
        );
        assert_eq!(caps(false, false), KnockbackCaps::default());
    }

    #[test]
    fn by_the_other_reading_gravity_keeps_every_widening_plain() {
        let rule = GravityAfterburner::Everywhere;
        assert!(!rule.thrust_widens(true, true));
        assert!(rule.thrust_widens(true, false));
        assert!(rule.box_gravity(true));
        assert!(!rule.box_gravity(false));
        assert_eq!(rule.knockback(true, true), KnockbackCaps::default());
        assert_eq!(
            rule.knockback(true, false),
            KnockbackCaps {
                shots: true,
                beams: true
            }
        );
        assert_eq!(rule.knockback(false, true), KnockbackCaps::default());
    }
}
