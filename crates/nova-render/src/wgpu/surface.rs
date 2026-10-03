//! A [`Gpu`] that draws into a window's surface.

use nova_data::graphics::Image;

use super::data::surface_format;
use super::overlay::{OverlayGpu, OverlayPainter, PaintTarget};
use super::{InitError, WgpuRenderer, acquire};
use crate::fonts::FontFaces;
use crate::gpu::{Frame, Gpu, PageId};
use crate::present::{
    AcquireOutcome, AcquireResult, SurfaceAction, acquire_outcome, surface_action,
};
use crate::viewport::PixelRect;

/// Draws frames into a window. Before each frame it asks
/// [`surface_action`] whether to reconfigure, draw or skip, and
/// [`acquire_outcome`] what the texture it got means; it only performs the
/// wgpu calls.
pub struct SurfaceGpu {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    configured: Option<(u32, u32)>,
    last_acquire: AcquireOutcome,
    renderer: WgpuRenderer,
}

impl SurfaceGpu {
    /// A surface on `window`, whose platform display is `display`, drawn
    /// with vsync (`Fifo`) in a non-sRGB format, with text in `faces`.
    pub fn new(
        display: Box<dyn wgpu::wgt::WgpuHasDisplayHandle>,
        window: impl wgpu::DisplayAndWindowHandle + 'static,
        faces: &FontFaces,
    ) -> Result<Self, InitError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(display));
        let surface = instance.create_surface(window)?;
        let (adapter, device, queue) = acquire(&instance, Some(&surface))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = surface_format(&capabilities.formats).ok_or(InitError::NoSurfaceFormat)?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: Vec::new(),
        };
        let renderer = WgpuRenderer::new(&device, &queue, format, faces);
        Ok(Self {
            surface,
            device,
            queue,
            config,
            configured: None,
            last_acquire: AcquireOutcome::Acquired,
            renderer,
        })
    }
}

impl Gpu for SurfaceGpu {
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

impl OverlayGpu for SurfaceGpu {
    fn submit_with(&mut self, frame: &Frame, painter: &mut dyn OverlayPainter) {
        self.present(frame, Some(painter));
    }
}

impl SurfaceGpu {
    /// Draws `frame` into the window, `painter` over it, and presents it;
    /// a dropped frame is neither drawn nor painted.
    fn present(&mut self, frame: &Frame, painter: Option<&mut dyn OverlayPainter>) {
        match surface_action(self.configured, frame.target, self.last_acquire) {
            SurfaceAction::Drop => return,
            SurfaceAction::Reconfigure => {
                (self.config.width, self.config.height) = frame.target;
                self.surface.configure(&self.device, &self.config);
                self.configured = Some(frame.target);
            }
            SurfaceAction::Render => {}
        }
        let (texture, outcome) = acquire_outcome(mirror(self.surface.get_current_texture()));
        self.last_acquire = outcome;
        let Some(texture) = texture else {
            return;
        };
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer.draw(frame, &view);
        if let Some(painter) = painter {
            painter.paint(&PaintTarget {
                device: &self.device,
                queue: &self.queue,
                view: &view,
                format: self.config.format,
                size_px: frame.target,
            });
        }
        self.queue.present(texture);
    }
}

/// wgpu's acquire result as the core's [`AcquireResult`], variant for
/// variant.
fn mirror(current: wgpu::CurrentSurfaceTexture) -> AcquireResult<wgpu::SurfaceTexture> {
    match current {
        wgpu::CurrentSurfaceTexture::Success(texture) => AcquireResult::Success(texture),
        wgpu::CurrentSurfaceTexture::Suboptimal(texture) => AcquireResult::Suboptimal(texture),
        wgpu::CurrentSurfaceTexture::Timeout => AcquireResult::Timeout,
        wgpu::CurrentSurfaceTexture::Occluded => AcquireResult::Occluded,
        wgpu::CurrentSurfaceTexture::Outdated => AcquireResult::Outdated,
        wgpu::CurrentSurfaceTexture::Lost => AcquireResult::Lost,
        wgpu::CurrentSurfaceTexture::Validation => AcquireResult::Validation,
    }
}
