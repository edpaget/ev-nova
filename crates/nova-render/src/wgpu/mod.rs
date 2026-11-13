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
//! copy of the rectangle the core says it covers, in the pixels of the
//! canvas it draws into
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
//! changes on screen; under a render scale below 100% its world is a
//! texture all the same, which it enlarges with sharp-bilinear
//! ([`enlarging`](crate::enlarging)). [`SharpBilinear`] has the world drawn into a texture
//! of its own ([`WorldSpace::Texture`](crate::WorldSpace::Texture)) first,
//! enlarges it onto the viewport ([`Upscale::enlarge`]), and the whole
//! overlay is drawn over it, in the frame's order: an overlay drawn before
//! the world in the frame lands over it, unlike natively (the renderer's
//! layout says why). The world's tail
//! ([`Frame::world_tail`](crate::Frame::world_tail)) is drawn last, into
//! the scene in the frame's order, world and overlay alike, as natively.
//! [`Fsr1`] draws the world into a texture as sharp-bilinear does, then
//! enlarges it in two passes in the frame's encoder: EASU into a texture
//! the viewport's size, then RCAS onto the viewport. On macOS, MetalFX's spatial scaler
//! (`metal_fx::MetalFxSpatial`, through the `nova_metalfx` crate's safe
//! API) draws the world into a texture as sharp-bilinear does, scales it
//! in a command buffer of its own, and copies the result onto the
//! viewport; a size pair MetalFX won't take is drawn with sharp-bilinear
//! and logged once ([`Gpu::take_warnings`](crate::Gpu::take_warnings)).
//! Both GPU adapters are the [`UpscalerProbe`](crate::UpscalerProbe) for
//! their device: every wgpu device can use native, sharp-bilinear and
//! FSR 1, and
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
//! A models batch ([`Batch::Models`](crate::Batch::Models)) draws in a
//! render pass of its own, its canvas's depth texture attached and
//! cleared, and the batch after it opens a fresh pass without one, so no
//! other pipeline meets a depth attachment. Each canvas, the scene or the
//! world's, makes its depth texture the first frame that draws models into
//! it. The model pipeline (`models.wgsl`) multiplies each vertex by the
//! core's transforms and colours each fragment as
//! [`camera::fragment`](crate::camera::fragment) defines: the vertex
//! colour times its section's texel, lit by
//! [`camera::shade`](crate::camera::shade)'s formula unless it is unlit.
//! Each mesh's buffers are made when it is uploaded
//! ([`Gpu::upload_mesh`](crate::Gpu::upload_mesh)), and each texture's
//! bind group when it is
//! ([`Gpu::upload_model_texture`](crate::Gpu::upload_model_texture)); a
//! section with no texture samples one white texel made with the
//! pipeline, and a mesh is one draw per section.
//!
//! A planets batch ([`Batch::Planets`](crate::Batch::Planets)) draws in
//! an ordinary pass, with no depth: one square per planet, its fragments
//! mirroring [`crate::planet`]'s functions (`planets.wgsl`), sampling the
//! textures each planet uploads ([`Gpu::upload_planet`](crate::Gpu::upload_planet))
//! with explicit gradients so no mip seam shows where the surface wraps.
//!
//! A sky batch ([`Batch::Sky`](crate::Batch::Sky)) draws in an ordinary
//! pass, with no depth: one square per star, its fragments adding the
//! star's colour by [`sky::coverage`](crate::sky::coverage) (`sky.wgsl`).
//!
//! A sky-images batch ([`Batch::SkyImages`](crate::Batch::SkyImages))
//! draws in an ordinary pass, with no depth: one quad per decoration, as
//! the core placed and turned it, its fragments sampling the image each
//! uploads ([`Gpu::upload_sky_image`](crate::Gpu::upload_sky_image)) at
//! the core's level of detail and leaving `beneath · (1 − cover) + light`
//! ([`sky::over`](crate::sky::over), `sky_images.wgsl`). An image's
//! texture and bind group are made at its upload, and the one sampler
//! with the pipeline.
//!
//! Every render pipeline, glyphon's included, the model pipeline too, is built when the renderer
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
mod fsr1;
mod hinted_text;
mod layout;
#[cfg(target_os = "macos")]
mod metal_fx;
mod metrics;
mod model_samplers;
mod models;
mod offscreen;
mod overlay;
mod planets;
mod renderer;
mod sharp_bilinear;
mod sky;
mod sky_images;
mod surface;
mod upscale;

pub use fsr1::Fsr1;
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
