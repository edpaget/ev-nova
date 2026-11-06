//! The touch controls' shared look: a square control is a translucent
//! fill, a one-point outline and its short label at [`LABEL_PT`], drawn
//! in [`HIGHLIGHT`] while held or armed. Flight's overlay
//! ([`crate::flight::touch_controls`]) and every touch aid draw with it.

use crate::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};

/// A control at rest.
pub const FILL: Color = Color::rgba(200, 210, 230, 72);
/// A control's outline and label.
pub const OUTLINE: Color = Color::rgba(230, 235, 245, 160);
/// A control held down.
pub const HIGHLIGHT: Color = Color::rgba(120, 200, 255, 160);
/// The labels' size, in points.
pub const LABEL_PT: f32 = 12.0;

/// Draws a square control at `rect`: a `fill`, a one-point outline and
/// `text`, sized for `points_per_pixel`.
pub fn draw_square(list: &mut DrawList, rect: Bounds, text: &str, fill: Color, points: f32) {
    fill_rect(list, rect, fill);
    outline(list, rect, 1.0 / points, OUTLINE);
    draw_label(list, rect, text, points);
}

/// Draws `text` as a control's label in `rect`, sized for
/// `points_per_pixel`: [`LABEL_PT`] high, half a label in from the left
/// and centred down.
pub fn draw_label(list: &mut DrawList, rect: Bounds, text: &str, points: f32) {
    let size = LABEL_PT / points;
    list.text(
        text,
        Point::new(rect.min.x + size / 2.0, rect.center().y - size / 2.0),
        size,
        Some(rect.width() - size),
        OUTLINE,
    );
}

/// A `width`-thick outline just inside `rect`.
fn outline(list: &mut DrawList, rect: Bounds, width: f32, color: Color) {
    let Bounds { min, max } = rect;
    for (from, to) in [
        (min, Point::new(max.x, min.y)),
        (Point::new(min.x, max.y), max),
        (min, Point::new(min.x, max.y)),
        (Point::new(max.x, min.y), max),
    ] {
        list.line(from, to, width, color);
    }
}
