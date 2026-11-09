//! The one place `nova_data`'s stand-in types ([`nova_data::StandIn`],
//! [`nova_data::Outline`] and their RGB bytes) become the view's
//! ([`Outline`], [`StellarStandIn`] and [`Color`]). The flight and system
//! screens' data adapters both call it.

use nova_data::StandIn;

use crate::color::Color;
use crate::geometry::Point;
use crate::outline::{Outline, OutlineTriangle, StellarStandIn};

/// `[r, g, b]` as a fully opaque view colour.
pub(crate) fn opaque([r, g, b]: [u8; 3]) -> Color {
    Color::rgba(r, g, b, 255)
}

/// `outline`, as the data draws it (`f64` corners, RGB bytes), in the
/// view's points and opaque colours.
pub(crate) fn outline(outline: &nova_data::Outline) -> Outline {
    // A pixel's place in an `f32`: what the view draws in.
    #[allow(clippy::cast_possible_truncation)]
    let point = |[x, y]: [f64; 2]| Point::new(x as f32, y as f32);
    Outline {
        triangles: outline
            .triangles
            .iter()
            .map(|triangle| OutlineTriangle {
                corners: triangle.corners.map(point),
                color: opaque(triangle.color),
            })
            .collect(),
    }
}

/// A stellar's stand-in from the data, in the view's terms.
pub(crate) fn stellar_stand_in(stand_in: &StandIn) -> StellarStandIn {
    match stand_in {
        StandIn::Outline(data) => StellarStandIn::Outline(outline(data)),
        StandIn::Disc { color } => StellarStandIn::Disc(opaque(*color)),
        StandIn::Ring { color } => StellarStandIn::Ring(opaque(*color)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_outline() -> nova_data::Outline {
        nova_data::Outline {
            triangles: vec![nova_data::OutlineTriangle {
                corners: [[0.0, -2.5], [1.5, 1.0], [-1.5, 1.0]],
                color: [10, 20, 30],
            }],
        }
    }

    fn view_outline() -> Outline {
        Outline {
            triangles: vec![OutlineTriangle {
                corners: [
                    Point::new(0.0, -2.5),
                    Point::new(1.5, 1.0),
                    Point::new(-1.5, 1.0),
                ],
                color: Color::rgba(10, 20, 30, 255),
            }],
        }
    }

    #[test]
    fn opaque_keeps_the_channels_and_is_fully_opaque() {
        assert_eq!(opaque([1, 2, 3]), Color::rgba(1, 2, 3, 255));
    }

    #[test]
    fn an_outline_is_in_view_points_and_opaque_colours() {
        assert_eq!(outline(&data_outline()), view_outline());
    }

    #[test]
    fn a_stellar_stand_in_maps_a_disc_a_ring_and_an_outline() {
        assert_eq!(
            stellar_stand_in(&StandIn::Disc { color: [4, 5, 6] }),
            StellarStandIn::Disc(Color::rgba(4, 5, 6, 255))
        );
        assert_eq!(
            stellar_stand_in(&StandIn::Ring { color: [7, 8, 9] }),
            StellarStandIn::Ring(Color::rgba(7, 8, 9, 255))
        );
        assert_eq!(
            stellar_stand_in(&StandIn::Outline(data_outline())),
            StellarStandIn::Outline(view_outline())
        );
    }
}
