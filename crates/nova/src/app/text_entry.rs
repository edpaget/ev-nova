//! The app's side of text entry
//! (rdm:phase/key-bindings/phase-17-text-entry-port): when the platform's
//! entry (the IME, a soft keyboard) starts, starts again and stops, from
//! what each frame asks ([`EntryAsk`], a field of the screen's
//! `DrawList`).
//!
//! - **Each frame**, after the screen draws, [`App::tell_text_entry`]
//!   compares the frame's ask with what it last told the port
//!   ([`TextEntry`]): an ask where there was none starts entry at the
//!   field's pixels ([`field_px`]); a field moved, or a new focus
//!   ([`EntryAsk::focused`]), starts it again; no ask where there was one
//!   stops it; an unchanged ask tells nothing. While the developer overlay
//!   shows, no field asks.
//! - **The surroundings** carry the port's
//!   [`TextEntry::soft_keyboard`], so the screens decide between the
//!   system's keyboard and their own.
//!
//! The ask is read at the frame, a little after the input that focused
//! the field: the frame is where the player first sees the field, so no
//! visible boundary is crossed. A key typed before it reaches the field
//! as plain key text, as it always did.

use nova_render::{ImageSource, Viewport};
use nova_view::geometry::Bounds;
use nova_view::{EntryAsk, Screen};

use super::{App, PxRect};
use crate::text_entry::TextEntry;

/// `field`, logical in the shape shown, as physical pixels of the window
/// through `viewport`: its edges widened to whole pixels (the min floored,
/// the max ceiled) and clamped to the window. `None` in a window of no
/// area.
#[must_use]
pub fn field_px(field: Bounds, viewport: &Viewport) -> Option<PxRect> {
    let (content, _) = viewport.content()?;
    let (across, down) = viewport.axis_scales()?;
    let (width, height) = viewport.window_px();
    let edge = |logical: f32, scale: f32, origin: u32, rounded: fn(f64) -> f64, max: u32| {
        let px = rounded(f64::from(origin) + f64::from(logical) * f64::from(scale));
        // Clamped to the window, which is at most a few thousand pixels.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = px.clamp(0.0, f64::from(max)) as u32;
        px
    };
    let left = edge(field.min.x, across, content.x, f64::floor, width);
    let top = edge(field.min.y, down, content.y, f64::floor, height);
    let right = edge(field.max.x, across, content.x, f64::ceil, width);
    let bottom = edge(field.max.y, down, content.y, f64::ceil, height);
    Some(PxRect {
        x: left,
        y: top,
        width: right.saturating_sub(left),
        height: bottom.saturating_sub(top),
    })
}

impl<S: ImageSource, C: Screen> App<S, C> {
    /// The app with the platform's text entry, which starts at the field a
    /// frame asks for and stops when none asks; its soft keyboard reaches
    /// the screens in the next surroundings told.
    #[must_use]
    pub fn with_text_entry(mut self, entry: Box<dyn TextEntry>) -> Self {
        self.text_entry = entry;
        self
    }

    /// Whether the platform shows its own keyboard on screen.
    pub(super) fn soft_keyboard(&self) -> bool {
        self.text_entry.soft_keyboard()
    }

    /// Tells the port what the frame just drawn asks, `ask` (see the
    /// module docs).
    pub(super) fn tell_text_entry(&mut self, ask: Option<EntryAsk>) {
        let overlay = self
            .overlay
            .as_ref()
            .is_some_and(nova_view::devtools::DevOverlay::visible);
        let now = ask
            .filter(|_| !overlay)
            .and_then(|ask| field_px(ask.field, &self.viewport).map(|px| (px, ask.focused)));
        match now {
            Some((px, _)) if self.told_entry != now => self.text_entry.start(px),
            None if self.told_entry.is_some() => self.text_entry.stop(),
            _ => {}
        }
        self.told_entry = now;
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use nova_view::ViewShape;
    use nova_view::geometry::Point;

    use super::*;

    fn field() -> Bounds {
        Bounds::at(Point::new(100.25, 200.0), 300.0, 31.5)
    }

    /// The logical point window pixel `(x, y)` shows through `viewport`.
    fn logical(viewport: &Viewport, x: u32, y: u32) -> Point {
        viewport
            .window_to_logical_unbounded((f64::from(x), f64::from(y)))
            .expect("an area")
    }

    /// Asserts `px` covers `field` through `viewport` to the whole pixel:
    /// its corners map back to the field's or just outside them, by less
    /// than a pixel.
    fn covers(px: PxRect, field: Bounds, viewport: &Viewport) {
        let (across, down) = viewport.axis_scales().expect("an area");
        let (pixel_x, pixel_y) = (1.0 / across, 1.0 / down);
        let min = logical(viewport, px.x, px.y);
        let max = logical(viewport, px.x + px.width, px.y + px.height);
        assert!(
            min.x <= field.min.x && field.min.x - min.x < pixel_x,
            "{min:?}"
        );
        assert!(
            min.y <= field.min.y && field.min.y - min.y < pixel_y,
            "{min:?}"
        );
        assert!(
            max.x >= field.max.x && max.x - field.max.x < pixel_x,
            "{max:?}"
        );
        assert!(
            max.y >= field.max.y && max.y - field.max.y < pixel_y,
            "{max:?}"
        );
    }

    #[test]
    fn a_classic_letterboxed_field_maps_to_its_pixels() {
        for (size, scale) in [((2048, 1536), 2.0), ((1024, 900), 1.0), ((3000, 1600), 2.0)] {
            let viewport = Viewport::for_shape(ViewShape::Classic, size, scale);
            let px = field_px(field(), &viewport).expect("an area");
            covers(px, field(), &viewport);
        }
    }

    #[test]
    fn a_widened_field_maps_through_its_own_across_scale() {
        let viewport = Viewport::for_shape(ViewShape::Widened, (2388, 1668), 2.0);
        let (across, down) = viewport.axis_scales().expect("an area");
        assert_ne!(across, down, "the widened view's own across scale");
        let px = field_px(field(), &viewport).expect("an area");
        covers(px, field(), &viewport);
    }

    #[test]
    fn a_field_partly_outside_the_window_is_clamped_to_it() {
        let viewport = Viewport::for_shape(ViewShape::Classic, (1024, 768), 1.0);
        let wide = Bounds::at(Point::new(-50.0, 700.0), 2000.0, 200.0);
        let px = field_px(wide, &viewport).expect("an area");
        assert_eq!((px.x, px.y), (0, 700));
        assert_eq!((px.x + px.width, px.y + px.height), (1024, 768));
    }

    #[test]
    fn a_window_of_no_area_has_no_field() {
        let viewport = Viewport::for_shape(ViewShape::Classic, (0, 0), 2.0);
        assert_eq!(field_px(field(), &viewport), None);
    }
}
