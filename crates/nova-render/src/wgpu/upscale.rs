//! The upscaler port in the renderer's terms: enlarge a world texture onto
//! the target. [`Native`] has one to enlarge only under a render scale
//! below 100%, and hands it to sharp-bilinear ([`enlarging`]); [`SharpBilinear`]
//! does, in the frame's own encoder, as [`Fsr1`] does in two passes
//! (EASU, then RCAS), and so does MetalFX on macOS
//! (`metal_fx::MetalFxSpatial`), in a command buffer of its own put
//! between the world's and the overlay's
//! ([`FrameCommands::insert`](super::frame_commands::FrameCommands::insert)).
//!
//! Which upscaler draws a choice is [`effective`]'s answer for the device's
//! [`abilities`]: MetalFX only where the device has it, sharp-bilinear in
//! its place everywhere else.

use super::frame_commands::WgpuCommands;
use super::fsr1::Fsr1;
use super::sharp_bilinear::SharpBilinear;
use crate::upscale::{Abilities, Upscaler, WorldResolution, effective, enlarging, resolution_of};
use crate::viewport::PixelRect;

/// Draws flight's world layer onto the target for the
/// [`WgpuRenderer`](super::WgpuRenderer).
///
/// An upscaler that draws the world at the logical resolution
/// ([`WorldResolution::Logical`]) is handed the world's texture, once it
/// is drawn, to enlarge onto the viewport, before the overlay is drawn
/// over it. One that draws it at the native resolution is asked only when
/// a render scale below 100% drew the world into a texture all the same;
/// otherwise the world is drawn straight into the target.
pub trait Upscale {
    /// The resolution the world layer is drawn at.
    fn resolution(&self) -> WorldResolution;

    /// Enlarges `world`, the whole world texture, onto `onto`, a
    /// rectangle of `target`, with commands recorded into `commands`'s
    /// encoder, or in a command buffer of its own put after what the frame
    /// recorded so far (`FrameCommands::insert`). A warning to show (a
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
///
/// A render scale below 100% draws even a native world into a texture
/// (`world_space`), and native has no enlarging pass of its own, so it
/// enlarges that texture with the upscaler [`enlarging`] names for it,
/// made with it rather than per frame.
pub struct Native {
    /// Enlarges a texture world.
    enlarger: Box<dyn Upscale>,
}

impl Native {
    /// Native, enlarging a texture world with `enlarger`.
    fn new(enlarger: Box<dyn Upscale>) -> Self {
        Self { enlarger }
    }
}

impl Upscale for Native {
    fn resolution(&self) -> WorldResolution {
        resolution_of(Upscaler::Native)
    }

    /// Hands a texture world, which only a render scale below 100% gives
    /// it, to its enlarger.
    fn enlarge(
        &mut self,
        commands: &mut WgpuCommands<'_>,
        world: &wgpu::TextureView,
        target: &wgpu::TextureView,
        onto: PixelRect,
        warnings: &mut Vec<String>,
    ) {
        self.enlarger
            .enlarge(commands, world, target, onto, warnings);
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

/// The upscaler that draws `choice` on a device with `abilities`
/// ([`effective`]), made on `device` and writing through `queue`: MetalFX
/// on a device without it is sharp-bilinear.
pub(super) fn upscale_for(
    choice: Upscaler,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    abilities: Abilities,
) -> Box<dyn Upscale> {
    made(effective(choice, abilities), device, queue)
}

/// `upscaler`, made on `device` and writing through `queue`: native with
/// the upscaler [`enlarging`] names for it (never native itself) beside
/// it.
fn made(upscaler: Upscaler, device: &wgpu::Device, queue: &wgpu::Queue) -> Box<dyn Upscale> {
    match upscaler {
        Upscaler::Native => {
            let enlarger = made(enlarging(Upscaler::Native), device, queue);
            Box::new(Native::new(enlarger))
        }
        Upscaler::SharpBilinear => Box::new(SharpBilinear::new(device, queue)),
        Upscaler::Fsr1 => Box::new(Fsr1::new(device, queue)),
        Upscaler::MetalFxSpatial => metal_fx(device, queue),
    }
}

/// MetalFX's upscaler, scaling with MetalFX's own scalers.
#[cfg(target_os = "macos")]
fn metal_fx(device: &wgpu::Device, queue: &wgpu::Queue) -> Box<dyn Upscale> {
    use super::metal_fx::{MetalFxScalers, MetalFxSpatial};
    let scalers = Box::new(MetalFxScalers::new(device, queue));
    Box::new(MetalFxSpatial::new(scalers, device, queue))
}

/// Never reached: no device off macOS has MetalFX ([`abilities`]), so
/// [`effective`] draws it with sharp-bilinear, as this does too.
#[cfg(not(target_os = "macos"))]
fn metal_fx(device: &wgpu::Device, queue: &wgpu::Queue) -> Box<dyn Upscale> {
    Box::new(SharpBilinear::new(device, queue))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An enlarger that draws at the logical resolution and enlarges
    /// nothing, standing in for one made on a device.
    struct Unused;

    impl Upscale for Unused {
        fn resolution(&self) -> WorldResolution {
            WorldResolution::Logical
        }

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

    #[test]
    fn the_native_upscaler_draws_the_world_at_the_native_resolution() {
        let native = Native::new(Box::new(Unused));
        assert_eq!(native.resolution(), WorldResolution::Native);
    }
}
