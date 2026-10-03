//! A [`Gpu`] that draws into its own texture, for tests and tools.

use std::sync::mpsc;
use std::time::Duration;

use nova_data::graphics::Image;

use super::data::{padded_bytes_per_row, strip_padding};
use super::overlay::{OverlayGpu, OverlayPainter, PaintTarget};
use super::{InitError, WgpuRenderer, acquire};
use crate::fonts::FontFaces;
use crate::gpu::{Frame, Gpu, PageId};
use crate::viewport::PixelRect;

/// The offscreen target's format: plain RGBA8, not sRGB.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// How long [`OffscreenGpu::read_pixels`] waits for the copy to map.
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Why the target could not be read back.
#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    /// Waiting for the GPU failed.
    #[error("waiting for the GPU: {0}")]
    Poll(#[from] wgpu::PollError),
    /// Mapping the read-back buffer failed.
    #[error("mapping the read-back buffer: {0}")]
    Map(#[from] wgpu::BufferAsyncError),
    /// The mapping did not finish in time.
    #[error("the read-back buffer did not map within {READ_TIMEOUT:?}")]
    Timeout,
}

/// Draws frames into an RGBA8 texture of a fixed size and reads it back.
pub struct OffscreenGpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: WgpuRenderer,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
}

impl OffscreenGpu {
    /// A `width` x `height` target on the default GPU adapter, drawing
    /// text in `faces`.
    pub fn new(width: u32, height: u32, faces: &FontFaces) -> Result<Self, InitError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (_adapter, device, queue) = acquire(&instance, None)?;
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nova offscreen target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let renderer = WgpuRenderer::new(&device, &queue, FORMAT, faces);
        Ok(Self {
            device,
            queue,
            renderer,
            target,
            view,
            size: (width, height),
        })
    }

    /// How many font faces the text renderer loaded.
    #[must_use]
    pub fn font_faces(&self) -> usize {
        self.renderer.font_faces()
    }

    /// Whether Charcoal's font file loaded a face.
    #[must_use]
    pub fn charcoal_loaded(&self) -> bool {
        self.renderer.charcoal_loaded()
    }

    /// The target's RGBA8 pixels, row-major, top row first.
    pub fn read_pixels(&self) -> Result<Vec<u8>, ReadError> {
        let (width, height) = self.size;
        let padded = padded_bytes_per_row(width);
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nova read-back"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));
        let (sender, receiver) = mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            // The receiver is gone only after a timeout; nothing to tell.
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver
            .recv_timeout(READ_TIMEOUT)
            .map_err(|_| ReadError::Timeout)??;
        let pixels = strip_padding(&buffer.get_mapped_range(..).expect("mapped"), width, padded);
        buffer.unmap();
        Ok(pixels)
    }

    /// Draws `frame` into the target, then `painter` over it.
    fn present(&mut self, frame: &Frame, painter: Option<&mut dyn OverlayPainter>) {
        self.renderer.draw(frame, &self.view);
        if let Some(painter) = painter {
            painter.paint(&PaintTarget {
                device: &self.device,
                queue: &self.queue,
                view: &self.view,
                format: FORMAT,
                size_px: self.size,
            });
        }
    }
}

impl Gpu for OffscreenGpu {
    fn create_page(&mut self, page: PageId, size: u32) {
        self.renderer.create_page(page, size);
    }

    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
        self.renderer.upload(page, at, image);
    }

    fn submit(&mut self, frame: &Frame) {
        self.present(frame, None);
    }
}

impl OverlayGpu for OffscreenGpu {
    fn submit_with(&mut self, frame: &Frame, painter: &mut dyn OverlayPainter) {
        self.present(frame, Some(painter));
    }
}
