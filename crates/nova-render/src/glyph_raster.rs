//! Rasterises glyphs with full hints, in both directions, as the system's
//! TrueType scaler drew the original's QuickDraw text.
//!
//! EV Nova chooses neither hinting nor smoothing: it draws through
//! QuickDraw (`TextFont`, `TextSize`, `DrawString`), which hands the
//! player's `Charcoal.ttf` to the system's TrueType scaler. That scaler
//! runs a face's own instructions in both directions, so Charcoal's stems
//! land on whole pixels. glyphon's rasteriser (swash) hard-codes a hinting
//! mode that ignores TrueType instructions in X, so text is rasterised
//! here instead and handed to glyphon as custom glyphs (the adapter's
//! `hinted_text` module).
//!
//! Each face is hinted by the engine skrifa's `prefer_interpreter` picks,
//! as `FreeType` does with no autohint flags ([`hinter_for`]):
//!
//! - A face with TrueType programs (an `fpgm` or a `prep`, as Charcoal has)
//!   runs its instructions through the interpreter with skrifa's `Target::Mono`
//!   (`FreeType`'s `FT_LOAD_TARGET_MONO`, the v35 interpreter). It is the one
//!   target that runs Charcoal's instructions in X: Charcoal's
//!   instruct-control flags don't mark it ClearType-native, so every smooth
//!   target runs it in backward-compatibility mode, which ignores X moves.
//!   The target also takes each glyph's hinted advance from the face's
//!   `hdmx`, the widths the layout advances by.
//!   The bundled Noto Sans has a `prep` too (7 bytes, setting only the
//!   scan-conversion mode) and no glyph instructions, so the interpreter
//!   leaves its outlines as they are, as swash drew them: the autohinter
//!   would be softer at 12 ppem (its `l` mid-row `[36, 244]` against
//!   `[252, 20]`).
//! - A face with no programs (the bundled Noto Serif) takes the autohinter
//!   with [`AUTOHINT_TARGET`].
//!
//! A glyph the interpreter fails on is drawn from its unhinted outline: at
//! a size whose font or control program (`fpgm`, `prep`) fails, every
//! glyph is, and that size is not set up again. An error in a glyph's own
//! program is not a failure: skrifa's interpreter runs it not pedantic, as
//! `FreeType` does without `FT_LOAD_PEDANTIC`, so the error is dropped and
//! the glyph keeps its points as far as the program moved them. A glyph
//! with no outline draws nothing. Coverage stays antialiased (8-bit
//! area coverage of the hinted outline): stems on the grid fill whole
//! columns while curves and diagonals stay smooth. Whether the original's
//! text was smoothed was the host's text-smoothing preference, not a game
//! rule, so no 1-bit raster is built.
//!
//! Colour and bitmap glyphs, synthetic italics and variation coordinates
//! are not drawn: none of the three faces has them, and the layout never
//! asks for them.

use std::sync::Arc;

use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlinePen, SmoothMode, Target,
};
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Command, Format, Mask, Origin, Point};

/// The target the autohinter hints an unhinted face for: `FreeType`'s
/// default antialiased target, which snaps stems in X as well as Y.
pub const AUTOHINT_TARGET: Target = Target::Smooth {
    mode: SmoothMode::Normal,
    symmetric_rendering: true,
    preserve_linear_metrics: false,
};

/// Which hinting engine draws a face.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hinter {
    /// The face's own TrueType instructions, in both directions
    /// (`Target::Mono`).
    Instructions,
    /// skrifa's autohinter, with [`AUTOHINT_TARGET`].
    Auto,
}

impl Hinter {
    /// The skrifa options this engine hints with.
    fn options(self) -> HintingOptions {
        match self {
            Self::Instructions => HintingOptions {
                engine: Engine::Interpreter,
                target: Target::Mono,
            },
            Self::Auto => HintingOptions {
                engine: Engine::Auto(None),
                target: AUTOHINT_TARGET,
            },
        }
    }
}

/// The engine for a face whose outlines do (`true`) or don't prefer the
/// interpreter: its instructions when it has them, else the autohinter.
#[must_use]
pub fn hinter_for(prefer_interpreter: bool) -> Hinter {
    if prefer_interpreter {
        Hinter::Instructions
    } else {
        Hinter::Auto
    }
}

/// One glyph's coverage at one size, placed as swash places its images.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphRaster {
    /// Pixels from the glyph's origin to the raster's left edge.
    pub left: i32,
    /// Pixels from the baseline up to the raster's top edge.
    pub top: i32,
    /// The raster's width in pixels.
    pub width: u32,
    /// The raster's height in pixels.
    pub height: u32,
    /// One coverage byte per pixel, row by row from the top.
    pub coverage: Vec<u8>,
    /// The advance the scaler fitted the glyph to, in pixels, when it gave
    /// one: the face's `hdmx` width for an instructed face.
    pub advance: Option<f32>,
}

impl GlyphRaster {
    /// Whether the raster covers no pixel.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// A font face and the hinting state it rasterises with.
pub struct HintedFace {
    bytes: Arc<[u8]>,
    index: u32,
    hinter: Hinter,
    /// The hinting instance for each ppem drawn (by its bits); `None` where
    /// the face's hints would not set up, so its glyphs draw unhinted.
    instances: Vec<(u32, Option<HintingInstance>)>,
}

impl HintedFace {
    /// Face `index` of the font file `bytes`; `None` when the bytes hold no
    /// such face.
    #[must_use]
    pub fn new(bytes: Arc<[u8]>, index: u32) -> Option<Self> {
        let font = FontRef::from_index(&bytes, index).ok()?;
        let hinter = hinter_for(font.outline_glyphs().prefer_interpreter());
        Some(Self {
            bytes,
            index,
            hinter,
            instances: Vec::new(),
        })
    }

    /// The engine this face is hinted with.
    #[must_use]
    pub fn hinter(&self) -> Hinter {
        self.hinter
    }

    /// Drops the hinting instance of every size drawn, so the cache holds
    /// only the sizes drawn after.
    pub fn forget_sizes(&mut self) {
        self.instances.clear();
    }

    /// How many sizes the face holds a hinting set-up for.
    #[cfg(test)]
    pub(crate) fn sizes_hinted(&self) -> usize {
        self.instances.len()
    }

    /// `glyph` at `ppem` pixels per em, hinted; `None` for a glyph the face
    /// has no outline for.
    pub fn rasterise(&mut self, glyph: u16, ppem: f32) -> Option<GlyphRaster> {
        let font = FontRef::from_index(&self.bytes, self.index).ok()?;
        let outlines = font.outline_glyphs();
        let outline = outlines.get(GlyphId::from(glyph))?;
        let bits = ppem.to_bits();
        let at = if let Some(at) = self.instances.iter().position(|(key, _)| *key == bits) {
            at
        } else {
            let options = self.hinter.options();
            let size = Size::new(ppem);
            let instance = HintingInstance::new(&outlines, size, LocationRef::default(), options);
            self.instances.push((bits, instance.ok()));
            self.instances.len() - 1
        };
        if let Some(instance) = &self.instances[at].1 {
            let mut pen = Path::default();
            if let Ok(metrics) = outline.draw(DrawSettings::hinted(instance, false), &mut pen) {
                return Some(pen.fill(metrics.advance_width));
            }
        }
        self.rasterise_unhinted(glyph, ppem)
    }

    /// `glyph` at `ppem` pixels per em with no hinting at all; `None` for a
    /// glyph the face has no outline for.
    #[must_use]
    pub fn rasterise_unhinted(&self, glyph: u16, ppem: f32) -> Option<GlyphRaster> {
        let font = FontRef::from_index(&self.bytes, self.index).ok()?;
        let outline = font.outline_glyphs().get(GlyphId::from(glyph))?;
        let mut pen = Path::default();
        let unhinted = DrawSettings::unhinted(Size::new(ppem), LocationRef::default());
        let metrics = outline.draw(unhinted, &mut pen).ok()?;
        Some(pen.fill(metrics.advance_width))
    }
}

/// An outline as zeno path commands. zeno fills every contour as closed,
/// whether or not the outline closes it.
#[derive(Default)]
struct Path(Vec<Command>);

impl Path {
    /// The path's coverage, placed with its origin at the bottom left as
    /// swash renders an outline.
    fn fill(self, advance: Option<f32>) -> GlyphRaster {
        if self.0.is_empty() {
            return GlyphRaster {
                left: 0,
                top: 0,
                width: 0,
                height: 0,
                coverage: Vec::new(),
                advance,
            };
        }
        let mut coverage = Vec::new();
        let placement = Mask::new(&self.0)
            .format(Format::Alpha)
            .origin(Origin::BottomLeft)
            .inspect(|format, w, h| coverage.resize(format.buffer_size(w, h), 0))
            .render_into(&mut coverage, None);
        GlyphRaster {
            left: placement.left,
            top: placement.top,
            width: placement.width,
            height: placement.height,
            coverage,
            advance,
        }
    }
}

impl OutlinePen for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::MoveTo(Point::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::LineTo(Point::new(x, y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0
            .push(Command::QuadTo(Point::new(cx0, cy0), Point::new(x, y)));
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(Command::CurveTo(
            Point::new(cx0, cy0),
            Point::new(cx1, cy1),
            Point::new(x, y),
        ));
    }

    fn close(&mut self) {
        self.0.push(Command::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{FALLBACK_FONT, SERIF_FONT};
    use glyphon::cosmic_text::{CacheKey, CacheKeyFlags};
    use glyphon::fontdb::{Database, Source, Weight};
    use glyphon::{FontSystem, SwashCache};
    use nova_data::fonts::fixture::{block_sfnt, instructed_block_sfnt};

    /// `face`'s glyph for `c`.
    fn glyph(face: &[u8], c: char) -> u16 {
        let font = FontRef::new(face).expect("a font");
        let id = font.charmap().map(c).expect("mapped");
        u16::try_from(id.to_u32()).expect("a 16-bit glyph")
    }

    /// `face` loaded as the only face of a glyphon font system.
    fn font_system(face: &'static [u8]) -> (FontSystem, glyphon::fontdb::ID) {
        let mut db = Database::new();
        let id = db.load_font_source(Source::Binary(Arc::new(face)))[0];
        (
            FontSystem::new_with_locale_and_db("en-US".to_owned(), db),
            id,
        )
    }

    /// swash's image of `glyph` at `ppem`, as glyphon draws text today:
    /// hinted vertically only, or with no hints at all.
    fn swash(face: &'static [u8], glyph: u16, ppem: f32, hinted: bool) -> GlyphRaster {
        let (mut system, id) = font_system(face);
        let flags = if hinted {
            CacheKeyFlags::empty()
        } else {
            CacheKeyFlags::DISABLE_HINTING
        };
        let (key, _, _) = CacheKey::new(id, glyph, ppem, (0.0, 0.0), Weight::NORMAL, flags);
        let image = SwashCache::new()
            .get_image_uncached(&mut system, key)
            .expect("an image");
        GlyphRaster {
            left: image.placement.left,
            top: image.placement.top,
            width: image.placement.width,
            height: image.placement.height,
            coverage: image.data,
            advance: None,
        }
    }

    fn noto() -> HintedFace {
        HintedFace::new(Arc::from(FALLBACK_FONT), 0).expect("Noto Sans loads")
    }

    /// The partly covered pixels (0 < c < 250) along `raster`'s middle row.
    fn mid_row_partials(raster: &GlyphRaster) -> usize {
        let width = raster.width as usize;
        let row = raster.height as usize / 2;
        let row = &raster.coverage[row * width..(row + 1) * width];
        row.iter().filter(|&&c| c > 0 && c < 250).count()
    }

    #[test]
    fn an_unhinted_glyph_draws_exactly_as_swash_draws_it() {
        let face = noto();
        for c in ['l', 'a', 'g', 'W'] {
            for ppem in [9.0, 12.0, 24.0] {
                let id = glyph(FALLBACK_FONT, c);
                let mut ours = face.rasterise_unhinted(id, ppem).expect("an outline");
                ours.advance = None;
                assert_eq!(
                    ours,
                    swash(FALLBACK_FONT, id, ppem, false),
                    "{c:?} at {ppem}"
                );
            }
        }
    }

    #[test]
    fn the_bundled_faces_stems_are_no_softer_than_swashs_hinted_ones() {
        for bytes in [FALLBACK_FONT, SERIF_FONT] {
            let mut face = HintedFace::new(Arc::from(bytes), 0).expect("loads");
            let l = glyph(bytes, 'l');
            for ppem in [9.0, 12.0, 18.0, 24.0] {
                let ours = face.rasterise(l, ppem).expect("an outline");
                let theirs = swash(bytes, l, ppem, true);
                assert!(
                    mid_row_partials(&ours) <= mid_row_partials(&theirs),
                    "{ppem} ppem: ours {ours:?}, swash's {theirs:?}"
                );
            }
        }
    }

    #[test]
    fn the_autohinter_snaps_the_serifs_stems_in_x() {
        // Noto Serif's `l` at 18 ppem: swash's light hints leave
        // `[0, 16, 255, 164, 0, 0]`, two soft columns.
        let mut face = HintedFace::new(Arc::from(SERIF_FONT), 0).expect("loads");
        let l = face
            .rasterise(glyph(SERIF_FONT, 'l'), 18.0)
            .expect("an outline");
        assert_eq!(mid_row_partials(&l), 1, "{l:?}");
    }

    #[test]
    fn a_face_is_hinted_by_its_instructions_when_it_prefers_the_interpreter() {
        assert_eq!(hinter_for(true), Hinter::Instructions);
        assert_eq!(hinter_for(false), Hinter::Auto);
    }

    #[test]
    fn each_engine_hints_with_its_own_target() {
        let instructions = Hinter::Instructions.options();
        assert!(matches!(instructions.engine, Engine::Interpreter));
        assert_eq!(instructions.target, Target::Mono);
        let auto = Hinter::Auto.options();
        assert!(matches!(auto.engine, Engine::Auto(None)));
        assert_eq!(auto.target, AUTOHINT_TARGET);
    }

    #[test]
    fn a_face_with_no_instructions_takes_the_autohinter() {
        let block: Arc<[u8]> = block_sfnt("Block").build().into();
        assert_eq!(
            HintedFace::new(block, 0).expect("loads").hinter(),
            Hinter::Auto
        );
        let serif = HintedFace::new(Arc::from(SERIF_FONT), 0).expect("loads");
        assert_eq!(serif.hinter(), Hinter::Auto);
    }

    #[test]
    fn a_face_with_only_a_control_program_runs_it() {
        // Noto Sans's 7-byte `prep` sets only the scan-conversion mode, and
        // its glyphs carry no instructions, but a `prep` is enough for the
        // interpreter, as in FreeType.
        assert_eq!(noto().hinter(), Hinter::Instructions);
    }

    #[test]
    fn a_face_with_a_font_program_runs_its_instructions() {
        // One `fpgm` instruction: PUSHB[0] 0, POP.
        let hinted: Arc<[u8]> = block_sfnt("Block")
            .table(b"fpgm", [0xB0, 0x00, 0x21])
            .build()
            .into();
        let mut face = HintedFace::new(hinted, 0).expect("loads");
        assert_eq!(face.hinter(), Hinter::Instructions);
        let raster = face.rasterise(glyph(&block_sfnt("Block").build(), 'A'), 12.0);
        assert!(!raster.expect("an outline").is_empty());
    }

    /// A program that fails in skrifa's interpreter whether or not it is
    /// pedantic: PUSHB[1] 1 0, DIV (1 / 0). POP on an empty stack fails
    /// only a pedantic run, which pops a 0 instead.
    const DIVIDE_BY_ZERO: [u8; 4] = [0xB1, 0x01, 0x00, 0x62];

    /// A program that runs: PUSHB[0] 0, POP.
    const PUSH_AND_POP: [u8; 3] = [0xB0, 0x00, 0x21];

    /// The block font's one glyph with an outline.
    fn block_glyph() -> u16 {
        glyph(&block_sfnt("Block").build(), 'A')
    }

    fn face_of(sfnt: &nova_data::fonts::fixture::SfntBuilder) -> HintedFace {
        HintedFace::new(sfnt.build().into(), 0).expect("loads")
    }

    #[test]
    fn a_face_whose_font_or_control_program_fails_draws_its_unhinted_outline() {
        let block = block_glyph();
        for table in [b"fpgm", b"prep"] {
            let mut face = face_of(&block_sfnt("Block").table(table, DIVIDE_BY_ZERO));
            assert_eq!(face.hinter(), Hinter::Instructions);
            for ppem in [12.0, 14.5] {
                let raster = face.rasterise(block, ppem);
                assert!(!raster.as_ref().expect("an outline").is_empty());
                assert_eq!(raster, face.rasterise_unhinted(block, ppem), "{table:?}");
            }
        }
        // A program that runs draws on the hinting grid instead, which at a
        // fractional size is the whole ppem's.
        let mut runs = face_of(&block_sfnt("Block").table(b"fpgm", PUSH_AND_POP));
        assert_ne!(
            runs.rasterise(block, 14.5),
            runs.rasterise_unhinted(block, 14.5)
        );
    }

    #[test]
    fn a_size_whose_program_failed_is_not_set_up_again_for_each_glyph() {
        let mut face = face_of(&block_sfnt("Block").table(b"prep", DIVIDE_BY_ZERO));
        for glyph in [block_glyph(), 0, block_glyph()] {
            face.rasterise(glyph, 12.0);
        }
        assert_eq!(face.sizes_hinted(), 1);
        assert!(face.instances[0].1.is_none());
        face.rasterise(block_glyph(), 9.0);
        assert_eq!(face.sizes_hinted(), 2);
    }

    #[test]
    fn a_glyph_whose_own_program_fails_still_draws_its_outline() {
        // skrifa's interpreter, run (as FreeType runs it without
        // `FT_LOAD_PEDANTIC`) not pedantic, drops an error in a glyph's own
        // program and keeps the points as far as the program moved them:
        // here, before it moved any, the outline as a glyph with no program
        // draws it. The draw never fails, so it is not drawn unhinted.
        let block = block_glyph();
        let mut failing = face_of(&instructed_block_sfnt("Block", &DIVIDE_BY_ZERO));
        assert_eq!(failing.hinter(), Hinter::Instructions);
        let mut plain = face_of(&block_sfnt("Block").table(b"fpgm", PUSH_AND_POP));
        for ppem in [12.0, 14.5] {
            let raster = failing.rasterise(block, ppem);
            assert!(!raster.as_ref().expect("an outline").is_empty());
            assert_eq!(raster, plain.rasterise(block, ppem), "{ppem}");
        }
        assert_ne!(
            failing.rasterise(block, 14.5),
            failing.rasterise_unhinted(block, 14.5)
        );
    }

    #[test]
    fn forgetting_its_sizes_empties_a_faces_hinting_cache() {
        let mut face = noto();
        face.rasterise(glyph(FALLBACK_FONT, 'l'), 12.0);
        face.rasterise(glyph(FALLBACK_FONT, 'l'), 9.0);
        assert_eq!(face.sizes_hinted(), 2);
        face.forget_sizes();
        assert_eq!(face.sizes_hinted(), 0);
        assert!(face.rasterise(glyph(FALLBACK_FONT, 'l'), 12.0).is_some());
        assert_eq!(face.sizes_hinted(), 1);
    }

    #[test]
    fn a_raster_with_no_width_or_no_height_is_empty() {
        let raster = |width, height| GlyphRaster {
            left: 0,
            top: 0,
            width,
            height,
            coverage: vec![255; (width * height) as usize],
            advance: None,
        };
        assert!(raster(0, 0).is_empty());
        assert!(raster(3, 0).is_empty());
        assert!(raster(0, 3).is_empty());
        assert!(!raster(1, 1).is_empty());
    }

    #[test]
    fn a_cubic_contour_fills_as_its_curve_bounds_it() {
        // A TrueType face never draws a cubic (a CFF face would): a square
        // 8 pixels wide whose top is one cubic from right to left.
        let mut path = Path::default();
        path.move_to(0.0, 0.0);
        path.line_to(8.0, 0.0);
        path.curve_to(8.0, 4.0, 0.0, 4.0, 0.0, 0.0);
        path.close();
        let raster = path.fill(None);
        // zeno bounds the raster by the curve's control points.
        assert_eq!((raster.width, raster.height), (8, 4));
        assert!(raster.coverage.contains(&255), "{raster:?}");
    }

    #[test]
    fn bytes_that_are_not_a_font_load_no_face() {
        assert!(HintedFace::new(Arc::from(&b"not a font"[..]), 0).is_none());
        assert!(HintedFace::new(Arc::from(FALLBACK_FONT), 1).is_none());
    }

    #[test]
    fn a_glyph_past_the_end_draws_nothing() {
        let mut face = noto();
        assert_eq!(face.rasterise(u16::MAX, 12.0), None);
        assert_eq!(face.rasterise_unhinted(u16::MAX, 12.0), None);
    }

    #[test]
    fn a_glyph_with_no_contours_has_an_empty_raster_and_its_advance() {
        let mut face = noto();
        let space = face
            .rasterise(glyph(FALLBACK_FONT, ' '), 12.0)
            .expect("a glyph");
        assert!(space.is_empty());
        assert_eq!(space.coverage, Vec::<u8>::new());
        assert!(space.advance.is_some_and(|advance| advance > 0.0));
    }
}
