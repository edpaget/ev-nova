//! The frame upscaler: which one draws the world, and what that makes of
//! the world layer.
//!
//! An upscaler is chosen as a plain value ([`Upscaler`]) where the GPU
//! adapter is made. The adapter says how it draws the world layer
//! ([`Gpu::world_resolution`](crate::Gpu::world_resolution)), and the
//! core states each frame's [`WorldSpace`] from that ([`world_space`]).
//!
//! The only upscaler so far is [`Upscaler::Native`]: the world is drawn
//! straight into the target at its own resolution, which changes nothing
//! on screen.

use crate::gpu::WorldSpace;
use crate::viewport::Viewport;

/// Which upscaler draws flight's world layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Upscaler {
    /// No upscaler: the world is drawn at the window's resolution, as the
    /// overlay is ([`WorldResolution::Native`]).
    #[default]
    Native,
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
}
