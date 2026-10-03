//! The screen trait: what every game screen does.

use std::time::Duration;

use crate::draw::DrawList;
use crate::input::Input;

/// What a screen asks of the app after an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenAction {
    /// Carry on.
    None,
    /// Quit the game.
    Quit,
}

/// A game screen: takes input, advances with time and draws itself.
pub trait Screen {
    /// Handles one input event.
    fn input(&mut self, input: &Input) -> ScreenAction;
    /// Advances the screen's clock by `dt`.
    fn tick(&mut self, dt: Duration);
    /// Draws the screen into `list`, in logical coordinates.
    fn draw(&self, list: &mut DrawList);
    /// Abandons any pointer gesture in progress (a held button, a drag)
    /// without completing it: what follows is not a click or a drop. Called
    /// when the screen stops receiving input, such as when the app hides it,
    /// since the button's release will then go elsewhere. Does nothing by
    /// default.
    fn cancel_pointer(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Key;
    use crate::{Color, Point};

    /// Counts ticks and quits on Escape.
    #[derive(Default)]
    struct Counter {
        elapsed: Duration,
    }

    impl Screen for Counter {
        fn input(&mut self, input: &Input) -> ScreenAction {
            match input {
                Input::Key {
                    key: Key::Escape,
                    pressed: true,
                } => ScreenAction::Quit,
                _ => ScreenAction::None,
            }
        }

        fn tick(&mut self, dt: Duration) {
            self.elapsed += dt;
        }

        fn draw(&self, list: &mut DrawList) {
            list.dot(Point::new(0.0, 0.0), 1.0, Color::WHITE);
        }
    }

    #[test]
    fn a_screen_is_driven_through_the_trait_object() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        let escape = Input::Key {
            key: Key::Escape,
            pressed: true,
        };
        let space = Input::Key {
            key: Key::Space,
            pressed: true,
        };
        assert_eq!(screen.input(&space), ScreenAction::None);
        assert_eq!(screen.input(&escape), ScreenAction::Quit);
        screen.tick(Duration::from_millis(5));
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::from_millis(5));
    }

    #[test]
    fn cancelling_the_pointer_does_nothing_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        screen.cancel_pointer();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::ZERO);
    }
}
