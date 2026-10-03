//! Draws `nova-view` draw lists: sprites and pictures from texture atlases,
//! text, lines and dots, in a fixed logical space letterboxed into the
//! window.
//!
//! # Core and adapters
//!
//! The core decides everything and never names a `wgpu` or `glyphon` type:
//!
//! - [`viewport`]: the logical-to-window transform and letterbox.
//! - [`atlas`]: atlas pages and the shelf packer.
//! - [`images`]: the [`ImageSource`] port, decoded frames by resource.
//! - [`gpu`]: the [`Gpu`] port and the [`Frame`]s handed to it.
//! - [`batch`]: the [`Renderer`], which packs images into the atlas lazily
//!   and turns a draw list into batches in draw order.
//! - [`source`]: [`ImageSource`] for `nova_data`'s `GameData`, a thin
//!   adapter over the picture and sprite decoders that needs no GPU.
//! - [`present`]: what a window surface does before each frame.
//!
//! The adapter, [`wgpu`], is the only module that names `wgpu` or
//! `glyphon` types: [`wgpu::OffscreenGpu`] draws into its own texture and
//! reads it back, and [`wgpu::SurfaceGpu`] draws into a window. Both hand
//! the work to [`wgpu::WgpuRenderer`].
//!
//! Both adapters can also draw an overlay over each frame, after the
//! renderer and before it is presented or read back
//! ([`wgpu::OverlayGpu`], [`wgpu::OverlayPainter`]): [`wgpu::WithOverlay`]
//! is a [`Gpu`] that submits every frame with a painter over it. This is
//! still wgpu glue with no decisions in it, and the [`Gpu`] port is
//! unchanged.
//!
//! `recording::RecordingGpu` (this crate's tests, or the `recording`
//! feature) is a [`Gpu`] that records its calls.

pub mod atlas;
pub mod batch;
pub mod gpu;
pub mod images;
pub mod present;
#[cfg(any(test, feature = "recording"))]
pub mod recording;
pub mod source;
pub mod viewport;
pub mod wgpu;

pub use atlas::{Atlas, AtlasEntry, GUTTER, PAGE_SIZE, PackError, PageId, ShelfPacker, Uv};
pub use batch::{RenderReport, Renderer};
pub use gpu::{Batch, Frame, Gpu, QuadInstance, Rect, SolidQuad, TextRun};
pub use images::{ImageError, ImageSource};
pub use present::{AcquireOutcome, AcquireResult, SurfaceAction, acquire_outcome, surface_action};
pub use viewport::{LOGICAL, Letterbox, LogicalSize, PixelRect, Viewport};
