//! Movie frames ([`nova_view::DrawCommand::MovieFrame`]) streamed onto
//! pages of their own.
//!
//! Each stream a frame is drawn from holds one square page, numbered down
//! from `PageId(u32::MAX)`, so it never takes an atlas page's id, which
//! count up from 0. The frame is uploaded to the page's top-left corner
//! only when its stream's generation moves on: the same frame drawn again
//! costs nothing. A page too small for a new frame is created again,
//! bigger, under the same id. A stream not drawn in a frame lets its page
//! go to the next stream drawn, so the pages never outnumber the movies
//! drawn at once.

use nova_view::{MovieFrame, StreamId};

use crate::atlas::{AtlasEntry, PageId, Uv};
use crate::gpu::{Gpu, MAX_TEXTURE_SIDE};
use crate::viewport::PixelRect;

/// The frame a page holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Held {
    stream: StreamId,
    generation: u64,
}

/// One stream page.
#[derive(Clone, Debug)]
struct Slot {
    page: PageId,
    /// Its side, 0 until it is created.
    side: u32,
    /// The frame on it, if a stream holds it.
    held: Option<Held>,
    /// Whether it was drawn in the frame being made.
    drawn: bool,
}

/// The stream pages and the frames on them.
#[derive(Debug, Default)]
pub(crate) struct Streams {
    slots: Vec<Slot>,
}

impl Streams {
    /// Whether any page has been created: each slot's page is, as the slot
    /// is first taken.
    pub(crate) fn has_pages(&self) -> bool {
        !self.slots.is_empty()
    }

    /// Where `frame` is on its page, uploading it first if the page doesn't
    /// hold it yet: none for a frame with no pixels, or one wider or taller
    /// than [`MAX_TEXTURE_SIDE`].
    pub(crate) fn place(&mut self, frame: &MovieFrame, gpu: &mut impl Gpu) -> Option<AtlasEntry> {
        let (w, h) = frame.size();
        let side = w.max(h);
        if w == 0 || h == 0 || side > MAX_TEXTURE_SIDE {
            return None;
        }
        let held = Held {
            stream: frame.stream(),
            generation: frame.generation(),
        };
        let index = self.slot_for(held.stream, side);
        let slot = &mut self.slots[index];
        slot.drawn = true;
        if slot.side < side {
            gpu.create_page(slot.page, side);
            slot.side = side;
            slot.held = None;
        }
        let rect = PixelRect { x: 0, y: 0, w, h };
        if slot.held != Some(held) {
            gpu.upload(slot.page, rect, frame.image());
            slot.held = Some(held);
        }
        let page_side = slot.side as f32;
        Some(AtlasEntry {
            page: slot.page,
            rect,
            uv: Uv {
                u0: 0.0,
                v0: 0.0,
                u1: w as f32 / page_side,
                v1: h as f32 / page_side,
            },
            opened: false,
        })
    }

    /// The slot `stream` holds; else a free one, one at least `side` big
    /// first; else a new one.
    fn slot_for(&mut self, stream: StreamId, side: u32) -> usize {
        let holds = |slot: &Slot| slot.held.is_some_and(|held| held.stream == stream);
        let free = |slot: &Slot| slot.held.is_none() && !slot.drawn;
        if let Some(index) = self.slots.iter().position(holds) {
            return index;
        }
        if let Some(index) = (self.slots.iter())
            .position(|slot| free(slot) && slot.side >= side)
            .or_else(|| self.slots.iter().position(free))
        {
            return index;
        }
        let page = PageId(u32::MAX - u32::try_from(self.slots.len()).unwrap_or(u32::MAX));
        self.slots.push(Slot {
            page,
            side: 0,
            held: None,
            drawn: false,
        });
        self.slots.len() - 1
    }

    /// Ends a frame: each page whose stream wasn't drawn in it is freed for
    /// the next stream drawn, which uploads onto it.
    pub(crate) fn end_frame(&mut self) {
        for slot in &mut self.slots {
            if !std::mem::take(&mut slot.drawn) {
                slot.held = None;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_data::graphics::Image;
    use nova_view::geometry::Bounds;
    use nova_view::{Blend, Color, DrawCommand, DrawList, ImageKey, ImageKind, MovieScreen, Point};

    use crate::batch::Renderer;
    use crate::enlarge::NativeDetail;
    use crate::gpu::{Batch, Frame, MAX_TEXTURE_SIDE, NO_COLORIZE, QuadInstance, Rect};
    use crate::images::{ImageError, ImageSource};
    use crate::recording::{GpuCall, RecordingGpu};
    use crate::viewport::{LogicalSize, PixelRect, Viewport};
    use crate::{PageId, Uv};

    /// One 4x4 picture, `PICT` 128; nothing else.
    struct OnePicture;

    impl ImageSource for OnePicture {
        fn frames(&self, kind: ImageKind, id: i16) -> Result<Vec<Image>, ImageError> {
            if (kind, id) == (ImageKind::Pict, 128) {
                Ok(vec![Image::from_rgba(4, 4, vec![9; 64]).expect("4x4")])
            } else {
                Err(ImageError::Missing)
            }
        }
    }

    const TOP: PageId = PageId(u32::MAX);
    const SECOND: PageId = PageId(u32::MAX - 1);

    fn viewport() -> Viewport {
        Viewport::new(LogicalSize { w: 64, h: 48 }, (128, 96), 2.0)
    }

    fn renderer() -> Renderer<OnePicture> {
        Renderer::with_page_size(OnePicture, 64)
    }

    fn area(x: f32, y: f32, w: f32, h: f32) -> Bounds {
        Bounds::at(Point::new(x, y), w, h)
    }

    /// Shows a `w` x `h` frame of `byte` on `screen`.
    fn show(screen: &mut MovieScreen, (w, h): (u32, u32), byte: u8) -> nova_view::MovieFrame {
        assert!(screen.show(w, h, vec![byte; (w * h * 4) as usize]));
        screen.frame().expect("shown")
    }

    fn drawing(frames: &[(&nova_view::MovieFrame, Bounds)]) -> DrawList {
        let mut list = DrawList::new();
        for (frame, area) in frames {
            list.movie_frame((*frame).clone(), *area);
        }
        list
    }

    fn rect(w: u32, h: u32) -> PixelRect {
        PixelRect { x: 0, y: 0, w, h }
    }

    /// The calls besides the submits.
    fn uploads(gpu: &mut RecordingGpu) -> Vec<GpuCall> {
        gpu.take()
            .into_iter()
            .filter(|call| !matches!(call, GpuCall::Submit(_)))
            .collect()
    }

    fn last_frame(gpu: &RecordingGpu) -> Frame {
        (*gpu.submits().last().expect("a frame")).clone()
    }

    fn quad(dest: Rect, uv: Uv) -> QuadInstance {
        QuadInstance {
            dest,
            uv,
            tint: [1.0; 4],
            colorize: NO_COLORIZE,
        }
    }

    #[test]
    fn a_movie_frame_opens_a_stream_page_uploads_it_and_draws_one_quad_stretched_into_its_area() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (4, 2), 7);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();

        let report = renderer.render(
            &drawing(&[(&frame, area(10.0, 5.0, 16.0, 8.0))]),
            &viewport(),
            &mut gpu,
        );

        assert_eq!(report, crate::RenderReport::default());
        let submitted = last_frame(&gpu);
        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage { page: TOP, size: 4 },
                GpuCall::upload(TOP, rect(4, 2), frame.image()),
            ]
        );
        assert_eq!(
            submitted.batches,
            [Batch::Sprites {
                page: TOP,
                blend: Blend::Normal,
                quads: vec![quad(
                    Rect {
                        x: 10.0,
                        y: 5.0,
                        w: 16.0,
                        h: 8.0
                    },
                    Uv {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 1.0,
                        v1: 0.5
                    }
                )],
            }]
        );
    }

    #[test]
    fn a_tall_frame_takes_a_page_its_height() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (2, 8), 7);
        let mut gpu = RecordingGpu::new();
        renderer().render(
            &drawing(&[(&frame, area(0.0, 0.0, 2.0, 8.0))]),
            &viewport(),
            &mut gpu,
        );
        let Batch::Sprites { quads, .. } = &last_frame(&gpu).batches[0] else {
            panic!("a sprite batch");
        };
        assert_eq!(
            quads[0].uv,
            Uv {
                u0: 0.0,
                v0: 0.0,
                u1: 0.25,
                v1: 1.0
            }
        );
        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage { page: TOP, size: 8 },
                GpuCall::upload(TOP, rect(2, 8), frame.image()),
            ]
        );
    }

    #[test]
    fn the_same_frame_drawn_again_is_not_uploaded_again() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (4, 2), 7);
        let list = drawing(&[(&frame, area(0.0, 0.0, 4.0, 2.0))]);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        let first = last_frame(&gpu);
        gpu.take();

        renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(last_frame(&gpu), first);
        assert_eq!(uploads(&mut gpu), []);
    }

    #[test]
    fn a_new_frame_of_the_stream_is_uploaded_onto_the_same_page() {
        let mut screen = MovieScreen::new();
        let first = show(&mut screen, (4, 2), 7);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.render(
            &drawing(&[(&first, area(0.0, 0.0, 4.0, 2.0))]),
            &viewport(),
            &mut gpu,
        );
        gpu.take();
        let second = show(&mut screen, (3, 2), 8);

        renderer.render(
            &drawing(&[(&second, area(0.0, 0.0, 4.0, 2.0))]),
            &viewport(),
            &mut gpu,
        );

        assert_eq!(
            uploads(&mut gpu),
            [GpuCall::upload(TOP, rect(3, 2), second.image())]
        );
    }

    #[test]
    fn a_frame_that_outgrows_its_page_recreates_it_under_the_same_id() {
        let mut screen = MovieScreen::new();
        let first = show(&mut screen, (4, 2), 7);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.render(
            &drawing(&[(&first, area(0.0, 0.0, 4.0, 2.0))]),
            &viewport(),
            &mut gpu,
        );
        gpu.take();
        let bigger = show(&mut screen, (6, 5), 8);

        renderer.render(
            &drawing(&[(&bigger, area(0.0, 0.0, 4.0, 2.0))]),
            &viewport(),
            &mut gpu,
        );

        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage { page: TOP, size: 6 },
                GpuCall::upload(TOP, rect(6, 5), bigger.image()),
            ]
        );
    }

    #[test]
    fn two_streams_drawn_together_get_two_pages() {
        let (mut one, mut two) = (MovieScreen::new(), MovieScreen::new());
        let a = show(&mut one, (4, 2), 7);
        let b = show(&mut two, (2, 2), 8);
        let mut gpu = RecordingGpu::new();

        renderer().render(
            &drawing(&[
                (&a, area(0.0, 0.0, 4.0, 2.0)),
                (&b, area(8.0, 0.0, 2.0, 2.0)),
            ]),
            &viewport(),
            &mut gpu,
        );

        let pages: Vec<PageId> = last_frame(&gpu)
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { page, .. } => *page,
                other => panic!("sprites: {other:?}"),
            })
            .collect();
        assert_eq!(pages, [TOP, SECOND]);
        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage { page: TOP, size: 4 },
                GpuCall::upload(TOP, rect(4, 2), a.image()),
                GpuCall::CreatePage {
                    page: SECOND,
                    size: 2
                },
                GpuCall::upload(SECOND, rect(2, 2), b.image()),
            ]
        );
    }

    #[test]
    fn a_stream_drawn_twice_in_a_frame_is_uploaded_once() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (2, 2), 7);
        let mut gpu = RecordingGpu::new();
        renderer().render(
            &drawing(&[
                (&frame, area(0.0, 0.0, 2.0, 2.0)),
                (&frame, area(8.0, 0.0, 2.0, 2.0)),
            ]),
            &viewport(),
            &mut gpu,
        );
        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage { page: TOP, size: 2 },
                GpuCall::upload(TOP, rect(2, 2), frame.image()),
            ]
        );
    }

    #[test]
    fn a_stream_left_undrawn_for_a_frame_frees_its_page_for_the_next_stream() {
        let (mut one, mut two) = (MovieScreen::new(), MovieScreen::new());
        let a = show(&mut one, (4, 2), 7);
        let b = show(&mut two, (3, 3), 8);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let a_list = drawing(&[(&a, area(0.0, 0.0, 4.0, 2.0))]);
        renderer.render(&a_list, &viewport(), &mut gpu);
        gpu.take();

        renderer.render(&DrawList::new(), &viewport(), &mut gpu);
        renderer.render(
            &drawing(&[(&b, area(0.0, 0.0, 3.0, 3.0))]),
            &viewport(),
            &mut gpu,
        );
        assert_eq!(
            uploads(&mut gpu),
            [GpuCall::upload(TOP, rect(3, 3), b.image())],
            "no new page"
        );

        renderer.render(&DrawList::new(), &viewport(), &mut gpu);
        renderer.render(&a_list, &viewport(), &mut gpu);
        assert_eq!(
            uploads(&mut gpu),
            [GpuCall::upload(TOP, rect(4, 2), a.image())],
            "the first stream, drawn again, uploads again"
        );
    }

    #[test]
    fn a_stream_still_drawn_keeps_its_page_from_a_new_one() {
        let (mut one, mut two) = (MovieScreen::new(), MovieScreen::new());
        let a = show(&mut one, (4, 2), 7);
        let b = show(&mut two, (2, 2), 8);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        let a_list = drawing(&[(&a, area(0.0, 0.0, 4.0, 2.0))]);
        renderer.render(&a_list, &viewport(), &mut gpu);
        renderer.render(&a_list, &viewport(), &mut gpu);
        gpu.take();

        renderer.render(
            &drawing(&[
                (&b, area(0.0, 0.0, 2.0, 2.0)),
                (&a, area(8.0, 0.0, 4.0, 2.0)),
            ]),
            &viewport(),
            &mut gpu,
        );

        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage {
                    page: SECOND,
                    size: 2
                },
                GpuCall::upload(SECOND, rect(2, 2), b.image()),
            ]
        );
    }

    #[test]
    fn a_free_page_big_enough_is_taken_before_a_smaller_one() {
        let (mut one, mut two, mut three) =
            (MovieScreen::new(), MovieScreen::new(), MovieScreen::new());
        let small = show(&mut one, (2, 2), 7);
        let big = show(&mut two, (6, 6), 8);
        let next = show(&mut three, (5, 5), 9);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.render(
            &drawing(&[
                (&small, area(0.0, 0.0, 2.0, 2.0)),
                (&big, area(8.0, 0.0, 6.0, 6.0)),
            ]),
            &viewport(),
            &mut gpu,
        );
        renderer.render(&DrawList::new(), &viewport(), &mut gpu);
        gpu.take();

        renderer.render(
            &drawing(&[(&next, area(0.0, 0.0, 5.0, 5.0))]),
            &viewport(),
            &mut gpu,
        );

        assert_eq!(
            uploads(&mut gpu),
            [GpuCall::upload(SECOND, rect(5, 5), next.image())],
            "the 6x6 page, not the 2x2 one made bigger"
        );
    }

    #[test]
    fn a_free_page_too_small_is_made_bigger_before_a_new_one_opens() {
        let (mut one, mut two) = (MovieScreen::new(), MovieScreen::new());
        let small = show(&mut one, (2, 2), 7);
        let big = show(&mut two, (5, 5), 8);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.render(
            &drawing(&[(&small, area(0.0, 0.0, 2.0, 2.0))]),
            &viewport(),
            &mut gpu,
        );
        renderer.render(&DrawList::new(), &viewport(), &mut gpu);
        gpu.take();

        renderer.render(
            &drawing(&[(&big, area(0.0, 0.0, 5.0, 5.0))]),
            &viewport(),
            &mut gpu,
        );

        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::CreatePage { page: TOP, size: 5 },
                GpuCall::upload(TOP, rect(5, 5), big.image()),
            ]
        );
    }

    #[test]
    fn a_new_enlarger_clears_stream_pages_and_the_next_frame_uploads_again() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (4, 2), 7);
        let list = drawing(&[(&frame, area(0.0, 0.0, 4.0, 2.0))]);
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.render(&list, &viewport(), &mut gpu);
        gpu.take();

        renderer.set_enlarger(Box::new(NativeDetail));
        renderer.render(&list, &viewport(), &mut gpu);

        assert_eq!(
            uploads(&mut gpu),
            [
                GpuCall::ClearPages,
                GpuCall::CreatePage { page: TOP, size: 4 },
                GpuCall::upload(TOP, rect(4, 2), frame.image()),
            ],
            "the atlas holds no page, the streams one"
        );
    }

    #[test]
    fn a_new_enlarger_with_no_page_open_clears_nothing() {
        let mut renderer = renderer();
        let mut gpu = RecordingGpu::new();
        renderer.set_enlarger(Box::new(NativeDetail));
        renderer.render(&DrawList::new(), &viewport(), &mut gpu);
        assert_eq!(uploads(&mut gpu), []);
    }

    #[test]
    fn an_undrawable_area_draws_and_uploads_nothing() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (4, 2), 7);
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
            list.push(DrawCommand::MovieFrame {
                frame: frame.clone(),
                area: Bounds {
                    min: Point::new(0.0, 0.0),
                    max: Point::new(w, h),
                },
            });
        }
        let mut gpu = RecordingGpu::new();
        renderer().render(&list, &viewport(), &mut gpu);
        assert_eq!(last_frame(&gpu).batches, []);
        assert_eq!(uploads(&mut gpu), []);
    }

    #[test]
    fn a_frame_with_no_pixels_draws_and_uploads_nothing() {
        for (w, h) in [(0, 0), (0, 2), (2, 0)] {
            let mut screen = MovieScreen::new();
            assert!(screen.show(w, h, Vec::new()));
            let frame = screen.frame().expect("shown");
            let mut gpu = RecordingGpu::new();
            renderer().render(
                &drawing(&[(&frame, area(0.0, 0.0, 4.0, 2.0))]),
                &viewport(),
                &mut gpu,
            );
            assert_eq!(last_frame(&gpu).batches, [], "{w} x {h}");
            assert_eq!(uploads(&mut gpu), [], "{w} x {h}");
        }
    }

    #[test]
    fn a_clipped_movie_frame_crops_its_quad_and_uv() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (4, 4), 7);
        let mut list = DrawList::new();
        list.clipped(
            area(0.0, 0.0, 10.0, 6.0),
            DrawCommand::MovieFrame {
                frame,
                area: area(2.0, 2.0, 16.0, 8.0),
            },
        );
        let mut gpu = RecordingGpu::new();
        renderer().render(&list, &viewport(), &mut gpu);
        assert_eq!(
            last_frame(&gpu).batches,
            [Batch::Sprites {
                page: TOP,
                blend: Blend::Normal,
                quads: vec![quad(
                    Rect {
                        x: 2.0,
                        y: 2.0,
                        w: 8.0,
                        h: 4.0
                    },
                    Uv {
                        u0: 0.0,
                        v0: 0.0,
                        u1: 0.5,
                        v1: 0.5
                    }
                )],
            }]
        );
    }

    #[test]
    fn a_frame_past_the_texture_limit_draws_nothing() {
        for (w, h) in [(MAX_TEXTURE_SIDE + 1, 1), (1, MAX_TEXTURE_SIDE + 1)] {
            let mut screen = MovieScreen::new();
            let frame = show(&mut screen, (w, h), 7);
            let mut gpu = RecordingGpu::new();
            renderer().render(
                &drawing(&[(&frame, area(0.0, 0.0, 4.0, 2.0))]),
                &viewport(),
                &mut gpu,
            );
            assert_eq!(last_frame(&gpu).batches, [], "{w} x {h}");
            assert_eq!(uploads(&mut gpu), [], "{w} x {h}");
        }
    }

    #[test]
    fn a_frame_at_the_texture_limit_draws() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (MAX_TEXTURE_SIDE, 1), 7);
        let mut gpu = RecordingGpu::new();
        renderer().render(
            &drawing(&[(&frame, area(0.0, 0.0, 4.0, 2.0))]),
            &viewport(),
            &mut gpu,
        );
        assert_eq!(last_frame(&gpu).batches.len(), 1);
    }

    #[test]
    fn a_movie_quad_keeps_its_place_in_draw_order_between_atlas_quads() {
        let mut screen = MovieScreen::new();
        let frame = show(&mut screen, (4, 2), 7);
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), Point::new(0.0, 0.0))
            .movie_frame(frame, area(0.0, 0.0, 4.0, 2.0))
            .picture(ImageKey::picture(128), Point::new(4.0, 0.0))
            .dot(Point::new(1.0, 1.0), 1.0, Color::WHITE);
        let mut gpu = RecordingGpu::new();
        renderer().render(&list, &viewport(), &mut gpu);
        let pages: Vec<Option<PageId>> = last_frame(&gpu)
            .batches
            .iter()
            .map(|batch| match batch {
                Batch::Sprites { page, .. } => Some(*page),
                _ => None,
            })
            .collect();
        assert_eq!(pages, [Some(PageId(0)), Some(TOP), Some(PageId(0)), None]);
    }

    #[test]
    fn stream_pages_never_take_an_atlas_page_id() {
        let screens: Vec<MovieScreen> = (0..3).map(|_| MovieScreen::new()).collect();
        let mut screens = screens;
        let frames: Vec<_> = screens
            .iter_mut()
            .map(|screen| show(screen, (2, 2), 7))
            .collect();
        let mut list = DrawList::new();
        list.picture(ImageKey::picture(128), Point::new(0.0, 0.0));
        for frame in &frames {
            list.movie_frame(frame.clone(), area(0.0, 0.0, 2.0, 2.0));
        }
        let mut gpu = RecordingGpu::new();
        renderer().render(&list, &viewport(), &mut gpu);
        let created: Vec<PageId> = uploads(&mut gpu)
            .into_iter()
            .filter_map(|call| match call {
                GpuCall::CreatePage { page, .. } => Some(page),
                _ => None,
            })
            .collect();
        assert_eq!(
            created,
            [PageId(0), TOP, SECOND, PageId(u32::MAX - 2)],
            "the atlas counts up from 0, the streams down from the top"
        );
    }
}
