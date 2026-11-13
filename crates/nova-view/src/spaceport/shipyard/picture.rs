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
//! - **Its model** ([`ShipPortrait`]), which only the new game's data has:
//!   its class's hull and as-built parts from the world models' table
//!   ([`WorldModels`]), in the still portrait pose, fitted to [`FIT`] of
//!   half the item's or the icon's shorter side by its radius. A grid
//!   draws its model icons last, after every cell and frame, as one run
//!   ([`draw_grid_pictures`]), so the renderer draws them in one models
//!   pass rather than one a cell; nothing the cells draw lies under an
//!   icon's model.
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
//! stock ship keeps them: a stock store holds no stand-ins, and stock
//! data has no models.

use super::{ShipLook, base_images, ship_picture};
use crate::draw::DrawList;
use crate::flight::world_models::WorldModels;
use crate::geometry::Bounds;
use crate::model_picture::{FIT, ShipPortrait};
use crate::outline::{self, Outline};
use crate::spaceport::grid::{self, CellGrid, GridColors};
use crate::spaceport::ship_movie::ShipMovie;
use crate::text::TextMetrics;
use nova_sim_frame::ShipId;

/// What a ship class shows, its movie aside.
#[derive(Clone, Debug, PartialEq)]
pub enum ShipPicture {
    /// `PICT` `id`, which the data has.
    Picture(i16),
    /// The class's model: it has no `PICT`.
    Model(ShipPortrait),
    /// The class's stand-in: it has no `PICT` and no model.
    StandIn(Outline),
    /// Nothing: no `PICT`, no model and no stand-in to draw.
    Missing,
}

impl ShipPicture {
    /// `picture` if there is one, else `model` if there is one, else
    /// `stand_in` if it reaches past its centre (one that doesn't would
    /// draw nothing), else nothing.
    #[must_use]
    pub fn of(
        picture: Option<i16>,
        model: Option<ShipPortrait>,
        stand_in: Option<&Outline>,
    ) -> Self {
        match (picture, model, stand_in) {
            (Some(id), _, _) => Self::Picture(id),
            (None, Some(model), _) => Self::Model(model),
            (None, None, Some(outline)) if outline.reach() > 0.0 => Self::StandIn(outline.clone()),
            (None, None, _) => Self::Missing,
        }
    }

    /// The picture of `ship`, from each class's `looks`, a `PICT` there
    /// when `exists` says so, and its model from `models`.
    #[must_use]
    pub fn read(
        ship: ShipId,
        looks: &[ShipLook],
        models: &WorldModels,
        exists: impl Fn(i16) -> bool,
    ) -> Self {
        let picture = ship_picture(ship, &base_images(looks), exists);
        let stand_in = looks
            .iter()
            .find(|look| look.ship == ship)
            .and_then(|look| look.stand_in.as_ref());
        Self::of(picture, ShipPortrait::of(models, ship), stand_in)
    }

    /// The `PICT`, when it is one.
    #[must_use]
    pub fn picture(&self) -> Option<i16> {
        match self {
            Self::Picture(id) => Some(*id),
            Self::Model(_) | Self::StandIn(_) | Self::Missing => None,
        }
    }

    /// Draws it in the picture item `area`: the `PICT` stretched to fill
    /// it, the model or the stand-in fitted to it, or "No Picture" and
    /// "Available".
    pub fn draw(&self, list: &mut DrawList, area: Bounds, metrics: &dyn TextMetrics) {
        match self {
            Self::Model(portrait) => {
                portrait.draw_in(list, area);
            }
            Self::StandIn(outline) => fitted(list, outline, area),
            Self::Picture(_) | Self::Missing => {
                grid::draw_picture(list, area, self.picture(), metrics);
            }
        }
    }

    /// Draws it in `cell`'s icon ([`grid::icon_bounds`]): the `PICT`
    /// stretched into it, the model or the stand-in fitted to it, or the
    /// icon painted black.
    pub fn draw_cell(&self, list: &mut DrawList, cell: Bounds) {
        match self {
            Self::Model(portrait) => {
                portrait.draw_in(list, grid::icon_bounds(cell));
            }
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

/// Draws the shown cells of `cells` on the grid `area` holding one cell
/// for each of `pictures`, `selected` framed in `colors`' bright and the
/// rest in its dim ([`CellGrid::draw_cells`]): each cell's picture
/// ([`ShipPicture::draw_cell`]), then the rest of it as `draw_rest` draws
/// it; then, after every cell, each model's icon, together as one run
/// (see the module docs).
pub fn draw_grid_pictures(
    list: &mut DrawList,
    cells: &CellGrid,
    (area, selected, colors): (Bounds, Option<usize>, GridColors),
    pictures: &[ShipPicture],
    mut draw_rest: impl FnMut(&mut DrawList, usize, Bounds),
) {
    let mut icons = Vec::new();
    cells.draw_cells(
        list,
        area,
        pictures.len(),
        selected,
        colors,
        |list, index, cell| {
            match &pictures[index] {
                ShipPicture::Model(portrait) => icons.push((portrait, cell)),
                picture => picture.draw_cell(list, cell),
            }
            draw_rest(list, index, cell);
        },
    );
    for (portrait, cell) in icons {
        portrait.draw_in(list, grid::icon_bounds(cell));
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
    use crate::flight::world_models::ShipModel;
    use crate::geometry::Point;
    use crate::model::ModelKey;
    use crate::outline::OutlineTriangle;
    use crate::spaceport::grid::GridColors;
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

    const HULL: ModelKey = ModelKey(5);

    /// Ship 130's hull model, of radius 2.
    fn models() -> WorldModels {
        WorldModels {
            ships: [(
                ShipId(130),
                ShipModel {
                    key: HULL,
                    scale: 1.0,
                    base_transp: 0,
                    radius: 2.0,
                },
            )]
            .into(),
            ..WorldModels::default()
        }
    }

    fn portrait() -> ShipPortrait {
        ShipPortrait::of(&models(), ShipId(130)).expect("a portrait")
    }

    /// `portrait` drawn into `area`.
    fn portrait_in(portrait: &ShipPortrait, area: Bounds) -> Vec<DrawCommand> {
        drawn(|list| {
            assert!(portrait.draw_in(list, area));
        })
    }

    #[test]
    fn a_picture_comes_before_a_model_a_model_before_a_stand_in_and_a_stand_in_before_nothing() {
        assert_eq!(
            ShipPicture::of(Some(5000), Some(portrait()), Some(&hull())),
            ShipPicture::Picture(5000)
        );
        assert_eq!(
            ShipPicture::of(Some(5000), None, None),
            ShipPicture::Picture(5000)
        );
        assert_eq!(
            ShipPicture::of(None, Some(portrait()), Some(&hull())),
            ShipPicture::Model(portrait())
        );
        assert_eq!(
            ShipPicture::of(None, Some(portrait()), None),
            ShipPicture::Model(portrait())
        );
        assert_eq!(
            ShipPicture::of(None, None, Some(&hull())),
            ShipPicture::StandIn(hull())
        );
        assert_eq!(ShipPicture::of(None, None, None), ShipPicture::Missing);
    }

    #[test]
    fn a_ship_s_model_is_read_from_the_table_after_its_pict() {
        let looks = [
            look(130, Some(1001), Some(hull())),
            look(128, Some(1000), None),
        ];
        assert_eq!(
            ShipPicture::read(ShipId(130), &looks, &models(), |_| false),
            ShipPicture::Model(portrait())
        );
        assert_eq!(
            ShipPicture::read(ShipId(130), &looks, &models(), |id| id == 5002),
            ShipPicture::Picture(5002)
        );
        assert_eq!(
            ShipPicture::read(ShipId(130), &looks, &WorldModels::default(), |_| false),
            ShipPicture::StandIn(hull()),
            "an empty table"
        );
        assert_eq!(ShipPicture::Model(portrait()).picture(), None);
    }

    #[test]
    fn a_model_draws_fitted_to_the_item_and_to_a_cell_s_icon() {
        let model = ShipPicture::Model(portrait());
        for area in [item(), Bounds::at(Point::new(10.0, 20.0), 100.0, 300.0)] {
            assert_eq!(
                drawn(|list| model.draw(list, area, &MonoMetrics)),
                portrait_in(&portrait(), area)
            );
        }
        let cell = Bounds::at(Point::new(40.0, 50.0), 83.0, 54.0);
        assert_eq!(
            drawn(|list| model.draw_cell(list, cell)),
            portrait_in(&portrait(), grid::icon_bounds(cell))
        );
        let movie = ShipMovie::new(Vec::new());
        assert_eq!(
            drawn(|list| model.draw_item(&movie, list, item(), &MonoMetrics)),
            portrait_in(&portrait(), item())
        );
    }

    #[test]
    fn a_grid_draws_its_model_icons_as_one_run_after_its_cells() {
        let cells = grid::CellGrid::default();
        let area = Bounds::at(Point::new(0.0, 0.0), 332.0, 110.0);
        let pictures = [
            ShipPicture::Model(portrait()),
            ShipPicture::Picture(5000),
            ShipPicture::Model(portrait()),
            ShipPicture::StandIn(hull()),
            ShipPicture::Missing,
        ];
        let colors = GridColors::STOCK;
        let commands = drawn(|list| {
            draw_grid_pictures(
                list,
                &cells,
                (area, Some(1), colors),
                &pictures,
                |_, _, _| {},
            );
        });
        let mut cells_only = DrawList::new();
        let mut shown = Vec::new();
        cells.draw_cells(
            &mut cells_only,
            area,
            pictures.len(),
            Some(1),
            colors,
            |list, i, cell| {
                shown.push((i, cell));
                if !matches!(pictures[i], ShipPicture::Model(_)) {
                    pictures[i].draw_cell(list, cell);
                }
            },
        );
        let mut want: Vec<DrawCommand> = cells_only.iter().cloned().collect();
        for (i, cell) in shown {
            if let ShipPicture::Model(portrait) = &pictures[i] {
                want.extend(portrait_in(portrait, grid::icon_bounds(cell)));
            }
        }
        assert_eq!(commands, want);
        let models: Vec<usize> = commands
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c, DrawCommand::Model { .. }))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(models.len(), 2);
        assert_eq!(
            models,
            [commands.len() - 2, commands.len() - 1],
            "one run, last"
        );
    }

    #[test]
    fn a_grid_with_no_model_draws_its_cells_as_it_always_has() {
        let cells = grid::CellGrid::default();
        let area = Bounds::at(Point::new(0.0, 0.0), 332.0, 110.0);
        let pictures = [
            ShipPicture::Picture(5000),
            ShipPicture::StandIn(hull()),
            ShipPicture::Missing,
        ];
        let colors = GridColors::STOCK;
        let named = |list: &mut DrawList, i: usize, cell: Bounds| {
            grid::draw_cell_name(list, &MonoMetrics, cell, &format!("ship {i}"));
        };
        let commands = drawn(|list| {
            draw_grid_pictures(list, &cells, (area, None, colors), &pictures, named);
        });
        let want = drawn(|list| {
            cells.draw_cells(list, area, pictures.len(), None, colors, |list, i, cell| {
                pictures[i].draw_cell(list, cell);
                named(list, i, cell);
            });
        });
        assert_eq!(commands, want);
    }

    #[test]
    fn a_stand_in_with_no_reach_is_nothing() {
        let flat = Outline {
            triangles: vec![OutlineTriangle {
                corners: [Point::new(0.0, 0.0); 3],
                color: RED,
            }],
        };
        assert_eq!(
            ShipPicture::of(None, None, Some(&flat)),
            ShipPicture::Missing
        );
        assert_eq!(
            ShipPicture::of(None, None, Some(&Outline::default())),
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
        let none = WorldModels::default();
        assert_eq!(
            ShipPicture::read(ShipId(128), &looks, &none, exists),
            ShipPicture::Picture(5000)
        );
        assert_eq!(
            ShipPicture::read(ShipId(129), &looks, &none, exists),
            ShipPicture::Picture(5000),
            "the base image's sharer's"
        );
        assert_eq!(
            ShipPicture::read(ShipId(130), &looks, &none, exists),
            ShipPicture::StandIn(hull())
        );
        assert_eq!(
            ShipPicture::read(ShipId(131), &looks, &none, exists),
            ShipPicture::Missing
        );
        assert_eq!(
            ShipPicture::read(ShipId(140), &looks, &none, exists),
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
