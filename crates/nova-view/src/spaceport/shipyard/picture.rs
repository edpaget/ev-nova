//! The ship picture the shipyard and the hire screen show: what the
//! picture item (item 8) and a grid cell's icon draw of a ship class.
//!
//! The choice goes, first to last:
//!
//! - **A movie.** In the picture item alone, a ship whose `MovieFile`
//!   loops draws its frame ([`ShipMovie::draw`]), over any picture, as
//!   `_ShipyardDialogUpdate` copies it there (@0x59042-0x590a3).
//! - **Its `PICT`** ([`ship_picture`]: its own, or that of the
//!   lowest-numbered ship sharing its `shän` base image), stretched into
//!   the item or the icon.
//! - **Its stand-in** ([`ShipLook::stand_in`]), which only the new game's
//!   data has: the outline flight draws for the ship
//!   ([`outline`]), nose up, centred and fitted to
//!   [`FIT`] of half the item's or the icon's shorter side, as the
//!   landscape fits a stellar's.
//! - **Nothing.** "No Picture" and "Available" in the item, as
//!   [`grid::draw_picture`] draws them, and the icon painted black, as
//!   [`grid::draw_cell_picture`] paints it.
//!
//! The movie and the `PICT` come first, in the original's order, so a
//! stock ship keeps them: a stock store holds no stand-ins.

use super::{ShipLook, base_images, ship_picture};
use crate::draw::DrawList;
use crate::geometry::Bounds;
use crate::outline::{self, Outline};
use crate::spaceport::grid;
use crate::spaceport::landscape::FIT;
use crate::spaceport::ship_movie::ShipMovie;
use crate::text::TextMetrics;
use nova_sim::ShipId;

/// What a ship class shows, its movie aside.
#[derive(Clone, Debug, PartialEq)]
pub enum ShipPicture {
    /// `PICT` `id`, which the data has.
    Picture(i16),
    /// The class's stand-in: it has no `PICT`.
    StandIn(Outline),
    /// Nothing: no `PICT` and no stand-in to draw.
    Missing,
}

impl ShipPicture {
    /// `picture` if there is one, else `stand_in` if it reaches past its
    /// centre (one that doesn't would draw nothing), else nothing.
    #[must_use]
    pub fn of(picture: Option<i16>, stand_in: Option<&Outline>) -> Self {
        match (picture, stand_in) {
            (Some(id), _) => Self::Picture(id),
            (None, Some(outline)) if outline.reach() > 0.0 => Self::StandIn(outline.clone()),
            (None, _) => Self::Missing,
        }
    }

    /// The picture of `ship`, from each class's `looks`, a `PICT` there
    /// when `exists` says so.
    #[must_use]
    pub fn read(ship: ShipId, looks: &[ShipLook], exists: impl Fn(i16) -> bool) -> Self {
        let picture = ship_picture(ship, &base_images(looks), exists);
        let stand_in = looks
            .iter()
            .find(|look| look.ship == ship)
            .and_then(|look| look.stand_in.as_ref());
        Self::of(picture, stand_in)
    }

    /// The `PICT`, when it is one.
    #[must_use]
    pub fn picture(&self) -> Option<i16> {
        match self {
            Self::Picture(id) => Some(*id),
            Self::StandIn(_) | Self::Missing => None,
        }
    }

    /// Draws it in the picture item `area`: the `PICT` stretched to fill
    /// it, the stand-in fitted to it, or "No Picture" and "Available".
    pub fn draw(&self, list: &mut DrawList, area: Bounds, metrics: &dyn TextMetrics) {
        match self {
            Self::StandIn(outline) => fitted(list, outline, area),
            Self::Picture(_) | Self::Missing => {
                grid::draw_picture(list, area, self.picture(), metrics);
            }
        }
    }

    /// Draws it in `cell`'s icon ([`grid::icon_bounds`]): the `PICT`
    /// stretched into it, the stand-in fitted to it, or the icon painted
    /// black.
    pub fn draw_cell(&self, list: &mut DrawList, cell: Bounds) {
        match self {
            Self::StandIn(outline) => fitted(list, outline, grid::icon_bounds(cell)),
            Self::Picture(_) | Self::Missing => {
                grid::draw_cell_picture(list, cell, self.picture());
            }
        }
    }

    /// Draws the picture item `area` as the screens show it: the movie's
    /// frame when one is playing, else this ([`ShipPicture::draw`]).
    pub fn draw_item(
        &self,
        movie: &ShipMovie,
        list: &mut DrawList,
        area: Bounds,
        metrics: &dyn TextMetrics,
    ) {
        if !movie.draw(area, list) {
            self.draw(list, area, metrics);
        }
    }
}

/// `outline` nose up round `area`'s centre, its farthest corner at
/// [`FIT`] of half `area`'s shorter side.
fn fitted(list: &mut DrawList, outline: &Outline, area: Bounds) {
    let radius = FIT * area.width().min(area.height()) / 2.0;
    outline::draw(list, outline, area.center(), 0.0, radius / outline.reach());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::draw::DrawCommand;
    use crate::geometry::Point;
    use crate::outline::OutlineTriangle;
    use crate::text::fixture::MonoMetrics;

    const RED: Color = Color::rgba(255, 0, 0, 255);

    /// A hull 3 wide and 8 long: its nose 4 ahead, its tail's corners,
    /// the farthest, 4 behind and 1.5 aside.
    fn hull() -> Outline {
        let at = Point::new;
        Outline {
            triangles: vec![OutlineTriangle {
                corners: [at(0.0, 4.0), at(1.5, -4.0), at(-1.5, -4.0)],
                color: RED,
            }],
        }
    }

    fn look(ship: i16, base_image: Option<i16>, stand_in: Option<Outline>) -> ShipLook {
        ShipLook {
            ship: ShipId(ship),
            base_image,
            movie_file: None,
            stand_in,
        }
    }

    /// The stock picture item: 200 square.
    fn item() -> Bounds {
        Bounds::at(Point::new(686.0, 230.0), 200.0, 200.0)
    }

    fn drawn(draw: impl FnOnce(&mut DrawList)) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        draw(&mut list);
        list.iter().cloned().collect()
    }

    fn corners(commands: &[DrawCommand]) -> Vec<Point> {
        commands
            .iter()
            .flat_map(|command| match command {
                DrawCommand::Triangle { corners, .. } => corners.to_vec(),
                other => panic!("a triangle: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_picture_comes_before_a_stand_in_and_a_stand_in_before_nothing() {
        assert_eq!(
            ShipPicture::of(Some(5000), Some(&hull())),
            ShipPicture::Picture(5000)
        );
        assert_eq!(
            ShipPicture::of(Some(5000), None),
            ShipPicture::Picture(5000)
        );
        assert_eq!(
            ShipPicture::of(None, Some(&hull())),
            ShipPicture::StandIn(hull())
        );
        assert_eq!(ShipPicture::of(None, None), ShipPicture::Missing);
    }

    #[test]
    fn a_stand_in_with_no_reach_is_nothing() {
        let flat = Outline {
            triangles: vec![OutlineTriangle {
                corners: [Point::new(0.0, 0.0); 3],
                color: RED,
            }],
        };
        assert_eq!(ShipPicture::of(None, Some(&flat)), ShipPicture::Missing);
        assert_eq!(
            ShipPicture::of(None, Some(&Outline::default())),
            ShipPicture::Missing
        );
    }

    #[test]
    fn a_ship_s_picture_is_read_from_its_own_pict_a_sharer_s_then_its_stand_in() {
        let looks = [
            look(128, Some(1000), Some(hull())),
            look(129, Some(1000), Some(hull())),
            look(130, Some(1001), Some(hull())),
            look(131, None, None),
        ];
        let exists = |id: i16| id == 5000;
        assert_eq!(
            ShipPicture::read(ShipId(128), &looks, exists),
            ShipPicture::Picture(5000)
        );
        assert_eq!(
            ShipPicture::read(ShipId(129), &looks, exists),
            ShipPicture::Picture(5000),
            "the base image's sharer's"
        );
        assert_eq!(
            ShipPicture::read(ShipId(130), &looks, exists),
            ShipPicture::StandIn(hull())
        );
        assert_eq!(
            ShipPicture::read(ShipId(131), &looks, exists),
            ShipPicture::Missing
        );
        assert_eq!(
            ShipPicture::read(ShipId(140), &looks, exists),
            ShipPicture::Missing,
            "a class with no looks"
        );
    }

    #[test]
    fn only_a_pict_is_a_picture() {
        assert_eq!(ShipPicture::Picture(5003).picture(), Some(5003));
        assert_eq!(ShipPicture::StandIn(hull()).picture(), None);
        assert_eq!(ShipPicture::Missing.picture(), None);
    }

    #[test]
    fn a_stand_in_is_drawn_nose_up_round_the_item_s_centre_at_the_fit_of_its_shorter_side() {
        for area in [item(), Bounds::at(Point::new(10.0, 20.0), 100.0, 300.0)] {
            let commands = drawn(|list| {
                ShipPicture::StandIn(hull()).draw(list, area, &MonoMetrics);
            });
            let radius = FIT * area.width().min(area.height()) / 2.0;
            let mut expected = DrawList::new();
            outline::draw(
                &mut expected,
                &hull(),
                area.center(),
                0.0,
                radius / hull().reach(),
            );
            assert_eq!(commands, expected.iter().cloned().collect::<Vec<_>>());
            let centre = area.center();
            let nose = corners(&commands)[0];
            assert!((nose.x - centre.x).abs() < 1e-3, "{nose:?}");
            assert!(nose.y < centre.y, "the nose up: {nose:?}");
            let farthest = (corners(&commands).iter())
                .map(|p| (p.x - centre.x).hypot(p.y - centre.y))
                .fold(0.0, f32::max);
            assert!((farthest - radius).abs() < 1e-3, "at the fit: {farthest}");
            assert!(corners(&commands).iter().all(|&p| area.contains(p)));
        }
    }

    #[test]
    fn a_pict_or_nothing_draws_in_the_item_as_the_screens_always_have() {
        for picture in [ShipPicture::Picture(5000), ShipPicture::Missing] {
            let commands = drawn(|list| picture.draw(list, item(), &MonoMetrics));
            let mut expected = DrawList::new();
            grid::draw_picture(&mut expected, item(), picture.picture(), &MonoMetrics);
            assert!(!expected.is_empty());
            assert_eq!(commands, expected.iter().cloned().collect::<Vec<_>>());
        }
    }

    #[test]
    fn a_stand_in_is_drawn_in_a_cell_s_icon_fitted_to_it() {
        let cell = Bounds::at(Point::new(40.0, 50.0), 83.0, 54.0);
        let icon = grid::icon_bounds(cell);
        let commands = drawn(|list| ShipPicture::StandIn(hull()).draw_cell(list, cell));
        let mut expected = DrawList::new();
        let radius = FIT * grid::ICON_SIZE / 2.0;
        outline::draw(
            &mut expected,
            &hull(),
            icon.center(),
            0.0,
            radius / hull().reach(),
        );
        assert_eq!(commands, expected.iter().cloned().collect::<Vec<_>>());
        assert!(corners(&commands).iter().all(|&p| icon.contains(p)));
    }

    #[test]
    fn a_pict_or_nothing_draws_in_a_cell_as_the_grid_always_has() {
        let cell = Bounds::at(Point::new(40.0, 50.0), 83.0, 54.0);
        for picture in [ShipPicture::Picture(5000), ShipPicture::Missing] {
            let commands = drawn(|list| picture.draw_cell(list, cell));
            let mut expected = DrawList::new();
            grid::draw_cell_picture(&mut expected, cell, picture.picture());
            assert_eq!(commands, expected.iter().cloned().collect::<Vec<_>>());
        }
    }
}
