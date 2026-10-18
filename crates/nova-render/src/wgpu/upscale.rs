//! The upscaler port in the renderer's terms: enlarge a world texture onto
//! the target. [`Native`] never has one to enlarge;
//! [`SharpBilinear`] does.

use super::frame_commands::WgpuCommands;
use super::sharp_bilinear::SharpBilinear;
use crate::upscale::{Abilities, Upscaler, WorldResolution, resolution_of};
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
    /// rectangle of `target`, with commands recorded into `commands`'s
    /// encoder, or in a command buffer of its own put after what the frame
    /// recorded so far
    /// ([`FrameCommands::insert`](super::frame_commands::FrameCommands::insert)).
    /// A warning to show (a
    /// fallback, say) goes into `warnings`.
    fn enlarge(
        &mut self,
        commands: &mut WgpuCommands<'_>,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
        warnings: &mut Vec<String>,
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
        _commands: &mut WgpuCommands<'_>,
        _world: &wgpu::TextureView,
        _target: &wgpu::TextureView,
        _onto: PixelRect,
        _warnings: &mut Vec<String>,
    ) {
    }
}

/// What `device` can draw the world with beyond what every device can:
/// MetalFX on macOS when MetalFX supports its GPU (which also means it is
/// on Metal), and nothing more anywhere else.
pub(super) fn abilities(device: &wgpu::Device) -> Abilities {
    Abilities {
        metal_fx_spatial: metal_fx_spatial(device),
    }
}

/// Whether MetalFX's spatial scaler runs on `device`.
#[cfg(target_os = "macos")]
fn metal_fx_spatial(device: &wgpu::Device) -> bool {
    nova_metalfx::supported(device)
}

/// Never: MetalFX exists only on macOS.
#[cfg(not(target_os = "macos"))]
fn metal_fx_spatial(_device: &wgpu::Device) -> bool {
    false
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
