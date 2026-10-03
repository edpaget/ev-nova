//! The wgpu adapter: draws [`Frame`](crate::Frame)s with wgpu, and text
//! with glyphon.
//!
//! This is the only module that names `wgpu` or `glyphon` types. It makes
//! no decisions: the core has already chosen every quad, colour and
//! rectangle, and [`surface_action`](crate::surface_action) decides what a
//! window surface does before each frame.
//!
//! [`OverlayGpu::submit_with`] draws a frame with an [`OverlayPainter`]
//! over it, which gets the frame's device, queue and target
//! ([`PaintTarget`]); [`WithOverlay`] turns that into a plain
//! [`Gpu`](crate::Gpu) for the renderer.

mod data;
mod offscreen;
mod overlay;
mod renderer;
mod surface;

pub use offscreen::{OffscreenGpu, ReadError};
pub use overlay::{MAX_TEXTURE_SIDE, OverlayGpu, OverlayPainter, PaintTarget, WithOverlay};
pub use renderer::WgpuRenderer;
pub use surface::SurfaceGpu;

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
