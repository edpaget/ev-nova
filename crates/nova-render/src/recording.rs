//! A [`Gpu`] that records what it is asked to do, for tests.
//!
//! Compiled for this crate's tests and, through the `recording` feature,
//! for other crates' tests.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use nova_data::graphics::Image;

use crate::gpu::{Frame, Gpu, PageId};
use crate::upscale::WorldResolution;
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
/// otherwise; the question is not a call.
#[derive(Clone, Debug, Default)]
pub struct RecordingGpu {
    /// The calls received, oldest first.
    pub calls: Vec<GpuCall>,
    world_resolution: WorldResolution,
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
}
