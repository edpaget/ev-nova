//! A [`Gpu`] that records what it is asked to do, for tests.
//!
//! Compiled for this crate's tests and, through the `recording` feature,
//! for other crates' tests.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use nova_data::graphics::Image;

use crate::gpu::{Frame, Gpu, PageId};
use crate::upscale::{Upscaler, UpscalerProbe, WorldResolution, resolution_of};
use crate::viewport::PixelRect;

/// One call a [`RecordingGpu`] received.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuCall {
    /// [`Gpu::create_page`].
    CreatePage {
        /// The page.
        page: PageId,
        /// Its side length.
        size: u32,
    },
    /// [`Gpu::upload`], with the image summarised.
    Upload {
        /// The page.
        page: PageId,
        /// Where on the page.
        rect: PixelRect,
        /// The image's width.
        width: u32,
        /// The image's height.
        height: u32,
        /// A hash of the image's pixels.
        pixels_hash: u64,
    },
    /// [`Gpu::submit`].
    Submit(Frame),
}

impl GpuCall {
    /// The call that uploads `image` to `page` at `rect`.
    #[must_use]
    pub fn upload(page: PageId, rect: PixelRect, image: &Image) -> Self {
        let mut hasher = DefaultHasher::new();
        image.pixels().hash(&mut hasher);
        Self::Upload {
            page,
            rect,
            width: image.width(),
            height: image.height(),
            pixels_hash: hasher.finish(),
        }
    }
}

/// Records every call, in order. Asked, it draws the world at
/// [`WorldResolution::Native`] unless made [`answering`](Self::answering)
/// otherwise, or until an upscaler is set ([`Gpu::set_upscaler`]): then at
/// that upscaler's resolution. The question is not a call, and nor is
/// setting the upscaler, which is kept ([`upscaler`](Self::upscaler)). As
/// an [`UpscalerProbe`] it answers native alone unless made
/// [`probing`](Self::probing) a list.
#[derive(Clone, Debug)]
pub struct RecordingGpu {
    /// The calls received, oldest first.
    pub calls: Vec<GpuCall>,
    world_resolution: WorldResolution,
    upscaler: Upscaler,
    probed: Vec<Upscaler>,
    warnings: Vec<String>,
}

impl Default for RecordingGpu {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            world_resolution: WorldResolution::Native,
            upscaler: Upscaler::Native,
            probed: vec![Upscaler::Native],
            warnings: Vec::new(),
        }
    }
}

impl RecordingGpu {
    /// A recorder with no calls.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A recorder with no calls that draws the world at `resolution`.
    #[must_use]
    pub fn answering(resolution: WorldResolution) -> Self {
        Self {
            world_resolution: resolution,
            ..Self::default()
        }
    }

    /// A recorder with no calls whose probe answers `upscalers`.
    #[must_use]
    pub fn probing(upscalers: &[Upscaler]) -> Self {
        Self {
            probed: upscalers.to_vec(),
            ..Self::default()
        }
    }

    /// The upscaler last set ([`Gpu::set_upscaler`]), native before any.
    #[must_use]
    pub fn upscaler(&self) -> Upscaler {
        self.upscaler
    }

    /// The frames submitted, oldest first.
    #[must_use]
    pub fn submits(&self) -> Vec<&Frame> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::Submit(frame) => Some(frame),
                _ => None,
            })
            .collect()
    }

    /// Raises `warning`, for [`Gpu::take_warnings`] to hand out. Raising
    /// one is not a call.
    pub fn warn(&mut self, warning: &str) {
        self.warnings.push(warning.to_owned());
    }

    /// Takes the calls received so far, leaving none.
    pub fn take(&mut self) -> Vec<GpuCall> {
        std::mem::take(&mut self.calls)
    }
}

impl Gpu for RecordingGpu {
    fn create_page(&mut self, page: PageId, size: u32) {
        self.calls.push(GpuCall::CreatePage { page, size });
    }

    fn upload(&mut self, page: PageId, at: PixelRect, image: &Image) {
        self.calls.push(GpuCall::upload(page, at, image));
    }

    fn submit(&mut self, frame: &Frame) {
        self.calls.push(GpuCall::Submit(frame.clone()));
    }

    fn world_resolution(&self) -> WorldResolution {
        self.world_resolution
    }

    fn set_upscaler(&mut self, upscaler: Upscaler) {
        self.upscaler = upscaler;
        self.world_resolution = resolution_of(upscaler);
    }

    fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }
}

impl UpscalerProbe for RecordingGpu {
    fn upscalers(&self) -> Vec<Upscaler> {
        self.probed.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_draws_the_world_natively_unless_told_and_asking_is_no_call() {
        let gpu = RecordingGpu::new();
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
        let logical = RecordingGpu::answering(WorldResolution::Logical);
        assert_eq!(logical.world_resolution(), WorldResolution::Logical);
        assert_eq!(logical.calls, []);
    }

    #[test]
    fn its_upscaler_is_kept_not_recorded_and_decides_the_resolution() {
        let mut gpu = RecordingGpu::new();
        assert_eq!(gpu.upscaler(), Upscaler::Native);
        gpu.set_upscaler(Upscaler::SharpBilinear);
        assert_eq!(gpu.upscaler(), Upscaler::SharpBilinear);
        assert_eq!(gpu.world_resolution(), WorldResolution::Logical);
        gpu.set_upscaler(Upscaler::Native);
        assert_eq!(gpu.upscaler(), Upscaler::Native);
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
        assert_eq!(gpu.calls, [], "not a call");
    }

    #[test]
    fn answering_still_decides_the_resolution_until_an_upscaler_is_set() {
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);
        assert_eq!(gpu.world_resolution(), WorldResolution::Logical);
        gpu.set_upscaler(Upscaler::Native);
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
    }

    #[test]
    fn its_warnings_are_handed_out_once_and_are_no_call() {
        let mut gpu = RecordingGpu::new();
        assert_eq!(gpu.take_warnings(), Vec::<String>::new());
        gpu.warn("first");
        gpu.warn("second");
        assert_eq!(gpu.take_warnings(), ["first", "second"]);
        assert_eq!(gpu.take_warnings(), Vec::<String>::new(), "once");
        assert_eq!(gpu.calls, []);
    }

    #[test]
    fn its_probe_answers_native_alone_unless_given_a_list() {
        assert_eq!(RecordingGpu::new().upscalers(), [Upscaler::Native]);
        let both = [Upscaler::SharpBilinear, Upscaler::Native];
        let gpu = RecordingGpu::probing(&both);
        assert_eq!(gpu.upscalers(), both);
        assert_eq!(gpu.calls, []);
        assert_eq!(gpu.world_resolution(), WorldResolution::Native);
        assert_eq!(RecordingGpu::probing(&[]).upscalers(), []);
    }
}
