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
//! What the sky looks like (its layers, depths, density and tints) is the
//! renderer's (`nova_render::sky`).
//!
//! [`RuleKey::LayeredStarfield`]: nova_sim::RuleKey::LayeredStarfield

use nova_sim::SystemId;

use crate::flight::stars::murk_level;
use crate::geometry::Point;
use crate::sky::SkyPose;

/// The sky the flight shows in `system` of `Murk` `murk` as loaded, the
/// eye over world point `eye`, drawn at `anchor`; `None` in a system of
/// negative murk.
#[must_use]
pub fn sky_pose(system: SystemId, eye: Point, anchor: Point, murk: i16) -> Option<SkyPose> {
    (murk >= 0).then(|| SkyPose {
        seed: u64::from(system.0.cast_unsigned()),
        eye,
        anchor,
        level: murk_level(murk),
    })
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
            murk,
        )
    }

    #[test]
    fn a_system_of_negative_murk_shows_no_sky() {
        assert_eq!(pose(128, -1), None);
        assert_eq!(pose(128, i16::MIN), None);
    }

    #[test]
    fn a_clear_system_shows_its_sky_at_full_level_where_the_camera_is() {
        assert_eq!(
            pose(128, 0),
            Some(SkyPose {
                seed: 128,
                eye: Point::new(10.0, -20.0),
                anchor: Point::new(400.0, 384.0),
                level: FULL_LEVEL,
            })
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
    fn each_system_has_its_own_sky() {
        let seed = |system| pose(system, 0).expect("a sky").seed;
        assert_eq!(seed(128), seed(128));
        assert_ne!(seed(128), seed(129));
        assert_ne!(seed(1000), seed(-2));
    }
}
