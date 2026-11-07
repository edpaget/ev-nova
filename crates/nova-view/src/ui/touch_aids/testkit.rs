//! The window fixtures the touch controls and the touch aids are laid out
//! in: six window sizes from a desktop window to Slide Over, each with a
//! fake safe area cut from its edges ([`CUTS`]). Flight's overlay is laid
//! out in its widened view ([`Window::view`]), every other host in the
//! classic canvas ([`Window::classic_surroundings`]).

#![allow(clippy::float_cmp)]

use crate::geometry::{Bounds, Point, intersection};
use crate::input::{Device, Surroundings};
use crate::view_shape::{CLASSIC_SIZE, widened_size};

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
            soft_keyboard: false,
        }
    }

    /// The classic canvas, 1024 × 768, as every host but flight draws.
    pub(crate) fn classic_view() -> (f32, f32) {
        #[allow(clippy::cast_precision_loss)]
        (CLASSIC_SIZE.0 as f32, CLASSIC_SIZE.1 as f32)
    }

    /// The surroundings a host in the classic canvas is told, by touch: the
    /// canvas letterboxed in the window, and the fake safe area cut from
    /// the window's edges, mapped into the canvas and clipped to it.
    pub(crate) fn classic_surroundings(&self) -> Surroundings {
        let (w, h) = Self::classic_view();
        let ppp = (self.points.0 / w).min(self.points.1 / h);
        let offset = Point::new(
            (self.points.0 - w * ppp) / 2.0,
            (self.points.1 - h * ppp) / 2.0,
        );
        let (top, bottom, side) = CUTS;
        let logical = |x: f32, y: f32| Point::new((x - offset.x) / ppp, (y - offset.y) / ppp);
        let safe = Bounds {
            min: logical(side, top),
            max: logical(self.points.0 - side, self.points.1 - bottom),
        };
        let canvas = Bounds::at(Point::new(0.0, 0.0), w, h);
        Surroundings {
            device: Device::Touch,
            safe_area: intersection(safe, canvas).unwrap_or(canvas),
            points_per_pixel: ppp,
            soft_keyboard: false,
        }
    }
}

/// Told touch, `screen`'s toggle at `toggle` tapped, then each of `taps`,
/// one finger through the touch router as the app delivers it.
pub(crate) fn toggle_then_taps<S: crate::screen::Screen>(
    screen: &mut S,
    toggle: impl Fn(&S) -> Point,
    taps: &[Point],
) {
    screen.input(&crate::input::Input::Surroundings(Surroundings::of(
        Device::Touch,
    )));
    let at = toggle(screen);
    let mut fingers = crate::touch::testkit::Fingers::on(screen);
    for &point in std::iter::once(&at).chain(taps) {
        fingers.down(1, point);
        fingers.up(1, point);
    }
}

/// `modifier` held through a mouse click at `at` (a move there, the
/// press and the release), as the keyboard and mouse player makes a
/// modifier click.
pub(crate) fn modifier_click(
    screen: &mut impl crate::screen::Screen,
    modifier: crate::input::Key,
    at: Point,
) {
    use crate::input::{Input, MouseButton};
    let key = |pressed| Input::Key {
        key: modifier,
        pressed,
        repeat: false,
    };
    screen.input(&key(true));
    screen.input(&Input::PointerMoved(at));
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
    screen.input(&key(false));
}

/// What `screen` draws.
pub(crate) fn drawn(screen: &impl crate::screen::Screen) -> Vec<crate::draw::DrawCommand> {
    let mut list = crate::draw::DrawList::new();
    screen.draw(&mut list);
    list.iter().cloned().collect()
}

/// The devices other than touch.
pub(crate) const OTHER_DEVICES: [Device; 3] = [
    Device::Keyboard,
    Device::Mouse,
    Device::Gamepad(crate::input::ControllerFamily::Generic),
];
