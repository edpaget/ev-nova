//! The spaceport's landscape: what its landscape item shows.
//!
//! A stellar's landscape `PICT`
//! ([`landscape_id`](super::layout::landscape_id)) is drawn stretched into
//! the item when the data has it. When it doesn't, a stellar with a
//! stand-in ([`PortRecord::stand_in`](super::catalog::PortRecord::stand_in),
//! the new game's only) draws that instead, centred in the item and
//! fitted to it ([`FIT`]), through the path the system scene draws it by
//! ([`outline::draw_stellar`]). With neither, the item is left as the
//! background shows it.
//!
//! The picture comes first, so a stock stellar keeps its `PICT`, and a
//! stock stellar whose `PICT` is missing keeps its blank box: a stock
//! store holds no stand-ins.

use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::image::ImageKey;
use crate::outline::{self, StellarStandIn};

pub use crate::model_picture::FIT;

/// What the landscape item shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Landscape {
    /// `PICT` `id`, which the data has.
    Picture(i16),
    /// The stellar's stand-in, its `PICT` `missing`.
    StandIn {
        /// The landscape `PICT` the data doesn't have.
        missing: i16,
        /// What is drawn instead.
        stand_in: StellarStandIn,
    },
    /// Nothing: `PICT` `id` is missing and the stellar has no stand-in.
    Missing(i16),
}

impl Landscape {
    /// The landscape of a stellar whose landscape `PICT` is `id`: the
    /// picture if it exists, else `stand_in` if there is one, else
    /// nothing.
    #[must_use]
    pub fn of(id: i16, picture_exists: bool, stand_in: Option<StellarStandIn>) -> Self {
        match (picture_exists, stand_in) {
            (true, _) => Self::Picture(id),
            (false, Some(stand_in)) => Self::StandIn {
                missing: id,
                stand_in,
            },
            (false, None) => Self::Missing(id),
        }
    }

    /// The landscape `PICT` that is missing, if it is: with or without a
    /// stand-in drawn in its place.
    #[must_use]
    pub fn missing(&self) -> Option<i16> {
        match self {
            Self::Picture(_) => None,
            Self::StandIn { missing, .. } | Self::Missing(missing) => Some(*missing),
        }
    }

    /// Draws it into `area`, the landscape item: the picture stretched to
    /// fill it, or the stand-in round its centre reaching [`FIT`] of half
    /// its shorter side, or nothing.
    pub fn draw(&self, list: &mut DrawList, area: Bounds) {
        match self {
            Self::Picture(id) => {
                list.stretched_picture(
                    ImageKey::picture(*id),
                    area.min,
                    area.width(),
                    area.height(),
                );
            }
            Self::StandIn { stand_in, .. } => {
                let radius = FIT * area.width().min(area.height()) / 2.0;
                outline::draw_stellar(list, stand_in, area.center(), radius);
            }
            Self::Missing(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::outline::{Outline, OutlineTriangle};

    const RED: Color = Color::rgba(255, 0, 0, 255);

    fn disc() -> StellarStandIn {
        StellarStandIn::Disc(RED)
    }

    /// An outline twice as wide as it is tall: 4 to port and starboard, 1
    /// to the nose and the tail.
    fn wide() -> StellarStandIn {
        let at = Point::new;
        StellarStandIn::Outline(Outline {
            triangles: vec![
                OutlineTriangle {
                    corners: [at(-4.0, 1.0), at(4.0, 1.0), at(4.0, -1.0)],
                    color: RED,
                },
                OutlineTriangle {
                    corners: [at(-4.0, 1.0), at(4.0, -1.0), at(-4.0, -1.0)],
                    color: Color::WHITE,
                },
            ],
        })
    }

    /// The stock landscape item, on screen: wider than it is tall.
    fn area() -> Bounds {
        Bounds::at(Point::new(206.0, 128.0), 612.0, 285.0)
    }

    fn drawn(landscape: &Landscape, area: Bounds) -> DrawList {
        let mut list = DrawList::new();
        landscape.draw(&mut list, area);
        list
    }

    #[test]
    fn a_picture_that_exists_is_drawn_even_with_a_stand_in() {
        assert_eq!(
            Landscape::of(10_003, true, Some(disc())),
            Landscape::Picture(10_003)
        );
        assert_eq!(Landscape::of(9000, true, None), Landscape::Picture(9000));
    }

    #[test]
    fn with_no_picture_a_stand_in_is_drawn_in_its_place() {
        assert_eq!(
            Landscape::of(10_003, false, Some(disc())),
            Landscape::StandIn {
                missing: 10_003,
                stand_in: disc()
            }
        );
    }

    #[test]
    fn with_no_picture_and_no_stand_in_nothing_is() {
        assert_eq!(
            Landscape::of(10_003, false, None),
            Landscape::Missing(10_003)
        );
    }

    #[test]
    fn the_missing_picture_is_named_with_or_without_a_stand_in() {
        assert_eq!(Landscape::Picture(10_003).missing(), None);
        assert_eq!(Landscape::Missing(10_003).missing(), Some(10_003));
        assert_eq!(
            Landscape::of(10_004, false, Some(disc())).missing(),
            Some(10_004)
        );
    }

    #[test]
    fn a_picture_is_stretched_over_the_item() {
        let list = drawn(&Landscape::Picture(10_003), area());
        assert_eq!(
            list.iter().cloned().collect::<Vec<_>>(),
            [DrawCommand::StretchedPicture {
                image: ImageKey::picture(10_003),
                top_left: Point::new(206.0, 128.0),
                width: 612.0,
                height: 285.0,
            }]
        );
    }

    #[test]
    fn a_missing_picture_draws_nothing() {
        assert!(drawn(&Landscape::Missing(10_003), area()).is_empty());
    }

    #[test]
    fn a_stand_in_is_drawn_round_the_centre_at_the_fit_of_the_shorter_side() {
        for (area, shorter) in [
            (area(), 285.0),
            (Bounds::at(Point::new(10.0, 20.0), 100.0, 300.0), 100.0),
        ] {
            for stand_in in [disc(), StellarStandIn::Ring(RED), wide()] {
                let list = drawn(&Landscape::of(10_003, false, Some(stand_in.clone())), area);
                let mut expected = DrawList::new();
                outline::draw_stellar(&mut expected, &stand_in, area.center(), 0.8 * shorter / 2.0);
                assert!(!expected.is_empty());
                assert_eq!(list, expected, "{stand_in:?} in {area:?}");
            }
        }
    }

    #[test]
    fn every_corner_of_a_stand_in_lies_in_the_item() {
        for area in [area(), Bounds::at(Point::new(10.0, 20.0), 100.0, 300.0)] {
            for stand_in in [disc(), StellarStandIn::Ring(RED), wide()] {
                let list = drawn(&Landscape::of(1, false, Some(stand_in.clone())), area);
                let corners: Vec<Point> = list
                    .iter()
                    .flat_map(|command| match command {
                        DrawCommand::Triangle { corners, .. } => corners.to_vec(),
                        other => panic!("a triangle: {other:?}"),
                    })
                    .collect();
                assert!(!corners.is_empty());
                assert!(
                    corners.iter().all(|&p| area.contains(p)),
                    "{stand_in:?} in {area:?}: {corners:?}"
                );
            }
        }
    }

    #[test]
    fn an_outline_with_no_reach_draws_nothing() {
        let flat = StellarStandIn::Outline(Outline {
            triangles: vec![OutlineTriangle {
                corners: [Point::new(0.0, 0.0); 3],
                color: RED,
            }],
        });
        assert!(drawn(&Landscape::of(1, false, Some(flat)), area()).is_empty());
    }
}
