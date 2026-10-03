//! The screen router: every screen the app can show, as one [`Screen`].

use std::time::Duration;

use nova_view::{DrawList, Input, Screen, ScreenAction};

use super::Placeholder;

/// The screen the app is showing. Each variant is one of the game's
/// screens; the router forwards input, ticks and drawing to it.
#[derive(Clone, Debug)]
pub enum AppScreen {
    /// The placeholder that stands in until the real screens arrive.
    Placeholder(Placeholder),
}

impl Screen for AppScreen {
    fn input(&mut self, input: &Input) -> ScreenAction {
        match self {
            Self::Placeholder(screen) => screen.input(input),
        }
    }

    fn tick(&mut self, dt: Duration) {
        match self {
            Self::Placeholder(screen) => screen.tick(dt),
        }
    }

    fn draw(&self, list: &mut DrawList) {
        match self {
            Self::Placeholder(screen) => screen.draw(list),
        }
    }
}

#[cfg(test)]
mod tests {
    use nova_view::Key;

    use super::*;
    use crate::app::PlaceholderContent;

    const CONTENT: PlaceholderContent = PlaceholderContent {
        picture: Some(128),
        sprite: Some((200, 4)),
    };

    fn drawn(screen: &impl Screen) -> DrawList {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list
    }

    #[test]
    fn input_goes_to_the_placeholder() {
        let mut screen = AppScreen::Placeholder(Placeholder::new(CONTENT));
        let up = Input::Key {
            key: Key::Up,
            pressed: true,
        };
        assert_eq!(screen.input(&up), ScreenAction::None);
        match &screen {
            AppScreen::Placeholder(placeholder) => assert_eq!(placeholder.count(), 600),
        }
    }

    #[test]
    fn ticks_and_drawing_go_to_the_placeholder() {
        let mut direct = Placeholder::new(CONTENT);
        let mut screen = AppScreen::Placeholder(direct.clone());
        let unticked = drawn(&direct);
        let dt = Duration::from_millis(100);

        direct.tick(dt);
        screen.tick(dt);

        assert_ne!(drawn(&direct), unticked, "the tick moves the animation");
        assert_eq!(drawn(&screen), drawn(&direct));
    }
}
