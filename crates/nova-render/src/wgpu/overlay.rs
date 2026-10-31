//! Drawing over a finished frame before it is presented or read back: the
//! hook a developer overlay (egui, in `nova`'s `dev-tools` build) draws
//! through. The [`Gpu`] port is unchanged; this is wgpu glue that a GPU
//! adapter offers on the side.

use nova_data::graphics::Image;

use crate::gpu::{Frame, Gpu, PageId};
use crate::upscale::{Upscaler, WorldResolution};
use crate::viewport::PixelRect;

pub use crate::gpu::MAX_TEXTURE_SIDE;

/// The target a finished frame was drawn into, for an overlay to draw over.
pub struct PaintTarget<'a> {
    /// The device the frame was drawn with.
    pub device: &'a wgpu::Device,
    /// Its queue; the frame's own commands are already submitted.
    pub queue: &'a wgpu::Queue,
    /// The frame's colour target.
    pub view: &'a wgpu::TextureView,
    /// The target's format.
    pub format: wgpu::TextureFormat,
    /// The target's size in physical pixels.
    pub size_px: (u32, u32),
}

/// Draws over a frame after the renderer and before it is presented or
/// read back.
pub trait OverlayPainter {
    /// Draws over `target`, submitting its own commands.
    fn paint(&mut self, target: &PaintTarget<'_>);
}

/// A GPU adapter that can draw an overlay over each frame it submits.
pub trait OverlayGpu: Gpu {
    /// Draws `frame` as [`Gpu::submit`] does, with `painter` over it. A
    /// frame the adapter drops is not painted either.
    fn submit_with(&mut self, frame: &Frame, painter: &mut dyn OverlayPainter);
}

/// A [`Gpu`] that submits every frame through `gpu` with `painter` over
/// it, and forwards everything else unchanged.
pub struct WithOverlay<'a, G: ?Sized> {
    /// The adapter that draws.
    pub gpu: &'a mut G,
    /// What it draws over each frame.
    pub painter: &'a mut dyn OverlayPainter,
}

impl<G: OverlayGpu + ?Sized> Gpu for WithOverlay<'_, G> {
    fn create_page(&mut self, page: PageId, size: u32) {
        self.gpu.create_page(page, size);
    }

    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
        self.gpu.upload(page, at, image);
    }

    fn clear_pages(&mut self) {
        self.gpu.clear_pages();
    }

    fn submit(&mut self, frame: &Frame) {
        self.gpu.submit_with(frame, self.painter);
    }

    fn world_resolution(&self) -> WorldResolution {
        self.gpu.world_resolution()
    }

    fn set_upscaler(&mut self, upscaler: Upscaler) {
        self.gpu.set_upscaler(upscaler);
    }

    fn take_warnings(&mut self) -> Vec<String> {
        self.gpu.take_warnings()
    }
}

#[cfg(test)]
mod tests {
    use nova_data::graphics::Image;
    use nova_view::Color;

    use super::*;
    use crate::gpu::WorldSpace;
    use crate::viewport::{LogicalSize, PixelRect};

    /// One call a [`FakeGpu`] received.
    #[derive(Debug, PartialEq)]
    enum Call {
        CreatePage(PageId, u32),
        Upload(PageId, PixelRect, Image),
        ClearPages,
        Submit(Frame),
        SubmitWith(Frame, *const ()),
        SetUpscaler(Upscaler),
    }

    /// Records every call, and the painter's address for `submit_with`;
    /// draws the world at `resolution` and hands out `warnings` once.
    #[derive(Default)]
    struct FakeGpu {
        calls: Vec<Call>,
        resolution: WorldResolution,
        warnings: Vec<String>,
    }

    impl Gpu for FakeGpu {
        fn create_page(&mut self, page: PageId, size: u32) {
            self.calls.push(Call::CreatePage(page, size));
        }

        fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
            self.calls.push(Call::Upload(page, at, image.clone()));
        }

        fn clear_pages(&mut self) {
            self.calls.push(Call::ClearPages);
        }

        fn submit(&mut self, frame: &Frame) {
            self.calls.push(Call::Submit(frame.clone()));
        }

        fn world_resolution(&self) -> WorldResolution {
            self.resolution
        }

        fn set_upscaler(&mut self, upscaler: Upscaler) {
            self.calls.push(Call::SetUpscaler(upscaler));
        }

        fn take_warnings(&mut self) -> Vec<String> {
            std::mem::take(&mut self.warnings)
        }
    }

    impl OverlayGpu for FakeGpu {
        fn submit_with(&mut self, frame: &Frame, painter: &mut dyn OverlayPainter) {
            let address = std::ptr::from_mut(painter).cast::<()>().cast_const();
            self.calls.push(Call::SubmitWith(frame.clone(), address));
        }
    }

    /// Paints nothing.
    struct NoPainter;

    impl OverlayPainter for NoPainter {
        fn paint(&mut self, _target: &PaintTarget<'_>) {}
    }

    fn frame() -> Frame {
        Frame {
            target: (64, 48),
            viewport: PixelRect {
                x: 0,
                y: 0,
                w: 64,
                h: 48,
            },
            logical: LogicalSize { w: 32, h: 24 },
            clear: Color::rgba(1, 2, 3, 4),
            world_space: WorldSpace::Target,
            batches: Vec::new(),
            world: Vec::new(),
            world_clip: None,
        }
    }

    #[test]
    fn with_overlay_draws_the_world_as_its_gpu_does() {
        for resolution in [WorldResolution::Native, WorldResolution::Logical] {
            let mut gpu = FakeGpu {
                resolution,
                ..FakeGpu::default()
            };
            let mut painter = NoPainter;
            let with = WithOverlay {
                gpu: &mut gpu,
                painter: &mut painter,
            };
            assert_eq!(with.world_resolution(), resolution);
        }
    }

    #[test]
    fn with_overlay_sets_its_gpus_upscaler() {
        let mut gpu = FakeGpu::default();
        let mut painter = NoPainter;
        {
            let mut with = WithOverlay {
                gpu: &mut gpu,
                painter: &mut painter,
            };
            with.set_upscaler(Upscaler::SharpBilinear);
        }
        assert_eq!(gpu.calls, [Call::SetUpscaler(Upscaler::SharpBilinear)]);
    }

    #[test]
    fn with_overlay_hands_out_its_gpus_warnings() {
        let mut gpu = FakeGpu {
            warnings: vec!["MetalFX said no".to_owned()],
            ..FakeGpu::default()
        };
        let mut painter = NoPainter;
        let mut with = WithOverlay {
            gpu: &mut gpu,
            painter: &mut painter,
        };
        assert_eq!(with.take_warnings(), ["MetalFX said no"]);
        assert_eq!(with.take_warnings(), Vec::<String>::new());
    }

    #[test]
    fn with_overlay_forwards_pages_and_uploads_unchanged() {
        let mut gpu = FakeGpu::default();
        let mut painter = NoPainter;
        let image = Image::from_rgba(1, 1, vec![9, 8, 7, 6]).expect("1x1");
        let at = PixelRect {
            x: 3,
            y: 4,
            w: 1,
            h: 1,
        };
        {
            let mut with = WithOverlay {
                gpu: &mut gpu,
                painter: &mut painter,
            };
            with.create_page(PageId(2), 256);
            with.upload(PageId(2), at, &image);
            with.clear_pages();
        }
        assert_eq!(
            gpu.calls,
            [
                Call::CreatePage(PageId(2), 256),
                Call::Upload(PageId(2), at, image),
                Call::ClearPages
            ]
        );
    }

    #[test]
    fn with_overlay_submits_each_frame_with_its_painter() {
        let mut gpu = FakeGpu::default();
        let mut painter = NoPainter;
        let address = std::ptr::from_ref(&painter).cast::<()>();
        {
            let mut with = WithOverlay {
                gpu: &mut gpu,
                painter: &mut painter,
            };
            with.submit(&frame());
        }
        assert_eq!(gpu.calls.len(), 1);
        let Call::SubmitWith(submitted, painted_by) = &gpu.calls[0] else {
            panic!("not submit_with: {:?}", gpu.calls);
        };
        assert_eq!(*submitted, frame());
        assert!(std::ptr::addr_eq(*painted_by, address));
    }

    #[test]
    fn the_largest_texture_side_is_wgpus_default_limit() {
        assert_eq!(
            MAX_TEXTURE_SIDE,
            wgpu::Limits::default().max_texture_dimension_2d
        );
    }
}
