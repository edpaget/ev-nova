//! Text drawn with full hints: the bundled fallback's stems against what
//! glyphon's own rasteriser drew before the renderer hinted its glyphs.

use crate::wgpu_offscreen::{STEMS, TEXT_AT, draw_text};

use nova_render::fonts::FontFaces;
use nova_view::{Font, Point};

#[test]
fn the_fallbacks_stems_are_no_softer_than_before() {
    let faces = FontFaces::bundled();
    let at = Point::new(TEXT_AT.0, TEXT_AT.1);
    // The partly covered pixels each `l` of `STEMS` drew in its inkiest row
    // with glyphon's rasteriser (recorded in the phase-2 body of
    // play-feel-3): 2, 1 and 2.
    for (size, scale, before) in [(9.0, 2, 2), (12.0, 1, 1), (12.0, 2, 2)] {
        let Some(picture) = draw_text(&faces, Font::Geneva, STEMS, at, size, scale) else {
            return;
        };
        let partials = picture.stem_partials();
        assert_eq!(partials.len(), STEMS.len(), "{size}pt at {scale}x");
        assert!(
            partials.iter().all(|&p| p <= before),
            "{size}pt at {scale}x: {:?}",
            picture.stems()
        );
    }
    // At 9pt and 1x the stems run together into one run, so the whole
    // row's partly covered pixels are counted: 24 before. A blank picture
    // has none, so the row must first light a column for every `l`.
    let Some(picture) = draw_text(&faces, Font::Geneva, STEMS, at, 9.0, 1) else {
        return;
    };
    let lit: usize = picture.stems().iter().map(Vec::len).sum();
    assert!(lit >= STEMS.len(), "{lit} lit: {:?}", picture.stems());
    assert!(picture.row_partials() <= 24, "{:?}", picture.stems());
}
