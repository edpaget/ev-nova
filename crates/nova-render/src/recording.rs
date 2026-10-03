//! A [`Gpu`] that records what it is asked to do, for tests.
//!
//! Compiled for this crate's tests and, through the `recording` feature,
//! for other crates' tests.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use nova_data::graphics::Image;

use crate::gpu::{Frame, Gpu, PageId};
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

/// Records every call, in order.
#[derive(Clone, Debug, Default)]
pub struct RecordingGpu {
    /// The calls received, oldest first.
    pub calls: Vec<GpuCall>,
}

impl RecordingGpu {
    /// A recorder with no calls.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
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
}
