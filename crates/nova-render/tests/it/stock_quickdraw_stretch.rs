//! The stock frames the original stretches over their dialogs, drawn by
//! QuickDraw's rule (`nova_render::quickdraw_stretch`): `PICT` 8527 over
//! `DLOG` 3004's port and `PICT` 8528 over `DLOG` 1020's. Each size is
//! read from the stock `DLOG`. Skips, passing, when `NOVA_DATA` is unset
//! or the interface file is absent.

use crate::common;

use nova_data::graphics::{EightBitReading, Image};
use nova_data::{GameData, open_interface};
use nova_render::quickdraw_stretch::{Span, columns, rows, stretched};
use nova_render::recording::{GpuCall, RecordingGpu};
use nova_render::{Batch, GameImages, ImageSource, LOGICAL, PAGE_SIZE, Renderer, Viewport};
use nova_view::{DrawList, ImageKey, ImageKind, Point};

/// `DLOG` `id`'s port size, from the stock interface file.
fn port_size(dir: &std::path::Path, id: i16) -> Option<(u32, u32)> {
    let interface = match open_interface(dir) {
        Ok(interface) => interface,
        Err(err) => {
            eprintln!("skipping: {err}");
            return None;
        }
    };
    let (dlog, _) = interface.dialog(id).expect("listed").expect("decodes");
    let bounds = dlog.record.bounds;
    let side = |far: i16, near: i16| u32::try_from(far - near).expect("positive");
    Some((
        side(bounds.right, bounds.left),
        side(bounds.bottom, bounds.top),
    ))
}

/// The image the renderer uploads for `PICT` `id` stretched to `size`,
/// checked to be what its one quad draws from.
fn drawn_stretch(images: impl ImageSource, id: i16, (w, h): (u32, u32)) -> GpuCall {
    let mut list = DrawList::new();
    list.stretched_picture(
        ImageKey::picture(id),
        Point::new(0.0, 0.0),
        w as f32,
        h as f32,
    );
    let mut renderer = Renderer::new(images);
    let mut gpu = RecordingGpu::new();
    let report = renderer.render(&list, &Viewport::new(LOGICAL, (1024, 768), 1.0), &mut gpu);
    assert_eq!(report.new_failures, vec![]);
    let quads: Vec<_> = gpu
        .submits()
        .iter()
        .flat_map(|frame| &frame.batches)
        .flat_map(|batch| match batch {
            Batch::Sprites { quads, .. } => quads.clone(),
            _ => Vec::new(),
        })
        .collect();
    let uploads: Vec<GpuCall> = gpu
        .calls
        .into_iter()
        .filter(|call| matches!(call, GpuCall::Upload { .. }))
        .collect();
    assert_eq!(uploads.len(), 2, "the picture and its stretch");
    let stretch = upload_rect(&uploads[1]);
    let page = PAGE_SIZE as f32;
    assert_eq!(quads.len(), 1);
    assert_eq!(
        (quads[0].uv.u0, quads[0].uv.v0),
        (stretch.x as f32 / page, stretch.y as f32 / page),
        "drawn from the stretch"
    );
    uploads[1].clone()
}

/// The pixel at (`x`, `y`).
fn at(image: &Image, x: u32, y: u32) -> [u8; 4] {
    image.pixel(x, y).expect("inside")
}

#[test]
fn picture_8527_over_dlog_3004_keeps_row_207_at_row_208_by_quickdraws_rule() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let Some(size) = port_size(&dir, 3004) else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let images = GameImages::new(&data, EightBitReading::AsDrawn);
    let picture = images.frames(ImageKind::Pict, 8527).expect("8527")[0].clone();
    assert_ne!((picture.width(), picture.height()), size, "stretched");
    let resampled = stretched(&picture, size.0, size.1);
    let upload = drawn_stretch(images, 8527, size);
    let GpuCall::Upload { width, height, .. } = upload else {
        unreachable!("an upload");
    };
    assert_eq!((width, height), size);
    assert_eq!(
        upload,
        GpuCall::upload(nova_render::PageId(0), upload_rect(&upload), &resampled)
    );
    let rows = rows(picture.height(), size.1);
    assert_eq!(
        rows[208],
        Span {
            first: 207,
            count: 1
        },
        "row 207 at 208"
    );
    assert_eq!(
        rows[122],
        Span {
            first: 121,
            count: 1
        },
        "row 121 doubled"
    );
    let columns = columns(picture.width(), size.0);
    for (x, span) in columns.iter().enumerate() {
        if span.count == 1 {
            let x = u32::try_from(x).expect("narrow");
            assert_eq!(
                at(&resampled, x, 208),
                at(&picture, span.first, 207),
                "column {x}"
            );
        }
    }
}

#[test]
fn picture_8528_over_dlog_1020_repeats_columns_162_and_487() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let Some(size) = port_size(&dir, 1020) else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let images = GameImages::new(&data, EightBitReading::AsDrawn);
    let picture = images.frames(ImageKind::Pict, 8528).expect("8528")[0].clone();
    assert_eq!(picture.height(), size.1, "stretched across only");
    let resampled = stretched(&picture, size.0, size.1);
    let upload = drawn_stretch(images, 8528, size);
    assert_eq!(
        upload,
        GpuCall::upload(nova_render::PageId(0), upload_rect(&upload), &resampled)
    );
    let columns = columns(picture.width(), size.0);
    let repeated: Vec<usize> = (1..columns.len())
        .filter(|&x| columns[x] == columns[x - 1])
        .collect();
    assert_eq!(repeated, [162, 487]);
    for y in [0, picture.height() / 2, picture.height() - 1] {
        assert_eq!(at(&resampled, 162, y), at(&picture, 161, y));
        assert_eq!(at(&resampled, 487, y), at(&picture, 485, y));
    }
}

/// Where `upload` put its image.
fn upload_rect(upload: &GpuCall) -> nova_render::PixelRect {
    match upload {
        GpuCall::Upload { rect, .. } => *rect,
        other => panic!("an upload: {other:?}"),
    }
}
