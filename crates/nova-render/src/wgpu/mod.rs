//! The wgpu adapter: draws [`Frame`](crate::Frame)s with wgpu, and text
//! with glyphon.
//!
//! This is the only module that names `wgpu` or `glyphon` types. It makes
//! no decisions: the core has already chosen every quad, colour and
//! rectangle, [`surface_action`](crate::surface_action) decides what a
//! window surface does before each frame, and
//! [`face_for`](crate::face_for) decides which loaded face draws each text
//! run. Text uses only the faces in the [`FontFaces`](crate::FontFaces) the
//! adapter is made with.
//!
//! [`WgpuRenderer`] draws every frame into an offscreen scene texture of
//! its own (always RGBA8, whatever the target's format), then copies it to
//! the window or offscreen target with one blit. An OR batch
//! ([`Blend::Or`](nova_view::Blend::Or)) reads a backdrop copy of the
//! rectangle the core says it covers, in its layer's pixels
//! ([`Frame::pixels_covered`](crate::Frame::pixels_covered)), taken just
//! before it.
//!
//! The renderer is given an [`Upscaler`](crate::Upscaler) (when it is
//! made, and again on [`Gpu::set_upscaler`](crate::Gpu::set_upscaler)),
//! which it turns into an [`Upscale`] and asks how it draws flight's world
//! layer ([`Gpu::world_resolution`](crate::Gpu::world_resolution)).
//! [`Native`] draws the world
//! ([`WorldSpace::Target`](crate::WorldSpace::Target)) straight into the
//! scene, every batch in the frame's order whatever its layer, so nothing
//! changes on screen. [`SharpBilinear`] has the world drawn into a texture
//! of its own ([`WorldSpace::Texture`](crate::WorldSpace::Texture)) first,
//! enlarges it onto the viewport ([`Upscale::enlarge`]), and the whole
//! overlay is drawn over it, in the frame's order: an overlay drawn before
//! the world in the frame lands over it, unlike natively (the renderer's
//! layout says why). Both GPU adapters are the
//! [`UpscalerProbe`](crate::UpscalerProbe) for their device: every wgpu
//! device can use both.
//!
//! [`GlyphonMetrics`] is `nova_view`'s text-metrics port over the same
//! font system the renderer draws with, so the widgets lay text out by the
//! widths it will be drawn at. It needs no GPU.
//!
//! [`OverlayGpu::submit_with`] draws a frame with an [`OverlayPainter`]
//! over it, which gets the frame's device, queue and target
//! ([`PaintTarget`]); [`WithOverlay`] turns that into a plain
//! [`Gpu`](crate::Gpu) for the renderer.

mod data;
mod fonts;
mod frame_commands;
mod metrics;
mod offscreen;
mod overlay;
mod renderer;
mod sharp_bilinear;
mod surface;
mod upscale;

pub use metrics::GlyphonMetrics;
pub use offscreen::{OffscreenGpu, ReadError};
pub use overlay::{MAX_TEXTURE_SIDE, OverlayGpu, OverlayPainter, PaintTarget, WithOverlay};
pub use renderer::WgpuRenderer;
pub use sharp_bilinear::SharpBilinear;
pub use surface::SurfaceGpu;
pub use upscale::{Native, Upscale};

/// Why a GPU could not be set up.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    /// No GPU adapter is available.
    #[error("no GPU adapter: {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),
    /// The adapter would not open a device.
    #[error("no GPU device: {0}")]
    NoDevice(#[from] wgpu::RequestDeviceError),
    /// The window's surface could not be created.
    #[error("creating the window surface: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    /// The window's surface offers no format that is not sRGB.
    #[error("the window surface offers no non-sRGB format")]
    NoSurfaceFormat,
}

/// The default adapter (compatible with `surface`, if given) and a device
/// on it.
fn acquire(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), InitError> {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: surface,
        apply_limit_buckets: false,
    }))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;
    Ok((adapter, device, queue))
}
