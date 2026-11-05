//! How a model sits on a ship, so its outline can be drawn against the
//! ship's footprint. Ported from game-bible's `galaxy` tool
//! (`placement.rs`).

use nova_galaxy_format::asset::Front;

use crate::model::Model;

/// How a model sits on a ship: centred on its footprint seen from above,
/// turned so its front is the nose, and scaled to the ship's length.
///
/// Ship space is metres seen from above, `[x, y]` with x to starboard and
/// y to the nose. Model space is glTF's `[x, y, z]`, +Y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// The footprint's middle, as model `[x, z]`.
    center: [f64; 2],
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
        let [cx, _, cz] = model.bounds.center().map(f64::from);
        let [sx, _, sz] = model.bounds.size().map(f64::from);
        let (forward, right) = (front.forward(), front.right());
        let along = |d: [f64; 2]| (d[0] * sx).abs() + (d[1] * sz).abs();
        Placement {
            center: [cx, cz],
            forward,
            right,
            scale: length / along(forward),
            width: along(right),
        }
    }

    /// `model`, facing `front`, a metre to each model unit: a stellar's
    /// model, which has no size of its own to be scaled to.
    #[must_use]
    pub fn unscaled(model: &Model, front: Front) -> Self {
        // At 1 m long, a metre is the model's whole length in units.
        let length = Self::new(model, front, 1.0).scale.recip();
        Self::new(model, front, length)
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
