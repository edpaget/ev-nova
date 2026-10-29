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
//! ([`Blend::Or`](nova_view::Blend::Or)), an OR crossfade batch
//! ([`Blend::OrCrossfade`](nova_view::Blend::OrCrossfade)) and a
//! translucent OR batch
//! ([`Blend::OrTranslucent`](nova_view::Blend::OrTranslucent)) read a backdrop
//! copy of the rectangle the core says it covers, in its layer's pixels
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
//! layout says why). On macOS, MetalFX's spatial scaler
//! (`metal_fx::MetalFxSpatial`, through the `nova_metalfx` crate's safe
//! API) draws the world into a texture as sharp-bilinear does, scales it
//! in a command buffer of its own, and copies the result onto the
//! viewport; a size pair MetalFX won't take is drawn with sharp-bilinear
//! and logged once ([`Gpu::take_warnings`](crate::Gpu::take_warnings)).
//! Both GPU adapters are the [`UpscalerProbe`](crate::UpscalerProbe) for
//! their device: every wgpu device can use native and sharp-bilinear, and
//! a macOS device on Metal with a GPU MetalFX supports can use MetalFX too
//! ([`offered`](crate::upscale::offered)). MetalFX needs macOS 13, which
//! the build's minimum is.
//!
//! A frame is recorded as a run of command buffers (`FrameCommands`,
//! submitted at once, in order) rather than one: wgpu panics if one
//! encoder mixes its API with the raw (`as_hal`) one MetalFX is driven
//! through, so MetalFX's scale goes between the world's passes and the
//! overlay's.
//!
//! Text is drawn as glyphon custom glyphs, rasterised with full hints by
//! [`glyph_raster`](crate::glyph_raster) rather than by glyphon's own
//! rasteriser, whose hinting ignores TrueType instructions in X.
//!
//! Every render pipeline, glyphon's included, is built when the renderer
//! is made or its upscaler is set, never on a frame: building one compiles
//! its shader, which takes tens of milliseconds when macOS's Metal shader
//! cache is cold. MetalFX's scaler, which is no wgpu pipeline, is made on
//! the first frame of each pair of sizes it scales between.
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
mod hinted_text;
mod layout;
#[cfg(target_os = "macos")]
mod metal_fx;
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
