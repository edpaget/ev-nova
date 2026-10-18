//! The maps' nebulae, as `_MapDialogUpdate`'s nebula pass draws them
//! (@0xf947-0xfc84): after item 3's black and the borders, before
//! `_DrawMap`.
//!
//! For each nebula, by ascending ID ([`draw_nebulae`]):
//!
//! 1. **The rect** ([`nebula_rect`]), in port coordinates, around item 3's
//!    centre: left and top truncated from the centre plus the nebula's
//!    corner over the zoom less the scroll over the zoom, right and bottom
//!    truncated from the truncated left and top plus its size over the
//!    zoom (@0xf965-0xfa2e).
//! 2. **The skip**: none drawn when the rect misses item 3, so also when
//!    the truncation leaves it no width or height (`SectRect`
//!    @0xfa42-0xfa49).
//! 3. **The picture** ([`choose`]): by [`PicturePick::LargestThatFits`],
//!    the engine's, the highest of its seven slots, `PICT` 9500 + 7i + k,
//!    whose bounds fit the rect in both width and height, with no early
//!    break; slot 0 when none does; none drawn when that slot is missing
//!    (@0xfa4f-0xfadf). [`RuleKey::NebulaPicture`]'s other reading is
//!    [`PicturePick::SmallestThatCovers`], the map's earlier rule.
//! 4. **The clip**: the picture stretched over the whole rect, drawn only
//!    in the rect's overlap with item 3 (`SectRect` @0xfb02).
//! 5. **The blend**: in a full update, copied over what is beneath with
//!    `addMax` ([`Blend::Max`], `CopyBits` mode 0x25 @0xfc42), so
//!    overlapping nebulae, and the borders under them, take the brighter
//!    of each channel; in a drag's partial update ([`MapUpdate::Partial`])
//!    a plain copy (`srcCopy` @0xfbae), the picture as it is over whatever
//!    came before it.
//!
//! Every edge is truncated in port coordinates, as the original's are,
//! and only then moved onto the screen by the port's origin.
//!
//! **The rect's arithmetic.** The original adds the corner over the zoom
//! to the centre in single precision (`divss`, `addss` @0xf990-0xf994),
//! then takes the scroll over the zoom off in double (`divsd` @0xf9a0)
//! before truncating (`cvttsd2si` @0xf9ac); right and bottom are single
//! throughout (`cvttss2si` @0xfa09). The single-precision terms move an
//! edge by a pixel where the exact value is a whole number, which is
//! common: at zoom 0.421875 every (x − scroll) divisible by 27 is one.
//! [`RectArithmetic::Original`] builds that mix, the default
//! ([`RECT_ARITHMETIC`]); [`RectArithmetic::Double`] is the plain double
//! arithmetic, kept one constant away until the user chooses between
//! them. The edges are the original's 16-bit words; ours are `i32`.
//!
//! [`RuleKey::NebulaPicture`]: nova_sim::RuleKey::NebulaPicture

use nova_sim::RuleSource;

use super::catalog::{NebulaEntry, NebulaPicture};
use super::panel::placed;
use super::update::MapUpdate;
use crate::draw::{Blend, DrawCommand, DrawList};
use crate::flight::radar::IRect;
use crate::{ImageKey, Point};

/// The geometry the nebula pass reads: where the port is, item 3's
/// centre and rect in it, and the zoom and scroll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NebulaFrame {
    /// The port's top-left corner on the screen.
    pub origin: Point,
    /// Item 3's centre in port coordinates, where the scroll is drawn.
    pub centre: (i32, i32),
    /// `_mapZoom`: map units per pixel.
    pub zoom: f64,
    /// `_mapScroll`: the galaxy point drawn at the centre.
    pub scroll: (f64, f64),
    /// Item 3 in port coordinates: what a nebula is clipped to.
    pub area: IRect,
}

/// How [`nebula_rect`] works its edges out (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RectArithmetic {
    /// The original's: the corner's term and the far edges in single
    /// precision, the scroll's term in double.
    Original,
    /// Double precision throughout.
    Double,
}

/// The arithmetic the maps draw their nebulae with.
pub const RECT_ARITHMETIC: RectArithmetic = RectArithmetic::Original;

/// Which picture fills a nebula's rect ([`RuleKey::NebulaPicture`]).
///
/// [`RuleKey::NebulaPicture`]: nova_sim::RuleKey::NebulaPicture
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PicturePick {
    /// The engine's: the largest that fits in width and height, slot 0
    /// when none does, none when slot 0 is missing.
    #[default]
    LargestThatFits,
    /// The other reading: the smallest at least as wide as the rect, or
    /// else the widest.
    SmallestThatCovers,
}

impl PicturePick {
    /// The pick `source` reads [`RuleKey::NebulaPicture`] as.
    ///
    /// [`RuleKey::NebulaPicture`]: nova_sim::RuleKey::NebulaPicture
    #[must_use]
    pub fn of(source: RuleSource) -> Self {
        match source {
            RuleSource::Engine => Self::LargestThatFits,
            RuleSource::Bible => Self::SmallestThatCovers,
        }
    }
}

/// `nebula`'s rect in port coordinates through `frame`, by
/// [`RECT_ARITHMETIC`] (see the module docs).
#[must_use]
pub fn nebula_rect(nebula: &NebulaEntry, frame: &NebulaFrame) -> IRect {
    nebula_rect_by(nebula, frame, RECT_ARITHMETIC)
}

/// `nebula`'s rect in port coordinates through `frame`, by `arithmetic`.
// Port pixels, from 16-bit sizes and corners.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#[must_use]
pub fn nebula_rect_by(
    nebula: &NebulaEntry,
    frame: &NebulaFrame,
    arithmetic: RectArithmetic,
) -> IRect {
    let (cx, cy) = frame.centre;
    let (sx, sy) = frame.scroll;
    let zoom = frame.zoom;
    let (x, y) = (nebula.left, nebula.top);
    let (w, h) = (nebula.width, nebula.height);
    match arithmetic {
        RectArithmetic::Original => {
            // (float)_mapZoom (@0xf971).
            let zf = zoom as f32;
            let near = |centre: i32, at: i16, scroll: f64| {
                let single = centre as f32 + f32::from(at) / zf;
                (f64::from(single) - scroll / zoom).trunc() as i32
            };
            let far = |near: i32, size: i16| (near as f32 + f32::from(size) / zf).trunc() as i32;
            let (left, top) = (near(cx, x, sx), near(cy, y, sy));
            IRect::new(left, top, far(left, w), far(top, h))
        }
        RectArithmetic::Double => {
            let near = |centre: i32, at: i16, scroll: f64| {
                (f64::from(centre) + f64::from(at) / zoom - scroll / zoom).trunc() as i32
            };
            let far =
                |near: i32, size: i16| (f64::from(near) + f64::from(size) / zoom).trunc() as i32;
            let (left, top) = (near(cx, x, sx), near(cy, y, sy));
            IRect::new(left, top, far(left, w), far(top, h))
        }
    }
}

/// The picture `pick` fills `rect` with from `nebula`'s, if any.
#[must_use]
pub fn choose(nebula: &NebulaEntry, rect: IRect, pick: PicturePick) -> Option<NebulaPicture> {
    let (width, height) = (
        i64::from(rect.right) - i64::from(rect.left),
        i64::from(rect.bottom) - i64::from(rect.top),
    );
    let pictures = nebula.pictures.iter();
    match pick {
        PicturePick::LargestThatFits => {
            // The last slot that fits, from a choice that starts at 0
            // (@0xfa5d-0xfac4).
            let slot = pictures
                .clone()
                .filter(|p| i64::from(p.width) <= width && i64::from(p.height) <= height)
                .map(|p| p.slot)
                .max()
                .unwrap_or(0);
            pictures.into_iter().find(|p| p.slot == slot).copied()
        }
        PicturePick::SmallestThatCovers => pictures
            .clone()
            .filter(|p| i64::from(p.width) >= width)
            .min_by_key(|p| p.width)
            .or_else(|| pictures.max_by_key(|p| p.width))
            .copied(),
    }
}

/// Draws `nebulae` through `frame` (see the module docs), by ascending ID,
/// each with the picture `pick` chooses, blended as `update` says.
pub fn draw_nebulae(
    list: &mut DrawList,
    nebulae: &[NebulaEntry],
    frame: &NebulaFrame,
    update: MapUpdate,
    pick: PicturePick,
) {
    let mut ordered: Vec<&NebulaEntry> = nebulae.iter().collect();
    ordered.sort_by_key(|nebula| nebula.id);
    for nebula in ordered {
        let rect = nebula_rect(nebula, frame);
        let Some(clip) = rect.intersection(frame.area) else {
            continue;
        };
        let Some(picture) = choose(nebula, rect, pick) else {
            continue;
        };
        let whole = placed(rect, frame.origin);
        let image = ImageKey::picture(picture.id);
        let (top_left, width, height) = (whole.min, whole.width(), whole.height());
        let command = match update {
            MapUpdate::Full => DrawCommand::BlendedPicture {
                image,
                top_left,
                width,
                height,
                blend: Blend::Max,
            },
            MapUpdate::Partial => DrawCommand::StretchedPicture {
                image,
                top_left,
                width,
                height,
            },
        };
        list.clipped(placed(clip, frame.origin), command);
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::galaxy::catalog::NebulaId;
    use crate::galaxy::place::{INITIAL_ZOOM, MapPlace};

    /// The in-flight maps' port: the dialog at (212, 128), item 3 (9, 8)
    /// to (467, 428), its centre (238, 218), at `zoom` and `scroll`.
    fn dialog(zoom: f64, scroll: (f64, f64)) -> NebulaFrame {
        NebulaFrame {
            origin: Point::new(212.0, 128.0),
            centre: (238, 218),
            zoom,
            scroll,
            area: IRect::new(9, 8, 467, 428),
        }
    }

    fn picture(slot: u8, width: u32, height: u32) -> NebulaPicture {
        NebulaPicture {
            id: 9500 + i16::from(slot),
            slot,
            width,
            height,
        }
    }

    /// Nebula 128 at (`left`, `top`), `width` × `height`, with these
    /// pictures.
    fn nebula(
        left: i16,
        top: i16,
        width: i16,
        height: i16,
        pictures: Vec<NebulaPicture>,
    ) -> NebulaEntry {
        NebulaEntry {
            id: NebulaId(128),
            name: "Holpa Nebula".to_owned(),
            left,
            top,
            width,
            height,
            pictures,
        }
    }

    /// The seven zooms a run reaches, from 0.421875 to 2.3703674…
    fn zooms() -> Vec<f64> {
        let mut place = MapPlace::new((0, 0));
        assert!(place.zoom_in());
        let mut zooms = vec![place.zoom()];
        while place.zoom_out() {
            zooms.push(place.zoom());
        }
        zooms
    }

    // The rect.

    #[test]
    fn the_rect_truncates_its_far_edges_from_its_truncated_near_ones() {
        // left = trunc(238 − 20 / 0.5625 − 10 / 0.5625) = trunc(184.67);
        // right = trunc(184 + 100 / 0.5625) = trunc(361.78): 177 wide,
        // where round(100 / 0.5625) is 178.
        let holpa = nebula(-20, -40, 100, 100, Vec::new());
        let rect = nebula_rect(&holpa, &dialog(INITIAL_ZOOM, (10.0, 20.0)));
        assert_eq!(rect, IRect::new(184, 111, 361, 288));
        assert_ne!(
            rect.right - rect.left,
            (100.0 / INITIAL_ZOOM).round() as i32
        );
    }

    #[test]
    fn an_edge_left_of_the_port_truncates_toward_zero_there() {
        // 238 − 136 / 0.5625 = −3.78: −3 in the port, where truncating the
        // screen's 208.22 would give 208, −4 in the port.
        let holpa = nebula(-136, 0, 100, 100, Vec::new());
        let rect = nebula_rect(&holpa, &dialog(INITIAL_ZOOM, (0.0, 0.0)));
        assert_eq!(rect.left, -3);
        let screen = 212.0 + 238.0 - 136.0 / INITIAL_ZOOM;
        assert_eq!(screen.trunc() - 212.0, -4.0);
    }

    #[test]
    fn the_rect_grows_as_the_zoom_falls() {
        let holpa = nebula(-20, -40, 100, 100, Vec::new());
        let widths: Vec<i32> = zooms()
            .into_iter()
            .map(|zoom| {
                let rect = nebula_rect(&holpa, &dialog(zoom, (10.0, 20.0)));
                assert_eq!(rect.right - rect.left, rect.bottom - rect.top);
                rect.right - rect.left
            })
            .collect();
        assert_eq!(widths, [237, 177, 133, 100, 75, 56, 42]);
    }

    #[test]
    fn the_originals_single_precision_term_moves_an_edge_where_double_would_not() {
        // x − scroll = −135 at zoom 0.421875 is −320 exactly: 238 − 320 =
        // −82 in double, but 238 + (float)(−400 / 0.421875f) rounds above
        // it before the scroll's −265 / 0.421875 comes off, giving −81.
        let zoom = 0.421_875;
        let gate = nebula(-400, 0, 100, 100, Vec::new());
        let frame = dialog(zoom, (-265.0, 0.0));
        let original = nebula_rect_by(&gate, &frame, RectArithmetic::Original);
        let double = nebula_rect_by(&gate, &frame, RectArithmetic::Double);
        assert_eq!((original.left, double.left), (-81, -82));
        assert_eq!(nebula_rect(&gate, &frame), original, "built by default");
        // Elsewhere the two agree.
        let holpa = nebula(-20, -40, 100, 100, Vec::new());
        let frame = dialog(INITIAL_ZOOM, (10.0, 20.0));
        assert_eq!(
            nebula_rect_by(&holpa, &frame, RectArithmetic::Double),
            nebula_rect_by(&holpa, &frame, RectArithmetic::Original)
        );
    }

    // The picture.

    fn chosen(nebula: &NebulaEntry, (w, h): (i32, i32), pick: PicturePick) -> Option<u8> {
        choose(nebula, IRect::new(10, 20, 10 + w, 20 + h), pick).map(|p| p.slot)
    }

    #[test]
    fn the_largest_picture_that_fits_is_chosen_and_enlarged() {
        let holpa = nebula(
            0,
            0,
            251,
            298,
            vec![
                picture(0, 63, 75),
                picture(1, 126, 149),
                picture(2, 251, 298),
            ],
        );
        let pick = PicturePick::LargestThatFits;
        assert_eq!(chosen(&holpa, (141, 167), pick), Some(1));
        assert_eq!(chosen(&holpa, (446, 529), pick), Some(2));
        assert_eq!(chosen(&holpa, (106, 125), pick), Some(0));
    }

    #[test]
    fn a_picture_that_fits_in_width_but_not_height_is_refused() {
        // Obatta at zoom 1: 65 × 90, and its "100%" picture 65 × 98.
        let obatta = nebula(
            60,
            40,
            65,
            90,
            vec![picture(0, 17, 25), picture(1, 33, 49), picture(2, 65, 98)],
        );
        assert_eq!(
            chosen(&obatta, (65, 90), PicturePick::LargestThatFits),
            Some(1)
        );
    }

    #[test]
    fn a_picture_exactly_the_rects_size_fits() {
        let holpa = nebula(
            0,
            0,
            251,
            298,
            vec![picture(0, 63, 75), picture(2, 251, 298)],
        );
        assert_eq!(
            chosen(&holpa, (251, 298), PicturePick::LargestThatFits),
            Some(2)
        );
        assert_eq!(
            chosen(&holpa, (250, 298), PicturePick::LargestThatFits),
            Some(0)
        );
        assert_eq!(
            chosen(&holpa, (251, 297), PicturePick::LargestThatFits),
            Some(0)
        );
    }

    #[test]
    fn the_last_slot_that_fits_wins_with_no_early_break() {
        let odd = nebula(
            0,
            0,
            50,
            50,
            vec![picture(0, 10, 10), picture(1, 80, 80), picture(2, 40, 40)],
        );
        assert_eq!(
            chosen(&odd, (50, 50), PicturePick::LargestThatFits),
            Some(2)
        );
    }

    #[test]
    fn with_none_fitting_slot_0_is_chosen_and_shrunk() {
        let holpa = nebula(
            0,
            0,
            251,
            298,
            vec![picture(0, 63, 75), picture(1, 126, 149)],
        );
        let shrunk = choose(
            &holpa,
            IRect::new(0, 0, 30, 30),
            PicturePick::LargestThatFits,
        );
        assert_eq!(shrunk, Some(picture(0, 63, 75)));
    }

    #[test]
    fn with_none_fitting_and_slot_0_missing_nothing_is_drawn() {
        let holpa = nebula(
            0,
            0,
            251,
            298,
            vec![picture(1, 126, 149), picture(2, 251, 298)],
        );
        assert_eq!(chosen(&holpa, (30, 30), PicturePick::LargestThatFits), None);
        assert_eq!(
            chosen(&holpa, (30, 30), PicturePick::SmallestThatCovers),
            Some(1),
            "the other reading draws one"
        );
        assert_eq!(
            chosen(&holpa, (200, 200), PicturePick::LargestThatFits),
            Some(1)
        );
    }

    #[test]
    fn the_other_reading_takes_the_smallest_not_narrower_than_the_rect_or_else_the_widest() {
        // Out of size order, as nebula 131's are not.
        let l1551 = nebula(
            0,
            0,
            100,
            100,
            vec![
                picture(0, 50, 50),
                picture(1, 200, 200),
                picture(2, 100, 100),
                picture(3, 25, 25),
            ],
        );
        let pick = PicturePick::SmallestThatCovers;
        assert_eq!(chosen(&l1551, (42, 42), pick), Some(0));
        assert_eq!(chosen(&l1551, (56, 56), pick), Some(2));
        assert_eq!(chosen(&l1551, (100, 100), pick), Some(2), "exactly as wide");
        assert_eq!(chosen(&l1551, (133, 133), pick), Some(1));
        assert_eq!(chosen(&l1551, (237, 237), pick), Some(1), "none as wide");
    }

    #[test]
    fn a_nebula_with_no_pictures_has_none_to_choose() {
        let bare = nebula(0, 0, 100, 100, Vec::new());
        for pick in [
            PicturePick::LargestThatFits,
            PicturePick::SmallestThatCovers,
        ] {
            assert_eq!(chosen(&bare, (100, 100), pick), None);
        }
    }

    #[test]
    fn the_rule_key_reads_the_engine_as_the_largest_that_fits() {
        assert_eq!(
            PicturePick::of(RuleSource::Engine),
            PicturePick::LargestThatFits
        );
        assert_eq!(
            PicturePick::of(RuleSource::Bible),
            PicturePick::SmallestThatCovers
        );
        assert_eq!(PicturePick::default(), PicturePick::LargestThatFits);
    }

    // The clip and the draw.

    /// At zoom 1 (a frame at exactly 1.0, so the rect is the corner plus
    /// the centre), scroll (0, 0).
    fn unit() -> NebulaFrame {
        dialog(1.0, (0.0, 0.0))
    }

    fn bounds(left: f32, top: f32, right: f32, bottom: f32) -> crate::geometry::Bounds {
        crate::geometry::Bounds {
            min: Point::new(left, top),
            max: Point::new(right, bottom),
        }
    }

    fn drawn(nebulae: &[NebulaEntry], update: MapUpdate) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw_nebulae(
            &mut list,
            nebulae,
            &unit(),
            update,
            PicturePick::LargestThatFits,
        );
        list.iter().cloned().collect()
    }

    fn blended(id: i16, (left, top): (f32, f32), (w, h): (f32, f32)) -> DrawCommand {
        DrawCommand::BlendedPicture {
            image: ImageKey::picture(id),
            top_left: Point::new(left, top),
            width: w,
            height: h,
            blend: Blend::Max,
        }
    }

    fn clipped(clip: crate::geometry::Bounds, command: DrawCommand) -> DrawCommand {
        DrawCommand::Clipped {
            clip,
            command: Box::new(command),
        }
    }

    #[test]
    fn a_rect_inside_item_3_is_its_picture_max_blended_clipped_to_it() {
        // Port (238 − 100, 218 − 50) to (238, 218 + 50): screen (350, 296)
        // to (450, 396).
        let inside = nebula(
            -100,
            -50,
            100,
            100,
            vec![picture(0, 50, 50), picture(1, 100, 100)],
        );
        assert_eq!(
            drawn(&[inside], MapUpdate::Full),
            [clipped(
                bounds(350.0, 296.0, 450.0, 396.0),
                blended(9501, (350.0, 296.0), (100.0, 100.0))
            )]
        );
    }

    #[test]
    fn a_rect_partly_outside_is_its_whole_picture_clipped_to_item_3() {
        // Port (238 − 300, 218 − 50) to (138, 268), item 3's left at 9:
        // screen (150, 296) to (350, 396), clipped from 221.
        let over = nebula(-300, -50, 200, 100, vec![picture(0, 200, 100)]);
        assert_eq!(
            drawn(&[over], MapUpdate::Full),
            [clipped(
                bounds(221.0, 296.0, 350.0, 396.0),
                blended(9500, (150.0, 296.0), (200.0, 100.0))
            )]
        );
    }

    #[test]
    fn a_rect_wholly_outside_or_with_no_width_draws_nothing() {
        let outside = nebula(-400, -50, 100, 100, vec![picture(0, 50, 50)]);
        let edge = nebula(-329, -50, 100, 100, vec![picture(0, 50, 50)]);
        let flat = NebulaEntry {
            id: NebulaId(129),
            ..nebula(0, 0, 0, 100, vec![picture(0, 50, 50)])
        };
        // At zoom 3 a 2-unit-wide nebula truncates to no width.
        let mut list = DrawList::new();
        let thin = nebula(0, 0, 2, 100, vec![picture(0, 1, 1)]);
        let far = dialog(3.0, (0.0, 0.0));
        assert_eq!(
            nebula_rect(&thin, &far).right,
            nebula_rect(&thin, &far).left
        );
        draw_nebulae(
            &mut list,
            &[thin],
            &far,
            MapUpdate::Full,
            PicturePick::LargestThatFits,
        );
        assert!(list.is_empty());
        assert_eq!(
            drawn(&[outside, edge, flat], MapUpdate::Full),
            [],
            "the second ends on item 3's left edge"
        );
    }

    #[test]
    fn a_nebula_with_no_picture_to_draw_is_skipped() {
        let missing = nebula(-100, -50, 100, 100, vec![picture(1, 500, 500)]);
        assert_eq!(drawn(&[missing], MapUpdate::Full), []);
    }

    #[test]
    fn a_partial_update_draws_plain_copies() {
        let inside = nebula(-100, -50, 100, 100, vec![picture(0, 50, 50)]);
        assert_eq!(
            drawn(&[inside], MapUpdate::Partial),
            [clipped(
                bounds(350.0, 296.0, 450.0, 396.0),
                DrawCommand::StretchedPicture {
                    image: ImageKey::picture(9500),
                    top_left: Point::new(350.0, 296.0),
                    width: 100.0,
                    height: 100.0,
                }
            )]
        );
    }

    #[test]
    fn nebulae_are_drawn_by_ascending_id() {
        let entry = |id, left| NebulaEntry {
            id: NebulaId(id),
            ..nebula(left, -50, 100, 100, vec![picture(0, 50, 50)])
        };
        let ids = |commands: Vec<DrawCommand>| -> Vec<f32> {
            commands
                .iter()
                .map(|command| match command {
                    DrawCommand::Clipped { clip, .. } => clip.min.x,
                    other => panic!("{other:?}"),
                })
                .collect()
        };
        // 131 at −100, 129 at −50, 130 at 0: in ID order, so in a partial
        // update a later one covers an earlier one.
        let nebulae = [entry(131, -100), entry(129, -50), entry(130, 0)];
        assert_eq!(
            ids(drawn(&nebulae, MapUpdate::Partial)),
            [400.0, 450.0, 350.0]
        );
        assert_eq!(ids(drawn(&nebulae, MapUpdate::Full)), [400.0, 450.0, 350.0]);
    }

    #[test]
    fn the_viewers_frame_draws_at_its_own_centre() {
        // The viewer: no port offset, centre (512, 304), zoom 2 (scale
        // 0.5), centred on (100, 50).
        let frame = NebulaFrame {
            origin: Point::new(0.0, 0.0),
            centre: (512, 304),
            zoom: 2.0,
            scroll: (100.0, 50.0),
            area: IRect::new(0, 0, 1024, 608),
        };
        let holpa = nebula(-20, -40, 100, 100, vec![picture(0, 50, 50)]);
        // left = 512 − 10 − 50 = 452; top = 304 − 20 − 25 = 259.
        assert_eq!(nebula_rect(&holpa, &frame), IRect::new(452, 259, 502, 309));
        let mut list = DrawList::new();
        draw_nebulae(
            &mut list,
            &[holpa],
            &frame,
            MapUpdate::Full,
            PicturePick::LargestThatFits,
        );
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [clipped(
                bounds(452.0, 259.0, 502.0, 309.0),
                blended(9500, (452.0, 259.0), (50.0, 50.0))
            )]
        );
    }
}
