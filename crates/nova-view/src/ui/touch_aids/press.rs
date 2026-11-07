//! A press on an aid's own control: from the primary button going down
//! inside one of the aid's rects to its release, every pointer input is
//! the aid's, and a release inside the rect it went down in acts, as
//! flight's More acts on the release (`flight::touch_controls`). A press
//! that slides off and lifts does nothing.

use crate::geometry::Bounds;
use crate::input::{Input, MouseButton};

/// What a pointer input was to an aid's press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Not the aid's: the host takes it.
    Passed,
    /// The aid's, and nothing more.
    Taken,
    /// The release inside the control at this index: it acts.
    Acted(usize),
}

/// The aid's control held down, if any, by its index.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Press {
    held: Option<usize>,
}

impl Press {
    /// Follows `input`, with `rects` the aid's controls as shown now (none
    /// while it is hidden, so it takes no new press).
    pub fn input(&mut self, input: &Input, rects: &[Bounds]) -> Step {
        if let Some(index) = self.held {
            return match *input {
                Input::PointerButton {
                    button: MouseButton::Left,
                    pressed: false,
                    at,
                } => {
                    self.held = None;
                    if rects.get(index).is_some_and(|rect| rect.contains(at)) {
                        Step::Acted(index)
                    } else {
                        Step::Taken
                    }
                }
                Input::PointerMoved(_)
                | Input::PointerMovedOutside(_)
                | Input::PointerLeft
                | Input::PointerButton { .. }
                | Input::Scroll { .. } => Step::Taken,
                _ => Step::Passed,
            };
        }
        if let Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        } = *input
            && let Some(index) = rects.iter().position(|rect| rect.contains(at))
        {
            self.held = Some(index);
            return Step::Taken;
        }
        Step::Passed
    }

    /// The control held down, if any.
    #[must_use]
    pub const fn held(&self) -> Option<usize> {
        self.held
    }

    /// Forgets a press held: it acts on nothing.
    pub fn cancel(&mut self) {
        self.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    fn button(pressed: bool, at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        }
    }

    const RECTS: [Bounds; 2] = [
        Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(10.0, 10.0),
        },
        Bounds {
            min: Point::new(20.0, 0.0),
            max: Point::new(30.0, 10.0),
        },
    ];
    const IN_1: Point = Point::new(25.0, 5.0);
    const OUT: Point = Point::new(50.0, 50.0);

    #[test]
    fn a_tap_on_a_control_acts_on_its_release_inside() {
        let mut press = Press::default();
        assert_eq!(press.input(&button(true, IN_1), &RECTS), Step::Taken);
        assert_eq!(press.held(), Some(1));
        assert_eq!(press.input(&Input::PointerMoved(IN_1), &RECTS), Step::Taken);
        assert_eq!(press.input(&button(false, IN_1), &RECTS), Step::Acted(1));
        assert_eq!(press.held(), None);
        assert_eq!(press.input(&Input::PointerLeft, &RECTS), Step::Passed);
    }

    #[test]
    fn a_press_that_slides_off_and_lifts_does_nothing() {
        let mut press = Press::default();
        press.input(&button(true, IN_1), &RECTS);
        for input in [
            Input::PointerMoved(OUT),
            Input::PointerMovedOutside(OUT),
            Input::PointerLeft,
        ] {
            assert_eq!(press.input(&input, &RECTS), Step::Taken, "{input:?}");
        }
        assert_eq!(press.input(&button(false, OUT), &RECTS), Step::Taken);
        assert_eq!(press.input(&button(false, IN_1), &RECTS), Step::Passed);
    }

    #[test]
    fn a_press_elsewhere_or_on_no_control_shown_is_the_hosts() {
        let mut press = Press::default();
        assert_eq!(press.input(&button(true, OUT), &RECTS), Step::Passed);
        assert_eq!(press.input(&button(true, IN_1), &[]), Step::Passed);
        let right = Input::PointerButton {
            button: MouseButton::Right,
            pressed: true,
            at: IN_1,
        };
        assert_eq!(press.input(&right, &RECTS), Step::Passed);
    }

    #[test]
    fn while_held_keys_pass_and_a_cancel_forgets_the_press() {
        let mut press = Press::default();
        press.input(&button(true, IN_1), &RECTS);
        assert_eq!(press.input(&Input::Text('a'), &RECTS), Step::Passed);
        press.cancel();
        assert_eq!(press.held(), None);
        assert_eq!(press.input(&button(false, IN_1), &RECTS), Step::Passed);
    }
}
