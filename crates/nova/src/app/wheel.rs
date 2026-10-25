//! The mouse wheel and trackpad: a window's scroll in the screens' terms.

use nova_render::Viewport;
use nova_view::{Input, Point, ScrollDelta};

/// How far a wheel or trackpad scroll moves, toward the text's end for
/// positive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    /// Whole or fractional wheel notches.
    Lines(f32),
    /// Physical window pixels: a trackpad's movement.
    Px(f64),
}

/// The screens' scroll for a window's `delta` with the pointer at `at`:
/// notches as they are, and pixels in logical units by `viewport`'s
/// vertical scale, so a trackpad moves the text with the fingers. `None`
/// in a zero-area viewport.
pub(super) fn wheel_input(delta: WheelDelta, viewport: &Viewport, at: Point) -> Option<Input> {
    let (_, down) = viewport.axis_scales()?;
    let delta = match delta {
        WheelDelta::Lines(lines) => ScrollDelta::Lines(lines),
        WheelDelta::Px(px) => ScrollDelta::Units((px / f64::from(down)) as f32),
    };
    Some(Input::Scroll { delta, at })
}

#[cfg(test)]
mod tests {
    use nova_render::LogicalSize;

    use super::*;

    const AT: Point = Point::new(100.0, 50.0);

    /// 640 x 480 in a 1280 x 960 window: 2 pixels a unit.
    fn doubled() -> Viewport {
        Viewport::new(LogicalSize { w: 640, h: 480 }, (1280, 960), 1.0)
    }

    #[test]
    fn notches_pass_through() {
        assert_eq!(
            wheel_input(WheelDelta::Lines(2.0), &doubled(), AT),
            Some(Input::Scroll {
                delta: ScrollDelta::Lines(2.0),
                at: AT
            })
        );
    }

    #[test]
    fn pixels_become_logical_units_by_the_vertical_scale() {
        assert_eq!(
            wheel_input(WheelDelta::Px(48.0), &doubled(), AT),
            Some(Input::Scroll {
                delta: ScrollDelta::Units(24.0),
                at: AT
            })
        );
    }

    #[test]
    fn a_zero_area_viewport_takes_no_scroll() {
        let empty = Viewport::new(LogicalSize { w: 640, h: 480 }, (0, 0), 1.0);
        for delta in [WheelDelta::Lines(1.0), WheelDelta::Px(48.0)] {
            assert_eq!(wheel_input(delta, &empty, AT), None, "{delta:?}");
        }
    }
}
