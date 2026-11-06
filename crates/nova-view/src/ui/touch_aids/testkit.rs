//! The window fixtures the touch controls and the touch aids are laid out
//! in: six window sizes from a desktop window to Slide Over, each with a
//! fake safe area cut from its edges ([`CUTS`]). Flight's overlay is laid
//! out in its widened view ([`Window::view`]).

#![allow(clippy::float_cmp)]

use crate::geometry::{Bounds, Point};
use crate::input::{Device, Surroundings};
use crate::view_shape::widened_size;

/// A window size in points and its scale factor: the inputs a fixture
/// derives its view and points per pixel from.
pub(crate) struct Window {
    pub(crate) name: &'static str,
    pub(crate) points: (f32, f32),
    pub(crate) scale: f32,
}

pub(crate) const WINDOWS: [Window; 6] = [
    Window {
        name: "desktop window",
        points: (1024.0, 768.0),
        scale: 1.0,
    },
    Window {
        name: "large desktop window",
        points: (1920.0, 1440.0),
        scale: 1.0,
    },
    Window {
        name: "full iPad",
        points: (1194.0, 834.0),
        scale: 2.0,
    },
    Window {
        name: "small iPad window",
        points: (700.0, 525.0),
        scale: 2.0,
    },
    Window {
        name: "iPad Split View half",
        points: (590.0, 834.0),
        scale: 2.0,
    },
    Window {
        name: "Slide Over",
        points: (320.0, 834.0),
        scale: 2.0,
    },
];

/// The fake safe area's cuts, in points: top, bottom, and each side.
pub(crate) const CUTS: (f32, f32, f32) = (24.0, 20.0, 32.0);

impl Window {
    /// Flight's view in this window: its widened shape
    /// ([`widened_size`] of the window's pixels).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub(crate) fn view(&self) -> (f32, f32) {
        let px = (
            (self.points.0 * self.scale).round() as u32,
            (self.points.1 * self.scale).round() as u32,
        );
        let (w, h) = widened_size(px);
        #[allow(clippy::cast_precision_loss)]
        (w as f32, h as f32)
    }

    /// Points per logical pixel: the window's points over the view, the
    /// smaller way (the view is letterboxed the other).
    pub(crate) fn points_per_pixel(&self) -> f32 {
        let (w, h) = self.view();
        (self.points.0 / w).min(self.points.1 / h)
    }

    /// The surroundings, told by touch, with the fake safe area cut from
    /// the canvas's edges.
    pub(crate) fn surroundings(&self) -> Surroundings {
        let ppp = self.points_per_pixel();
        let (w, h) = self.view();
        let (top, bottom, side) = CUTS;
        Surroundings {
            device: Device::Touch,
            safe_area: Bounds {
                min: Point::new(side / ppp, top / ppp),
                max: Point::new(w - side / ppp, h - bottom / ppp),
            },
            points_per_pixel: ppp,
        }
    }
}
