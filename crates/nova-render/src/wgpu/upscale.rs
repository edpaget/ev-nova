//! The upscaler port in the renderer's terms: enlarge a world texture onto
//! the target. [`Native`] never has one to enlarge;
//! [`SharpBilinear`] does.

use super::sharp_bilinear::SharpBilinear;
use crate::upscale::{Upscaler, WorldResolution, resolution_of};
use crate::viewport::PixelRect;

/// Draws flight's world layer onto the target for the
/// [`WgpuRenderer`](super::WgpuRenderer).
///
/// An upscaler that draws the world at the logical resolution
/// ([`WorldResolution::Logical`]) is handed the world's texture, once it
/// is drawn, to enlarge onto the viewport, before the overlay is drawn
/// over it. One that draws it at the native resolution is never asked to
/// enlarge anything: the world is drawn straight into the target.
pub trait Upscale {
    /// The resolution the world layer is drawn at.
    fn resolution(&self) -> WorldResolution;

    /// Enlarges `world`, the whole world texture, onto `onto`, a
    /// rectangle of `target`, with commands recorded into `encoder`.
    fn enlarge(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
    );
}

/// No upscaler: the world is drawn into the target at its own resolution,
/// in the frame's order, as the overlay is.
#[derive(Clone, Copy, Debug, Default)]
pub struct Native;

impl Upscale for Native {
    fn resolution(&self) -> WorldResolution {
        resolution_of(Upscaler::Native)
    }

    /// Never asked: a native world has no texture of its own.
    fn enlarge(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _world: &wgpu::TextureView,
        _target: &wgpu::TextureView,
        _onto: PixelRect,
    ) {
    }
}

/// The upscaler `choice` names, made on `device` and writing through
/// `queue`.
pub(super) fn upscale_for(
    choice: Upscaler,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Box<dyn Upscale> {
    match choice {
        Upscaler::Native => Box::new(Native),
        Upscaler::SharpBilinear | Upscaler::MetalFxSpatial => {
            Box::new(SharpBilinear::new(device, queue))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_native_upscaler_draws_the_world_at_the_native_resolution() {
        assert_eq!(Native.resolution(), WorldResolution::Native);
    }
}
