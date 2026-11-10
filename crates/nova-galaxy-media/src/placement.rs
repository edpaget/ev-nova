//! How a model sits on a ship, so its outline can be drawn against the
//! ship's footprint. Ported from game-bible's `galaxy` tool
//! (`placement.rs`).

use nova_galaxy_format::asset::Front;

use crate::model::{Bounds, Model};

/// How a model sits on a ship: centred on its footprint seen from above,
/// turned so its front is the nose, and scaled to the ship's length.
///
/// Ship space is metres seen from above, `[x, y]` with x to starboard and
/// y to the nose. Model space is glTF's `[x, y, z]`, +Y up.
///
/// In 3D ([`ship_point_3d`](Self::ship_point_3d)), ship space is
/// `[x, y, z]`: starboard, the nose, and up (glTF's +Y), centred on the
/// model's middle in height too, so the model's furthest point is as near
/// its middle as it can be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// The footprint's middle, as model `[x, z]`.
    center: [f64; 2],
    /// The model's middle in height, model y.
    center_y: f64,
    /// The nose's direction, as model `[x, z]`.
    forward: [f64; 2],
    /// Starboard, as model `[x, z]`.
    right: [f64; 2],
    /// Metres per model unit.
    scale: f64,
    /// The model's width across `right`, in model units.
    width: f64,
}

impl Placement {
    /// `model`, facing `front`, scaled so it is `length` metres long.
    #[must_use]
    pub fn new(model: &Model, front: Front, length: f64) -> Self {
        Self::from_bounds(model.bounds, front, length)
    }

    /// A model filling `bounds`, facing `front`, scaled so it is `length`
    /// metres long: [`new`](Self::new) for a model read some other way.
    #[must_use]
    pub fn from_bounds(bounds: Bounds, front: Front, length: f64) -> Self {
        Self::scaled(bounds, front, |depth| length / depth)
    }

    /// [`from_bounds`](Self::from_bounds), or `None` when the model can't
    /// be scaled to `length`: it has no depth along `front` (a flat panel
    /// facing that way), or the scale that would take isn't finite. Every
    /// point it places is then finite.
    #[must_use]
    pub fn try_from_bounds(bounds: Bounds, front: Front, length: f64) -> Option<Self> {
        Some(Self::from_bounds(bounds, front, length)).filter(|p| p.scale.is_finite())
    }

    /// A model filling `bounds`, facing `front`, at the scale `scale`
    /// gives for its depth along `front` in model units.
    fn scaled(bounds: Bounds, front: Front, scale: impl FnOnce(f64) -> f64) -> Self {
        let [cx, cy, cz] = bounds.center().map(f64::from);
        let [sx, _, sz] = bounds.size().map(f64::from);
        let (forward, right) = (front.forward(), front.right());
        let along = |d: [f64; 2]| (d[0] * sx).abs() + (d[1] * sz).abs();
        Placement {
            center: [cx, cz],
            center_y: cy,
            forward,
            right,
            scale: scale(along(forward)),
            width: along(right),
        }
    }

    /// `model`, facing `front`, a metre to each model unit: a stellar's
    /// model, which has no size of its own to be scaled to.
    #[must_use]
    pub fn unscaled(model: &Model, front: Front) -> Self {
        Self::unscaled_bounds(model.bounds, front)
    }

    /// A model filling `bounds`, facing `front`, a metre to each model
    /// unit: [`unscaled`](Self::unscaled) for a model read some other way.
    /// It divides by nothing, so a model with no depth along `front` still
    /// places, every point finite.
    #[must_use]
    pub fn unscaled_bounds(bounds: Bounds, front: Front) -> Self {
        Self::scaled(bounds, front, |_| 1.0)
    }

    /// Where the model point `p` falls in 3D ship space: `[starboard,
    /// nose, up]` in metres, from the model's middle.
    #[must_use]
    pub fn ship_point_3d(&self, p: [f32; 3]) -> [f64; 3] {
        let [x, y] = self.ship(p);
        [x, y, (f64::from(p[1]) - self.center_y) * self.scale]
    }

    /// The model direction `n` (a normal) in 3D ship space: turned as
    /// points are, neither moved nor scaled, so it keeps its length.
    #[must_use]
    pub fn ship_normal_3d(&self, n: [f32; 3]) -> [f64; 3] {
        let [x, y, z] = n.map(f64::from);
        let dot = |v: [f64; 2]| x * v[0] + z * v[1];
        [dot(self.right), dot(self.forward), y]
    }

    /// Where the model point `p` falls in ship space.
    #[must_use]
    pub fn ship(&self, p: [f32; 3]) -> [f64; 2] {
        let d = [
            f64::from(p[0]) - self.center[0],
            f64::from(p[2]) - self.center[1],
        ];
        let dot = |v: [f64; 2]| (d[0] * v[0] + d[1] * v[1]) * self.scale;
        [dot(self.right), dot(self.forward)]
    }

    /// Where the ship-space point `[x, y]` falls on the model, as `[x, z]`.
    #[must_use]
    pub fn model(&self, [x, y]: [f64; 2]) -> [f64; 2] {
        let (x, y) = (x / self.scale, y / self.scale);
        [0, 1].map(|i| self.center[i] + x * self.right[i] + y * self.forward[i])
    }

    /// The model's width in metres at this scale.
    #[must_use]
    pub fn beam(&self) -> f64 {
        self.width * self.scale
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::model::model;

    /// A 2 by 4 box (x by z) at the origin, scaled to 8 m long.
    fn placed(front: Front) -> Placement {
        let glb = crate::fixture::glb([2.0, 1.0, 4.0], [0.0; 3]);
        let model = model("m.glb", &glb, |_| None).unwrap();
        Placement::new(&model, front, 8.0)
    }

    #[test]
    fn plus_z_is_the_nose_and_minus_x_the_right() {
        let p = placed(Front::PlusZ);
        assert_eq!(p.ship([0.0, 0.0, 2.0]), [0.0, 4.0]);
        assert_eq!(p.ship([-1.0, 0.0, 0.0]), [2.0, 0.0]);
        assert_eq!(p.model([2.0, 4.0]), [-1.0, 2.0]);
        assert_eq!(p.beam(), 4.0);
    }

    #[test]
    fn minus_z_turns_the_model_around() {
        let p = placed(Front::MinusZ);
        assert_eq!(p.ship([0.0, 0.0, -2.0]), [0.0, 4.0]);
        assert_eq!(p.ship([1.0, 0.0, 0.0]), [2.0, 0.0]);
        assert_eq!(p.model([2.0, 4.0]), [1.0, -2.0]);
    }

    #[test]
    fn a_side_front_measures_length_along_x() {
        let p = placed(Front::PlusX);
        // 2 units long scaled to 8 m: 4 m a unit, so 4 units wide is 16 m.
        assert_eq!(p.beam(), 16.0);
        assert_eq!(p.ship([1.0, 0.0, 0.0]), [0.0, 4.0]);
        assert_eq!(p.ship([0.0, 0.0, 2.0]), [8.0, 0.0]);
    }

    #[test]
    fn unscaled_keeps_a_metre_to_a_model_unit() {
        let p = placed(Front::MinusZ);
        let glb = crate::fixture::glb([2.0, 1.0, 4.0], [0.0; 3]);
        let model = model("m.glb", &glb, |_| None).unwrap();
        let unscaled = Placement::unscaled(&model, Front::MinusZ);
        assert_eq!(unscaled.ship([1.0, 0.0, -2.0]), [1.0, 2.0]);
        assert_eq!(unscaled.beam(), 2.0);
        assert_eq!(p.beam(), 4.0, "the 8 m placement is twice as big");
    }

    /// The box `size` big at `at`, read as a model.
    fn boxed(size: [f32; 3], at: [f32; 3]) -> Model {
        model("m.glb", &crate::fixture::glb(size, at), |_| None).unwrap()
    }

    /// Each corner of the box `size` big at `at`.
    fn corners(size: [f32; 3], at: [f32; 3]) -> Vec<[f32; 3]> {
        (0..8)
            .map(|i| {
                let s = [i & 4, i & 2, i & 1].map(|b| if b == 0 { -0.5 } else { 0.5 });
                [0, 1, 2].map(|k| at[k] + s[k] * size[k])
            })
            .collect()
    }

    /// How far the 3D ship points span along each ship axis.
    fn spans(points: &[[f64; 3]]) -> [f64; 3] {
        [0, 1, 2].map(|k| {
            let max = points.iter().map(|p| p[k]).fold(f64::MIN, f64::max);
            let min = points.iter().map(|p| p[k]).fold(f64::MAX, f64::min);
            max - min
        })
    }

    #[test]
    fn each_front_puts_the_nose_on_ship_y_and_the_top_on_ship_z() {
        // The 2 by 1 by 4 box (x, y, z), scaled to 8 m along its front:
        // 2 m a unit along z, 4 m a unit along x.
        let size = [2.0, 1.0, 4.0];
        let cases = [
            (
                Front::PlusZ,
                [0.0, 0.0, 2.0],
                [-1.0, 0.0, 0.0],
                [4.0, 8.0, 2.0],
            ),
            (
                Front::MinusZ,
                [0.0, 0.0, -2.0],
                [1.0, 0.0, 0.0],
                [4.0, 8.0, 2.0],
            ),
            (
                Front::PlusX,
                [1.0, 0.0, 0.0],
                [0.0, 0.0, 2.0],
                [16.0, 8.0, 4.0],
            ),
            (
                Front::MinusX,
                [-1.0, 0.0, 0.0],
                [0.0, 0.0, -2.0],
                [16.0, 8.0, 4.0],
            ),
        ];
        for (front, nose, starboard, span) in cases {
            let p = Placement::new(&boxed(size, [0.0; 3]), front, 8.0);
            let points: Vec<_> = corners(size, [0.0; 3])
                .into_iter()
                .map(|c| p.ship_point_3d(c))
                .collect();
            assert_eq!(spans(&points), span, "{front:?}");
            assert_eq!(p.ship_point_3d(nose), [0.0, 4.0, 0.0], "{front:?}");
            let flank = p.ship_point_3d(starboard);
            assert!(flank[0] > 0.0 && flank[1] == 0.0, "{front:?}: {flank:?}");
            let top = p.ship_point_3d([0.0, 0.5, 0.0]);
            assert_eq!(top, [0.0, 0.0, span[2] / 2.0], "{front:?}");
        }
    }

    #[test]
    fn a_model_is_centred_in_height_too() {
        let p = Placement::new(&boxed([2.0, 1.0, 4.0], [1.0, 3.0, -2.0]), Front::PlusZ, 4.0);
        assert_eq!(p.ship_point_3d([1.0, 3.0, -2.0]), [0.0, 0.0, 0.0]);
        assert_eq!(p.ship_point_3d([1.0, 3.5, -2.0]), [0.0, 0.0, 0.5]);
        assert_eq!(p.ship_point_3d([1.0, 2.5, 0.0]), [0.0, 2.0, -0.5]);
    }

    #[test]
    fn normals_turn_with_the_front_and_keep_their_length() {
        // Scaled to 8 m (2 m a unit), yet a normal keeps its length.
        let p = Placement::new(&boxed([2.0, 1.0, 4.0], [3.0, 1.0, 1.0]), Front::MinusZ, 8.0);
        assert_eq!(p.ship_normal_3d([0.0, 0.0, -1.0]), [0.0, 1.0, 0.0]);
        assert_eq!(p.ship_normal_3d([1.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        assert_eq!(p.ship_normal_3d([0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(p.ship_normal_3d([0.0, 0.0, 2.0]), [0.0, -2.0, 0.0]);
        let side = Placement::new(&boxed([2.0, 1.0, 4.0], [0.0; 3]), Front::PlusX, 8.0);
        assert_eq!(side.ship_normal_3d([1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]);
        assert_eq!(side.ship_normal_3d([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn unscaled_3d_keeps_a_metre_to_a_unit() {
        let p = Placement::unscaled(&boxed([2.0, 1.0, 4.0], [0.0; 3]), Front::MinusZ);
        let points: Vec<_> = corners([2.0, 1.0, 4.0], [0.0; 3])
            .into_iter()
            .map(|c| p.ship_point_3d(c))
            .collect();
        assert_eq!(spans(&points), [2.0, 4.0, 1.0]);
        assert_eq!(p.ship_point_3d([1.0, 0.5, -2.0]), [1.0, 2.0, 0.5]);
    }

    #[test]
    fn placing_by_bounds_is_placing_by_the_model() {
        let model = boxed([2.0, 1.0, 4.0], [1.0, 3.0, -2.0]);
        assert_eq!(
            Placement::from_bounds(model.bounds, Front::PlusX, 6.0),
            Placement::new(&model, Front::PlusX, 6.0)
        );
        assert_eq!(
            Placement::unscaled_bounds(model.bounds, Front::PlusX),
            Placement::unscaled(&model, Front::PlusX)
        );
    }

    #[test]
    fn a_model_flat_along_its_front_has_no_length_to_scale_to() {
        // A flat panel in the XY plane: no depth along z.
        let flat = boxed([2.0, 1.0, 0.0], [0.0; 3]).bounds;
        assert_eq!(Placement::try_from_bounds(flat, Front::PlusZ, 8.0), None);
        assert_eq!(Placement::try_from_bounds(flat, Front::MinusZ, 0.0), None);
        // Along x it has depth, and places as it would unchecked.
        assert_eq!(
            Placement::try_from_bounds(flat, Front::PlusX, 8.0),
            Some(Placement::from_bounds(flat, Front::PlusX, 8.0))
        );
        let deep = boxed([2.0, 1.0, 4.0], [0.0; 3]).bounds;
        assert_eq!(
            Placement::try_from_bounds(deep, Front::PlusZ, f64::INFINITY),
            None
        );
    }

    #[test]
    fn a_model_flat_along_its_front_still_places_unscaled() {
        let size = [2.0, 1.0, 0.0];
        let p = Placement::unscaled_bounds(boxed(size, [0.0; 3]).bounds, Front::PlusZ);
        let points: Vec<_> = corners(size, [0.0; 3])
            .into_iter()
            .map(|c| p.ship_point_3d(c))
            .collect();
        assert!(points.iter().flatten().all(|c| c.is_finite()), "{points:?}");
        assert_eq!(spans(&points), [2.0, 0.0, 1.0]);
        assert_eq!(p.ship_point_3d([1.0, 0.5, 0.0]), [-1.0, 0.0, 0.5]);
        assert_eq!(p.beam(), 2.0);
    }

    #[test]
    fn a_model_off_the_origin_is_centred_on_its_footprint() {
        // The 2 by 4 box moved to (2, 0, 1.5), a metre a unit.
        let glb = crate::fixture::glb([2.0, 1.0, 4.0], [2.0, 0.0, 1.5]);
        let model = model("m.glb", &glb, |_| None).unwrap();
        let p = Placement::new(&model, Front::PlusZ, 4.0);
        assert_eq!(p.ship([2.0, 0.0, 1.5]), [0.0, 0.0]);
        assert_eq!(p.ship([3.0, 0.0, 3.5]), [-1.0, 2.0]);
    }
}
