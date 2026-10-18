//! The renderer: resolves image keys into the atlas and turns draw lists
//! into frames of batches.

use std::collections::{HashMap, HashSet};

use nova_data::graphics::Image;
use nova_view::geometry::Bounds;
use nova_view::text::LINE_HEIGHT;
use nova_view::{Blend, Color, DrawCommand, DrawLayer, DrawList, ImageKey, ImageKind, Point};

use crate::atlas::{Atlas, AtlasEntry, PAGE_SIZE, Uv};
use crate::enlarge::{Enlarger, NativeDetail};
use crate::gpu::{Batch, Frame, Gpu, QuadInstance, Rect, SolidQuad, TextRun};
use crate::images::{ImageError, ImageSource};
use crate::upscale::world_space;
use crate::viewport::{PixelRect, Viewport};

/// What went wrong while rendering one frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderReport {
    /// Images that failed for the first time this frame, each with the
    /// first key that hit it. A resource that fails as a whole (missing or
    /// undecodable) is listed once; a frame that fails on its own (out of
    /// range or too large) is listed once per key. Later frames skip them
    /// silently.
    pub new_failures: Vec<(ImageKey, ImageError)>,
}

/// A frame's place in the atlas and the size it is drawn at.
#[derive(Clone, Copy, Debug)]
struct Placed {
    /// Where the atlas holds it, perhaps enlarged.
    entry: AtlasEntry,
    /// The decoded frame's width and height: the logical size it is drawn
    /// at, whatever size the atlas holds it.
    size: (f32, f32),
}

/// One resource's frames once it has been asked for.
#[derive(Debug)]
enum Resident {
    /// Each frame's placement, or why it has none.
    Frames(Vec<Result<Placed, ImageError>>),
    /// The whole resource failed.
    Failed,
}

/// Turns draw lists into frames for a [`Gpu`], packing images into the
/// atlas the first time a draw list names them, each as its [`Enlarger`]
/// turns it, and drawing each at its own logical size.
#[derive(Debug)]
pub struct Renderer<S> {
    images: S,
    atlas: Atlas,
    resident: HashMap<(ImageKind, i16), Resident>,
    reported: HashSet<ImageKey>,
    /// What each decoded frame becomes in the atlas.
    enlarger: Box<dyn Enlarger>,
    /// Whether the atlas starts afresh at the next frame, for a new
    /// enlarger.
    rebuild: bool,
}

impl<S: ImageSource> Renderer<S> {
    /// A renderer drawing images from `images` on [`PAGE_SIZE`] pages, as
    /// they decode ([`NativeDetail`]).
    #[must_use]
    pub fn new(images: S) -> Self {
        Self::with_page_size(images, PAGE_SIZE)
    }

    /// A renderer drawing images from `images` on `page_size` pages, as
    /// they decode ([`NativeDetail`]).
    #[must_use]
    pub fn with_page_size(images: S, page_size: u32) -> Self {
        Self {
            images,
            atlas: Atlas::new(page_size),
            resident: HashMap::new(),
            reported: HashSet::new(),
            enlarger: Box::new(NativeDetail),
            rebuild: false,
        }
    }

    /// The image source.
    pub fn images(&self) -> &S {
        &self.images
    }

    /// Places images in the atlas as `enlarger` turns them from the next
    /// frame drawn on. That frame starts the atlas afresh: it drops every
    /// page on the GPU ([`Gpu::clear_pages`], when any is open) and every
    /// image placed, then loads again, through `enlarger`, each image it
    /// draws. A resource that failed as a whole stays failed, and no
    /// failure is reported again. It needs no GPU, so it can be called
    /// before the first frame.
    pub fn set_enlarger(&mut self, enlarger: Box<dyn Enlarger>) {
        self.enlarger = enlarger;
        self.rebuild = true;
    }

    /// Starts the atlas afresh, if a new enlarger asks it to.
    fn rebuild(&mut self, gpu: &mut impl Gpu) {
        if !std::mem::take(&mut self.rebuild) {
            return;
        }
        if self.atlas.page_count() > 0 {
            gpu.clear_pages();
        }
        self.atlas = Atlas::new(self.atlas.page_size());
        self.resident
            .retain(|_, resident| matches!(resident, Resident::Failed));
    }

    /// Draws `list` into `viewport` through `gpu`: uploads images seen for
    /// the first time, then submits one frame. A zero-area viewport draws
    /// nothing. Text that is empty, or whose size is not a positive, finite
    /// number of pixels, is skipped, as is a stretched or tiled picture
    /// whose width or height is not a positive, finite number.
    ///
    /// The frame's batches keep the list's order. Each run of the list's
    /// world commands ([`DrawList::world`]) is batched apart from the
    /// overlay's around it, so no batch holds both, and its batches are
    /// marked world ([`Frame::world`]). The world is drawn where `gpu`'s
    /// resolution for it says ([`world_space`]). Text is never world, so
    /// every text run is in target pixels.
    ///
    /// A frame drawn after [`Renderer::set_enlarger`] first starts the
    /// atlas afresh; a zero-area viewport leaves that to the next frame.
    pub fn render(
        &mut self,
        list: &DrawList,
        viewport: &Viewport,
        gpu: &mut impl Gpu,
    ) -> RenderReport {
        let mut report = RenderReport::default();
        let (Some((rect, _)), Some(scale)) = (viewport.content(), viewport.axis_scales()) else {
            return report;
        };
        self.rebuild(gpu);
        let target = Target { rect, scale };
        let mut layers = Layers::default();
        for (layer, command) in list.layered() {
            let run = layers.run_in(layer);
            self.draw(command, None, target, run, gpu, &mut report);
        }
        let Layers { batches, world, .. } = layers.settled();
        gpu.submit(&Frame {
            target: viewport.window_px(),
            viewport: rect,
            logical: viewport.logical(),
            clear: Color::BLACK,
            world_space: world_space(gpu.world_resolution(), viewport),
            batches,
            world,
        });
        report
    }

    /// Appends `command`'s batches, drawn only inside `clip` when it is
    /// given (see [`DrawCommand::Clipped`]).
    #[allow(clippy::too_many_lines)] // one arm for each command
    fn draw(
        &mut self,
        command: &DrawCommand,
        clip: Option<Bounds>,
        target: Target,
        batches: &mut Vec<Batch>,
        gpu: &mut impl Gpu,
        report: &mut RenderReport,
    ) {
        let Target { rect, scale } = target;
        match *command {
            DrawCommand::Sprite {
                image,
                center,
                tint,
                blend,
            } => {
                if let Some(Placed {
                    entry,
                    size: (w, h),
                }) = self.entry(image, gpu, report)
                {
                    let dest = Rect {
                        x: center.x - w / 2.0,
                        y: center.y - h / 2.0,
                        w,
                        h,
                    };
                    if let Some((entry, dest)) = cropped(&entry, dest, clip) {
                        push_quad(batches, &entry, dest, tint, blend);
                    }
                }
            }
            DrawCommand::Picture { image, top_left } => {
                let picture = (image, top_left, None);
                self.push_picture(batches, picture, Blend::Normal, clip, gpu, report);
            }
            DrawCommand::StretchedPicture {
                image,
                top_left,
                width,
                height,
            } => {
                let picture = (image, top_left, (width, height));
                self.push_stretched(batches, picture, Blend::Normal, clip, gpu, report);
            }
            DrawCommand::BlendedPicture {
                image,
                top_left,
                width,
                height,
                blend,
            } => {
                let picture = (image, top_left, (width, height));
                self.push_stretched(batches, picture, blend, clip, gpu, report);
            }
            DrawCommand::TiledPicture {
                image,
                area,
                origin,
            } => {
                let area = match clip {
                    Some(clip) => meet(area, clip),
                    None => Some(area),
                };
                // An area with no drawable size draws nothing, so its
                // picture is not even loaded.
                if let Some(area) = area
                    && drawable(area.width())
                    && drawable(area.height())
                    && let Some(placed) = self.entry(image, gpu, report)
                {
                    push_tiles(batches, &placed.entry, placed.size, area, origin);
                }
            }
            DrawCommand::Text {
                ref text,
                font,
                origin,
                size,
                wrap_width,
                color,
            } => {
                let (across, down) = scale;
                // Glyphs are sized by the height: the axes' scales differ
                // only in a widened view, by under half a unit's width.
                let size_px = size * down;
                let line_height_px = LINE_HEIGHT * size_px;
                // Text with nothing in it draws nothing. Nor does text
                // whose size is zero, negative, NaN or too big to give
                // a finite line height in pixels, and the text shaper
                // would panic on some of those, so skip both.
                if text.is_empty() || !(line_height_px.is_finite() && line_height_px > 0.0) {
                    return;
                }
                let Some(clip) = clip.map_or(Some(rect), |clip| pixel_clip(rect, scale, clip))
                else {
                    return;
                };
                let run = TextRun {
                    text: text.clone(),
                    font,
                    origin_px: (
                        rect.x as f32 + origin.x * across,
                        rect.y as f32 + origin.y * down,
                    ),
                    size_px,
                    line_height_px,
                    wrap_px: wrap_width.map(|w| w * across),
                    color,
                    clip,
                };
                if let Some(Batch::Text(runs)) = batches.last_mut() {
                    runs.push(run);
                } else {
                    batches.push(Batch::Text(vec![run]));
                }
            }
            DrawCommand::Line {
                from,
                to,
                width,
                color,
            } => push_solid(batches, line_corners(from, to, width), color),
            DrawCommand::Dot {
                center,
                size,
                color,
            } => push_solid(batches, square(center, size), color),
            DrawCommand::Clipped {
                clip: inner,
                ref command,
            } => {
                let clip = match clip {
                    Some(outer) => meet(outer, inner),
                    None => Some(inner),
                };
                // A clip with no area draws nothing at all.
                if let Some(clip) = clip {
                    self.draw(command, Some(clip), target, batches, gpu, report);
                }
            }
        }
    }

    /// Appends a stretched picture's quad, `(image, top_left, size)`,
    /// combined by `blend`. A rectangle with no drawable size draws
    /// nothing, so its picture is not even loaded.
    fn push_stretched(
        &mut self,
        batches: &mut Vec<Batch>,
        (image, top_left, (width, height)): (ImageKey, Point, (f32, f32)),
        blend: Blend,
        clip: Option<Bounds>,
        gpu: &mut impl Gpu,
        report: &mut RenderReport,
    ) {
        if drawable(width) && drawable(height) {
            let picture = (image, top_left, Some((width, height)));
            self.push_picture(batches, picture, blend, clip, gpu, report);
        }
    }

    /// Appends a picture's quad: `(image, top_left, size)`, drawn at `size`
    /// when given and at the picture's own size otherwise, combined by
    /// `blend`.
    fn push_picture(
        &mut self,
        batches: &mut Vec<Batch>,
        (image, top_left, size): (ImageKey, Point, Option<(f32, f32)>),
        blend: Blend,
        clip: Option<Bounds>,
        gpu: &mut impl Gpu,
        report: &mut RenderReport,
    ) {
        if let Some(Placed { entry, size: own }) = self.entry(image, gpu, report) {
            let (w, h) = size.unwrap_or(own);
            let dest = Rect {
                x: top_left.x,
                y: top_left.y,
                w,
                h,
            };
            if let Some((entry, dest)) = cropped(&entry, dest, clip) {
                push_quad(batches, &entry, dest, Color::WHITE, blend);
            }
        }
    }

    /// The placement of `key`, loading its resource on first use. Reports
    /// each new failure.
    fn entry(
        &mut self,
        key: ImageKey,
        gpu: &mut impl Gpu,
        report: &mut RenderReport,
    ) -> Option<Placed> {
        let resource = (key.kind, key.id);
        if !self.resident.contains_key(&resource) {
            let loaded = self.load(key, gpu, report);
            self.resident.insert(resource, loaded);
        }
        let Resident::Frames(frames) = &self.resident[&resource] else {
            return None;
        };
        let error = match frames.get(usize::from(key.frame)) {
            Some(Ok(placed)) => return Some(*placed),
            Some(Err(error)) => error.clone(),
            None => ImageError::NoFrame {
                frame: key.frame,
                count: frames.len(),
            },
        };
        if self.reported.insert(key) {
            report.new_failures.push((key, error));
        }
        None
    }

    /// Decodes every frame of `key`'s resource and places and uploads each.
    fn load(&mut self, key: ImageKey, gpu: &mut impl Gpu, report: &mut RenderReport) -> Resident {
        match self.images.frames(key.kind, key.id) {
            Err(error) => {
                report.new_failures.push((key, error));
                Resident::Failed
            }
            Ok(frames) => Resident::Frames(
                frames
                    .iter()
                    .map(|image| self.place(key.kind, image, gpu))
                    .collect(),
            ),
        }
    }

    /// Places `image`, a frame of a `kind` resource, in the atlas as the
    /// enlarger turns it, and uploads it. An enlarged frame too big for a
    /// page is placed as it decoded instead, so every frame that fits
    /// natively draws.
    fn place(
        &mut self,
        kind: ImageKind,
        image: &Image,
        gpu: &mut impl Gpu,
    ) -> Result<Placed, ImageError> {
        let enlarged = self.enlarger.enlarge(kind, image);
        let (entry, held) = match self.atlas.place(enlarged.width(), enlarged.height()) {
            Ok(entry) => (entry, enlarged.as_ref()),
            Err(_) => (self.atlas.place(image.width(), image.height())?, image),
        };
        if entry.opened {
            gpu.create_page(entry.page, self.atlas.page_size());
        }
        gpu.upload(entry.page, entry.rect, held);
        Ok(Placed {
            entry,
            size: (image.width() as f32, image.height() as f32),
        })
    }
}

/// Where a frame draws: the viewport's content rect in window pixels, and
/// how many pixels a logical unit is across and down
/// ([`Viewport::axis_scales`]).
#[derive(Clone, Copy, Debug)]
struct Target {
    rect: PixelRect,
    scale: (f32, f32),
}

/// A frame's batches as they are made, in draw order, each run of one
/// layer's commands batched on its own so that no batch holds both.
#[derive(Debug, Default)]
struct Layers {
    /// The batches of the runs settled so far.
    batches: Vec<Batch>,
    /// Which of `batches` are world.
    world: Vec<usize>,
    /// The run being batched, and its layer.
    run: Vec<Batch>,
    layer: DrawLayer,
}

impl Layers {
    /// The batches a command in `layer` extends: the current run's, once
    /// any run of the other layer is settled.
    fn run_in(&mut self, layer: DrawLayer) -> &mut Vec<Batch> {
        if layer != self.layer {
            self.settle();
            self.layer = layer;
        }
        &mut self.run
    }

    /// Appends the current run's batches to the frame's.
    fn settle(&mut self) {
        if self.layer == DrawLayer::World {
            let start = self.batches.len();
            self.world.extend(start..start + self.run.len());
        }
        self.batches.append(&mut self.run);
    }

    /// Every run settled.
    fn settled(mut self) -> Self {
        self.settle();
        self
    }
}

/// Where `a` and `b` overlap, if they share any area.
fn meet(a: Bounds, b: Bounds) -> Option<Bounds> {
    let min = Point::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
    let max = Point::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
    (min.x < max.x && min.y < max.y).then_some(Bounds { min, max })
}

/// `entry`'s quad at `dest` cropped to `clip`, its texture coordinates
/// cut to match; as it is with no clip, and none when nothing of it is
/// inside.
fn cropped(entry: &AtlasEntry, dest: Rect, clip: Option<Bounds>) -> Option<(AtlasEntry, Rect)> {
    let Some(clip) = clip else {
        return Some((*entry, dest));
    };
    let whole = Bounds {
        min: Point::new(dest.x, dest.y),
        max: Point::new(dest.x + dest.w, dest.y + dest.h),
    };
    let kept = meet(whole, clip)?;
    if kept == whole {
        return Some((*entry, dest));
    }
    let uv = entry.uv;
    let (du, dv) = (uv.u1 - uv.u0, uv.v1 - uv.v0);
    let u = |x: f32| (x - dest.x) / dest.w * du + uv.u0;
    let v = |y: f32| (y - dest.y) / dest.h * dv + uv.v0;
    let cut = AtlasEntry {
        uv: Uv {
            u0: u(kept.min.x),
            v0: v(kept.min.y),
            u1: u(kept.max.x),
            v1: v(kept.max.y),
        },
        ..*entry
    };
    let dest = Rect {
        x: kept.min.x,
        y: kept.min.y,
        w: kept.width(),
        h: kept.height(),
    };
    Some((cut, dest))
}

/// `clip`, in logical units, as window pixels inside `rect`, the
/// viewport's content at `scale` across and down, each edge rounded to
/// the nearest pixel; none when nothing of it is inside.
fn pixel_clip(rect: PixelRect, (across, down): (f32, f32), clip: Bounds) -> Option<PixelRect> {
    let to_px = |origin: u32, logical: f32, scale: f32| (origin as f32 + logical * scale).round();
    let left = to_px(rect.x, clip.min.x, across).max(rect.x as f32);
    let top = to_px(rect.y, clip.min.y, down).max(rect.y as f32);
    let right = to_px(rect.x, clip.max.x, across).min((rect.x + rect.w) as f32);
    let bottom = to_px(rect.y, clip.max.y, down).min((rect.y + rect.h) as f32);
    (left < right && top < bottom).then_some(PixelRect {
        x: left as u32,
        y: top as u32,
        w: (right - left) as u32,
        h: (bottom - top) as u32,
    })
}

/// Whether a side `side` long can be drawn: a positive, finite number.
fn drawable(side: f32) -> bool {
    side.is_finite() && side > 0.0
}

/// Appends the quads tiling `area` with `entry`'s picture, each tile `(w,
/// h)`, the picture's logical size: copies of it edge to edge from
/// `origin`, each cut to `area` with its texture coordinates cut to
/// match. A picture with no size draws nothing: its first edge, a
/// division by 0 times 0, is NaN, which no loop runs past.
fn push_tiles(
    batches: &mut Vec<Batch>,
    entry: &AtlasEntry,
    (w, h): (f32, f32),
    area: Bounds,
    origin: Point,
) {
    let uv = entry.uv;
    let (du, dv) = (uv.u1 - uv.u0, uv.v1 - uv.v0);
    let mut top = ((area.min.y - origin.y) / h).floor().mul_add(h, origin.y);
    while top < area.max.y {
        let (y0, y1) = (top.max(area.min.y), (top + h).min(area.max.y));
        let mut left = ((area.min.x - origin.x) / w).floor().mul_add(w, origin.x);
        while left < area.max.x {
            let (x0, x1) = (left.max(area.min.x), (left + w).min(area.max.x));
            let cut = AtlasEntry {
                uv: Uv {
                    u0: (x0 - left) / w * du + uv.u0,
                    v0: (y0 - top) / h * dv + uv.v0,
                    u1: (x1 - left) / w * du + uv.u0,
                    v1: (y1 - top) / h * dv + uv.v0,
                },
                ..*entry
            };
            let dest = Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            };
            push_quad(batches, &cut, dest, Color::WHITE, Blend::Normal);
            left += w;
        }
        top += h;
    }
}

/// `color` as 0-to-1 floats.
fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0)
}

/// The per-channel levels, in 32nds, that `tint` asks a [`Blend::Or`]
/// sprite to be scaled by: `round(c × a × 32 / 255²)` for each colour
/// channel `c` and the alpha `a`. [`lights_tint`](nova_view::draw::lights_tint)
/// `(n)` is exactly n in every channel.
fn or_levels(tint: Color) -> [u8; 3] {
    const FULL: u32 = 255 * 255;
    let alpha = u32::from(tint.a);
    [tint.r, tint.g, tint.b].map(|c| ((u32::from(c) * alpha * 32 + FULL / 2) / FULL) as u8)
}

/// Whether `a` and `b` share any area; rectangles that only touch along
/// an edge do not.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Appends a quad, extending the last batch when it is the same page's
/// and blends the same way. An [`Blend::Or`] quad's tint becomes its
/// levels as fractions of 32 (alpha 1), and it starts a new batch when it
/// overlaps a quad already in the last one: every quad of an OR batch
/// reads the same copy of what is beneath, so a later overlapping one
/// would not see the earlier.
fn push_quad(batches: &mut Vec<Batch>, entry: &AtlasEntry, dest: Rect, tint: Color, blend: Blend) {
    let tint = if blend == Blend::Or {
        let [r, g, b] = or_levels(tint).map(|n| f32::from(n) / 32.0);
        [r, g, b, 1.0]
    } else {
        rgba(tint)
    };
    let quad = QuadInstance {
        dest,
        uv: entry.uv,
        tint,
    };
    if let Some(Batch::Sprites {
        page,
        blend: last,
        quads,
    }) = batches.last_mut()
        && *page == entry.page
        && *last == blend
        && !(blend == Blend::Or && quads.iter().any(|q| overlaps(q.dest, dest)))
    {
        quads.push(quad);
        return;
    }
    batches.push(Batch::Sprites {
        page: entry.page,
        blend,
        quads: vec![quad],
    });
}

/// Appends an untextured quad, extending the last batch when it is solid.
fn push_solid(batches: &mut Vec<Batch>, corners: [Point; 4], color: Color) {
    let quad = SolidQuad {
        corners,
        color: rgba(color),
    };
    if let Some(Batch::Solid(quads)) = batches.last_mut() {
        quads.push(quad);
    } else {
        batches.push(Batch::Solid(vec![quad]));
    }
}

/// The corners of a line `width` thick: the segment offset half the width
/// either side along its normal. A zero-length line is a square.
fn line_corners(from: Point, to: Point, width: f32) -> [Point; 4] {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return square(from, width);
    }
    let half = width / 2.0;
    let (nx, ny) = (-dy * half / length, dx * half / length);
    [
        Point::new(from.x + nx, from.y + ny),
        Point::new(to.x + nx, to.y + ny),
        Point::new(to.x - nx, to.y - ny),
        Point::new(from.x - nx, from.y - ny),
    ]
}

/// The corners of a `size`-wide square centred on `center`.
fn square(center: Point, size: f32) -> [Point; 4] {
    let half = size / 2.0;
    [
        Point::new(center.x - half, center.y - half),
        Point::new(center.x + half, center.y - half),
        Point::new(center.x + half, center.y + half),
        Point::new(center.x - half, center.y + half),
    ]
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::borrow::Cow;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use nova_data::graphics::Image;
    use nova_view::draw::lights_tint;
    use nova_view::{Blend, Color, DrawList, Font, ImageKey, ImageKind, Point};

    use super::*;
    use crate::enlarge::{Enlarger, NativeDetail, enlarger_for};
    use crate::gpu::{Batch, Frame, QuadInstance, Rect, SolidQuad, TextRun, WorldSpace};
    use crate::recording::{GpuCall, RecordingGpu};
    use crate::super_xbr::SuperXbr;
    use crate::upscale::WorldResolution;
    use crate::viewport::{LogicalSize, PixelRect, Viewport};
    use crate::{PackError, PageId, Uv};
    use nova_view::DrawLayer;

    /// Canned frames by resource; `Missing` for anything else. Records
    /// every request.
    #[derive(Default)]
    struct FakeImages {
        resources: BTreeMap<(ImageKind, i16), Result<Vec<Image>, ImageError>>,
        calls: RefCell<Vec<(ImageKind, i16)>>,
    }

    impl FakeImages {
        fn with(mut self, kind: ImageKind, id: i16, frames: Vec<Image>) -> Self {
            self.resources.insert((kind, id), Ok(frames));
            self
        }

        fn failing(mut self, kind: ImageKind, id: i16, error: ImageError) -> Self {
            self.resources.insert((kind, id), Err(error));
            self
        }

        fn calls(&self) -> Vec<(ImageKind, i16)> {
            self.calls.borrow().clone()
        }
    }

    impl ImageSource for FakeImages {
        fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            self.calls.borrow_mut().push((kind, id));
            self.resources
                .get(&(kind, id))
                .cloned()
                .unwrap_or(Err(ImageError::Missing))
        }
    }

    /// A `w` x `h` image of one colour.
    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Image {
        Image::from_rgba(w, h, rgba.repeat((w * h) as usize)).unwrap()
    }

    fn at(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }

    fn rect(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
        PixelRect { x, y, w, h }
    }

    fn uv(x: u32, y: u32, w: u32, h: u32) -> Uv {
        let page = 64.0;
        Uv {
            u0: x as f32 / page,
            v0: y as f32 / page,
            u1: (x + w) as f32 / page,
            v1: (y + h) as f32 / page,
        }
    }

    const SMALL: LogicalSize = LogicalSize { w: 64, h: 48 };
    const WHITE: [f32; 4] = [1.0; 4];

    /// 64x48 logical at scale 2, filling a 128x96 window.
    fn viewport() -> Viewport {
        Viewport::new(SMALL, (128, 96), 2.0)
    }

    fn red_picture() -> Image {
        solid(8, 4, [255, 0, 0, 255])
    }

    fn sheet() -> Vec<Image> {
        vec![
            solid(5, 6, [0, 0, 255, 255]),
            solid(5, 6, [0, 255, 0, 255]),
            solid(5, 6, [255, 255, 0, 128]),
        ]
    }

    fn images() -> FakeImages {
        FakeImages::default()
            .with(ImageKind::Pict, 128, vec![red_picture()])
            .with(ImageKind::Rled, 200, sheet())
    }

    fn renderer(images: FakeImages) -> Renderer<FakeImages> {
        Renderer::with_page_size(images, 64)
    }

    /// The one call, a submit of a frame with no world layer.
    fn only_submit(calls: &[GpuCall]) -> &Frame {
        match calls {
            [GpuCall::Submit(frame)] => {
                assert_eq!(frame.world, [0_usize; 0], "no world");
                frame
            }
            other => panic!("expected one submit, got {other:?}"),
        }
    }

    #[test]
    fn pictures_and_sprites_upload_then_draw_in_order() {
        let tint = Color::rgba(51, 102, 204, 255);
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(10.0, 20.0))
            .sprite(ImageKey::sprite(200, 2), at(30.0, 40.0), tint)
            .sprite(ImageKey::sprite(200, 0), at(31.0, 41.0), Color::WHITE);
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        let sheet = sheet();
        let picture_at = rect(0, 0, 8, 4);
        let frame_at = [rect(0, 5, 5, 6), rect(6, 5, 5, 6), rect(12, 5, 5, 6)];
        let expected_quads = vec![
            QuadInstance {
                dest: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 8.0,
                    h: 4.0,
                },
                uv: uv(0, 0, 8, 4),
                tint: WHITE,
            },
            QuadInstance {
                dest: Rect {
                    x: 27.5,
                    y: 37.0,
                    w: 5.0,
                    h: 6.0,
                },
                uv: uv(12, 5, 5, 6),
                tint: [51.0 / 255.0, 102.0 / 255.0, 204.0 / 255.0, 1.0],
            },
            QuadInstance {
                dest: Rect {
                    x: 28.5,
                    y: 38.0,
                    w: 5.0,
                    h: 6.0,
                },
                uv: uv(0, 5, 5, 6),
                tint: WHITE,
            },
        ];
        assert_eq!(
            gpu.calls,
            vec![
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), picture_at, &red_picture()),
                GpuCall::upload(PageId(0), frame_at[0], &sheet[0]),
                GpuCall::upload(PageId(0), frame_at[1], &sheet[1]),
                GpuCall::upload(PageId(0), frame_at[2], &sheet[2]),
                GpuCall::Submit(Frame {
                    target: (128, 96),
                    viewport: rect(0, 0, 128, 96),
                    logical: SMALL,
                    clear: Color::BLACK,
                    world_space: WorldSpace::Target,
                    batches: vec![Batch::Sprites {
                        page: PageId(0),
                        blend: Blend::Normal,
                        quads: expected_quads,
                    }],
                    world: Vec::new(),
                }),
            ]
        );
    }

    #[test]
    fn a_stretched_picture_fills_its_rectangle_from_the_whole_picture() {
        let mut list = DrawList::new();
        list.stretched_picture(ImageKey::picture(128), at(-2.5, 3.0), 20.0, 1.5);
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        assert_eq!(
            gpu.submits()[0].batches,
            vec![Batch::Sprites {
                page: PageId(0),
                blend: Blend::Normal,
                quads: vec![QuadInstance {
                    dest: Rect {
                        x: -2.5,
                        y: 3.0,
                        w: 20.0,
                        h: 1.5,
                    },
                    uv: uv(0, 0, 8, 4),
                    tint: WHITE,
                }],
            }]
        );
    }

    #[test]
    fn a_stretched_picture_shares_its_pages_batch_with_sprites() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(200, 0), at(10.0, 10.0), Color::WHITE)
            .stretched_picture(ImageKey::picture(128), at(0.0, 0.0), 16.0, 8.0)
            .picture(ImageKey::picture(128), at(1.0, 1.0));
        let frame = render_one(&list, &viewport());
        let dests: Vec<(f32, f32)> = match frame.batches.as_slice() {
            [Batch::Sprites { quads, .. }] => quads.iter().map(|q| (q.dest.w, q.dest.h)).collect(),
            other => panic!("one sprite batch: {other:?}"),
        };
        assert_eq!(dests, [(5.0, 6.0), (16.0, 8.0), (8.0, 4.0)]);
    }

    #[test]
    fn a_stretched_picture_with_no_drawable_size_draws_and_loads_nothing() {
        let mut list = DrawList::new();
        for (w, h) in [
            (0.0, 4.0),
            (4.0, 0.0),
            (-1.0, 4.0),
            (4.0, -1.0),
            (f32::NAN, 4.0),
            (4.0, f32::NAN),
            (f32::INFINITY, 4.0),
            (4.0, f32::INFINITY),
        ] {
            list.stretched_picture(ImageKey::picture(128), at(0.0, 0.0), w, h);
        }
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        assert_eq!(only_submit(&gpu.calls).batches, vec![]);
        assert_eq!(renderer.images().calls(), []);
    }

    /// A 16x16 `ppat` 130, placed at the page's top-left.
    fn tiles() -> FakeImages {
        images().with(ImageKind::Pattern, 130, vec![solid(16, 16, [7, 7, 7, 255])])
    }

    fn area(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    fn tiled_quads(area: Bounds) -> Vec<QuadInstance> {
        let mut list = DrawList::new();
        list.tiled_picture(ImageKey::pattern(130), area);
        let frame = render_one_with(tiles(), &list);
        match frame.batches.as_slice() {
            [Batch::Sprites { quads, blend, .. }] => {
                assert_eq!(*blend, Blend::Normal);
                quads.clone()
            }
            other => panic!("one sprite batch: {other:?}"),
        }
    }

    fn render_one_with(images: FakeImages, list: &DrawList) -> Frame {
        let mut renderer = renderer(images);
        let mut gpu = RecordingGpu::new();
        let report = renderer.render(list, &viewport(), &mut gpu);
        assert_eq!(report, RenderReport::default());
        let frame = gpu.submits()[0].clone();
        assert_eq!(frame.world, [0_usize; 0], "no world");
        frame
    }

    /// The quad showing the part of the tile at `tile` (its top-left) from
    /// `(x0, y0)` to `(x1, y1)`, the tile 16 square at the page's
    /// top-left, of 64.
    fn tile_quad(tile: (f32, f32), (x0, y0): (f32, f32), (x1, y1): (f32, f32)) -> QuadInstance {
        let to_uv = |offset: f32| offset / 16.0 * 0.25;
        QuadInstance {
            dest: Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            },
            uv: Uv {
                u0: to_uv(x0 - tile.0),
                v0: to_uv(y0 - tile.1),
                u1: to_uv(x1 - tile.0),
                v1: to_uv(y1 - tile.1),
            },
            tint: WHITE,
        }
    }

    #[test]
    fn a_tiled_picture_repeats_from_the_origin_cut_to_its_area() {
        // Columns cut at 16, 32 and 48; rows at 16 and 32.
        let columns = [
            (0.0, 10.0, 16.0),
            (16.0, 16.0, 32.0),
            (32.0, 32.0, 48.0),
            (48.0, 48.0, 50.0),
        ];
        let rows = [(0.0, 5.0, 16.0), (16.0, 16.0, 32.0), (32.0, 32.0, 45.0)];
        let mut expected = Vec::new();
        for (tile_y, y0, y1) in rows {
            for (tile_x, x0, x1) in columns {
                expected.push(tile_quad((tile_x, tile_y), (x0, y0), (x1, y1)));
            }
        }
        assert_eq!(tiled_quads(area(10.0, 5.0, 50.0, 45.0)), expected);
    }

    #[test]
    fn tiles_left_of_and_above_the_origin_line_up_with_it() {
        assert_eq!(
            tiled_quads(area(-10.0, -10.0, 5.0, 5.0)),
            [
                tile_quad((-16.0, -16.0), (-10.0, -10.0), (0.0, 0.0)),
                tile_quad((0.0, -16.0), (0.0, -10.0), (5.0, 0.0)),
                tile_quad((-16.0, 0.0), (-10.0, 0.0), (0.0, 5.0)),
                tile_quad((0.0, 0.0), (0.0, 0.0), (5.0, 5.0)),
            ]
        );
    }

    #[test]
    fn a_tiled_picture_moved_with_its_origin_draws_the_same_tiles_moved() {
        let mut at_home = DrawList::new();
        at_home.tiled_picture(ImageKey::pattern(130), area(10.0, 5.0, 50.0, 45.0));
        let mut moved = DrawList::new();
        // Far enough down that the tiling's first row moves with it.
        moved.extend_translated(&at_home, at(170.0, 40.0));
        let quads = |list: &DrawList| match &render_one_with(tiles(), list).batches[..] {
            [Batch::Sprites { quads, .. }] => quads.clone(),
            other => panic!("{other:?}"),
        };
        let expected: Vec<QuadInstance> = quads(&at_home)
            .into_iter()
            .map(|quad| QuadInstance {
                dest: Rect {
                    x: quad.dest.x + 170.0,
                    y: quad.dest.y + 40.0,
                    ..quad.dest
                },
                ..quad
            })
            .collect();
        assert_eq!(quads(&moved), expected);
    }

    #[test]
    fn an_area_on_tile_edges_takes_whole_tiles_and_no_more() {
        assert_eq!(
            tiled_quads(area(16.0, 32.0, 32.0, 48.0)),
            [tile_quad((16.0, 32.0), (16.0, 32.0), (32.0, 48.0))]
        );
    }

    #[test]
    fn a_tiled_picture_with_no_drawable_size_draws_and_loads_nothing() {
        let mut list = DrawList::new();
        for (w, h) in [
            (0.0, 4.0),
            (4.0, 0.0),
            (-1.0, 4.0),
            (4.0, -1.0),
            (f32::NAN, 4.0),
            (4.0, f32::NAN),
            (f32::INFINITY, 4.0),
            (4.0, f32::INFINITY),
        ] {
            list.tiled_picture(ImageKey::pattern(130), Bounds::at(at(0.0, 0.0), w, h));
        }
        let mut renderer = renderer(tiles());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        assert_eq!(only_submit(&gpu.calls).batches, vec![]);
        assert_eq!(renderer.images().calls(), []);
    }

    #[test]
    fn a_missing_pattern_draws_nothing_and_is_reported_once() {
        let mut list = DrawList::new();
        list.tiled_picture(ImageKey::pattern(131), area(0.0, 0.0, 20.0, 20.0))
            .tiled_picture(ImageKey::pattern(131), area(0.0, 0.0, 20.0, 20.0));
        let mut renderer = renderer(tiles());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(
            report.new_failures,
            [(ImageKey::pattern(131), ImageError::Missing)]
        );
        assert_eq!(only_submit(&gpu.calls).batches, vec![]);
    }

    #[test]
    fn a_tiles_texture_coordinates_are_cut_from_its_place_on_the_page() {
        // A 60x5 picture fills the first shelf and a 10x16 sprite starts
        // the second, so the pattern lands away from the page's edges.
        let images = tiles()
            .with(ImageKind::Pict, 1, vec![solid(60, 5, [1, 1, 1, 255])])
            .with(ImageKind::Rled, 2, vec![solid(10, 16, [2, 2, 2, 255])]);
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(1), at(0.0, 0.0))
            .sprite(ImageKey::sprite(2, 0), at(0.0, 0.0), Color::WHITE)
            .tiled_picture(ImageKey::pattern(130), area(4.0, 8.0, 20.0, 16.0));
        let mut renderer = renderer(images);
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        let placed = gpu
            .calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::Upload { rect, .. } => Some(*rect),
                _ => None,
            })
            .next_back()
            .expect("the pattern uploaded");
        assert!(placed.x > 0 && placed.y > 0, "{placed:?}");
        let quads = match gpu.submits()[0].batches.as_slice() {
            [Batch::Sprites { quads, .. }] => quads.clone(),
            other => panic!("one sprite batch: {other:?}"),
        };
        let page = |texel: u32, offset: f32| (texel as f32 + offset) / 64.0;
        let cut = |x0: f32, x1: f32| Uv {
            u0: page(placed.x, x0),
            v0: page(placed.y, 8.0),
            u1: page(placed.x, x1),
            v1: page(placed.y, 16.0),
        };
        let uvs: Vec<Uv> = quads[2..].iter().map(|quad| quad.uv).collect();
        assert_eq!(uvs, [cut(4.0, 16.0), cut(0.0, 4.0)]);
    }

    #[test]
    fn a_pattern_with_no_size_tiles_nothing() {
        let mut batches = Vec::new();
        let entry = AtlasEntry {
            page: PageId(0),
            rect: rect(0, 0, 0, 16),
            uv: uv(0, 0, 0, 16),
            opened: false,
        };
        let origin = at(0.0, 0.0);
        let tile = (0.0, 16.0);
        push_tiles(
            &mut batches,
            &entry,
            tile,
            area(0.0, 0.0, 20.0, 20.0),
            origin,
        );
        let entry = AtlasEntry {
            rect: rect(0, 0, 16, 0),
            ..entry
        };
        let tile = (16.0, 0.0);
        push_tiles(
            &mut batches,
            &entry,
            tile,
            area(0.0, 0.0, 20.0, 20.0),
            origin,
        );
        assert_eq!(batches, vec![]);
    }

    #[test]
    fn a_frame_carries_the_letterboxed_viewport() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0));
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &Viewport::new(SMALL, (160, 96), 2.0), &mut gpu);
        let frame = gpu.submits()[0].clone();
        assert_eq!(frame.target, (160, 96));
        assert_eq!(frame.viewport, rect(16, 0, 128, 96));
    }

    #[test]
    fn a_second_frame_uploads_nothing_and_decodes_nothing() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(10.0, 20.0))
            .sprite(ImageKey::sprite(200, 2), at(30.0, 40.0), Color::WHITE)
            .sprite(ImageKey::sprite(200, 0), at(31.0, 41.0), Color::WHITE);
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        let first = gpu.take();

        renderer.render(&list, &viewport(), &mut gpu);

        let second = only_submit(&gpu.calls);
        assert_eq!(Some(&GpuCall::Submit(second.clone())), first.last());
        assert_eq!(
            renderer.images().calls(),
            [(ImageKind::Pict, 128), (ImageKind::Rled, 200)]
        );
    }

    #[test]
    fn batches_follow_draw_order_across_pages() {
        let images = FakeImages::default()
            .with(ImageKind::Pict, 1, vec![solid(64, 40, [1, 1, 1, 255])])
            .with(ImageKind::Pict, 2, vec![solid(64, 40, [2, 2, 2, 255])]);
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(1), at(0.0, 0.0))
            .picture(ImageKey::picture(2), at(0.0, 0.0))
            .picture(ImageKey::picture(1), at(1.0, 0.0));
        let mut renderer = renderer(images);
        let mut gpu = RecordingGpu::new();

        renderer.render(&list, &viewport(), &mut gpu);

        let creates: Vec<&GpuCall> = gpu
            .calls
            .iter()
            .filter(|call| matches!(call, GpuCall::CreatePage { .. }))
            .collect();
        assert_eq!(creates.len(), 2);
        let pages: Vec<(PageId, usize)> = gpu.submits()[0]
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { page, quads, .. } => (*page, quads.len()),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(pages, [(PageId(0), 1), (PageId(1), 1), (PageId(0), 1)]);
    }

    /// Each sprites batch's blend and quad count, in order.
    fn sprite_blends(frame: &Frame) -> Vec<(Blend, usize)> {
        frame
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { blend, quads, .. } => (*blend, quads.len()),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_change_of_blend_starts_a_new_batch_in_order() {
        let sprite = |frame| ImageKey::sprite(200, frame);
        let mut list = DrawList::new();
        list.sprite(sprite(0), at(10.0, 10.0), Color::WHITE)
            .additive_sprite(sprite(1), at(20.0, 10.0), Color::WHITE)
            .additive_sprite(sprite(2), at(30.0, 10.0), Color::WHITE)
            .or_sprite(sprite(1), at(40.0, 10.0), Color::WHITE)
            .or_sprite(sprite(2), at(50.0, 10.0), Color::WHITE)
            .sprite(sprite(0), at(60.0, 10.0), Color::WHITE);

        let frame = render_one(&list, &viewport());

        assert_eq!(
            sprite_blends(&frame),
            [
                (Blend::Normal, 1),
                (Blend::Additive, 2),
                (Blend::Or, 2),
                (Blend::Normal, 1)
            ]
        );
        let xs: Vec<f32> = frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Sprites { quads, .. } => quads.iter().map(|q| q.dest.x).collect(),
                _ => Vec::new(),
            })
            .collect();
        assert_eq!(xs, [7.5, 17.5, 27.5, 37.5, 47.5, 57.5]);
    }

    /// Every sprite quad's tint, in draw order.
    fn tints(frame: &Frame) -> Vec<[f32; 4]> {
        frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Sprites { quads, .. } => quads.iter().map(|q| q.tint).collect(),
                _ => Vec::new(),
            })
            .collect()
    }

    #[test]
    fn or_tints_become_exact_levels() {
        let sprite = ImageKey::sprite(200, 0);
        let mut list = DrawList::new();
        list.or_sprite(sprite, at(10.0, 10.0), lights_tint(10))
            .or_sprite(sprite, at(20.0, 10.0), Color::WHITE)
            .or_sprite(sprite, at(30.0, 10.0), Color::rgba(255, 0, 0, 128))
            .sprite(sprite, at(40.0, 10.0), Color::rgba(255, 0, 0, 128));

        let frame = render_one(&list, &viewport());

        let level = 10.0 / 32.0;
        assert_eq!(
            tints(&frame),
            [
                [level, level, level, 1.0],
                [1.0; 4],
                [0.5, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 128.0 / 255.0],
            ]
        );
    }

    #[test]
    fn lights_tint_levels_round_trip() {
        for n in 0..=32 {
            assert_eq!(or_levels(lights_tint(n)), [n; 3], "level {n}");
        }
        assert_eq!(or_levels(Color::rgba(128, 64, 0, 255)), [16, 8, 0]);
    }

    #[test]
    fn overlapping_or_sprites_split_into_batches_in_order() {
        let sprite = ImageKey::sprite(200, 0);
        let mut list = DrawList::new();
        list.or_sprite(sprite, at(20.0, 10.0), Color::WHITE)
            .or_sprite(sprite, at(20.0, 10.0), Color::WHITE)
            .or_sprite(sprite, at(40.0, 10.0), Color::WHITE);

        let frame = render_one(&list, &viewport());

        // The third overlaps neither quad in the second batch, so joins it.
        assert_eq!(sprite_blends(&frame), [(Blend::Or, 1), (Blend::Or, 2)]);
    }

    #[test]
    fn an_or_sprite_overlapping_from_near_the_top_splits() {
        // The first spans y 5..11 and the second 0.5..6.5: they overlap
        // in y 5..6.5, which only the sum of the second's top and height
        // shows (their product, 3, is above neither).
        let sprite = ImageKey::sprite(200, 0);
        let mut list = DrawList::new();
        list.or_sprite(sprite, at(20.0, 8.0), Color::WHITE)
            .or_sprite(sprite, at(20.0, 3.5), Color::WHITE);

        let frame = render_one(&list, &viewport());

        assert_eq!(sprite_blends(&frame), [(Blend::Or, 1), (Blend::Or, 1)]);
    }

    #[test]
    fn or_sprites_touching_edge_to_edge_share_a_batch() {
        // The 5x6 frame centred on (20, 10) spans (17.5..22.5, 7..13).
        let sprite = ImageKey::sprite(200, 0);
        for (side, beside) in [
            ("left", at(15.0, 10.0)),
            ("right", at(25.0, 10.0)),
            ("above", at(20.0, 4.0)),
            ("below", at(20.0, 16.0)),
        ] {
            let mut list = DrawList::new();
            list.or_sprite(sprite, at(20.0, 10.0), Color::WHITE)
                .or_sprite(sprite, beside, Color::WHITE);
            let frame = render_one(&list, &viewport());
            assert_eq!(sprite_blends(&frame), [(Blend::Or, 2)], "{side}");
        }
    }

    #[test]
    fn overlapping_normal_and_additive_sprites_still_merge() {
        let sprite = ImageKey::sprite(200, 0);
        let mut list = DrawList::new();
        list.sprite(sprite, at(20.0, 10.0), Color::WHITE)
            .sprite(sprite, at(20.0, 10.0), Color::WHITE)
            .additive_sprite(sprite, at(20.0, 10.0), Color::WHITE)
            .additive_sprite(sprite, at(20.0, 10.0), Color::WHITE);

        let frame = render_one(&list, &viewport());

        assert_eq!(
            sprite_blends(&frame),
            [(Blend::Normal, 2), (Blend::Additive, 2)]
        );
    }

    #[test]
    fn pictures_draw_normally() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0))
            .sprite(ImageKey::sprite(200, 0), at(10.0, 10.0), Color::WHITE)
            .additive_sprite(ImageKey::sprite(200, 1), at(20.0, 10.0), Color::WHITE)
            .picture(ImageKey::picture(128), at(0.0, 20.0))
            .additive_sprite(ImageKey::sprite(200, 1), at(20.0, 10.0), Color::WHITE)
            .stretched_picture(ImageKey::picture(128), at(0.0, 30.0), 16.0, 8.0);

        let frame = render_one(&list, &viewport());

        assert_eq!(
            sprite_blends(&frame),
            [
                (Blend::Normal, 2),
                (Blend::Additive, 1),
                (Blend::Normal, 1),
                (Blend::Additive, 1),
                (Blend::Normal, 1)
            ]
        );
    }

    #[test]
    fn a_max_picture_lowers_to_a_max_batch_at_its_rectangle() {
        let mut list = DrawList::new();
        list.blended_picture(ImageKey::picture(128), at(-2.5, 3.0), 20.0, 1.5, Blend::Max);

        let frame = render_one(&list, &viewport());

        assert_eq!(
            frame.batches,
            vec![Batch::Sprites {
                page: PageId(0),
                blend: Blend::Max,
                quads: vec![QuadInstance {
                    dest: Rect {
                        x: -2.5,
                        y: 3.0,
                        w: 20.0,
                        h: 1.5,
                    },
                    uv: uv(0, 0, 8, 4),
                    tint: WHITE,
                }],
            }]
        );
    }

    #[test]
    fn a_max_picture_after_a_solid_keeps_its_place() {
        let mut list = DrawList::new();
        list.dot(at(5.0, 5.0), 4.0, Color::WHITE).blended_picture(
            ImageKey::picture(128),
            at(0.0, 0.0),
            16.0,
            8.0,
            Blend::Max,
        );

        let frame = render_one(&list, &viewport());

        let kinds: Vec<Option<Blend>> = frame
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { blend, .. } => Some(*blend),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, [None, Some(Blend::Max)]);
        assert!(matches!(frame.batches[0], Batch::Solid(_)));
    }

    #[test]
    fn overlapping_max_pictures_share_a_batch() {
        // A fixed-function max sees earlier quads of the same draw, so it
        // needs no split as OR's copy of what is beneath does.
        let mut list = DrawList::new();
        for x in [0.0, 4.0, 8.0] {
            list.blended_picture(ImageKey::picture(128), at(x, 0.0), 16.0, 8.0, Blend::Max);
        }

        let frame = render_one(&list, &viewport());

        assert_eq!(sprite_blends(&frame), [(Blend::Max, 3)]);
    }

    #[test]
    fn a_blended_picture_lowers_to_its_own_blend() {
        let mut list = DrawList::new();
        for (y, blend) in [
            (0.0, Blend::Normal),
            (10.0, Blend::Additive),
            (20.0, Blend::Or),
            (30.0, Blend::Max),
        ] {
            list.blended_picture(ImageKey::picture(128), at(0.0, y), 16.0, 8.0, blend);
        }

        let frame = render_one(&list, &viewport());

        assert_eq!(
            sprite_blends(&frame),
            [
                (Blend::Normal, 1),
                (Blend::Additive, 1),
                (Blend::Or, 1),
                (Blend::Max, 1)
            ]
        );
    }

    #[test]
    fn a_clipped_max_picture_is_cropped_with_its_texture_coordinates() {
        // 8x4 stretched to 16x8: each texel two units wide and tall.
        let mut list = DrawList::new();
        list.clipped(
            bounds(4.0, 2.0, 40.0, 40.0),
            DrawCommand::BlendedPicture {
                image: ImageKey::picture(128),
                top_left: at(0.0, 0.0),
                width: 16.0,
                height: 8.0,
                blend: Blend::Max,
            },
        );

        let frame = render_one(&list, &viewport());

        let page = 64.0;
        assert_eq!(
            frame.batches,
            vec![Batch::Sprites {
                page: PageId(0),
                blend: Blend::Max,
                quads: vec![QuadInstance {
                    dest: Rect {
                        x: 4.0,
                        y: 2.0,
                        w: 12.0,
                        h: 6.0,
                    },
                    uv: Uv {
                        u0: 2.0 / page,
                        v0: 1.0 / page,
                        u1: 8.0 / page,
                        v1: 4.0 / page,
                    },
                    tint: WHITE,
                }],
            }]
        );
    }

    #[test]
    fn a_blended_picture_with_no_drawable_size_draws_and_loads_nothing() {
        let mut list = DrawList::new();
        for (w, h) in [
            (0.0, 4.0),
            (4.0, 0.0),
            (-1.0, 4.0),
            (4.0, -1.0),
            (f32::NAN, 4.0),
            (4.0, f32::INFINITY),
        ] {
            list.blended_picture(ImageKey::picture(128), at(0.0, 0.0), w, h, Blend::Max);
        }
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        assert_eq!(only_submit(&gpu.calls).batches, vec![]);
        assert_eq!(renderer.images().calls(), []);
    }

    #[test]
    fn failures_are_skipped_and_reported_once() {
        let images = images().with(
            ImageKind::Rled,
            300,
            vec![solid(70, 2, [9, 9, 9, 255]), solid(2, 2, [8, 8, 8, 255])],
        );
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(99), at(0.0, 0.0))
            .sprite(ImageKey::sprite(200, 7), at(0.0, 0.0), Color::WHITE)
            .sprite(ImageKey::sprite(300, 0), at(0.0, 0.0), Color::WHITE)
            .sprite(ImageKey::sprite(300, 1), at(10.0, 10.0), Color::WHITE)
            .picture(ImageKey::picture(128), at(0.0, 0.0));
        let mut renderer = renderer(images);
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(
            report.new_failures,
            vec![
                (ImageKey::picture(99), ImageError::Missing),
                (
                    ImageKey::sprite(200, 7),
                    ImageError::NoFrame { frame: 7, count: 3 }
                ),
                (
                    ImageKey::sprite(300, 0),
                    ImageError::Pack(PackError::TooLarge {
                        width: 70,
                        height: 2,
                        page: 64
                    })
                ),
            ]
        );
        let uploaded: Vec<(u32, u32)> = gpu
            .calls
            .iter()
            .filter_map(|call| match call {
                GpuCall::Upload { width, height, .. } => Some((*width, *height)),
                _ => None,
            })
            .collect();
        assert_eq!(uploaded, [(5, 6), (5, 6), (5, 6), (2, 2), (8, 4)]);
        let frame = gpu.submits()[0].clone();
        let Batch::Sprites { quads, .. } = &frame.batches[0] else {
            panic!("{frame:?}");
        };
        assert_eq!(frame.batches.len(), 1);
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].dest.x, 9.0);

        gpu.take();
        let again = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(again, RenderReport::default());
        assert_eq!(only_submit(&gpu.calls), &frame);
        assert_eq!(
            renderer.images().calls(),
            [
                (ImageKind::Pict, 99),
                (ImageKind::Rled, 200),
                (ImageKind::Rled, 300),
                (ImageKind::Pict, 128)
            ]
        );
    }

    #[test]
    fn a_failed_resource_is_reported_once_whichever_frame_hits_it() {
        let broken =
            ImageError::Decode(nova_data::graphics::GraphicsError::UnexpectedEnd { offset: 0 });
        let images = FakeImages::default().failing(ImageKind::Rled, 5, broken.clone());
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(5, 0), at(0.0, 0.0), Color::WHITE)
            .sprite(ImageKey::sprite(5, 1), at(0.0, 0.0), Color::WHITE);
        let mut renderer = renderer(images);
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report.new_failures, vec![(ImageKey::sprite(5, 0), broken)]);
        assert_eq!(only_submit(&gpu.calls).batches, vec![]);
    }

    #[test]
    fn a_zero_area_viewport_submits_nothing() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0));
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &Viewport::new(SMALL, (0, 96), 2.0), &mut gpu);

        assert_eq!(report, RenderReport::default());
        assert_eq!(gpu.calls, vec![]);
        assert_eq!(renderer.images().calls(), []);
    }

    #[test]
    fn the_default_renderer_uses_full_size_pages() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0));
        let mut renderer = Renderer::new(images());
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(
            gpu.calls[0],
            GpuCall::CreatePage {
                page: PageId(0),
                size: crate::PAGE_SIZE
            }
        );
    }

    #[test]
    fn a_borrowed_source_is_a_source() {
        fn frames(source: impl ImageSource, id: i16) -> Result<Vec<Image>, ImageError> {
            source.frames(ImageKind::Pict, id)
        }
        let images = images();
        assert_eq!(frames(&images, 128), Ok(vec![red_picture()]));
        assert_eq!(frames(&images, 1), Err(ImageError::Missing));
        assert_eq!(images.calls().len(), 2);
    }

    fn solid_quads(frame: &Frame) -> Vec<SolidQuad> {
        frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Solid(quads) => quads.clone(),
                _ => Vec::new(),
            })
            .collect()
    }

    fn render_one(list: &DrawList, viewport: &Viewport) -> Frame {
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();
        renderer.render(list, viewport, &mut gpu);
        let submits = gpu.submits();
        assert_eq!(submits.len(), 1);
        assert_eq!(submits[0].world, [0_usize; 0], "no world");
        submits[0].clone()
    }

    #[test]
    fn in_a_widened_view_text_dots_and_clips_meet_on_the_same_pixel_column() {
        let viewport = Viewport::for_shape(nova_view::ViewShape::Widened, (1920, 1080), 1.0);
        let x = 1365.0 - 10.0;
        let clip = Bounds {
            min: at(x, 0.0),
            max: at(1365.0, 768.0),
        };
        let mut list = DrawList::new();
        list.text("Edge", at(x, 100.0), 12.0, Some(100.0), Color::WHITE)
            .dot(at(x, 200.0), 2.0, Color::WHITE)
            .clipped(
                clip,
                DrawCommand::Text {
                    text: "Cut".to_owned(),
                    font: Font::Geneva,
                    origin: at(0.0, 300.0),
                    size: 12.0,
                    wrap_width: None,
                    color: Color::WHITE,
                },
            );
        let frame = render_one(&list, &viewport);
        let across = 1920.0 / 1365.0;
        let Batch::Text(runs) = &frame.batches[0] else {
            panic!("{frame:?}");
        };
        assert_eq!(runs[0].origin_px, (x * across, 100.0 * 1.406_25));
        assert_eq!(runs[0].size_px, 12.0 * 1.406_25, "sized by the height");
        assert_eq!(runs[0].wrap_px, Some(100.0 * across));
        // The GPU projects quads from the logical space onto the content.
        let dot = solid_quads(&frame)[0].corners;
        let dot_centre = f32::midpoint(dot[0].x, dot[2].x);
        let projected = dot_centre * frame.viewport.w as f32 / frame.logical.w as f32;
        let Batch::Text(cut) = &frame.batches[2] else {
            panic!("{frame:?}");
        };
        let column = runs[0].origin_px.0.round();
        assert_eq!(projected.round(), column);
        assert_eq!(cut[0].clip.x as f32, column);
        assert_eq!(cut[0].clip.x + cut[0].clip.w, 1920, "to the window's edge");
        assert_eq!(cut[0].clip.h, 1080);
    }

    #[test]
    fn text_is_converted_to_physical_pixels() {
        let color = Color::rgba(1, 2, 3, 4);
        let mut list = DrawList::new();
        list.text_in(
            Font::Charcoal,
            "Hello",
            at(1.5, 2.0),
            10.0,
            Some(20.0),
            color,
        )
        .text("Unwrapped", at(0.0, 0.0), 8.0, None, Color::WHITE);

        let frame = render_one(&list, &viewport());

        assert_eq!(
            frame.batches,
            vec![Batch::Text(vec![
                TextRun {
                    text: "Hello".to_owned(),
                    font: Font::Charcoal,
                    origin_px: (3.0, 4.0),
                    size_px: 20.0,
                    line_height_px: 1.2 * 20.0,
                    wrap_px: Some(40.0),
                    color,
                    clip: rect(0, 0, 128, 96),
                },
                TextRun {
                    text: "Unwrapped".to_owned(),
                    font: Font::Geneva,
                    origin_px: (0.0, 0.0),
                    size_px: 16.0,
                    line_height_px: 1.2 * 16.0,
                    wrap_px: None,
                    color: Color::WHITE,
                    clip: rect(0, 0, 128, 96),
                },
            ])]
        );
    }

    fn bounds(left: f32, top: f32, right: f32, bottom: f32) -> Bounds {
        Bounds {
            min: at(left, top),
            max: at(right, bottom),
        }
    }

    #[test]
    fn a_clipped_text_run_is_cut_to_its_clip_in_pixels() {
        let mut list = DrawList::new();
        let text = DrawCommand::Text {
            text: "Sol".to_owned(),
            font: Font::Geneva,
            origin: at(1.0, 2.0),
            size: 8.0,
            wrap_width: None,
            color: Color::WHITE,
        };
        list.clipped(bounds(10.0, 5.25, 30.0, 70.0), text.clone())
            .clipped(bounds(70.0, 0.0, 90.0, 10.0), text);

        let frame = render_one(&list, &viewport());

        let [Batch::Text(runs)] = frame.batches.as_slice() else {
            panic!("one text batch: {frame:?}");
        };
        assert_eq!(runs.len(), 1, "the clip off the viewport drops its run");
        assert_eq!(runs[0].origin_px, (2.0, 4.0));
        assert_eq!(
            runs[0].clip,
            rect(20, 11, 40, 85),
            "rounded to pixels, and cut to the viewport"
        );
    }

    #[test]
    fn a_clipped_picture_is_cropped_with_its_texture_coordinates() {
        let mut list = DrawList::new();
        let picture = DrawCommand::Picture {
            image: ImageKey::picture(128),
            top_left: at(10.0, 20.0),
        };
        list.clipped(bounds(12.0, 21.0, 40.0, 40.0), picture.clone())
            .clipped(bounds(0.0, 0.0, 40.0, 40.0), picture.clone())
            .clipped(bounds(30.0, 30.0, 40.0, 40.0), picture);

        let frame = render_one_with(images(), &list);

        let [Batch::Sprites { quads, .. }] = frame.batches.as_slice() else {
            panic!("one sprite batch: {frame:?}");
        };
        let page = 64.0;
        assert_eq!(
            quads.as_slice(),
            [
                QuadInstance {
                    dest: Rect {
                        x: 12.0,
                        y: 21.0,
                        w: 6.0,
                        h: 3.0,
                    },
                    uv: Uv {
                        u0: 2.0 / page,
                        v0: 1.0 / page,
                        u1: 8.0 / page,
                        v1: 4.0 / page,
                    },
                    tint: WHITE,
                },
                QuadInstance {
                    dest: Rect {
                        x: 10.0,
                        y: 20.0,
                        w: 8.0,
                        h: 4.0,
                    },
                    uv: uv(0, 0, 8, 4),
                    tint: WHITE,
                },
            ],
            "whole inside its clip, as it is; wholly outside, nothing"
        );
    }

    #[test]
    fn a_clip_that_only_touches_or_misses_one_axis_draws_nothing() {
        let mut list = DrawList::new();
        let picture = DrawCommand::Picture {
            image: ImageKey::picture(128),
            top_left: at(10.0, 20.0),
        };
        // The 8 x 4 picture covers (10, 20)-(18, 24).
        for clip in [
            bounds(18.0, 0.0, 40.0, 40.0),
            bounds(0.0, 24.0, 40.0, 40.0),
            bounds(12.0, 30.0, 16.0, 40.0),
            bounds(30.0, 21.0, 40.0, 23.0),
        ] {
            list.clipped(clip, picture.clone());
        }
        list.clipped(bounds(10.0, 0.0, 10.2, 40.0), text_at(at(1.0, 2.0)))
            .clipped(bounds(0.0, 10.0, 40.0, 10.2), text_at(at(1.0, 2.0)));

        let frame = render_one_with(images(), &list);

        assert_eq!(frame.batches, [], "no area left, in pixels too");
    }

    fn text_at(origin: Point) -> DrawCommand {
        DrawCommand::Text {
            text: "Sol".to_owned(),
            font: Font::Geneva,
            origin,
            size: 8.0,
            wrap_width: None,
            color: Color::WHITE,
        }
    }

    #[test]
    fn a_cropped_quad_keeps_its_place_on_the_page() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0)).clipped(
            bounds(0.0, 0.0, 30.0, 40.0),
            DrawCommand::Sprite {
                image: ImageKey::sprite(200, 1),
                center: at(30.0, 40.0),
                tint: Color::WHITE,
                blend: Blend::Normal,
            },
        );

        let frame = render_one_with(images(), &list);

        let [Batch::Sprites { quads, .. }] = frame.batches.as_slice() else {
            panic!("one sprite batch: {frame:?}");
        };
        let page = 64.0;
        // Frame 1 is at (6, 5) on the page, under the picture's shelf.
        assert_eq!(
            quads[1].uv,
            Uv {
                u0: 6.0 / page,
                v0: 5.0 / page,
                u1: 8.5 / page,
                v1: 8.0 / page,
            }
        );
    }

    #[test]
    fn a_clipped_sprite_is_cropped_and_keeps_its_tint() {
        let mut list = DrawList::new();
        let tint = Color::rgba(255, 255, 255, 128);
        let sprite = DrawCommand::Sprite {
            image: ImageKey::sprite(200, 0),
            center: at(30.0, 40.0),
            tint,
            blend: Blend::Normal,
        };
        list.clipped(bounds(0.0, 0.0, 30.0, 40.0), sprite);

        let frame = render_one_with(images(), &list);

        let [Batch::Sprites { quads, .. }] = frame.batches.as_slice() else {
            panic!("one sprite batch: {frame:?}");
        };
        let page = 64.0;
        assert_eq!(
            quads[0].dest,
            Rect {
                x: 27.5,
                y: 37.0,
                w: 2.5,
                h: 3.0,
            }
        );
        assert_eq!(
            quads[0].uv,
            Uv {
                u0: 0.0,
                v0: 0.0,
                u1: 2.5 / page,
                v1: 3.0 / page,
            },
            "the sheet alone on its page, its first frame at the corner"
        );
        assert_eq!(quads[0].tint, [1.0, 1.0, 1.0, 128.0 / 255.0]);
    }

    #[test]
    fn clips_meet_and_lines_and_dots_are_not_cut() {
        let mut list = DrawList::new();
        let mut inner = DrawList::new();
        inner.clipped(
            bounds(20.0, 0.0, 64.0, 48.0),
            DrawCommand::TiledPicture {
                image: ImageKey::picture(128),
                area: bounds(0.0, 0.0, 64.0, 48.0),
                origin: at(0.0, 0.0),
            },
        );
        let nested = inner.iter().next().cloned().expect("one");
        list.clipped(bounds(0.0, 0.0, 28.0, 4.0), nested).clipped(
            bounds(0.0, 0.0, 1.0, 1.0),
            DrawCommand::Line {
                from: at(0.0, 10.0),
                to: at(10.0, 10.0),
                width: 2.0,
                color: Color::WHITE,
            },
        );

        let frame = render_one_with(images(), &list);

        let [Batch::Sprites { quads, .. }, Batch::Solid(solids)] = frame.batches.as_slice() else {
            panic!("tiles then a line: {frame:?}");
        };
        let dests: Vec<Rect> = quads.iter().map(|quad| quad.dest).collect();
        assert_eq!(
            dests,
            [
                Rect {
                    x: 20.0,
                    y: 0.0,
                    w: 4.0,
                    h: 4.0,
                },
                Rect {
                    x: 24.0,
                    y: 0.0,
                    w: 4.0,
                    h: 4.0,
                },
            ],
            "the tiles in both clips"
        );
        assert_eq!(solids.len(), 1, "the line drawn whole");
        assert_eq!(solids[0].corners[0], at(0.0, 11.0));
    }

    #[test]
    fn runs_in_either_font_share_one_text_batch_in_draw_order() {
        let mut list = DrawList::new();
        list.text("a", at(0.0, 0.0), 8.0, None, Color::WHITE)
            .text_in(Font::Charcoal, "b", at(0.0, 0.0), 8.0, None, Color::WHITE)
            .text("c", at(0.0, 0.0), 8.0, None, Color::WHITE);

        let frame = render_one(&list, &viewport());

        let [Batch::Text(runs)] = frame.batches.as_slice() else {
            panic!("one text batch: {frame:?}");
        };
        let fonts: Vec<(&str, Font)> = runs.iter().map(|r| (r.text.as_str(), r.font)).collect();
        assert_eq!(
            fonts,
            [
                ("a", Font::Geneva),
                ("b", Font::Charcoal),
                ("c", Font::Geneva)
            ]
        );
    }

    #[test]
    fn text_with_no_drawable_size_or_no_characters_is_skipped() {
        let white = Color::WHITE;
        let dot = SolidQuad {
            corners: [at(0.5, 0.5), at(1.5, 0.5), at(1.5, 1.5), at(0.5, 1.5)],
            color: WHITE,
        };
        let line = SolidQuad {
            corners: [at(0.0, 2.5), at(4.0, 2.5), at(4.0, 1.5), at(0.0, 1.5)],
            color: WHITE,
        };
        let mut list = DrawList::new();
        list.text("zero", at(0.0, 0.0), 0.0, None, white)
            .dot(at(1.0, 1.0), 1.0, white)
            .text("negative", at(0.0, 0.0), -8.0, None, white)
            .text("nan", at(0.0, 0.0), f32::NAN, None, white)
            .text("infinite", at(0.0, 0.0), f32::INFINITY, None, white)
            // Finite, but infinite once scaled to pixels.
            .text("huge", at(0.0, 0.0), f32::MAX, None, white)
            // Finite in pixels, but its line height is not.
            .text("tall", at(0.0, 0.0), f32::MAX / 2.2, None, white)
            .text("", at(0.0, 0.0), 8.0, None, white)
            .line(at(0.0, 2.0), at(4.0, 2.0), 1.0, white)
            .text("ok", at(0.0, 0.0), 8.0, None, white);

        let frame = render_one(&list, &viewport());

        // The skipped runs leave no text batch, nor split the solid one.
        assert_eq!(
            frame.batches,
            vec![
                Batch::Solid(vec![dot, line]),
                Batch::Text(vec![TextRun {
                    text: "ok".to_owned(),
                    font: Font::Geneva,
                    origin_px: (0.0, 0.0),
                    size_px: 16.0,
                    line_height_px: 1.2 * 16.0,
                    wrap_px: None,
                    color: white,
                    clip: rect(0, 0, 128, 96),
                }]),
            ]
        );
    }

    #[test]
    fn text_in_a_pillarboxed_window_is_offset_by_the_bar() {
        let mut list = DrawList::new();
        list.text("Hi", at(1.0, 2.0), 10.0, None, Color::WHITE);
        let frame = render_one(&list, &Viewport::new(SMALL, (160, 96), 2.0));
        let Batch::Text(runs) = &frame.batches[0] else {
            panic!("{frame:?}");
        };
        // 64x48 at scale 2 is 128x96, centred in 160x96.
        assert_eq!(runs[0].origin_px, (18.0, 4.0));
        assert_eq!(runs[0].clip, rect(16, 0, 128, 96));
        let letterboxed = render_one(&list, &Viewport::new(SMALL, (128, 128), 2.0));
        let Batch::Text(runs) = &letterboxed.batches[0] else {
            panic!("{letterboxed:?}");
        };
        assert_eq!(runs[0].origin_px, (2.0, 20.0));
    }

    #[test]
    fn a_horizontal_line_is_a_quad_its_width_tall() {
        let blue = Color::rgba(0, 0, 255, 255);
        let mut list = DrawList::new();
        list.line(at(2.0, 10.0), at(6.0, 10.0), 2.0, blue);
        assert_eq!(
            solid_quads(&render_one(&list, &viewport())),
            vec![SolidQuad {
                corners: [at(2.0, 11.0), at(6.0, 11.0), at(6.0, 9.0), at(2.0, 9.0)],
                color: [0.0, 0.0, 1.0, 1.0],
            }]
        );
    }

    #[test]
    fn a_vertical_line_is_a_quad_its_width_wide() {
        let mut list = DrawList::new();
        list.line(at(5.0, 1.0), at(5.0, 9.0), 4.0, Color::WHITE);
        assert_eq!(
            solid_quads(&render_one(&list, &viewport()))[0].corners,
            [at(3.0, 1.0), at(3.0, 9.0), at(7.0, 9.0), at(7.0, 1.0)]
        );
    }

    #[test]
    fn a_diagonal_line_is_offset_along_its_normal() {
        // A 3-4-5 line: the unit normal is (-0.8, 0.6), half-width 2.5.
        let mut list = DrawList::new();
        list.line(at(0.0, 0.0), at(3.0, 4.0), 5.0, Color::WHITE);
        assert_eq!(
            solid_quads(&render_one(&list, &viewport()))[0].corners,
            [at(-2.0, 1.5), at(1.0, 5.5), at(5.0, 2.5), at(2.0, -1.5)]
        );
    }

    #[test]
    fn a_zero_length_line_is_a_square_its_width_wide() {
        let mut list = DrawList::new();
        list.line(at(4.0, 4.0), at(4.0, 4.0), 2.0, Color::WHITE);
        assert_eq!(
            solid_quads(&render_one(&list, &viewport()))[0].corners,
            [at(3.0, 3.0), at(5.0, 3.0), at(5.0, 5.0), at(3.0, 5.0)]
        );
    }

    #[test]
    fn a_dot_is_a_square_centred_on_its_point() {
        let mut list = DrawList::new();
        list.dot(at(10.0, 20.0), 3.0, Color::rgba(255, 255, 0, 0));
        assert_eq!(
            solid_quads(&render_one(&list, &viewport())),
            vec![SolidQuad {
                corners: [at(8.5, 18.5), at(11.5, 18.5), at(11.5, 21.5), at(8.5, 21.5)],
                color: [1.0, 1.0, 0.0, 0.0],
            }]
        );
    }

    #[test]
    fn different_kinds_interleave_in_draw_order() {
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(200, 0), at(10.0, 10.0), Color::WHITE)
            .line(at(0.0, 0.0), at(4.0, 0.0), 1.0, Color::WHITE)
            .dot(at(1.0, 1.0), 1.0, Color::WHITE)
            .text("a", at(0.0, 0.0), 8.0, None, Color::WHITE)
            .text("b", at(0.0, 0.0), 8.0, None, Color::WHITE)
            .line(at(0.0, 0.0), at(4.0, 0.0), 1.0, Color::WHITE)
            .sprite(ImageKey::sprite(200, 1), at(10.0, 10.0), Color::WHITE);

        let frame = render_one(&list, &viewport());

        let shape: Vec<(&str, usize)> = frame
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { quads, .. } => ("sprites", quads.len()),
                Batch::Solid(quads) => ("solid", quads.len()),
                Batch::Text(runs) => ("text", runs.len()),
            })
            .collect();
        assert_eq!(
            shape,
            [
                ("sprites", 1),
                ("solid", 2),
                ("text", 2),
                ("solid", 1),
                ("sprites", 1)
            ]
        );
    }

    // The world layer and the overlay.

    const WORLD: DrawLayer = DrawLayer::World;
    const OVERLAY: DrawLayer = DrawLayer::Overlay;

    /// Each batch's kind, size and layer, in order.
    fn layered_shape(frame: &Frame) -> Vec<(&'static str, usize, DrawLayer)> {
        frame
            .batches
            .iter()
            .enumerate()
            .map(|(at, batch)| {
                let (kind, size) = match batch {
                    Batch::Sprites { quads, .. } => ("sprites", quads.len()),
                    Batch::Solid(quads) => ("solid", quads.len()),
                    Batch::Text(runs) => ("text", runs.len()),
                };
                (kind, size, frame.layer(at))
            })
            .collect()
    }

    /// `list`'s one frame, rendered through `gpu`.
    fn render_through(list: &DrawList, gpu: &mut RecordingGpu) -> Frame {
        let mut renderer = renderer(images());
        let report = renderer.render(list, &viewport(), gpu);
        assert_eq!(report, RenderReport::default());
        gpu.submits()[0].clone()
    }

    fn sprite(frame: u16) -> ImageKey {
        ImageKey::sprite(200, frame)
    }

    #[test]
    fn world_commands_are_batched_in_the_world_layer_in_draw_order() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0));
        list.world(|list| {
            list.sprite(sprite(0), at(10.0, 10.0), Color::WHITE)
                .or_sprite(sprite(1), at(20.0, 10.0), Color::WHITE)
                .line(at(0.0, 0.0), at(4.0, 0.0), 1.0, Color::WHITE)
                .dot(at(1.0, 1.0), 1.0, Color::WHITE)
                .text("a", at(0.0, 0.0), 8.0, None, Color::WHITE)
                .picture(ImageKey::picture(128), at(1.0, 1.0));
        });
        list.text("b", at(0.0, 0.0), 8.0, None, Color::WHITE)
            .sprite(sprite(2), at(30.0, 10.0), Color::WHITE);

        let frame = render_through(&list, &mut RecordingGpu::new());

        assert_eq!(
            layered_shape(&frame),
            [
                ("sprites", 1, OVERLAY),
                ("sprites", 1, WORLD),
                ("sprites", 1, WORLD),
                ("solid", 2, WORLD),
                ("text", 1, OVERLAY),
                ("sprites", 1, WORLD),
                ("text", 1, OVERLAY),
                ("sprites", 1, OVERLAY),
            ]
        );
        assert_eq!(frame.world, [1, 2, 3, 5]);
        assert_eq!(frame.world_space, WorldSpace::Target);
    }

    #[test]
    fn no_batch_holds_both_layers() {
        let mut list = DrawList::new();
        list.world(|list| {
            list.dot(at(1.0, 1.0), 1.0, Color::WHITE);
        });
        list.dot(at(2.0, 2.0), 1.0, Color::WHITE);
        list.world(|list| {
            list.dot(at(3.0, 3.0), 1.0, Color::WHITE).sprite(
                sprite(0),
                at(10.0, 10.0),
                Color::WHITE,
            );
        });
        list.sprite(sprite(1), at(20.0, 10.0), Color::WHITE);

        let frame = render_through(&list, &mut RecordingGpu::new());

        assert_eq!(
            layered_shape(&frame),
            [
                ("solid", 1, WORLD),
                ("solid", 1, OVERLAY),
                ("solid", 1, WORLD),
                ("sprites", 1, WORLD),
                ("sprites", 1, OVERLAY),
            ]
        );
    }

    #[test]
    fn a_world_drawn_between_overlays_keeps_its_place() {
        // A dialog's map over flight: the dialog, flight's world, then its
        // HUD and the map.
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0));
        list.world(|list| {
            list.sprite(sprite(0), at(10.0, 10.0), Color::WHITE).line(
                at(0.0, 0.0),
                at(4.0, 0.0),
                1.0,
                Color::WHITE,
            );
        });
        list.line(at(0.0, 0.0), at(4.0, 0.0), 1.0, Color::WHITE)
            .picture(ImageKey::picture(128), at(1.0, 1.0));

        let frame = render_through(&list, &mut RecordingGpu::new());

        assert_eq!(
            layered_shape(&frame),
            [
                ("sprites", 1, OVERLAY),
                ("sprites", 1, WORLD),
                ("solid", 1, WORLD),
                ("solid", 1, OVERLAY),
                ("sprites", 1, OVERLAY),
            ]
        );
    }

    #[test]
    fn a_list_with_no_world_draws_every_batch_as_the_unlayered_list_did() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0))
            .sprite(sprite(0), at(10.0, 10.0), Color::WHITE)
            .or_sprite(sprite(1), at(20.0, 10.0), Color::WHITE)
            .or_sprite(sprite(2), at(21.0, 10.0), Color::WHITE)
            .line(at(0.0, 0.0), at(4.0, 0.0), 1.0, Color::WHITE)
            .text("a", at(0.0, 0.0), 8.0, None, Color::WHITE);

        let frame = render_through(&list, &mut RecordingGpu::new());

        assert_eq!(frame.world, [0_usize; 0]);
        assert_eq!(frame.in_layer(WORLD).count(), 0);
        assert_eq!(
            layered_shape(&frame),
            [
                ("sprites", 2, OVERLAY),
                ("sprites", 1, OVERLAY),
                ("sprites", 1, OVERLAY),
                ("solid", 1, OVERLAY),
                ("text", 1, OVERLAY),
            ]
        );
    }

    /// An OR sprite (5x6, so at (7.5, 7)) in the world, and the same in
    /// the overlay.
    fn or_in_both_layers() -> DrawList {
        let mut list = DrawList::new();
        list.world(|list| {
            list.or_sprite(sprite(0), at(10.0, 10.0), Color::WHITE);
        });
        list.or_sprite(sprite(0), at(10.0, 10.0), Color::WHITE);
        list
    }

    /// What each OR batch of `frame` covers, in its layer's pixels.
    fn or_covers(frame: &Frame) -> Vec<(DrawLayer, Option<PixelRect>)> {
        frame
            .batches
            .iter()
            .enumerate()
            .map(|(at, batch)| match batch {
                Batch::Sprites {
                    blend: Blend::Or,
                    quads,
                    ..
                } => {
                    let layer = frame.layer(at);
                    (layer, frame.pixels_covered(layer, quads))
                }
                other => panic!("an OR batch: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_world_at_the_logical_resolution_is_a_texture_and_its_or_batches_cover_texels() {
        let mut gpu = RecordingGpu::answering(WorldResolution::Logical);

        let frame = render_through(&or_in_both_layers(), &mut gpu);

        assert_eq!(frame.world_space, WorldSpace::Texture { size: (64, 48) });
        // (7.5, 7)-(12.5, 13) in texels; twice that in the 128x96 target.
        assert_eq!(
            or_covers(&frame),
            [
                (WORLD, Some(rect(7, 7, 6, 6))),
                (OVERLAY, Some(rect(15, 14, 10, 12))),
            ]
        );
    }

    #[test]
    fn a_native_world_is_drawn_into_the_target_and_its_or_batches_cover_target_pixels() {
        let frame = render_through(&or_in_both_layers(), &mut RecordingGpu::new());

        assert_eq!(frame.world_space, WorldSpace::Target);
        assert_eq!(
            or_covers(&frame),
            [
                (WORLD, Some(rect(15, 14, 10, 12))),
                (OVERLAY, Some(rect(15, 14, 10, 12))),
            ]
        );
    }

    /// Replicates each texel 2x2 for every kind, patterns included, and
    /// records the kinds it is asked about.
    #[derive(Debug, Default)]
    struct Doubled {
        kinds: Rc<RefCell<Vec<ImageKind>>>,
    }

    impl Enlarger for Doubled {
        fn enlarge<'a>(&self, kind: ImageKind, image: &'a Image) -> Cow<'a, Image> {
            self.kinds.borrow_mut().push(kind);
            Cow::Owned(doubled(image))
        }
    }

    /// `image` with each texel replicated 2x2.
    fn doubled(image: &Image) -> Image {
        let (w, h) = (image.width(), image.height());
        let pixels = (0..h * 2)
            .flat_map(|y| (0..w * 2).flat_map(move |x| image.pixel(x / 2, y / 2).expect("inside")))
            .collect();
        Image::from_rgba(w * 2, h * 2, pixels).expect("doubled")
    }

    /// The renderer on 64-texel pages, enlarging with [`Doubled`].
    fn doubling(images: FakeImages) -> Renderer<FakeImages> {
        let mut renderer = renderer(images);
        renderer.set_enlarger(Box::new(Doubled::default()));
        renderer
    }

    /// Every textured quad of `frame`, in draw order.
    fn all_quads(frame: &Frame) -> Vec<QuadInstance> {
        frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Sprites { quads, .. } => quads.clone(),
                _ => Vec::new(),
            })
            .collect()
    }

    /// The quads `renderer` draws `list` with, reporting no failure.
    fn quads_of(mut renderer: Renderer<FakeImages>, list: &DrawList) -> Vec<QuadInstance> {
        let mut gpu = RecordingGpu::new();
        let report = renderer.render(list, &viewport(), &mut gpu);
        assert_eq!(report, RenderReport::default());
        all_quads(gpu.submits()[0])
    }

    /// `uv` twice as far from the page's corner: an image at the corner
    /// placed at twice its size.
    fn twice(uv: Uv) -> Uv {
        Uv {
            u0: uv.u0 * 2.0,
            v0: uv.v0 * 2.0,
            u1: uv.u1 * 2.0,
            v1: uv.v1 * 2.0,
        }
    }

    #[test]
    fn an_enlarged_picture_and_sprite_upload_twice_their_size_and_draw_at_their_own() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(10.0, 20.0)).sprite(
            ImageKey::sprite(200, 2),
            at(30.5, 40.25),
            Color::WHITE,
        );
        let native = quads_of(renderer(images()), &list);
        let mut renderer = doubling(images());
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        let sheet = sheet();
        // The 16x8 picture fills the first shelf, and the 10x12 frames
        // start a second below it and its gutter.
        assert_eq!(
            gpu.calls[..5],
            [
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), rect(0, 0, 16, 8), &doubled(&red_picture())),
                GpuCall::upload(PageId(0), rect(0, 9, 10, 12), &doubled(&sheet[0])),
                GpuCall::upload(PageId(0), rect(11, 9, 10, 12), &doubled(&sheet[1])),
                GpuCall::upload(PageId(0), rect(22, 9, 10, 12), &doubled(&sheet[2])),
            ]
        );
        let enlarged = all_quads(gpu.submits()[0]);
        let dests: Vec<Rect> = enlarged.iter().map(|quad| quad.dest).collect();
        assert_eq!(
            dests,
            native.iter().map(|quad| quad.dest).collect::<Vec<_>>()
        );
        assert_eq!(
            dests[1],
            Rect {
                x: 28.0,
                y: 37.25,
                w: 5.0,
                h: 6.0
            },
            "the sprite's own size, centred to the quarter unit"
        );
        let uvs: Vec<Uv> = enlarged.iter().map(|quad| quad.uv).collect();
        assert_eq!(uvs, [uv(0, 0, 16, 8), uv(22, 9, 10, 12)]);
    }

    #[test]
    fn enlarged_stretched_clipped_and_tiled_images_keep_their_places_and_take_the_bigger_rect() {
        let mut picture = DrawList::new();
        picture.picture(ImageKey::picture(128), at(1.25, 2.5));
        let mut stretched = DrawList::new();
        stretched.stretched_picture(ImageKey::picture(128), at(-2.5, 3.0), 20.0, 1.5);
        let mut clipped = DrawList::new();
        let sprite = DrawCommand::Sprite {
            image: ImageKey::sprite(200, 0),
            center: at(30.0, 40.0),
            tint: Color::WHITE,
            blend: Blend::Normal,
        };
        clipped.clipped(bounds(28.5, 38.0, 30.0, 40.0), sprite);
        let mut tiled = DrawList::new();
        tiled.tiled_picture(ImageKey::pattern(130), area(10.0, 5.0, 50.0, 45.0));
        for list in [picture, stretched, clipped, tiled] {
            // Each image alone at its page's corner, so the enlarged one's
            // texture coordinates are the native one's doubled.
            let native = quads_of(renderer(tiles()), &list);
            let enlarged = quads_of(doubling(tiles()), &list);
            assert!(!native.is_empty(), "{list:?}");
            let expected: Vec<QuadInstance> = native
                .iter()
                .map(|quad| QuadInstance {
                    uv: twice(quad.uv),
                    ..*quad
                })
                .collect();
            assert_eq!(enlarged, expected, "{list:?}");
        }
    }

    #[test]
    fn an_enlarged_frame_too_big_for_a_page_falls_back_to_its_native_frame() {
        let fits = solid(40, 4, [3, 3, 3, 255]);
        let images = FakeImages::default()
            .with(ImageKind::Pict, 1, vec![fits.clone()])
            .with(ImageKind::Pict, 2, vec![solid(70, 2, [4, 4, 4, 255])]);
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(1), at(1.0, 2.0))
            .picture(ImageKey::picture(2), at(0.0, 0.0));
        let mut renderer = doubling(images);
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(
            report.new_failures,
            [(
                ImageKey::picture(2),
                ImageError::Pack(PackError::TooLarge {
                    width: 70,
                    height: 2,
                    page: 64
                })
            )],
            "only what is too big natively fails"
        );
        assert_eq!(
            gpu.calls[..2],
            [
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), rect(0, 0, 40, 4), &fits),
            ]
        );
        assert_eq!(
            all_quads(gpu.submits()[0]),
            [QuadInstance {
                dest: Rect {
                    x: 1.0,
                    y: 2.0,
                    w: 40.0,
                    h: 4.0
                },
                uv: uv(0, 0, 40, 4),
                tint: WHITE,
            }]
        );
    }

    #[test]
    fn a_new_enlarger_clears_the_pages_and_the_next_frame_reloads_what_it_draws() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(10.0, 20.0)).sprite(
            ImageKey::sprite(200, 2),
            at(30.0, 40.0),
            Color::WHITE,
        );
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        let native = all_quads(gpu.take().last().and_then(submitted).expect("a frame"));
        let kinds = Rc::new(RefCell::new(Vec::new()));
        renderer.set_enlarger(Box::new(Doubled {
            kinds: Rc::clone(&kinds),
        }));

        renderer.render(&list, &viewport(), &mut gpu);

        let sheet = sheet();
        assert_eq!(
            gpu.calls[..6],
            [
                GpuCall::ClearPages,
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), rect(0, 0, 16, 8), &doubled(&red_picture())),
                GpuCall::upload(PageId(0), rect(0, 9, 10, 12), &doubled(&sheet[0])),
                GpuCall::upload(PageId(0), rect(11, 9, 10, 12), &doubled(&sheet[1])),
                GpuCall::upload(PageId(0), rect(22, 9, 10, 12), &doubled(&sheet[2])),
            ]
        );
        let rebuilt = all_quads(gpu.submits()[0]);
        let dests = |quads: &[QuadInstance]| quads.iter().map(|q| q.dest).collect::<Vec<_>>();
        assert_eq!(dests(&rebuilt), dests(&native));
        assert_eq!(
            *kinds.borrow(),
            [
                ImageKind::Pict,
                ImageKind::Rled,
                ImageKind::Rled,
                ImageKind::Rled
            ]
        );
        assert_eq!(
            renderer.images().calls(),
            [
                (ImageKind::Pict, 128),
                (ImageKind::Rled, 200),
                (ImageKind::Pict, 128),
                (ImageKind::Rled, 200)
            ],
            "decoded again"
        );

        gpu.take();
        renderer.render(&list, &viewport(), &mut gpu);
        let again = only_submit(&gpu.calls);
        assert_eq!(all_quads(again), rebuilt, "a second frame uploads nothing");
    }

    /// The frame `call` submits, if it is a submit.
    fn submitted(call: &GpuCall) -> Option<&Frame> {
        match call {
            GpuCall::Submit(frame) => Some(frame),
            _ => None,
        }
    }

    #[test]
    fn pages_are_cleared_only_by_a_new_enlarger_once_some_are_open() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), at(0.0, 0.0));
        let mut never = renderer(images());
        let mut gpu = RecordingGpu::new();
        never.render(&list, &viewport(), &mut gpu);
        never.render(&list, &viewport(), &mut gpu);
        assert!(!gpu.calls.contains(&GpuCall::ClearPages), "never set");

        let mut early = doubling(images());
        let mut gpu = RecordingGpu::new();
        early.render(&list, &viewport(), &mut gpu);
        assert_eq!(
            gpu.take()[..2],
            [
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), rect(0, 0, 16, 8), &doubled(&red_picture())),
            ],
            "no page to clear"
        );

        early.set_enlarger(Box::new(NativeDetail));
        early.render(&list, &viewport(), &mut gpu);
        assert_eq!(
            gpu.calls[..3],
            [
                GpuCall::ClearPages,
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), rect(0, 0, 8, 4), &red_picture()),
            ]
        );
    }

    #[test]
    fn after_a_rebuild_failures_stay_failed_and_are_not_reported_again() {
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(99), at(0.0, 0.0))
            .sprite(ImageKey::sprite(200, 7), at(0.0, 0.0), Color::WHITE)
            .picture(ImageKey::picture(128), at(0.0, 0.0));
        let mut renderer = renderer(images());
        let mut gpu = RecordingGpu::new();
        let first = renderer.render(&list, &viewport(), &mut gpu);
        assert_eq!(first.new_failures.len(), 2, "{first:?}");
        renderer.set_enlarger(Box::new(Doubled::default()));
        gpu.take();

        let again = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(again, RenderReport::default());
        assert_eq!(
            renderer.images().calls(),
            [
                (ImageKind::Pict, 99),
                (ImageKind::Rled, 200),
                (ImageKind::Pict, 128),
                (ImageKind::Rled, 200),
                (ImageKind::Pict, 128)
            ],
            "the missing picture is not asked for again"
        );
        assert_eq!(gpu.calls[0], GpuCall::ClearPages);
        assert_eq!(
            all_quads(gpu.submits()[0]),
            [QuadInstance {
                dest: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 8.0,
                    h: 4.0
                },
                uv: uv(33, 0, 16, 8),
                tint: WHITE,
            }],
            "the picture placed after the sheet's three 10x12 frames"
        );
    }

    #[test]
    fn with_super_xbr_a_sprites_atlas_entry_is_its_super_xbr_image_at_its_native_place() {
        let frame = solid(5, 6, [0, 0, 255, 255]);
        let images = FakeImages::default().with(ImageKind::Rled, 200, vec![frame.clone()]);
        let mut list = DrawList::new();
        list.sprite(ImageKey::sprite(200, 0), at(30.5, 40.25), Color::WHITE);
        let native = quads_of(
            renderer(FakeImages::default().with(ImageKind::Rled, 200, vec![frame.clone()])),
            &list,
        );
        let mut renderer = renderer(images);
        renderer.set_enlarger(enlarger_for(nova_view::Enlargement::SuperXbr));
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(report, RenderReport::default());
        let enlarged = SuperXbr.enlarge(ImageKind::Rled, &frame);
        assert_eq!(
            gpu.calls[..2],
            [
                GpuCall::CreatePage {
                    page: PageId(0),
                    size: 64
                },
                GpuCall::upload(PageId(0), rect(0, 0, 10, 12), &enlarged),
            ]
        );
        let quads = all_quads(gpu.submits()[0]);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].dest, native[0].dest);
        assert_eq!(quads[0].uv, uv(0, 0, 10, 12));
    }
}
