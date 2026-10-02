//! Wiring: a sprite sheet and its `spïn` read from a synthetic fork,
//! decoded through the public API only.

use nova_data::graphics::fixture::RledBuilder;
use nova_data::graphics::{Image, SheetLayout, decode_rled};
use nova_data::records::spin::Spin;
use nova_data::{Record, decode_bytes};
use nova_rsrc::fixture::ForkBuilder;
use nova_rsrc::{ResType, ResourceFile};

const FRAME_WIDTH: u32 = 3;
const FRAME_HEIGHT: u32 = 2;

/// A fork holding a six-frame `rlëD` (every frame different) and a `spïn`
/// that lays it out two frames per row.
fn fork() -> ResourceFile {
    let mut sheet = RledBuilder::new(FRAME_WIDTH as u16, FRAME_HEIGHT as u16);
    for i in 0..6u16 {
        let colour = 0x0421 * (i + 1);
        sheet = sheet.frame(|f| {
            f.line()
                .pixels(&[colour, colour ^ 0x7C00, colour ^ 0x03E0])
                .line()
                .skip(1)
                .pixels(&[colour ^ 0x001F])
        });
    }
    // sprites 1000, masks -1, 3x2 frames in a 2x3 grid.
    let spin = [0x03, 0xE8, 0xFF, 0xFF, 0, 3, 0, 2, 0, 2, 0, 3];
    let rled = ResType::from_mac_roman("rlëD").expect("Mac Roman type");
    let fork = ForkBuilder::new()
        .resource(rled, 1000, None, &sheet.build())
        .resource(Spin::TYPE, 128, None, &spin)
        .build();
    ResourceFile::from_bytes(fork.bytes).expect("valid fork")
}

/// The `width` x `height` region of `image` at (`x`, `y`).
fn crop(image: &Image, x: u32, y: u32, width: u32, height: u32) -> Image {
    let pixels = (y..y + height)
        .flat_map(|py| (x..x + width).map(move |px| (px, py)))
        .flat_map(|(px, py)| image.pixel(px, py).expect("inside the image"))
        .collect();
    Image::from_rgba(width, height, pixels).expect("exact size")
}

#[test]
fn a_spin_layout_and_the_default_layout_arrange_the_same_frames() {
    let file = fork();
    let spin_res = file.get(Spin::TYPE, 128).expect("spïn 128");
    let spin = decode_bytes::<Spin>(spin_res.data())
        .expect("spïn decodes")
        .record;
    let rled = ResType::from_mac_roman("rlëD").expect("Mac Roman type");
    let sprites = file.get(rled, spin.sprites_id).expect("its rlëD");

    let laid_out = decode_rled(sprites.data(), spin.sheet_layout()).expect("decodes");
    let default = decode_rled(sprites.data(), None).expect("decodes");

    assert_eq!(laid_out.frames(), default.frames());
    assert_eq!(laid_out.frames().len(), 6);
    assert_eq!(laid_out.layout(), SheetLayout::new(2).expect("2 columns"));
    assert_eq!(default.layout(), SheetLayout::DEFAULT);

    let grid = laid_out.compose();
    let strip = default.compose();
    assert_eq!(
        (grid.width(), grid.height()),
        (2 * FRAME_WIDTH, 3 * FRAME_HEIGHT)
    );
    assert_eq!(
        (strip.width(), strip.height()),
        (6 * FRAME_WIDTH, FRAME_HEIGHT)
    );
    assert_ne!(grid, strip);
    for (i, frame) in (0..).zip(laid_out.frames()) {
        let (gx, gy) = (i % 2 * FRAME_WIDTH, i / 2 * FRAME_HEIGHT);
        assert_eq!(
            &crop(&grid, gx, gy, FRAME_WIDTH, FRAME_HEIGHT),
            frame,
            "grid {i}"
        );
        let sx = i * FRAME_WIDTH;
        assert_eq!(
            &crop(&strip, sx, 0, FRAME_WIDTH, FRAME_HEIGHT),
            frame,
            "strip {i}"
        );
    }
    // Every frame differs from the others.
    for (i, a) in laid_out.frames().iter().enumerate() {
        for b in &laid_out.frames()[i + 1..] {
            assert_ne!(a, b);
        }
    }
}
