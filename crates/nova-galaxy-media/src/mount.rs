//! Where a hardpoint's outfit sits on its hull, and how an outfit's model
//! sits on its mount: the geometry behind asset decision 2 of game-bible's
//! data format, "a hardpoint has no height: a mount sits on top of the
//! hull".
//!
//! A hardpoint gives only `[x, y]`, metres from the middle of the ship
//! seen from above. [`mount_point`] casts a ray straight down at that
//! point onto the hull's model, placed as the renderer places it
//! ([`Placement::try_from_bounds`] at the ship's length), and stands the
//! mount on the highest surface it hits ([`surface`]). So the surface found
//! is the surface drawn.
//!
//! - A face counts when its corners are all finite and its shape seen from
//!   above has area. A wall seen edge-on, or a collapsed triangle, can't be
//!   stood on; a wall's top edge is shared with the faces it meets.
//! - A point on an edge or a corner of a face counts as inside it, with a
//!   small tolerance relative to the face's size, so a point on the
//!   diagonal two triangles of a quad share finds the quad.
//! - Faces facing down are hit as well, but the highest hit wins, so which
//!   way a face's normal points doesn't matter.
//!
//! A mount's [`Footing`] says what it stands on: a surface; the plane, when
//! the hull has no model to stand on; or nothing, when the hull has a
//! model but no face lies under the point. The last two both stand at
//! height 0, the ship's plane; the loader reports the last.
//!
//! An outfit's own model sits on its mount by [`fitting`]: at a metre a
//! model unit whatever the hull, as the renderer loads an outfit's model
//! (`nova_render::ModelRequest` with no length). It is placed by its
//! footprint's middle, facing its `front`, like every model, so it is
//! raised by half its height to rest on the mount, it turns about its
//! footprint's middle, and its muzzle is the middle of its footprint's front
//! edge, half its length ahead. The format gives an outfit no size, no
//! muzzle and no pivot; these are this module's choices, kept in
//! [`fitting`] alone, so a later data field changes one function.
//!
//! Every height and point here is in 3D ship space (`[starboard, nose,
//! up]` in metres, up from the hull model's middle in height), as
//! [`Placement::ship_point_3d`] gives it.

use nova_galaxy_format::asset::Front;

use crate::model::{Bounds, Face, Model};
use crate::placement::Placement;

/// How far outside a face a point may be, in barycentric weight, and
/// still count as on it: a point on an edge is inside after rounding.
const ON_EDGE: f64 = 1e-9;

/// The least area, as a fraction of a face's longest side squared, that a
/// face seen from above must have to be stood on. A model's corners are
/// `f32`, so an upright face can lean off the vertical by its rounding,
/// about 1e-8 of its size; this is well above that, and well below any
/// face drawn to be stood on.
const LEAST_AREA: f64 = 1e-6;

/// What a mount stands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footing {
    /// The highest surface of the hull's model under the point.
    Surface,
    /// The ship's plane, height 0: the hull has no model to stand on (a
    /// placeholder, a model that doesn't read, an LFS pointer, or one that
    /// can't be scaled to the ship's length).
    Plane,
    /// The ship's plane, height 0, though the hull has a model: no face
    /// lies under the point (it is outside the model's footprint, or isn't
    /// finite).
    Off,
}

/// Where a mount stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Standing {
    /// The point, in 3D ship space (`[starboard, nose, up]` in metres):
    /// the hardpoint's `[x, y]` as given, at the height its footing gives.
    pub at: [f64; 3],
    /// What it stands on.
    pub footing: Footing,
}

/// How an outfit's model sits on its mount, in metres at a metre a model
/// unit (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fitting {
    /// How far its middle stands above the mount: half its height, so its
    /// base rests on the mount.
    pub lift: f64,
    /// How far its muzzle is ahead of its pivot, the middle of its
    /// footprint: half its length along its `front`.
    pub reach: f64,
}

/// The height of the highest face of `model`, placed by `placement`, under
/// the ship-space point `point`, `[x, y]`; `None` when no face lies under it, or
/// the point isn't finite (see the module docs).
///
/// A point that isn't finite is under no face without a check of its own:
/// a face that can be stood on has area, so at least one of the point's
/// weights on it is then infinite or NaN, and no such weight counts as
/// inside.
#[must_use]
pub fn surface(model: &Model, placement: &Placement, point: [f64; 2]) -> Option<f64> {
    model
        .faces
        .iter()
        .filter_map(|face| height_on(face, placement, point))
        .reduce(f64::max)
}

/// The height of `face`, placed by `placement`, over `point`; `None` when
/// the point isn't over it, or it can't be stood on.
fn height_on(face: &Face, placement: &Placement, point: [f64; 2]) -> Option<f64> {
    let [a, b, c] = face.points.map(|p| placement.ship_point_3d(p));
    if ![a, b, c].iter().flatten().all(|v| v.is_finite()) {
        return None;
    }
    let (ab, ac) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
    let area = ab[0] * ac[1] - ab[1] * ac[0];
    let side = |p: [f64; 3], q: [f64; 3]| (p[0] - q[0]).hypot(p[1] - q[1]);
    let longest = side(a, b).max(side(b, c)).max(side(c, a));
    if area.abs() <= LEAST_AREA * longest * longest {
        return None;
    }
    let ap = [point[0] - a[0], point[1] - a[1]];
    let wb = (ap[0] * ac[1] - ap[1] * ac[0]) / area;
    let wc = (ab[0] * ap[1] - ab[1] * ap[0]) / area;
    let wa = 1.0 - wb - wc;
    [wa, wb, wc]
        .iter()
        .all(|&w| w >= -ON_EDGE)
        .then(|| wa * a[2] + wb * b[2] + wc * c[2])
}

/// Where the hardpoint at `position` stands on a hull `length` metres
/// long whose model is `hull` (its faces and which way it faces), or which
/// has none (see the module docs).
///
/// A hull with no model, or one that can't be scaled to `length` (no depth
/// along its front, or a `length` that isn't finite and positive), stands
/// its mounts on the plane. A hull with a model stands each on its highest
/// surface under `position`, or on the plane, [`Footing::Off`], with none.
#[must_use]
pub fn mount_point(hull: Option<(&Model, Front)>, length: f64, position: [f64; 2]) -> Standing {
    let [x, y] = position;
    let placed = hull
        .filter(|_| length.is_finite() && length > 0.0)
        .and_then(|(model, front)| {
            Placement::try_from_bounds(model.bounds, front, length).map(|p| (model, p))
        });
    let Some((model, placement)) = placed else {
        return Standing {
            at: [x, y, 0.0],
            footing: Footing::Plane,
        };
    };
    match surface(model, &placement, position) {
        Some(height) => Standing {
            at: [x, y, height],
            footing: Footing::Surface,
        },
        None => Standing {
            at: [x, y, 0.0],
            footing: Footing::Off,
        },
    }
}

/// How the outfit model `model`, facing `front`, sits on its mount, at a
/// metre a model unit whatever the hull (see the module docs). A model
/// whose bounds aren't finite has no lift and no reach.
#[must_use]
pub fn fitting(model: &Model, front: Front) -> Fitting {
    let placement = Placement::unscaled_bounds(model.bounds, front);
    let top = farthest(&model.bounds, |c| placement.ship_point_3d(c)[2]);
    let nose = farthest(&model.bounds, |c| placement.ship_point_3d(c)[1]);
    let finite = |v: f64| if v.is_finite() { v } else { 0.0 };
    Fitting {
        lift: finite(top),
        reach: finite(nose),
    }
}

/// The largest `along` gives at a corner of `bounds`.
fn farthest(bounds: &Bounds, along: impl Fn([f32; 3]) -> f64) -> f64 {
    let ends = |k: usize| [bounds.min[k], bounds.max[k]];
    ends(0)
        .into_iter()
        .flat_map(|x| ends(1).into_iter().map(move |y| (x, y)))
        .flat_map(|(x, y)| ends(2).map(|z| [x, y, z]))
        .map(along)
        .fold(f64::NEG_INFINITY, f64::max)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::model::model;

    const FRONTS: [Front; 4] = [Front::PlusZ, Front::MinusZ, Front::PlusX, Front::MinusX];

    /// A box `size` big at `at`, read as a model through the fixture.
    fn boxed(size: [f32; 3], at: [f32; 3]) -> Model {
        model("m.glb", &crate::fixture::glb(size, at), |_| None).unwrap()
    }

    /// The twelve faces of the box from `min` to `max`.
    fn cuboid(min: [f32; 3], max: [f32; 3]) -> Vec<Face> {
        let corner = |i: usize| {
            [0, 1, 2].map(|k| {
                let bit = 4 >> k;
                if i & bit == 0 { min[k] } else { max[k] }
            })
        };
        let quads = [
            [0, 1, 3, 2],
            [4, 6, 7, 5],
            [0, 4, 5, 1],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 5, 7, 3],
        ];
        quads
            .iter()
            .flat_map(|&[a, b, c, d]| [[a, b, c], [a, c, d]])
            .map(|tri| face(tri.map(corner)))
            .collect()
    }

    fn face(points: [[f32; 3]; 3]) -> Face {
        Face {
            points,
            color: [1.0; 4],
            material: None,
        }
    }

    /// A model of `faces`, its bounds the box they fill.
    fn of(faces: Vec<Face>) -> Model {
        let all = || faces.iter().flat_map(|f| f.points);
        let min = [0, 1, 2].map(|k| all().map(|p| p[k]).fold(f32::INFINITY, f32::min));
        let max = [0, 1, 2].map(|k| all().map(|p| p[k]).fold(f32::NEG_INFINITY, f32::max));
        Model {
            triangles: u32::try_from(faces.len()).unwrap(),
            bounds: Bounds { min, max },
            faces,
            materials: Vec::new(),
            external: Vec::new(),
        }
    }

    /// The light freighter's box: 2.6 wide, 1.2 tall, 4 long, front −z.
    const FREIGHTER: [f32; 3] = [2.6, 1.2, 4.0];

    #[test]
    fn a_box_s_top_is_found_at_its_half_height_once_scaled_to_its_length() {
        let hull = boxed(FREIGHTER, [0.0; 3]);
        let placement = Placement::new(&hull, Front::MinusZ, 30.0);
        // The top's height, from the placement of the box's top corner.
        let top = placement.ship_point_3d([1.3, 0.6, 2.0])[2];
        assert!(top > 0.0);
        assert_eq!(surface(&hull, &placement, [0.0, -2.0]), Some(top));
        let standing = mount_point(Some((&hull, Front::MinusZ)), 30.0, [0.0, -2.0]);
        assert_eq!(
            standing,
            Standing {
                at: [0.0, -2.0, top],
                footing: Footing::Surface
            }
        );
    }

    #[test]
    fn the_highest_surface_under_a_point_wins() {
        // A low deck, 4 long and 0.5 tall, with a tall block, 1 tall, over
        // its +z half.
        let mut faces = cuboid([-1.0, 0.0, -2.0], [1.0, 0.5, 2.0]);
        faces.extend(cuboid([-0.5, 0.0, 0.5], [0.5, 1.0, 1.5]));
        let hull = of(faces);
        let placement = Placement::new(&hull, Front::PlusZ, 8.0);
        let block = placement.ship_point_3d([0.0, 1.0, 1.0]);
        let deck = placement.ship_point_3d([0.0, 0.5, -1.0]);
        assert!(block[2] > deck[2]);
        assert_eq!(
            surface(&hull, &placement, [block[0], block[1]]),
            Some(block[2])
        );
        assert_eq!(
            surface(&hull, &placement, [deck[0], deck[1]]),
            Some(deck[2])
        );
    }

    #[test]
    fn a_sloped_face_is_stood_on_at_its_height_under_the_point() {
        // A ramp rising one unit for each along z, over a flat floor.
        let mut faces = cuboid([-1.0, -0.5, -1.0], [3.0, 0.0, 3.0]);
        faces.push(face([[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 2.0]]));
        let hull = of(faces);
        let placement = Placement::new(&hull, Front::PlusZ, 12.0);
        // Model (0.5, 1, 1) lies on the ramp.
        let on = placement.ship_point_3d([0.5, 1.0, 1.0]);
        let found = surface(&hull, &placement, [on[0], on[1]]).unwrap();
        assert!((found - on[2]).abs() < 1e-9, "{found} against {on:?}");
        let floor = placement.ship_point_3d([0.0, 0.0, 0.0])[2];
        assert!(found > floor);
    }

    #[test]
    fn a_sliver_seen_almost_edge_on_is_not_stood_on() {
        // A face in an upright plane, its corners off that plane seen
        // from above only by f32 rounding, standing on a floor.
        let mut faces = cuboid([-1.0, -0.5, -1.0], [2.0, 0.0, 3.0]);
        let sliver = [[0.1, 0.0, 0.1], [0.3, 0.0, 0.7], [0.7, 2.0, 1.9]];
        faces.push(face(sliver));
        let hull = of(faces);
        // Small, and scaled up a thousandfold: the least area is a share
        // of the face's own size, not a fixed one.
        for length in [7.0, 7000.0] {
            let placement = Placement::new(&hull, Front::PlusZ, length);
            let floor = placement.ship_point_3d([0.0, 0.0, 0.0])[2];
            let [a, _, c] = sliver.map(|p| placement.ship_point_3d(p));
            for t in [0.25, 0.5, 0.75] {
                // On its edge from its foot to its top, seen from above.
                let on = [a[0] + t * (c[0] - a[0]), a[1] + t * (c[1] - a[1])];
                let found = surface(&hull, &placement, on);
                assert_eq!(found, Some(floor), "{length} {t}");
            }
        }
    }

    #[test]
    fn a_long_sliver_along_either_axis_is_measured_by_its_own_sides() {
        // Two slivers raised over a floor 6 km across, a metre a unit, each
        // a kilometre long and seen from above an eighth of a square metre
        // in area: far under the least share of its longest side squared.
        let d = 2.0_f32.powi(-12);
        let along_x = [
            [1024.0, 2.0, 1.0],
            [1536.0, 2.0, 1.0],
            [2048.0, 5.0, 1.0 + d],
        ];
        let along_z = along_x.map(|[x, y, z]| [z, y, x]);
        for sliver in [along_x, along_z] {
            let mut faces = cuboid([-3000.0, -1.0, -3000.0], [3000.0, 0.0, 3000.0]);
            faces.push(face(sliver));
            let hull = of(faces);
            let placement = Placement::new(&hull, Front::PlusZ, 6000.0);
            let floor = placement.ship_point_3d([0.0, 0.0, 0.0])[2];
            let corners = sliver.map(|p| placement.ship_point_3d(p));
            let middle = [0, 1].map(|k| corners.iter().map(|c| c[k]).sum::<f64>() / 3.0);
            assert_eq!(
                surface(&hull, &placement, middle),
                Some(floor),
                "{sliver:?}"
            );
        }
    }

    #[test]
    fn a_small_face_far_from_the_middle_is_stood_on() {
        // A pad a tenth of a metre across, 50 m out on a 200 m floor, a
        // metre a unit: its own size, not its distance, sets its least
        // area.
        let mut faces = cuboid([-100.0, -1.0, -100.0], [100.0, 0.0, 100.0]);
        let pad = [[50.0, 1.0, 50.0], [50.1, 1.0, 50.0], [50.0, 1.0, 50.1]];
        faces.push(face(pad));
        let hull = of(faces);
        let placement = Placement::new(&hull, Front::PlusZ, 200.0);
        let on = placement.ship_point_3d([50.02, 1.0, 50.02]);
        assert_eq!(surface(&hull, &placement, [on[0], on[1]]), Some(on[2]));
    }

    #[test]
    fn a_point_on_an_edge_a_diagonal_or_a_corner_finds_the_top() {
        let hull = boxed(FREIGHTER, [0.0; 3]);
        let placement = Placement::new(&hull, Front::MinusZ, 30.0);
        let point = |p: [f32; 3]| placement.ship_point_3d(p);
        let top = point([0.0, 0.6, 0.0])[2];
        let corner = point([1.3, 0.6, 2.0]);
        let opposite = point([-1.3, 0.6, -2.0]);
        let edge = point([1.3, 0.6, 0.0]);
        // The middle of the top lies on the diagonal its two triangles
        // share.
        let diagonal = [
            f64::midpoint(corner[0], opposite[0]),
            f64::midpoint(corner[1], opposite[1]),
        ];
        for at in [diagonal, [edge[0], edge[1]], [corner[0], corner[1]]] {
            let found = surface(&hull, &placement, at).expect("on the top");
            assert!((found - top).abs() < 1e-9, "{at:?}: {found} against {top}");
        }
    }

    #[test]
    fn just_outside_the_footprint_is_off() {
        let hull = boxed(FREIGHTER, [0.0; 3]);
        let placement = Placement::new(&hull, Front::MinusZ, 30.0);
        let corner = placement.ship_point_3d([1.3, 0.6, 2.0]);
        let beyond = [corner[0].abs() + 0.01, 0.0];
        assert_eq!(surface(&hull, &placement, beyond), None);
        assert_eq!(
            mount_point(Some((&hull, Front::MinusZ)), 30.0, beyond),
            Standing {
                at: [beyond[0], beyond[1], 0.0],
                footing: Footing::Off
            }
        );
    }

    #[test]
    fn walls_seen_edge_on_are_not_stood_on() {
        // A box's four walls, without its top and bottom.
        let walls: Vec<Face> = cuboid([-1.0, 0.0, -2.0], [1.0, 1.0, 2.0])
            .into_iter()
            .filter(|f| {
                let ys = f.points.map(|p| p[1]);
                ys.iter().any(|&y| y != ys[0])
            })
            .collect();
        assert_eq!(walls.len(), 8);
        let hull = of(walls);
        let placement = Placement::new(&hull, Front::PlusZ, 4.0);
        // On the middle, and on a wall's own line.
        assert_eq!(surface(&hull, &placement, [0.0, 0.0]), None);
        let wall = placement.ship_point_3d([1.0, 0.5, 0.0]);
        assert_eq!(surface(&hull, &placement, [wall[0], wall[1]]), None);
        assert_eq!(
            mount_point(Some((&hull, Front::PlusZ)), 4.0, [0.0, 0.0]).footing,
            Footing::Off
        );
    }

    #[test]
    fn a_collapsed_face_is_not_stood_on() {
        // A face with two corners in one place, over a low deck.
        let mut faces = cuboid([-1.0, 0.0, -1.0], [1.0, 0.2, 1.0]);
        faces.push(face([
            [-1.0, 5.0, -1.0],
            [-1.0, 5.0, -1.0],
            [1.0, 5.0, 1.0],
        ]));
        let hull = of(faces);
        let placement = Placement::new(&hull, Front::PlusZ, 2.0);
        let deck = placement.ship_point_3d([0.0, 0.2, 0.0])[2];
        assert_eq!(surface(&hull, &placement, [0.0, 0.0]), Some(deck));
    }

    #[test]
    fn a_face_with_a_corner_that_is_not_finite_is_skipped() {
        let mut faces = cuboid([-1.0, 0.0, -1.0], [1.0, 0.2, 1.0]);
        for bad in [f32::NAN, f32::INFINITY] {
            faces.push(face([[-1.0, 5.0, -1.0], [1.0, bad, -1.0], [1.0, 5.0, 1.0]]));
        }
        // The bounds of the deck alone, so the placement stays finite.
        let mut hull = of(faces);
        hull.bounds = Bounds {
            min: [-1.0, 0.0, -1.0],
            max: [1.0, 0.2, 1.0],
        };
        let placement = Placement::new(&hull, Front::PlusZ, 2.0);
        let deck = placement.ship_point_3d([0.0, 0.2, 0.0])[2];
        // Under the bad faces' corners' [x, z]: ship (-0.5, -0.5) is model
        // (0.5, -0.5), inside their (1, -1), (1, 1) side.
        assert_eq!(surface(&hull, &placement, [-0.5, -0.5]), Some(deck));
    }

    #[test]
    fn a_position_that_is_not_finite_is_off() {
        let hull = boxed(FREIGHTER, [0.0; 3]);
        let placement = Placement::new(&hull, Front::MinusZ, 30.0);
        for bad in [
            [f64::NAN, 0.0],
            [0.0, f64::NAN],
            [f64::INFINITY, 0.0],
            [0.0, f64::NEG_INFINITY],
        ] {
            assert_eq!(surface(&hull, &placement, bad), None, "{bad:?}");
            let standing = mount_point(Some((&hull, Front::MinusZ)), 30.0, bad);
            assert_eq!(standing.footing, Footing::Off, "{bad:?}");
            assert_eq!(standing.at[2], 0.0, "{bad:?}");
        }
    }

    #[test]
    fn each_front_puts_the_same_ship_point_on_the_same_surface() {
        // A deck with a block on its nose half, turned four ways: the
        // block's top is the same ship-space height over the same point.
        let mut found = Vec::new();
        for front in FRONTS {
            // The nose along `front`, in model [x, z].
            let [fx, fz] = front.forward().map(|v| v as f32);
            let along = |lo: f32, hi: f32, w: f32| {
                let (a, b) = (lo * fx + lo * fz, hi * fx + hi * fz);
                let (a, b) = (a.min(b), a.max(b));
                if fx == 0.0 {
                    ([-w, a], [w, b])
                } else {
                    ([a, -w], [b, w])
                }
            };
            let (dmin, dmax) = along(-2.0, 2.0, 1.0);
            let (bmin, bmax) = along(0.5, 1.5, 0.5);
            let mut faces = cuboid([dmin[0], 0.0, dmin[1]], [dmax[0], 0.5, dmax[1]]);
            faces.extend(cuboid([bmin[0], 0.0, bmin[1]], [bmax[0], 1.0, bmax[1]]));
            let hull = of(faces);
            let block = mount_point(Some((&hull, front)), 8.0, [0.0, 2.0]);
            let deck = mount_point(Some((&hull, front)), 8.0, [0.0, -2.0]);
            assert_eq!(block.footing, Footing::Surface, "{front:?}");
            assert_eq!(deck.footing, Footing::Surface, "{front:?}");
            assert!(block.at[2] > deck.at[2], "{front:?}");
            found.push((block.at, deck.at));
        }
        assert!(found.iter().all(|f| *f == found[0]), "{found:?}");
    }

    #[test]
    fn a_hull_with_no_model_stands_its_mounts_on_the_plane() {
        assert_eq!(
            mount_point(None, 30.0, [1.0, -2.0]),
            Standing {
                at: [1.0, -2.0, 0.0],
                footing: Footing::Plane
            }
        );
    }

    #[test]
    fn a_hull_that_cannot_be_scaled_to_its_length_stands_on_the_plane() {
        let hull = boxed(FREIGHTER, [0.0; 3]);
        for length in [0.0, -4.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                mount_point(Some((&hull, Front::MinusZ)), length, [0.0, 0.0]),
                Standing {
                    at: [0.0, 0.0, 0.0],
                    footing: Footing::Plane
                },
                "{length}"
            );
        }
        // A flat panel with no depth along its front.
        let flat = boxed([2.0, 1.0, 0.0], [0.0; 3]);
        assert_eq!(
            mount_point(Some((&flat, Front::PlusZ)), 8.0, [0.0, 0.0]).footing,
            Footing::Plane
        );
    }

    #[test]
    fn an_outfit_is_lifted_by_half_its_height_and_reaches_half_its_length() {
        // 0.4 wide (x), 0.3 tall and 1.2 long (z), a metre a unit.
        let long = boxed([0.4, 0.3, 1.2], [0.0; 3]);
        for front in FRONTS {
            let placement = Placement::unscaled(&long, front);
            let corner = |p| placement.ship_point_3d(p);
            let fitted = fitting(&long, front);
            assert_eq!(fitted.lift, corner([0.2, 0.15, 0.6])[2], "{front:?}");
            let reach = [0.2, -0.2]
                .iter()
                .flat_map(|&x| [0.6, -0.6].map(|z| corner([x, 0.15, z])[1]))
                .fold(f64::MIN, f64::max);
            assert_eq!(fitted.reach, reach, "{front:?}");
            assert!((fitted.lift - 0.15).abs() < 1e-6, "{front:?}: {fitted:?}");
        }
        // Along z it is 1.2 long; along x, 0.4.
        for (front, half) in [
            (Front::PlusZ, 0.6),
            (Front::MinusZ, 0.6),
            (Front::PlusX, 0.2),
            (Front::MinusX, 0.2),
        ] {
            let fitted = fitting(&long, front);
            assert!((fitted.reach - half).abs() < 1e-6, "{front:?}: {fitted:?}");
        }
        // A cube off the origin is placed by its own middle: it reaches
        // half its side whichever way it faces, and is lifted by half its
        // height, not by where it sits.
        let cube = boxed([1.2, 0.3, 1.2], [0.2, 0.1, -0.3]);
        for front in FRONTS {
            let fitted = fitting(&cube, front);
            assert!((fitted.reach - 0.6).abs() < 1e-6, "{front:?}: {fitted:?}");
            assert!((fitted.lift - 0.15).abs() < 1e-6, "{front:?}: {fitted:?}");
        }
    }

    #[test]
    fn an_outfit_with_bounds_that_are_not_finite_has_no_lift_or_reach() {
        let mut turret = boxed([0.4, 0.3, 1.2], [0.0; 3]);
        turret.bounds.max = [f32::INFINITY; 3];
        assert_eq!(
            fitting(&turret, Front::MinusZ),
            Fitting {
                lift: 0.0,
                reach: 0.0
            }
        );
    }
}
