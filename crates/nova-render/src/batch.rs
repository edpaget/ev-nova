//! The renderer: resolves image keys into the atlas and turns draw lists
//! into frames of batches.

use std::collections::{HashMap, HashSet};

use nova_view::{Color, DrawCommand, DrawList, ImageKey, ImageKind};

use crate::atlas::{Atlas, AtlasEntry, PAGE_SIZE};
use crate::gpu::{Batch, Frame, Gpu, QuadInstance, Rect};
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
    /// nothing.
    pub fn render(
        &mut self,
        list: &DrawList,
        viewport: &Viewport,
        gpu: &mut impl Gpu,
    ) -> RenderReport {
        let mut report = RenderReport::default();
        let Some((rect, _scale)) = viewport.content() else {
            return report;
        };
        let mut batches: Vec<Batch> = Vec::new();
        for command in list {
            match *command {
                DrawCommand::Sprite {
                    image,
                    center,
                    tint,
                } => {
                    if let Some(entry) = self.entry(image, gpu, &mut report) {
                        let (w, h) = (entry.rect.w as f32, entry.rect.h as f32);
                        let dest = Rect {
                            x: center.x - w / 2.0,
                            y: center.y - h / 2.0,
                            w,
                            h,
                        };
                        push_quad(&mut batches, &entry, dest, rgba(tint));
                    }
                }
                DrawCommand::Picture { image, top_left } => {
                    if let Some(entry) = self.entry(image, gpu, &mut report) {
                        let dest = Rect {
                            x: top_left.x,
                            y: top_left.y,
                            w: entry.rect.w as f32,
                            h: entry.rect.h as f32,
                        };
                        push_quad(&mut batches, &entry, dest, rgba(Color::WHITE));
                    }
                }
                _ => {}
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

/// Appends a quad, extending the last batch when it is the same page's.
fn push_quad(batches: &mut Vec<Batch>, entry: &AtlasEntry, dest: Rect, tint: [f32; 4]) {
    let quad = QuadInstance {
        dest,
        uv: entry.uv,
        tint,
    };
    if let Some(Batch::Sprites { page, quads }) = batches.last_mut()
        && *page == entry.page
    {
        quads.push(quad);
        return;
    }
    batches.push(Batch::Sprites {
        page: entry.page,
        quads: vec![quad],
    });
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use nova_data::graphics::Image;
    use nova_view::{Color, DrawList, ImageKey, ImageKind, Point};

    use super::*;
    use crate::gpu::{Batch, Frame, QuadInstance, Rect};
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
                        quads: expected_quads,
                    }],
                }),
            ]
        );
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
                Batch::Sprites { page, quads } => (*page, quads.len()),
                other => panic!("unexpected {other:?}"),
            })
            .collect();
        assert_eq!(pages, [(PageId(0), 1), (PageId(1), 1), (PageId(0), 1)]);
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
}
