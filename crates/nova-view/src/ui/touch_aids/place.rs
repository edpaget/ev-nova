//! Where a touch aid goes: a corner of the safe area inside its host's
//! canvas, clear of the host's own dialog, every size in points.
//!
//! - **The region** is the safe area the app told
//!   ([`Surroundings::safe_area`]) within the host's canvas, inset by
//!   [`MARGIN_PT`].
//! - **Corners** are tried in the order the aid gives: the first whose
//!   rect lies in the region and clear of the host's dialog (`occupied`)
//!   wins.
//! - **None clear** (a Slide Over-sized window): the aid takes the first
//!   corner of its order anyway, over the dialog. No aid is shrunk.
//! - **Sizes** are in points: one point is `1 / points_per_pixel`
//!   logical units ([`Surroundings::points_per_pixel`]).

use crate::geometry::{Bounds, Point, intersection};
use crate::input::Surroundings;

/// The smallest an aid is along either axis, in points: Apple's minimum
/// hit target, as flight's controls (`flight::touch_controls::layout`).
pub const MIN_PT: f32 = 44.0;
/// The margin inside the region's edges, in points.
pub const MARGIN_PT: f32 = 8.0;
/// A one-control aid's size in points: a cell of flight's More menu, wide
/// enough for its label.
pub const CELL_PT: (f32, f32) = (96.0, 44.0);

/// A corner of the region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Corner {
    BottomLeft,
    BottomRight,
    TopLeft,
    TopRight,
}

/// The bottom corners first, left before right: a thumb's reach.
pub const BOTTOM_FIRST: [Corner; 4] = [
    Corner::BottomLeft,
    Corner::BottomRight,
    Corner::TopLeft,
    Corner::TopRight,
];

/// The rect of an aid `size_pt` points across placed by the rules above,
/// in `canvas`'s logical units, trying `corners` in order.
#[must_use]
pub fn place(
    size_pt: (f32, f32),
    told: &Surroundings,
    canvas: Bounds,
    occupied: Bounds,
    corners: &[Corner],
) -> Bounds {
    let ppp = told.points_per_pixel;
    let size = (size_pt.0 / ppp, size_pt.1 / ppp);
    let margin = MARGIN_PT / ppp;
    let region = intersection(told.safe_area, canvas)
        .unwrap_or(canvas)
        .inset(margin, margin);
    let at = |corner| at_corner(region, corner, size);
    corners
        .iter()
        .map(|&corner| at(corner))
        .find(|&rect| inside(rect, region) && intersection(rect, occupied).is_none())
        .unwrap_or_else(|| at(corners.first().copied().unwrap_or(Corner::BottomLeft)))
}

/// The rect of `size` in `region`'s `corner`, flush with the region's
/// edges there (a far edge worked out as near edge plus size can land a
/// float's width outside).
fn at_corner(region: Bounds, corner: Corner, (w, h): (f32, f32)) -> Bounds {
    let (min_x, max_x) = match corner {
        Corner::BottomLeft | Corner::TopLeft => (region.min.x, region.min.x + w),
        Corner::BottomRight | Corner::TopRight => (region.max.x - w, region.max.x),
    };
    let (min_y, max_y) = match corner {
        Corner::TopLeft | Corner::TopRight => (region.min.y, region.min.y + h),
        Corner::BottomLeft | Corner::BottomRight => (region.max.y - h, region.max.y),
    };
    Bounds {
        min: Point::new(min_x, min_y),
        max: Point::new(max_x, max_y),
    }
}

/// Whether `rect` lies in `region`, edges included.
fn inside(rect: Bounds, region: Bounds) -> bool {
    rect.min.x >= region.min.x
        && rect.min.y >= region.min.y
        && rect.max.x <= region.max.x
        && rect.max.y <= region.max.y
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ui::touch_aids::testkit::{WINDOWS, Window};

    const EPS: f32 = 1e-3;

    fn canvas() -> Bounds {
        let (w, h) = Window::classic_view();
        Bounds::at(Point::new(0.0, 0.0), w, h)
    }

    /// A dialog of `w` × `h` centred in the canvas.
    fn centred(w: f32, h: f32) -> Bounds {
        let c = canvas().center();
        Bounds::at(Point::new(c.x - w / 2.0, c.y - h / 2.0), w, h)
    }

    fn placed(told: &Surroundings, occupied: Bounds) -> Bounds {
        place(CELL_PT, told, canvas(), occupied, &BOTTOM_FIRST)
    }

    #[test]
    fn an_aid_is_at_least_44_points_each_way_at_each_size() {
        for window in &WINDOWS {
            let told = window.classic_surroundings();
            let rect = placed(&told, centred(600.0, 400.0));
            let ppp = told.points_per_pixel;
            assert!(rect.width() * ppp >= MIN_PT - EPS, "{}", window.name);
            assert!(rect.height() * ppp >= MIN_PT - EPS, "{}", window.name);
            assert!(
                (rect.width() * ppp - CELL_PT.0).abs() < EPS,
                "{}",
                window.name
            );
        }
    }

    #[test]
    fn an_aid_lies_inside_the_safe_area_and_the_canvas_at_each_size() {
        for window in &WINDOWS {
            let told = window.classic_surroundings();
            let rect = placed(&told, centred(600.0, 400.0));
            let margin = MARGIN_PT / told.points_per_pixel;
            let region = intersection(told.safe_area, canvas())
                .expect("a region")
                .inset(margin, margin);
            assert!(
                inside(rect, region),
                "{}: {rect:?} in {region:?}",
                window.name
            );
        }
    }

    #[test]
    fn an_aid_takes_the_first_corner_clear_of_the_host() {
        let told = WINDOWS[0].classic_surroundings();
        let margin = MARGIN_PT / told.points_per_pixel;
        let region = intersection(told.safe_area, canvas())
            .expect("a region")
            .inset(margin, margin);
        let size = CELL_PT;
        let rect = placed(&told, centred(600.0, 400.0));
        assert_eq!(rect, at_corner(region, Corner::BottomLeft, size));
        let left_column = Bounds::at(Point::new(0.0, 0.0), 300.0, 768.0);
        assert_eq!(
            placed(&told, left_column),
            at_corner(region, Corner::BottomRight, size)
        );
        let bottom_row = Bounds::at(Point::new(0.0, 400.0), 1024.0, 368.0);
        assert_eq!(
            placed(&told, bottom_row),
            at_corner(region, Corner::TopLeft, size)
        );
        let left_strip = Bounds::at(Point::new(0.0, 0.0), 500.0, 768.0);
        let rect = place(
            size,
            &told,
            canvas(),
            left_strip,
            &[Corner::TopLeft, Corner::TopRight],
        );
        assert_eq!(rect, at_corner(region, Corner::TopRight, size), "its order");
    }

    #[test]
    fn with_no_corner_clear_it_takes_the_first_corner_of_its_order() {
        let told = WINDOWS[0].classic_surroundings();
        let margin = MARGIN_PT / told.points_per_pixel;
        let region = intersection(told.safe_area, canvas())
            .expect("a region")
            .inset(margin, margin);
        assert_eq!(
            placed(&told, canvas()),
            at_corner(region, Corner::BottomLeft, CELL_PT)
        );
        let rect = place(
            CELL_PT,
            &told,
            canvas(),
            canvas(),
            &[Corner::TopRight, Corner::BottomLeft],
        );
        assert_eq!(rect, at_corner(region, Corner::TopRight, CELL_PT));
    }

    #[test]
    fn a_tiny_region_still_gives_an_aid_its_size_at_its_corner() {
        let told = Surroundings {
            safe_area: Bounds::at(Point::new(500.0, 400.0), 30.0, 20.0),
            ..WINDOWS[0].classic_surroundings()
        };
        let rect = placed(&told, centred(10.0, 10.0));
        assert!((rect.width() - CELL_PT.0).abs() < EPS, "{rect:?}");
        assert!((rect.height() - CELL_PT.1).abs() < EPS, "{rect:?}");
        assert_eq!(rect.min.x, 508.0, "the region's left, past the margin");
        assert_eq!(rect.max.y, 412.0, "its bottom");
    }

    #[test]
    fn a_right_or_bottom_corner_lies_in_the_region_at_any_scale() {
        // A third of a unit per point leaves the rect's far edge a float's
        // width past the region's unless it is placed flush.
        let told = Surroundings {
            safe_area: canvas().inset(40.0, 12.0),
            points_per_pixel: 0.75,
            ..WINDOWS[0].classic_surroundings()
        };
        let top = Bounds::at(Point::new(0.0, 100.0), 1024.0, 668.0);
        let rect = place((304.0, 44.0), &told, canvas(), top, &[Corner::TopRight]);
        let margin = MARGIN_PT / 0.75;
        assert_eq!(rect.max.x, 984.0 - margin, "flush right");
        assert!(intersection(rect, top).is_none());
        let rect = place(
            (304.0, 44.0),
            &told,
            canvas(),
            Bounds::at(Point::new(0.0, 0.0), 1024.0, 600.0),
            &[Corner::TopRight, Corner::BottomRight],
        );
        assert_eq!(rect.max, Point::new(984.0 - margin, 756.0 - margin));
    }

    #[test]
    fn with_no_safe_area_in_the_canvas_the_canvas_is_the_region() {
        let told = Surroundings {
            safe_area: Bounds::at(Point::new(2000.0, 2000.0), 30.0, 20.0),
            ..WINDOWS[0].classic_surroundings()
        };
        let rect = placed(&told, centred(10.0, 10.0));
        assert_eq!(rect.min.x, 8.0);
        assert_eq!(rect.max.y, 760.0);
    }
}
