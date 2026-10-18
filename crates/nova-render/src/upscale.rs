//! The frame upscaler: which one draws the world, which ones the device
//! can use, and what that makes of the world layer.
//!
//! An upscaler is chosen as a plain value ([`Upscaler`], `nova_view`'s,
//! which the Preferences dialog lists and the settings keep) and handed to
//! the GPU adapter ([`Gpu::set_upscaler`](crate::Gpu::set_upscaler)). The
//! adapter says how it draws the world layer
//! ([`Gpu::world_resolution`](crate::Gpu::world_resolution)), and the
//! core states each frame's [`WorldSpace`] from that ([`world_space`]).
//!
//! [`Upscaler::Native`] draws the world straight into the target at its
//! own resolution, which changes nothing on screen.
//! [`Upscaler::SharpBilinear`] draws it into a texture of the logical
//! size and enlarges that onto the viewport.
//!
//! Which upscalers the running device can use is the [`UpscalerProbe`]
//! port's answer; [`listed`] turns it into the list the player picks
//! from, native first and always.

use crate::gpu::WorldSpace;
use crate::viewport::Viewport;

pub use nova_view::Upscaler;

/// Says which per-frame upscalers the running device can use.
pub trait UpscalerProbe {
    /// The upscalers it can use, in the order to list them. Native may be
    /// left out: it is always available ([`listed`]).
    fn upscalers(&self) -> Vec<Upscaler>;
}

/// The upscalers to list from a probe's `answer`: native first, whatever
/// the answer says, then the answer's others in its order, each once.
#[must_use]
pub fn listed(answer: &[Upscaler]) -> Vec<Upscaler> {
    let mut list = vec![Upscaler::Native];
    for &upscaler in answer {
        if !list.contains(&upscaler) {
            list.push(upscaler);
        }
    }
    list
}

/// The resolution a GPU draws the world layer at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WorldResolution {
    /// The target's own: the world is drawn straight into it.
    #[default]
    Native,
    /// The logical space's, one texel a unit, into a texture an upscaler
    /// enlarges onto the viewport.
    Logical,
}

/// The resolution `upscaler` draws the world layer at: native's own, and
/// the logical size for every upscaler that enlarges a world texture. A
/// GPU's [`Gpu::world_resolution`](crate::Gpu::world_resolution) answers
/// this for the upscaler it was set to.
#[must_use]
pub fn resolution_of(upscaler: Upscaler) -> WorldResolution {
    match upscaler {
        Upscaler::Native => WorldResolution::Native,
        Upscaler::SharpBilinear => WorldResolution::Logical,
    }
}

/// Where a frame drawn into `viewport` draws its world, at `resolution`:
/// straight into the target at the native resolution, and into a texture
/// of the logical size at the logical one.
#[must_use]
pub fn world_space(resolution: WorldResolution, viewport: &Viewport) -> WorldSpace {
    match resolution {
        WorldResolution::Native => WorldSpace::Target,
        WorldResolution::Logical => {
            let logical = viewport.logical();
            WorldSpace::Texture {
                size: (logical.w, logical.h),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_view::ViewShape;

    use super::*;
    use crate::viewport::LOGICAL;

    #[test]
    fn native_draws_the_world_natively_and_sharp_bilinear_at_the_logical_size() {
        assert_eq!(resolution_of(Upscaler::Native), WorldResolution::Native);
        assert_eq!(
            resolution_of(Upscaler::SharpBilinear),
            WorldResolution::Logical
        );
    }

    #[test]
    fn a_native_world_is_drawn_into_the_target() {
        let viewport = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(WorldResolution::Native, &viewport),
            WorldSpace::Target
        );
        assert_eq!(Upscaler::default(), Upscaler::Native);
        assert_eq!(WorldResolution::default(), WorldResolution::Native);
    }

    #[test]
    fn a_logical_world_is_drawn_into_a_texture_of_the_logical_size() {
        let classic = Viewport::new(LOGICAL, (2048, 1536), 2.0);
        assert_eq!(
            world_space(WorldResolution::Logical, &classic),
            WorldSpace::Texture { size: (1024, 768) }
        );
        let widened = Viewport::for_shape(ViewShape::Widened, (1920, 1080), 1.0);
        let logical = widened.logical();
        assert_eq!((logical.w, logical.h), (1365, 768));
        assert_eq!(
            world_space(WorldResolution::Logical, &widened),
            WorldSpace::Texture { size: (1365, 768) }
        );
    }

    /// Answers what it is given.
    struct Probe(Vec<Upscaler>);

    impl UpscalerProbe for Probe {
        fn upscalers(&self) -> Vec<Upscaler> {
            self.0.clone()
        }
    }

    fn listed_from(answer: &[Upscaler]) -> Vec<Upscaler> {
        listed(&Probe(answer.to_vec()).upscalers())
    }

    const NATIVE: Upscaler = Upscaler::Native;
    const SHARP: Upscaler = Upscaler::SharpBilinear;

    #[test]
    fn native_is_listed_first_whatever_the_probe_says() {
        assert_eq!(listed_from(&[SHARP]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[SHARP, NATIVE]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[NATIVE, SHARP]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[]), [NATIVE]);
    }

    #[test]
    fn an_upscaler_answered_twice_is_listed_once() {
        assert_eq!(listed_from(&[SHARP, SHARP]), [NATIVE, SHARP]);
        assert_eq!(listed_from(&[NATIVE, NATIVE]), [NATIVE]);
    }
}
