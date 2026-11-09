//! Exit points from hardpoints: the new game's ships fire from the
//! hardpoints their class and their mod kits place, not from four fixed
//! points of a `shän` alone.
//!
//! A [`Spot`] is a hardpoint where shots of one kind leave from, in the
//! `shän`'s pixels (+x right, +y ahead, the ship facing up). The new
//! game's store keeps each class's own spots and each kit's
//! (`nova_sim_catalog::fittings`). [`fitted_exits`] lays a ship's spots
//! over its class's raw exit points: each kind with spots takes the four
//! points [`Spot::exit_points`] makes of them, the one rule the new
//! game's `shän`s are made by too, at z 0; a kind with none keeps the
//! class's points. With no spots at all the class's points stand as they
//! are, so the stock game, which has no fittings, is untouched.
//!
//! The session flies the player with its class's spots and those of the
//! kits it carries ([`HullSpec::with_spots`]), so a kit bought at a yard
//! adds its mount at once; an NPC flies its class's `shän`, which the new
//! game's mapping already fills from the class's chain and its kits. The
//! original has no such hardpoints: this is the new game's alone, and it
//! plays no sound.

use crate::catalog::ShipExits;
use nova_sim_catalog::fittings::{Spot, SpotKind};

use super::exits::ExitPoints;
use super::hull::HullSpec;

/// The `shän` set a spot of `kind` fills: gun 0, turret 1, guided 2.
fn set(kind: SpotKind) -> usize {
    match kind {
        SpotKind::Gun => 0,
        SpotKind::Turret => 1,
        SpotKind::Guided => 2,
    }
}

/// The exit points a ship of a class whose raw points are `raw` flies
/// with, carrying `spots` (see the module docs): `raw` as it is with no
/// spots; else `raw`, or a class with none's centred points (36 frames,
/// no compression), with each kind that has spots cycled through them.
#[must_use]
pub fn fitted_exits(raw: Option<&ShipExits>, spots: &[Spot]) -> Option<ShipExits> {
    if spots.is_empty() {
        return raw.copied();
    }
    let mut exits = raw.copied().unwrap_or_default();
    for kind in [SpotKind::Gun, SpotKind::Turret, SpotKind::Guided] {
        if let Some((points, _)) = Spot::exit_points(spots, kind) {
            let set = set(kind);
            exits.points[set] = points;
            exits.z[set] = [0; Spot::EXIT_POINTS];
        }
    }
    Some(exits)
}

impl HullSpec {
    /// This hull, of a class whose raw exit points are `raw`, flying with
    /// `spots` ([`fitted_exits`]): unchanged with none, so its points
    /// stay its class's; else its exit points rebuilt from them, turning
    /// as its class's frames do.
    #[must_use]
    pub fn with_spots(mut self, raw: Option<&ShipExits>, spots: &[Spot]) -> Self {
        if spots.is_empty() {
            return self;
        }
        self.exits = fitted_exits(raw, spots)
            .as_ref()
            .and_then(ExitPoints::new)
            .map(|exits| exits.with_angle_table(self.frames.table.is_some()));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spot(kind: SpotKind, at: (i16, i16)) -> Spot {
        Spot { kind, at }
    }

    /// A class's raw points: each gun point at (1, n), each turret point
    /// at (2, n), each guided at (3, n), each beam at (4, n), z 7.
    fn raw() -> ShipExits {
        let mut exits = ShipExits {
            up_compress: (100, 100),
            down_compress: (100, 100),
            frames: 36,
            ..ShipExits::default()
        };
        for set in 0..4 {
            for n in 0..4 {
                exits.points[set][n] = (set as i16 + 1, n as i16);
                exits.z[set][n] = 7;
            }
        }
        exits
    }

    #[test]
    fn no_spots_keep_the_class_s_points_and_none_stays_none() {
        assert_eq!(fitted_exits(Some(&raw()), &[]), Some(raw()));
        assert_eq!(fitted_exits(None, &[]), None);
    }

    #[test]
    fn one_turret_spot_fills_the_four_turret_points_and_leaves_the_rest() {
        let fitted = fitted_exits(Some(&raw()), &[spot(SpotKind::Turret, (0, -2))]).unwrap();
        assert_eq!(fitted.points[1], [(0, -2); 4]);
        assert_eq!(fitted.z[1], [0; 4]);
        assert_eq!(fitted.points[0], raw().points[0], "the gun points");
        assert_eq!(fitted.points[2], raw().points[2], "the guided points");
        assert_eq!(fitted.points[3], raw().points[3], "the beam points");
        assert_eq!(fitted.z[0], [7; 4]);
        assert_eq!(fitted.frames, 36);
    }

    #[test]
    fn each_kind_takes_the_points_its_spots_make_in_its_own_set() {
        let spots = [
            spot(SpotKind::Gun, (-3, 5)),
            spot(SpotKind::Guided, (0, -9)),
            spot(SpotKind::Gun, (3, 5)),
            spot(SpotKind::Turret, (0, 8)),
        ];
        let fitted = fitted_exits(Some(&raw()), &spots).unwrap();
        for (set, kind) in [SpotKind::Gun, SpotKind::Turret, SpotKind::Guided]
            .into_iter()
            .enumerate()
        {
            let (points, _) = Spot::exit_points(&spots, kind).unwrap();
            assert_eq!(fitted.points[set], points, "{kind:?}");
            assert_eq!(fitted.z[set], [0; 4], "{kind:?}");
        }
        assert_eq!(fitted.points[3], raw().points[3], "the beam points");
        assert_eq!(fitted.z[3], [7; 4]);
    }

    #[test]
    fn a_class_with_no_points_flies_its_spots_unsquashed() {
        let fitted = fitted_exits(None, &[spot(SpotKind::Turret, (0, -2))]).unwrap();
        assert_eq!(fitted.points[1], [(0, -2); 4]);
        assert_eq!(fitted.points[0], [(0, 0); 4]);
        assert_eq!(fitted.frames, 0, "36 frames, as a `FramesPer` of 0 reads");
        assert!(ExitPoints::new(&fitted).is_some());
    }

    #[test]
    fn a_hull_with_spots_flies_them_and_one_without_keeps_its_own() {
        let hull = HullSpec {
            exits: ExitPoints::new(&raw()),
            ..HullSpec::default()
        };
        assert_eq!(hull.with_spots(Some(&raw()), &[]), hull);
        let fitted = hull.with_spots(Some(&raw()), &[spot(SpotKind::Turret, (0, -2))]);
        let expected = fitted_exits(Some(&raw()), &[spot(SpotKind::Turret, (0, -2))]);
        assert_eq!(fitted.exits, expected.as_ref().and_then(ExitPoints::new));
        assert_ne!(fitted.exits, hull.exits);
    }
}
