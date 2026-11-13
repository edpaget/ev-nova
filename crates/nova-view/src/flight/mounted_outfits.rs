//! A ship's outfits drawn on its hardpoints: [`mounted_outfits`], the
//! draw commands for a ship's 3D model and each outfit model its mounts
//! show.
//!
//! Where each mount stands, and what it shows, the loader works out from
//! the data and keeps on the store (`nova_data::store::GameData::mounts`,
//! a ray cast down at each hardpoint onto the hull's model). This module
//! only places them: the hull's [`DrawCommand::Model`] first, then a
//! [`DrawCommand::Mounted`] for each mount that shows an outfit with a
//! model key, in the mounts' order, so the renderer draws each on the
//! hull and in its depth. Emitting the hull and its parts together keeps
//! a part with its hull.
//!
//! - **Where**: the mount's point raised by its outfit's lift, so the
//!   outfit's base rests on the hull, mapped from ship space to the view's
//!   model axes by [`view_axes`], the map the hull's mesh takes.
//! - **Turned**: a fixed mount's outfit faces the hull's nose. A turret's
//!   ([`Mount::turns`]) faces the world heading the caller gives it, so it
//!   turns on the hull by that heading less the hull's angle; given none,
//!   it faces the nose.
//!
//! The caller supplies what this module can't know: the hull's model key
//! and pose (its orientation from [`ship_orientation`]), its hull angle
//! (`FrameSpec::hull_angle`, the angle that orientation turns by), each
//! turret's facing, and each outfit asset's model key. The flight screen
//! calls it for each ship it draws as a model (its view's `ship_models`
//! module), with the keys the edge registered
//! ([`WorldModels`](super::world_models::WorldModels)) and, for every
//! turning mount, the ship's turret facing as the frame the sim port
//! fills carries it
//! ([`FrameShip::turret_facing`](nova_sim_frame::flight_frame::FrameShip::turret_facing)),
//! swung toward at a rate: the view's eased facing
//! ([`TurretSwings::facing`](super::turret_swing::TurretSwings::facing)),
//! the readout itself once a swing has settled.
//!
//! A ship's mounts are its class's and its kits' hardpoints, each showing
//! the outfit the data puts on it, the ship as built. A weapon the player
//! buys loose sits on no hardpoint, since nothing in the data assigns it
//! one, so nothing draws it.
//!
//! [`ship_orientation`]: super::ship_model::ship_orientation

use nova_data::store::Mount;

use crate::draw::DrawCommand;
use crate::model::{ModelKey, ModelPose, Mounting, view_axes};

/// The commands that draw a ship's model `hull` at `pose`, its hull at
/// `hull_angle` degrees, and its `mounts`' outfits on it (see the module
/// docs). `facings` gives each mount's facing by its index, a world
/// heading in degrees as the simulation's are; a turret with none, past
/// the end or `None`, faces the nose. `key_of` gives an outfit asset's
/// model key; one with none isn't drawn.
#[must_use]
pub fn mounted_outfits(
    hull: ModelKey,
    pose: ModelPose,
    hull_angle: f32,
    mounts: &[Mount],
    facings: &[Option<f32>],
    key_of: impl Fn(&str) -> Option<ModelKey>,
) -> Vec<DrawCommand> {
    let parts = mounts.iter().enumerate().filter_map(|(i, mount)| {
        let shown = mount.shows.as_ref()?;
        let model = key_of(&shown.asset)?;
        let [x, y, z] = mount.at;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a mount's place in metres is well inside f32"
        )]
        let at = view_axes([x as f32, y as f32, (z + shown.lift) as f32]);
        let facing = facings.get(i).copied().flatten();
        let turn = match facing {
            Some(heading) if mount.turns => (heading - hull_angle).to_radians(),
            _ => 0.0,
        };
        Some(DrawCommand::Mounted {
            model,
            mounting: Mounting { at, turn },
        })
    });
    std::iter::once(DrawCommand::model(hull, pose))
        .chain(parts)
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_data::store::{Footing, Shown};
    use nova_sim_frame::combat::hull_turn::HullTurn;
    use nova_sim_frame::combat::ship_frames::FrameSpec;

    use super::*;
    use crate::flight::ship_model::ship_orientation;
    use crate::geometry::Point;

    const HULL: ModelKey = ModelKey(1);
    const TURRET: ModelKey = ModelKey(2);
    const GUN: ModelKey = ModelKey(3);

    fn key_of(asset: &str) -> Option<ModelKey> {
        match asset {
            "turret-mount" => Some(TURRET),
            "gun" => Some(GUN),
            _ => None,
        }
    }

    fn mount(hardpoint: &str, at: [f64; 3], turns: bool, shows: Option<(&str, f64)>) -> Mount {
        Mount {
            hardpoint: hardpoint.to_owned(),
            at,
            footing: Footing::Surface,
            turns,
            shows: shows.map(|(asset, lift)| Shown {
                asset: asset.to_owned(),
                lift,
            }),
        }
    }

    fn pose(heading: f32) -> ModelPose {
        ModelPose {
            center: Point::new(100.0, 80.0),
            orientation: ship_orientation(&FrameSpec::NONE, heading, HullTurn::Continuous),
            scale: 2.0,
        }
    }

    fn mountings(commands: &[DrawCommand]) -> Vec<(ModelKey, Mounting)> {
        commands
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::Mounted { model, mounting } => Some((model, mounting)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_hull_comes_first_then_each_shown_outfit_in_mount_order() {
        let mounts = [
            mount("nose", [0.0, 3.0, 1.0], false, Some(("gun", 0.25))),
            mount("bed", [0.0, -2.0, 6.0], true, Some(("turret-mount", 0.3))),
        ];
        let drawn = mounted_outfits(HULL, pose(30.0), 30.0, &mounts, &[], key_of);
        assert_eq!(drawn[0], DrawCommand::model(HULL, pose(30.0)));
        let keys: Vec<ModelKey> = mountings(&drawn).iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, [GUN, TURRET]);
        assert_eq!(drawn.len(), 3);
    }

    #[test]
    fn a_part_stands_at_its_mount_raised_by_its_lift_on_the_view_s_axes() {
        let mounts = [mount(
            "bed",
            [0.5, -2.0, 6.0],
            true,
            Some(("turret-mount", 0.3)),
        )];
        let drawn = mounted_outfits(HULL, pose(0.0), 0.0, &mounts, &[], key_of);
        let [(_, mounting)] = mountings(&drawn)[..] else {
            panic!("{drawn:?}");
        };
        assert_eq!(mounting.at, view_axes([0.5, -2.0, 6.3]));
    }

    #[test]
    fn a_fixed_mount_faces_the_nose_whatever_facing_it_is_given() {
        let mounts = [mount("nose", [0.0, 3.0, 1.0], false, Some(("gun", 0.25)))];
        for facing in [None, Some(0.0), Some(90.0), Some(271.5)] {
            let drawn = mounted_outfits(HULL, pose(40.0), 40.0, &mounts, &[facing], key_of);
            assert_eq!(mountings(&drawn)[0].1.turn, 0.0, "{facing:?}");
        }
    }

    #[test]
    fn a_turret_turns_by_its_facing_less_the_hull_s_angle_under_either_reading() {
        let mounts = [mount(
            "bed",
            [0.0, -2.0, 6.0],
            true,
            Some(("turret-mount", 0.3)),
        )];
        let spec = FrameSpec::NONE;
        for turn in [HullTurn::ByFrame, HullTurn::Continuous] {
            for (heading, facing) in [(0.0, 90.0), (12.5, 200.0), (97.3, 45.0)] {
                let angle = spec.hull_angle(heading, turn);
                let drawn =
                    mounted_outfits(HULL, pose(heading), angle, &mounts, &[Some(facing)], key_of);
                assert_eq!(
                    mountings(&drawn)[0].1.turn,
                    (facing - angle).to_radians(),
                    "{turn:?} {heading} {facing}"
                );
            }
        }
    }

    #[test]
    fn a_turret_with_no_facing_faces_the_nose() {
        let mounts = [
            mount("bed", [0.0, -2.0, 6.0], true, Some(("turret-mount", 0.3))),
            mount("ring", [0.0, 2.0, 6.0], true, Some(("turret-mount", 0.3))),
        ];
        // The first given none, the second past the end of the facings.
        let drawn = mounted_outfits(HULL, pose(40.0), 40.0, &mounts, &[None], key_of);
        let turns: Vec<f32> = mountings(&drawn).iter().map(|(_, m)| m.turn).collect();
        assert_eq!(turns, [0.0, 0.0]);
    }

    #[test]
    fn a_mount_with_no_outfit_or_an_asset_with_no_key_draws_nothing() {
        let mounts = [
            mount("empty", [0.0, 0.0, 1.0], true, None),
            mount("odd", [0.0, 1.0, 1.0], false, Some(("unknown", 0.1))),
            mount("bed", [0.0, -2.0, 6.0], true, Some(("turret-mount", 0.3))),
        ];
        let facings = [Some(10.0), Some(20.0), Some(130.0)];
        let drawn = mounted_outfits(HULL, pose(40.0), 40.0, &mounts, &facings, key_of);
        let parts = mountings(&drawn);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].0, TURRET);
        // Its own facing, by its own index.
        assert_eq!(parts[0].1.turn, (130.0_f32 - 40.0).to_radians());
    }

    #[test]
    fn a_hull_with_no_mounts_draws_its_model_alone() {
        let drawn = mounted_outfits(HULL, pose(40.0), 40.0, &[], &[], key_of);
        assert_eq!(drawn, [DrawCommand::model(HULL, pose(40.0))]);
    }
}
