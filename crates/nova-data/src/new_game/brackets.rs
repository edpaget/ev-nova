//! The new game's bracket corners: the art round the nav target and the
//! ship target, built in code.
//!
//! The engine draws the brackets as four 16 x 16 `cïcn`s a set, one per
//! corner, from Nova Graphics 1 (`nova_view::flight::brackets`). The bible
//! authors no interface art, so the new game builds them here and the
//! loader keeps them on the store as built icons
//! ([`GameData::with_built_icons`](crate::GameData::with_built_icons)), at
//! the IDs the view draws. A store whose data holds its own `cïcn` at an
//! ID keeps that instead ([`GameData::icon`](crate::GameData::icon)).
//!
//! Each corner is an L along the two outer edges of its position, its
//! vertex at the outer corner, in its set's one colour. The colours are
//! placeholders that keep the original's meaning ([`SETS`]).

use std::collections::BTreeMap;

use crate::graphics::Image;

/// The first bracket `cïcn`, Nova Graphics 1's: the nav brackets'. The
/// sets follow four apiece in [`SETS`]' order, each top-left, top-right,
/// bottom-right then bottom-left, as the view draws them.
pub const FIRST: i16 = 10000;

/// Each corner's width and height in pixels.
pub const SIDE: u32 = 16;

/// How long each arm of a corner's L is, in pixels from the outer corner.
const ARM: u32 = 10;

/// How thick each arm of a corner's L is, in pixels.
const THICKNESS: u32 = 2;

/// One set of four corners: what it marks and its colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BracketSet {
    /// What the set marks, as the original's art means it.
    pub meaning: &'static str,
    /// Its one colour, red, green and blue.
    pub rgb: [u8; 3],
}

/// The six sets in `cïcn` order from [`FIRST`]: the nav target, a
/// dominated stellar, then a targeted ship that is hostile, neutral, an
/// escort or disabled. Their colours are placeholders.
pub const SETS: [BracketSet; 6] = [
    BracketSet {
        meaning: "nav target",
        rgb: [224, 224, 224],
    },
    BracketSet {
        meaning: "dominated stellar",
        rgb: [128, 255, 128],
    },
    BracketSet {
        meaning: "hostile ship",
        rgb: [255, 64, 64],
    },
    BracketSet {
        meaning: "neutral ship",
        rgb: [255, 224, 64],
    },
    BracketSet {
        meaning: "escort",
        rgb: [64, 176, 64],
    },
    BracketSet {
        meaning: "disabled ship",
        rgb: [128, 128, 128],
    },
];

/// The 24 corners by `cïcn` ID, from [`FIRST`] on (see the module docs).
#[must_use]
pub fn corners() -> BTreeMap<i16, Image> {
    (FIRST..)
        .zip(SETS.iter().flat_map(|set| {
            [
                Corner::TopLeft,
                Corner::TopRight,
                Corner::BottomRight,
                Corner::BottomLeft,
            ]
            .map(|corner| corner.drawn(set.rgb))
        }))
        .collect()
}

/// A corner's position round the square, in the view's order.
#[derive(Clone, Copy)]
enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl Corner {
    /// The corner's L in `rgb`, on clear.
    fn drawn(self, [r, g, b]: [u8; 3]) -> Image {
        let far = SIDE - 1;
        let (left, top) = match self {
            Self::TopLeft => (true, true),
            Self::TopRight => (false, true),
            Self::BottomRight => (false, false),
            Self::BottomLeft => (true, false),
        };
        let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
        for y in 0..SIDE {
            for x in 0..SIDE {
                let across = if left { x } else { far - x };
                let down = if top { y } else { far - y };
                let inked =
                    (across < ARM && down < THICKNESS) || (across < THICKNESS && down < ARM);
                pixels.extend(if inked { [r, g, b, 255] } else { [0; 4] });
            }
        }
        Image::from_rgba(SIDE, SIDE, pixels).expect("SIDE square")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The IDs of every corner, in order.
    pub(crate) fn ids() -> std::ops::Range<i16> {
        FIRST..FIRST + 24
    }

    fn alpha(image: &Image, x: u32, y: u32) -> u8 {
        image.pixel(x, y).expect("inside")[3]
    }

    fn opaque_rgbs(image: &Image) -> Vec<[u8; 3]> {
        image
            .pixels()
            .chunks(4)
            .filter(|pixel| pixel[3] > 0)
            .map(|pixel| [pixel[0], pixel[1], pixel[2]])
            .collect()
    }

    /// Each corner is `SIDE` square with an opaque pixel.
    pub(crate) fn assert_squares_with_ink(icons: &BTreeMap<i16, Image>) {
        assert_eq!(
            icons.keys().copied().collect::<Vec<_>>(),
            ids().collect::<Vec<_>>()
        );
        for (id, image) in icons {
            assert_eq!((image.width(), image.height()), (SIDE, SIDE), "{id}");
            assert!(!opaque_rgbs(image).is_empty(), "{id} draws something");
        }
    }

    /// The four corners of each set differ, pairwise.
    pub(crate) fn assert_sets_differ_within(icons: &BTreeMap<i16, Image>) {
        let images: Vec<&Image> = icons.values().collect();
        assert_eq!(images.len(), 4 * SETS.len(), "every set");
        for (set, four) in images.chunks(4).enumerate() {
            for (i, a) in four.iter().enumerate() {
                for b in &four[i + 1..] {
                    assert_ne!(a, b, "set {set}");
                }
            }
        }
    }

    /// Each set is one colour, and the six sets six colours.
    pub(crate) fn assert_six_colours(icons: &BTreeMap<i16, Image>) {
        let images: Vec<&Image> = icons.values().collect();
        let colours: Vec<[u8; 3]> = images
            .chunks(4)
            .map(|four| {
                let mut rgbs: Vec<[u8; 3]> = four.iter().flat_map(|i| opaque_rgbs(i)).collect();
                rgbs.dedup();
                assert_eq!(rgbs.len(), 1, "one colour a set: {rgbs:?}");
                rgbs[0]
            })
            .collect();
        assert_eq!(colours.len(), SETS.len());
        for (i, a) in colours.iter().enumerate() {
            for b in &colours[i + 1..] {
                assert_ne!(a, b, "{colours:?}");
            }
        }
    }

    #[test]
    fn each_corner_is_16_square_with_some_opaque_pixels() {
        assert_squares_with_ink(&corners());
    }

    #[test]
    fn each_corner_is_an_l_at_its_outer_corner() {
        let icons = corners();
        let far = SIDE - 1;
        // (outer corner, opposite corner) for top-left, top-right,
        // bottom-right and bottom-left.
        let ends = [
            ((0, 0), (far, far)),
            ((far, 0), (0, far)),
            ((far, far), (0, 0)),
            ((0, far), (far, 0)),
        ];
        for id in ids() {
            let image = &icons[&id];
            let ((ox, oy), (cx, cy)) = ends[usize::try_from(id - FIRST).expect("from FIRST") % 4];
            assert!(alpha(image, ox, oy) > 0, "{id}: inked at its outer corner");
            assert_eq!(alpha(image, cx, cy), 0, "{id}: clear opposite");
            // An L: both arms run from the outer corner, the middle clear.
            let mid = SIDE / 2;
            assert!(alpha(image, ox.abs_diff(mid - 1), oy) > 0, "{id}: across");
            assert!(alpha(image, ox, oy.abs_diff(mid - 1)) > 0, "{id}: down");
            assert_eq!(alpha(image, mid, mid), 0, "{id}: hollow");
        }
    }

    #[test]
    fn the_fixture_answers_every_bracket_icon_with_a_built_corner() {
        let game = crate::new_game::open(&crate::new_game::fixture::folder()).expect("lists");
        let built = corners();
        let answered: BTreeMap<i16, Image> = ids()
            .map(|id| {
                let icon = game.data.icon(id);
                let Some(Ok(image)) = icon else {
                    panic!("{id}: {icon:?}");
                };
                assert_eq!(image, built[&id], "{id} is the built corner");
                (id, image)
            })
            .collect();
        assert_squares_with_ink(&answered);
        assert_sets_differ_within(&answered);
        assert_six_colours(&answered);
    }

    #[test]
    fn the_four_corners_of_a_set_differ() {
        assert_sets_differ_within(&corners());
    }

    #[test]
    fn the_six_sets_are_six_colours() {
        let icons = corners();
        assert_six_colours(&icons);
        let first: Vec<[u8; 3]> = icons
            .values()
            .step_by(4)
            .map(|image| opaque_rgbs(image)[0])
            .collect();
        let sets: Vec<[u8; 3]> = SETS.iter().map(|set| set.rgb).collect();
        assert_eq!(first, sets, "in SETS' order");
    }
}
