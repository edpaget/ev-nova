//! The in-flight maps' click, as `_DoSystemMap` hit-tests a press in its
//! item 3 (@0x12dd4-0x12f5a) and hands what it hit to the session: the
//! hit box ([`hits`]), the press held for the map's owner
//! ([`PendingPress`]) and the owner's answer ([`PressAnswer`]). Which hit
//! counts, and what it does, is `nova_sim::Session::map_click`'s.
//!
//! `_MapFilter` gives item 3 on mouse-down (@0x14552), so the click acts
//! at the press: a press that hits a system the owner accepts acts at
//! once and drags nothing, and any other drags the map. The viewer keeps
//! its own click, on the release, with its stack cycling.

use super::catalog::SystemId;
use super::model::GalaxyModel;
use super::view::Transform;
use crate::geometry::{Bounds, Point};

/// How far each side of a system's point its hit box reaches: the point
/// rect inset by −10 (`InsetRect` @0x12f3e), a 20 × 20 box.
pub const HIT_HALF: i32 = 10;

/// Every system `model` clicks (its visible ones), by ascending ID, whose
/// hit box holds the press at `at` drawn through `transform`: with its
/// point (h, v) in whole pixels ([`Transform::pixel`]) and the pointer
/// truncated to (x, y), h − 10 ≤ x < h + 10 and v − 10 ≤ y < v + 10, as
/// `PtInRect` leaves the box's right and bottom edges out (@0x12f53).
#[must_use]
pub fn hits(model: &GalaxyModel, at: Point, transform: &Transform) -> Vec<SystemId> {
    let (x, y) = truncated(at);
    model
        .systems()
        .iter()
        .filter(|system| {
            let (h, v) = transform.pixel(system.position());
            hit_box(h, v).contains_half_open(pixel(x, y))
        })
        .map(|system| system.entry.id)
        .collect()
}

/// The 20 × 20 hit box round the whole-pixel point (h, v), tested half-open
/// as `PtInRect` tests it ([`Bounds::contains_half_open`]).
#[allow(clippy::cast_precision_loss)] // Screen pixels.
fn hit_box(h: i32, v: i32) -> Bounds {
    let side = (2 * HIT_HALF) as f32;
    Bounds::at(pixel(h - HIT_HALF, v - HIT_HALF), side, side)
}

/// The whole pixel (x, y) as a point.
#[allow(clippy::cast_precision_loss)] // Screen pixels.
fn pixel(x: i32, y: i32) -> Point {
    Point::new(x as f32, y as f32)
}

/// `at` truncated toward zero, as the event's point is whole pixels.
#[allow(clippy::cast_possible_truncation)] // Screen pixels.
fn truncated(at: Point) -> (i32, i32) {
    (at.x.trunc() as i32, at.y.trunc() as i32)
}

/// A left press in an in-flight map's item 3 that hit systems, held for
/// the map's owner to answer (`GalaxyMap::take_press`).
#[derive(Clone, Debug, PartialEq)]
pub struct PendingPress {
    /// Where it was pressed.
    pub at: Point,
    /// The systems its hit box holds ([`hits`]), by ascending ID.
    pub hits: Vec<SystemId>,
    /// Whether Shift was held, as the map last saw it.
    pub shift: bool,
}

/// What the owner made of an accepted press (`GalaxyMap::answer_press`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PressAnswer {
    /// The map's new selection (`_selSystem`); `None` leaves it.
    pub selected: Option<SystemId>,
    /// On the hypergate map, its new pick, `Some(None)` for none; `None`
    /// leaves it.
    pub gate_pick: Option<Option<SystemId>>,
    /// What the press writes to `showClearRouteButton`; `None` leaves it.
    pub clear_route: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::galaxy::catalog::Galaxy;
    use crate::galaxy::model::star_system;
    use crate::galaxy::place::MapPlace;
    use crate::galaxy::testkit::{galaxy, system};
    use nova_sim::story::{GalaxyOverlay, OverlaidGalaxy};

    fn ids(list: &[i16]) -> Vec<SystemId> {
        list.iter().copied().map(SystemId).collect()
    }

    /// The course map's transform drawing (0, 0) at (450, 346), at
    /// `zoom`: the dialog's port at (212, 128), item 3's centre (238, 218)
    /// in it.
    fn transform(zoom: f64) -> Transform {
        MapPlace::new((0, 0))
            .with_zoom(zoom)
            .transform((212, 128), (238, 218))
    }

    /// One system, 128, at (40, 20).
    fn one() -> GalaxyModel {
        GalaxyModel::new(Galaxy {
            systems: vec![system(128, 40, 20)],
            ..Galaxy::default()
        })
    }

    #[test]
    fn the_box_holds_its_left_and_top_edges_and_not_its_right_and_bottom() {
        for zoom in [1.0, 0.5625] {
            let transform = transform(zoom);
            let model = one();
            let (h, v) = transform.pixel(Point::new(40.0, 20.0));
            let hit = |x: i32, y: i32| {
                #[allow(clippy::cast_precision_loss)] // Screen pixels.
                let at = Point::new(x as f32 + 0.5, y as f32 + 0.5);
                hits(&model, at, &transform)
            };
            assert_eq!(hit(h, v), ids(&[128]), "{zoom}: the point");
            assert_eq!(hit(h - 10, v - 10), ids(&[128]), "{zoom}: left, top");
            assert_eq!(
                hit(h + 9, v + 9),
                ids(&[128]),
                "{zoom}: inside right, bottom"
            );
            assert_eq!(hit(h + 10, v), ids(&[]), "{zoom}: right");
            assert_eq!(hit(h, v + 10), ids(&[]), "{zoom}: bottom");
            assert_eq!(hit(h - 11, v), ids(&[]), "{zoom}: past left");
            assert_eq!(hit(h, v - 11), ids(&[]), "{zoom}: past top");
        }
    }

    #[test]
    fn a_system_is_hit_where_the_original_draws_it_at_two_zooms() {
        // From 0.5625, two steps out put (−36, −36) at (413, 309) and one
        // (−66, −66) at (361, 257), a pixel up and left of the f32 scale's.
        for (steps, (x, y), hit, missed) in [
            (2, (-36, -36), (403.5, 299.5), (423.5, 319.5)),
            (1, (-66, -66), (351.5, 247.5), (371.5, 267.5)),
        ] {
            let mut place = MapPlace::new((0, 0));
            for _ in 0..steps {
                assert!(place.zoom_out());
            }
            let transform = place.transform((212, 128), (238, 218));
            let model = GalaxyModel::new(Galaxy {
                systems: vec![system(128, x, y)],
                ..Galaxy::default()
            });
            let press = |(x, y): (f32, f32)| hits(&model, Point::new(x, y), &transform);
            assert_eq!(press(hit), ids(&[128]), "{steps}: the box's top-left");
            assert_eq!(press(missed), ids(&[]), "{steps}: past its bottom-right");
        }
    }

    #[test]
    fn the_pointer_is_truncated_toward_zero() {
        let transform = transform(1.0);
        let model = one();
        let (h, v) = transform.pixel(Point::new(40.0, 20.0));
        #[allow(clippy::cast_precision_loss)] // Screen pixels.
        let edge = Point::new((h + 9) as f32 + 0.99, v as f32);
        assert_eq!(hits(&model, edge, &transform), ids(&[128]));
    }

    #[test]
    fn every_hit_is_given_by_ascending_id_and_a_hidden_system_never() {
        // Alpha (128) and Alpha Reborn (131) share (0, 0).
        let transform = transform(1.0);
        let mut model = GalaxyModel::new(galaxy());
        let at = transform.world_to_screen(Point::new(3.0, -2.0));
        assert_eq!(hits(&model, at, &transform), ids(&[128, 131]));
        let mut overlay = GalaxyOverlay::default();
        overlay.set_visible(SystemId(128), Some(false));
        let systems = galaxy().systems.iter().map(star_system).collect();
        model.apply(&nova_sim::galaxy_map::GalaxyShown::of(
            &OverlaidGalaxy::new(systems, overlay),
        ));
        assert_eq!(hits(&model, at, &transform), ids(&[131]), "128 hidden");
    }
}
