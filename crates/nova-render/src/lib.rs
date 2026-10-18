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
//! - [`gpu`]: the [`Gpu`] port and the [`Frame`]s handed to it. Each
//!   [`TextRun`] names its `nova_view` [`Font`](nova_view::Font).
//! - [`fonts`]: the font files text is drawn in ([`FontFaces`], with the
//!   bundled [`FALLBACK_FONT`] and [`SERIF_FONT`]) and which face draws
//!   each font ([`face_for`]).
//! - [`batch`]: the [`Renderer`], which packs images into the atlas lazily
//!   and turns a draw list into batches in draw order, keeping flight's
//!   world layer ([`nova_view::DrawLayer`]) apart from the overlay: no
//!   batch holds both.
//! - [`upscale`]: the [`Upscaler`] the adapter draws the world with
//!   ([`Gpu::set_upscaler`]), the [`UpscalerProbe`] port that says which
//!   the device can use and the list the player picks from ([`listed`]),
//!   and the [`WorldSpace`] a frame's world layer is drawn in, from the
//!   [`WorldResolution`] the [`Gpu`] draws it at ([`world_space`]).
//! - [`source`]: [`ImageSource`] for `nova_data`'s `GameData`, a thin
//!   adapter over the picture and sprite decoders (and the mask that gives
//!   a masked picture its alpha) that needs no GPU.
//! - [`present`]: what a window surface does before each frame.
//!
//! The adapter, [`wgpu`], is the only module that names `wgpu` or
//! `glyphon` types: [`wgpu::OffscreenGpu`] draws into its own texture and
//! reads it back, and [`wgpu::SurfaceGpu`] draws into a window. Both hand
//! the work to [`wgpu::WgpuRenderer`], which draws each frame into an
//! offscreen scene and blits it to the target; an OR batch reads a backdrop
//! copy of the rectangle the core says it covers, in its layer's pixels
//! ([`Frame::pixels_covered`]). The renderer draws the world with an
//! upscaler ([`wgpu::Upscale`]): native draws it straight into the scene
//! in the frame's order, as before there were layers, and
//! [`wgpu::SharpBilinear`] draws it into a texture of the logical size
//! and enlarges that onto the viewport, the overlay over it. The core
//! turns each OR tint into exact
//! levels and keeps overlapping OR quads in separate batches. [`wgpu::GlyphonMetrics`] is
//! `nova_view`'s text-metrics port over the same glyphon font system, with
//! no GPU: the widgets lay text out by the widths it is drawn at, and the
//! batcher spaces lines by `nova_view`'s shared line-height rule.
//!
//! The adapters are handed their fonts ([`FontFaces`]) when they are made.
//! They load those faces alone, never the system's fonts, so text draws the
//! same on every machine, and they draw each run in the face [`face_for`]
//! picks.
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
pub mod fonts;
pub mod gpu;
pub mod images;
pub mod present;
#[cfg(any(test, feature = "recording"))]
pub mod recording;
pub mod source;
pub mod upscale;
pub mod viewport;
pub mod wgpu;

pub use atlas::{Atlas, AtlasEntry, GUTTER, PAGE_SIZE, PackError, PageId, ShelfPacker, Uv};
pub use batch::{RenderReport, Renderer};
pub use fonts::{FALLBACK_FONT, Face, FontFaces, Loaded, SERIF_FONT, face_for};
pub use gpu::{Batch, Frame, Gpu, QuadInstance, Rect, SolidQuad, TextRun, WorldSpace};
pub use images::{ImageError, ImageSource};
pub use present::{AcquireOutcome, AcquireResult, SurfaceAction, acquire_outcome, surface_action};
pub use upscale::{Upscaler, UpscalerProbe, WorldResolution, listed, world_space};
pub use viewport::{LOGICAL, Letterbox, LogicalSize, PixelRect, Viewport};
