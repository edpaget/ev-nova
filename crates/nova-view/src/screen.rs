//! The screen trait: what every game screen does.

use std::time::Duration;

use crate::draw::DrawList;
use crate::input::Input;
use crate::sound::Sound;

/// What a screen asks of the app after an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenAction {
    /// Carry on.
    None,
    /// Quit the game.
    Quit,
}

/// Which screen the app is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Showing {
    /// The ship browser.
    ShipBrowser,
    /// The galaxy map.
    GalaxyMap,
    /// A system opened from the galaxy map.
    System,
    /// The player's ship in flight.
    Flight,
    /// Flight's course map, opened with M.
    FlightMap,
    /// The spaceport of the stellar landed on.
    Spaceport,
    /// The About text, over another screen.
    About,
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
    /// Forgets every key it holds as down, as if each had been released.
    /// Called when the screen stops receiving input (the app hides it, or
    /// the window loses focus), since the keys' releases will not reach
    /// it. A screen that acts on held keys over time stops acting on them.
    /// Does nothing by default.
    fn release_keys(&mut self) {}
    /// The sounds the screen has made since they were last taken, in
    /// order; taking them empties the list. None by default.
    fn take_sounds(&mut self) -> Vec<Sound> {
        Vec::new()
    }
    /// Which screen is showing, for a screen that routes between others;
    /// `None` by default.
    fn now_showing(&self) -> Option<Showing> {
        None
    }
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
                    ..
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
            repeat: false,
        };
        let space = Input::Key {
            key: Key::Space,
            pressed: true,
            repeat: false,
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

    #[test]
    fn a_screen_makes_no_sounds_and_names_nothing_showing_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(screen.now_showing(), None);
    }

    #[test]
    fn releasing_the_keys_does_nothing_by_default() {
        let mut counter = Counter::default();
        let screen: &mut dyn Screen = &mut counter;
        screen.release_keys();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list.len(), 1);
        assert_eq!(counter.elapsed, Duration::ZERO);
    }
}
