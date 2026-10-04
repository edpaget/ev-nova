//! The renderer: resolves image keys into the atlas and turns draw lists
//! into frames of batches.

use std::collections::{HashMap, HashSet};

use nova_view::text::LINE_HEIGHT;
use nova_view::{Blend, Color, DrawCommand, DrawList, ImageKey, ImageKind, Point};

use crate::atlas::{Atlas, AtlasEntry, PAGE_SIZE};
use crate::gpu::{Batch, Frame, Gpu, QuadInstance, Rect, SolidQuad, TextRun};
use crate::images::{ImageError, ImageSource};
use crate::viewport::Viewport;

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

/// One resource's frames once it has been asked for.
#[derive(Debug)]
enum Resident {
    /// Each frame's placement, or why it has none.
    Frames(Vec<Result<AtlasEntry, ImageError>>),
    /// The whole resource failed.
    Failed,
}

/// Turns draw lists into frames for a [`Gpu`], packing images into the
/// atlas the first time a draw list names them.
#[derive(Debug)]
pub struct Renderer<S> {
    images: S,
    atlas: Atlas,
    resident: HashMap<(ImageKind, i16), Resident>,
    reported: HashSet<ImageKey>,
}

impl<S: ImageSource> Renderer<S> {
    /// A renderer drawing images from `images` on [`PAGE_SIZE`] pages.
    #[must_use]
    pub fn new(images: S) -> Self {
        Self::with_page_size(images, PAGE_SIZE)
    }

    /// A renderer drawing images from `images` on `page_size` pages.
    #[must_use]
    pub fn with_page_size(images: S, page_size: u32) -> Self {
        Self {
            images,
            atlas: Atlas::new(page_size),
            resident: HashMap::new(),
            reported: HashSet::new(),
        }
    }

    /// The image source.
    pub fn images(&self) -> &S {
        &self.images
    }

    /// Draws `list` into `viewport` through `gpu`: uploads images seen for
    /// the first time, then submits one frame. A zero-area viewport draws
    /// nothing. Text that is empty, or whose size is not a positive, finite
    /// number of pixels, is skipped, as is a stretched picture whose width
    /// or height is not a positive, finite number.
    pub fn render(
        &mut self,
        list: &DrawList,
        viewport: &Viewport,
        gpu: &mut impl Gpu,
    ) -> RenderReport {
        let mut report = RenderReport::default();
        let Some((rect, scale)) = viewport.content() else {
            return report;
        };
        let mut batches: Vec<Batch> = Vec::new();
        for command in list {
            match *command {
                DrawCommand::Sprite {
                    image,
                    center,
                    tint,
                    blend,
                } => {
                    if let Some(entry) = self.entry(image, gpu, &mut report) {
                        let (w, h) = (entry.rect.w as f32, entry.rect.h as f32);
                        let dest = Rect {
                            x: center.x - w / 2.0,
                            y: center.y - h / 2.0,
                            w,
                            h,
                        };
                        push_quad(&mut batches, &entry, dest, rgba(tint), blend);
                    }
                }
                DrawCommand::Picture { image, top_left } => {
                    let picture = (image, top_left, None);
                    self.push_picture(&mut batches, picture, gpu, &mut report);
                }
                DrawCommand::StretchedPicture {
                    image,
                    top_left,
                    width,
                    height,
                } => {
                    // A rectangle with no drawable size draws nothing, so
                    // its picture is not even loaded.
                    let drawable = |side: f32| side.is_finite() && side > 0.0;
                    if drawable(width) && drawable(height) {
                        let picture = (image, top_left, Some((width, height)));
                        self.push_picture(&mut batches, picture, gpu, &mut report);
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
                    let size_px = size * scale;
                    let line_height_px = LINE_HEIGHT * size_px;
                    // Text with nothing in it draws nothing. Nor does text
                    // whose size is zero, negative, NaN or too big to give
                    // a finite line height in pixels, and the text shaper
                    // would panic on some of those, so skip both.
                    if text.is_empty() || !(line_height_px.is_finite() && line_height_px > 0.0) {
                        continue;
                    }
                    let run = TextRun {
                        text: text.clone(),
                        font,
                        origin_px: (
                            rect.x as f32 + origin.x * scale,
                            rect.y as f32 + origin.y * scale,
                        ),
                        size_px,
                        line_height_px,
                        wrap_px: wrap_width.map(|w| w * scale),
                        color,
                        clip: rect,
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
                } => push_solid(&mut batches, line_corners(from, to, width), color),
                DrawCommand::Dot {
                    center,
                    size,
                    color,
                } => push_solid(&mut batches, square(center, size), color),
            }
        }
        gpu.submit(&Frame {
            target: viewport.window_px(),
            viewport: rect,
            logical: viewport.logical(),
            clear: Color::BLACK,
            batches,
        });
        report
    }

    /// Appends a picture's quad: `(image, top_left, size)`, drawn at `size`
    /// when given and at the picture's own size otherwise.
    fn push_picture(
        &mut self,
        batches: &mut Vec<Batch>,
        (image, top_left, size): (ImageKey, Point, Option<(f32, f32)>),
        gpu: &mut impl Gpu,
        report: &mut RenderReport,
    ) {
        if let Some(entry) = self.entry(image, gpu, report) {
            let (w, h) = size.unwrap_or((entry.rect.w as f32, entry.rect.h as f32));
            let dest = Rect {
                x: top_left.x,
                y: top_left.y,
                w,
                h,
            };
            push_quad(batches, &entry, dest, rgba(Color::WHITE), Blend::Normal);
        }
    }

    /// The placement of `key`, loading its resource on first use. Reports
    /// each new failure.
    fn entry(
        &mut self,
        key: ImageKey,
        gpu: &mut impl Gpu,
        report: &mut RenderReport,
    ) -> Option<AtlasEntry> {
        let resource = (key.kind, key.id);
        if !self.resident.contains_key(&resource) {
            let loaded = self.load(key, gpu, report);
            self.resident.insert(resource, loaded);
        }
        let Resident::Frames(frames) = &self.resident[&resource] else {
            return None;
        };
        let error = match frames.get(usize::from(key.frame)) {
            Some(Ok(entry)) => return Some(*entry),
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
                    .map(|image| {
                        let entry = self.atlas.place(image.width(), image.height())?;
                        if entry.opened {
                            gpu.create_page(entry.page, self.atlas.page_size());
                        }
                        gpu.upload(entry.page, entry.rect, image);
                        Ok(entry)
                    })
                    .collect(),
            ),
        }
    }
}

/// `color` as 0-to-1 floats.
fn rgba(color: Color) -> [f32; 4] {
    [color.r, color.g, color.b, color.a].map(|c| f32::from(c) / 255.0)
}

/// Appends a quad, extending the last batch when it is the same page's
/// and blends the same way.
fn push_quad(
    batches: &mut Vec<Batch>,
    entry: &AtlasEntry,
    dest: Rect,
    tint: [f32; 4],
    blend: Blend,
) {
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
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use nova_data::graphics::Image;
    use nova_view::{Blend, Color, DrawList, Font, ImageKey, ImageKind, Point};

    use super::*;
    use crate::gpu::{Batch, Frame, QuadInstance, Rect, SolidQuad, TextRun};
    use crate::recording::{GpuCall, RecordingGpu};
    use crate::viewport::{LogicalSize, PixelRect, Viewport};
    use crate::{PackError, PageId, Uv};

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

    fn only_submit(calls: &[GpuCall]) -> &Frame {
        match calls {
            [GpuCall::Submit(frame)] => frame,
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
                    batches: vec![Batch::Sprites {
                        page: PageId(0),
                        blend: Blend::Normal,
                        quads: expected_quads,
                    }],
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
            .sprite(sprite(0), at(40.0, 10.0), Color::WHITE);

        let frame = render_one(&list, &viewport());

        assert_eq!(
            sprite_blends(&frame),
            [(Blend::Normal, 1), (Blend::Additive, 2), (Blend::Normal, 1)]
        );
        let xs: Vec<f32> = frame
            .batches
            .iter()
            .flat_map(|batch| match batch {
                Batch::Sprites { quads, .. } => quads.iter().map(|q| q.dest.x).collect(),
                _ => Vec::new(),
            })
            .collect();
        assert_eq!(xs, [7.5, 17.5, 27.5, 37.5]);
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
        submits[0].clone()
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
}
