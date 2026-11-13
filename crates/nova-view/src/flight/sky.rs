//! The flight's layered sky ([`DrawCommand::Sky`](crate::DrawCommand::Sky)),
//! drawn under [`RuleKey::LayeredStarfield`]'s other reading in place of
//! the original's starfield ([`crate::flight::stars`]).
//!
//! [`sky_pose`] is where the flight's sky is seen from: the system's own
//! sky (its seed is the system's ID, so each system shows the same sky on
//! every visit and another than its neighbours'), the eye over the point
//! the camera centres on, drawn at the camera's anchor, and dimmed to the
//! level the original's stars take in the system's murk
//! ([`murk_level`]): a system of negative murk shows none, as the
//! original's shows no stars. The murk is read each frame rather than
//! latched at a scattering; murk does not change within a system, so the
//! two agree.
//!
//! Through a jump's warp the pose carries the rush the view gives it
//! ([`SkyPose::rush`]), the eye's move over the step, so the renderer
//! streaks the stars by depth; out of the warp it is zero. The view also
//! sets the pose's clock ([`SkyPose::time`]) to the flight's elapsed
//! time, the one its planets turn by.
//!
//! What the sky looks like (its layers, depths, density and tints) is the
//! renderer's (`nova_render::sky`). A system's own look, set by its data
//! (its star density, tint and distant backdrop), is the renderer's too:
//! it looks the look up by the pose's seed, which [`sky_seed`] gives, so
//! the pose carries no look.
//!
//! [`RuleKey::LayeredStarfield`]: nova_sim::RuleKey::LayeredStarfield

use nova_sim::SystemId;

use crate::flight::stars::murk_level;
use crate::geometry::Point;
use crate::sky::SkyPose;

/// The sky the flight shows in `system` of `Murk` `murk` as loaded, the
/// eye over world point `eye`, drawn at `anchor`, rushing by `rush` (the
/// eye's move over the last step through a jump's warp, zero otherwise);
/// `None` in a system of negative murk. It is at time zero: the view
/// that draws it sets its clock ([`SkyPose::with_time`]).
#[must_use]
pub fn sky_pose(
    system: SystemId,
    eye: Point,
    anchor: Point,
    rush: Point,
    murk: i16,
) -> Option<SkyPose> {
    (murk >= 0)
        .then(|| SkyPose::new(sky_seed(system), eye, anchor, murk_level(murk)).with_rush(rush))
}

/// The seed of `system`'s sky ([`SkyPose::seed`]): its ID, so each system
/// shows the same sky on every visit and another than its neighbours'.
/// The renderer keys a system's own sky look by it.
#[must_use]
pub fn sky_seed(system: SystemId) -> u64 {
    u64::from(system.0.cast_unsigned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flight::stars::FULL_LEVEL;

    fn pose(system: i16, murk: i16) -> Option<SkyPose> {
        sky_pose(
            SystemId(system),
            Point::new(10.0, -20.0),
            Point::new(400.0, 384.0),
            Point::new(0.0, 0.0),
            murk,
        )
    }

    #[test]
    fn a_system_of_negative_murk_shows_no_sky() {
        assert_eq!(pose(128, -1), None);
        assert_eq!(pose(128, i16::MIN), None);
        let rushing = |murk| {
            sky_pose(
                SystemId(128),
                Point::new(0.0, 0.0),
                Point::new(0.0, 0.0),
                Point::new(25.0, -9.0),
                murk,
            )
        };
        assert_eq!(rushing(-1), None, "not even rushing");
    }

    #[test]
    fn the_rush_given_comes_back_on_the_pose() {
        let rush = Point::new(25.0, -9.0);
        let sky = sky_pose(
            SystemId(128),
            Point::new(10.0, -20.0),
            Point::new(400.0, 384.0),
            rush,
            0,
        );
        assert_eq!(sky, pose(128, 0).map(|sky| sky.with_rush(rush)));
        assert_eq!(sky.map(|sky| sky.rush), Some(rush));
    }

    #[test]
    fn a_clear_system_shows_its_sky_at_full_level_where_the_camera_is() {
        assert_eq!(
            pose(128, 0),
            Some(SkyPose::new(
                128,
                Point::new(10.0, -20.0),
                Point::new(400.0, 384.0),
                FULL_LEVEL,
            ))
        );
    }

    #[test]
    fn a_murky_systems_sky_is_dimmed_as_its_stars_are() {
        for murk in [1, 8, 20, 40, i16::MAX] {
            let sky = pose(128, murk).expect("a sky");
            assert_eq!(sky.level, murk_level(murk), "{murk}");
        }
        assert_eq!(pose(128, 8).map(|sky| sky.level), Some(7));
    }

    #[test]
    fn a_systems_sky_is_seeded_by_its_seed_rule() {
        for system in [128, 129, 1000, -2, 0, i16::MAX, i16::MIN] {
            let pose = pose(system, 0).expect("a sky");
            assert_eq!(pose.seed, sky_seed(SystemId(system)), "{system}");
        }
        assert_eq!(sky_seed(SystemId(128)), 128);
        assert_ne!(sky_seed(SystemId(-1)), sky_seed(SystemId(1)));
    }

    #[test]
    fn each_system_has_its_own_sky() {
        let seed = |system| pose(system, 0).expect("a sky").seed;
        assert_eq!(seed(128), seed(128));
        assert_ne!(seed(128), seed(129));
        assert_ne!(seed(1000), seed(-2));
    }
}
