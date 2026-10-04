//! The stock data through the renderer: a real `PICT` and `rlëD` decode,
//! pack, upload and batch. Skips, passing, when `NOVA_DATA` is unset.

mod common;

use nova_data::GameData;
use nova_data::graphics::PICT;
use nova_render::recording::{GpuCall, RecordingGpu};
use nova_render::{Batch, ImageSource, LOGICAL, PAGE_SIZE, PageId, Renderer, Viewport};
use nova_rsrc::ResType;
use nova_view::{Color, DrawList, ImageKey, ImageKind, Point};

/// The first resource of `kind` whose frames decode, with its frames,
/// needing at least `min_frames`.
fn first_decodable(
    data: &GameData,
    ty: ResType,
    kind: ImageKind,
    min_frames: usize,
) -> (i16, Vec<nova_data::graphics::Image>) {
    data.ids(ty)
        .iter()
        .find_map(|&id| {
            let frames = data.frames(kind, id).ok()?;
            (frames.len() >= min_frames).then_some((id, frames))
        })
        .expect("a decodable resource in the stock data")
}

#[test]
fn a_stock_picture_and_sprite_render_through_the_atlas() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let (pict, picture) = first_decodable(&data, PICT, ImageKind::Pict, 1);
    let (rled, sprites) = first_decodable(&data, nova_data::graphics::RLED, ImageKind::Rled, 2);
    let mut list = DrawList::new();
    list.picture(ImageKey::picture(pict), Point::new(0.0, 0.0))
        .sprite(
            ImageKey::sprite(rled, 0),
            Point::new(100.0, 100.0),
            Color::WHITE,
        )
        .sprite(
            ImageKey::sprite(rled, 1),
            Point::new(200.0, 100.0),
            Color::WHITE,
        );
    let mut renderer = Renderer::new(&data);
    let mut gpu = RecordingGpu::new();

    let report = renderer.render(&list, &Viewport::new(LOGICAL, (1024, 768), 1.0), &mut gpu);

    assert_eq!(report.new_failures, vec![]);
    assert_eq!(
        gpu.calls[0],
        GpuCall::CreatePage {
            page: PageId(0),
            size: PAGE_SIZE
        }
    );
    let uploaded: Vec<(u32, u32)> = gpu.calls[1..gpu.calls.len() - 1]
        .iter()
        .map(|call| match call {
            GpuCall::Upload { width, height, .. } => (*width, *height),
            other => panic!("expected an upload, got {other:?}"),
        })
        .collect();
    let decoded: Vec<(u32, u32)> = picture
        .iter()
        .chain(&sprites)
        .map(|image| (image.width(), image.height()))
        .collect();
    assert_eq!(uploaded, decoded);
    let GpuCall::Submit(frame) = gpu.calls.last().expect("calls") else {
        panic!("the last call is not a submit");
    };
    match frame.batches.as_slice() {
        [Batch::Sprites { page, quads }] => {
            assert_eq!(*page, PageId(0));
            assert_eq!(quads.len(), 3);
        }
        other => panic!("expected one sprite batch, got {other:?}"),
    }
}

/// Stock Nova's normal-state left button cap, masked by `PICT` 7600,
/// resolves with its rounded corners clear and its centre opaque, as does
/// every other cap.
#[test]
fn a_stock_button_cap_resolves_masked() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for cap in [7500, 7502, 7503, 7505, 7506, 7508] {
        let frames = data
            .frames(ImageKind::MaskedPict { mask: cap + 100 }, cap)
            .unwrap_or_else(|err| panic!("cap {cap}: {err}"));
        let [cap_image] = frames.as_slice() else {
            panic!("cap {cap} is one frame");
        };
        assert_eq!((cap_image.width(), cap_image.height()), (13, 25));
        let alpha = |x, y| cap_image.pixel(x, y).expect("inside")[3];
        assert_eq!(alpha(6, 12), 255, "cap {cap}'s centre");
        let corners = [(0, 0), (12, 0), (0, 24), (12, 24)];
        let clear = corners.iter().filter(|&&(x, y)| alpha(x, y) == 0).count();
        assert_eq!(clear, 2, "cap {cap}'s two outer corners are clear");
        let plain = data.frames(ImageKind::Pict, cap).expect("decodes");
        assert!(
            plain[0].pixels().chunks(4).all(|pixel| pixel[3] == 255),
            "cap {cap} is opaque unmasked"
        );
    }
}
