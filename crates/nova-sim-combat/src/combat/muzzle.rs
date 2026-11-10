//! Where a shot starts, as the rulebook's
//! [`RuleKey::MountedMuzzles`](crate::RuleKey::MountedMuzzles) says
//! ([`MuzzleReading`], [`offset`]).
//!
//! - By the engine ([`MuzzleReading::AtExit`]), a shot starts at its exit
//!   point, placed as `_ModifyShotStartPosition` @0x7348-0x74d2 places it
//!   ([`exits`](super::exits)): turned with the hull, compressed for the
//!   tilted view, raised by its z.
//! - By the other reading ([`MuzzleReading::Ahead`]), the new game's, a
//!   shot whose weapon has a muzzle reach
//!   ([`WeaponSpec::muzzle_reach`]) starts that many pixels further on,
//!   ahead of the exit along the way its outfit points: the shot's aim
//!   for a weapon that turns to aim (a turret, a quadrant turret or point
//!   defence), the hull's angle for every other. The reach is added after
//!   the exit's compression and z, unscaled: the new game's view is seen
//!   from straight above.
//!
//! Only the start moves. The exit taken (the cursor's, or the nearest to
//! the target) and the shot's heading, lead and inaccuracy are the
//! engine's under both readings, aimed from the exit, the mount: aiming
//! from the muzzle would be circular, since where the muzzle is depends on
//! the aim. A beam keeps starting at its exit every tick, and a fighter at
//! its carrier; the new game maps neither. A weapon without a reach,
//! every stock weapon among them, starts at its exit under both, so
//! turning the key on over stock data changes nothing.
//!
//! The reach is the weapon's, not the hardpoint's: as in the original, a
//! shot of a weapon leaves from any exit of its kind, whichever outfit is
//! drawn there. Where a hull carries one weapon of a kind, as the sample's
//! do, the two agree. The new game's loader keeps the reach of each
//! weapon's outfit model (`nova_data::new_game::mounts`), and the view
//! draws the same outfit at its mount with the same reach to its muzzle,
//! in metres where the simulation rounds to whole pixels.
//!
//! The fight holds the reading ([`Combat::set_hull_rules`]) and places a
//! shot by [`offset`] in one place, its launch ([`Combat::muzzle_start`]).
//! It plays no sound.

use crate::flight::{ShipState, facing};
use crate::geometry::Vec2;
use crate::rulebook::RuleSource;

use super::Combat;
use super::armament::Launch;
use super::weapon::{Guidance, WeaponSpec};

/// Where a shot starts (see the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MuzzleReading {
    /// The original's: at its exit point.
    #[default]
    AtExit,
    /// The new game's: its weapon's muzzle reach ahead of its exit, where
    /// it has one.
    Ahead,
}

impl From<RuleSource> for MuzzleReading {
    fn from(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::AtExit,
            RuleSource::Bible => Self::Ahead,
        }
    }
}

/// How far a shot of `weapon` starts from its exit point under
/// `reading`, its hull at `hull_angle` degrees and the shot aimed along
/// `aim()` degrees from the exit, before its inaccuracy (see the module
/// docs): nothing at the exit; the weapon's reach along the aim for one
/// that turns to aim, along the hull's angle for any other; nothing for a
/// weapon with no reach. The aim is worked out only when it is read.
#[must_use]
pub fn offset(
    reading: MuzzleReading,
    weapon: &WeaponSpec,
    hull_angle: f32,
    aim: impl FnOnce() -> f32,
) -> Vec2 {
    let Some(reach) = weapon
        .muzzle_reach
        .filter(|_| reading == MuzzleReading::Ahead)
    else {
        return Vec2::ZERO;
    };
    let along = if turns_to_aim(weapon.guidance) {
        aim()
    } else {
        hull_angle
    };
    facing(along) * reach
}

impl Combat {
    /// Where a shot of `launch` starts, leaving its exit as `leaving` with
    /// its hull at `hull_angle` degrees: moved by [`offset`] under the
    /// fight's reading, aimed from the exit. It is the one place the fight
    /// reads the reading (`Combat::launch`).
    pub(super) fn muzzle_start(
        &self,
        launch: &Launch,
        hull_angle: f32,
        leaving: &ShipState,
    ) -> ShipState {
        let aim = || f32::from(launch.pointing.whole_from(leaving));
        ShipState {
            position: leaving.position
                + offset(self.mounted_muzzles, &launch.weapon, hull_angle, aim),
            ..*leaving
        }
    }
}

/// Whether a weapon of `guidance` turns to aim, so its outfit points
/// where its shot is aimed: a turret, a quadrant turret or point defence.
fn turns_to_aim(guidance: Guidance) -> bool {
    matches!(
        guidance,
        Guidance::Turret | Guidance::FrontTurret | Guidance::RearTurret | Guidance::PointDefence
    )
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_sim_catalog::fixture::weapon;

    use super::*;
    use crate::catalog::WeaponRecord;

    /// A weapon of raw `guidance` reaching `reach` pixels.
    fn of(guidance: i16, reach: Option<i16>) -> WeaponSpec {
        WeaponSpec::new(&WeaponRecord {
            guidance,
            muzzle_reach: reach,
            ..weapon(128)
        })
    }

    #[test]
    fn the_engine_reading_starts_at_the_exit_and_is_the_default() {
        assert_eq!(MuzzleReading::default(), MuzzleReading::AtExit);
        assert_eq!(
            MuzzleReading::from(RuleSource::Engine),
            MuzzleReading::AtExit
        );
        assert_eq!(MuzzleReading::from(RuleSource::Bible), MuzzleReading::Ahead);
    }

    #[test]
    fn at_the_exit_a_shot_moves_nowhere_whatever_its_reach() {
        for guidance in [-1, 4, 9] {
            let turret = of(guidance, Some(5));
            assert_eq!(
                offset(MuzzleReading::AtExit, &turret, 30.0, || 120.0),
                Vec2::ZERO
            );
        }
    }

    #[test]
    fn ahead_a_weapon_with_no_reach_moves_nowhere() {
        for reach in [None, Some(0), Some(-3)] {
            let gun = of(-1, reach);
            assert_eq!(
                offset(MuzzleReading::Ahead, &gun, 30.0, || 120.0),
                Vec2::ZERO
            );
        }
    }

    #[test]
    fn ahead_a_weapon_that_turns_to_aim_reaches_along_its_aim() {
        // Turret, front and rear quadrant turrets, point defence.
        for guidance in [4, 7, 8, 9] {
            let turret = of(guidance, Some(5));
            assert_eq!(
                offset(MuzzleReading::Ahead, &turret, 30.0, || 120.0),
                facing(120.0) * 5.0,
                "{guidance}"
            );
        }
    }

    #[test]
    fn the_aim_is_worked_out_only_for_a_weapon_that_turns_to_aim_ahead() {
        let unread = || -> f32 { panic!("the aim is not read") };
        let gun = of(-1, Some(5));
        let turret = of(4, Some(5));
        let bare = of(4, None);
        let _ = offset(MuzzleReading::Ahead, &gun, 30.0, unread);
        let _ = offset(MuzzleReading::AtExit, &turret, 30.0, unread);
        let _ = offset(MuzzleReading::Ahead, &bare, 30.0, unread);
    }

    #[test]
    fn ahead_any_other_weapon_reaches_along_the_hull_s_angle() {
        // Unguided, homing, a bomb, a rocket.
        for guidance in [-1, 1, 5, 6] {
            let gun = of(guidance, Some(5));
            assert_eq!(
                offset(MuzzleReading::Ahead, &gun, 30.0, || 120.0),
                facing(30.0) * 5.0,
                "{guidance}"
            );
        }
    }
}
