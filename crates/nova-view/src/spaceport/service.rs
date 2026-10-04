//! A spaceport service's placeholder: the service's name, a line saying
//! it is not available yet, and a Done button that goes back to the
//! spaceport. Each service replaces it when it is built.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::Service;

use super::layout::{DONE_LABEL, label};
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::text::TextMetrics;
use crate::ui::button::{Button, ButtonSkin, ButtonStyle, ButtonTracker};

/// The middle of the screen, across.
const CENTRE_X: f32 = 512.0;
/// The title's top, font, size and colour.
pub const TITLE_Y: f32 = 300.0;
/// The title's font.
pub const TITLE_FONT: Font = Font::Charcoal;
/// The title's size.
pub const TITLE_SIZE: f32 = 20.0;
/// The note under the title.
pub const NOT_AVAILABLE: &str = "Not available yet";
/// The note's top.
pub const NOTE_Y: f32 = 340.0;
/// The note's size, in Geneva.
pub const NOTE_SIZE: f32 = 14.0;
/// The Done button: a spaceport button's size, centred below the note.
pub const DONE_BUTTON: Bounds = Bounds {
    min: Point::new(439.5, 380.0),
    max: Point::new(584.5, 405.0),
};

/// A service not built yet: its name, a note and a Done button, which a
/// click, Return or Escape activates, closing it.
#[derive(Clone)]
pub struct ServiceScreen {
    service: Service,
    done: Button,
    tracker: ButtonTracker,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    closed: bool,
}

impl std::fmt::Debug for ServiceScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServiceScreen")
            .field("service", &self.service)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl ServiceScreen {
    /// `service`'s placeholder, its button labelled in `style` and its text
    /// measured by `metrics`.
    #[must_use]
    pub fn new(service: Service, style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            service,
            done: Button::new(DONE_BUTTON, DONE_LABEL),
            tracker: ButtonTracker::default(),
            style,
            metrics,
            closed: false,
        }
    }

    /// The service it stands in for.
    #[must_use]
    pub fn service(&self) -> Service {
        self.service
    }

    /// Whether Done has been activated.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// `text` centred across the screen with its top at `y`.
    fn centred(
        &self,
        list: &mut DrawList,
        text: &str,
        font: Font,
        size: f32,
        y: f32,
        color: Color,
    ) {
        let width = self.metrics.width(font, size, text);
        let origin = Point::new(CENTRE_X - width / 2.0, y);
        list.text_in(font, text, origin, size, None, color);
    }
}

impl Screen for ServiceScreen {
    /// A click on Done, or a Return or Escape press, closes it. It never
    /// quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        let done = match *input {
            Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } => true,
            // The tracker ignores every other key.
            _ => self.tracker.input(&self.done, input),
        };
        self.closed |= done;
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        self.centred(
            list,
            label(self.service),
            TITLE_FONT,
            TITLE_SIZE,
            TITLE_Y,
            Color::WHITE,
        );
        self.centred(
            list,
            NOT_AVAILABLE,
            Font::Geneva,
            NOTE_SIZE,
            NOTE_Y,
            Color::DIM,
        );
        self.done.draw(
            self.tracker.pressed(),
            &ButtonSkin::NOVA,
            &self.style,
            &self.metrics,
            list,
        );
    }

    fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;

    fn bar() -> ServiceScreen {
        ServiceScreen::new(Service::Bar, ButtonStyle::STOCK, Rc::new(MonoMetrics))
    }

    fn drawn(screen: &ServiceScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    fn key(key: Key, pressed: bool, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed,
            repeat,
        }
    }

    fn click(screen: &mut ServiceScreen, at: Point) {
        for pressed in [true, false] {
            screen.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
        }
    }

    #[test]
    fn it_draws_the_title_and_note_centred_then_done() {
        let screen = bar();
        assert_eq!(screen.service(), Service::Bar);
        let text = |text: &str, font, size, x, y, color| DrawCommand::Text {
            text: text.to_owned(),
            font,
            origin: Point::new(x, y),
            size,
            wrap_width: None,
            color,
        };
        let mut expected = vec![
            text(
                "Bar",
                Font::Charcoal,
                20.0,
                512.0 - 3.0 * 12.0 / 2.0,
                300.0,
                Color::WHITE,
            ),
            text(
                "Not available yet",
                Font::Geneva,
                14.0,
                512.0 - 17.0 * 7.0 / 2.0,
                340.0,
                Color::DIM,
            ),
        ];
        let mut button = DrawList::new();
        Button::new(DONE_BUTTON, "Done").draw(
            false,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut button,
        );
        expected.extend(button.iter().cloned());
        assert_eq!(drawn(&screen), expected);
        assert_eq!(DONE_BUTTON.center(), Point::new(512.0, 392.5));
        assert_eq!((DONE_BUTTON.width(), DONE_BUTTON.height()), (145.0, 25.0));
    }

    #[test]
    fn done_shows_pressed_while_held() {
        let mut screen = bar();
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: DONE_BUTTON.center(),
        });
        let mut pressed = DrawList::new();
        Button::new(DONE_BUTTON, "Done").draw(
            true,
            &ButtonSkin::NOVA,
            &ButtonStyle::STOCK,
            &MonoMetrics,
            &mut pressed,
        );
        assert_eq!(
            drawn(&screen)[2..],
            pressed.iter().cloned().collect::<Vec<_>>()[..]
        );
    }

    #[test]
    fn a_click_on_done_return_or_escape_closes_it() {
        let mut screen = bar();
        click(&mut screen, DONE_BUTTON.center());
        assert!(screen.closed());
        for k in [Key::Enter, Key::Escape] {
            let mut screen = bar();
            assert_eq!(screen.input(&key(k, true, false)), ScreenAction::None);
            assert!(screen.closed(), "{k:?}");
        }
    }

    #[test]
    fn other_input_leaves_it_open() {
        let mut screen = bar();
        for input in [
            key(Key::Enter, true, true),
            key(Key::Escape, false, false),
            key(Key::Space, true, false),
            key(Key::Char('l'), true, false),
        ] {
            screen.input(&input);
        }
        click(&mut screen, Point::new(10.0, 10.0));
        screen.tick(Duration::from_secs(1));
        assert!(!screen.closed());
        // Closed stays closed.
        screen.input(&key(Key::Escape, true, false));
        screen.input(&key(Key::Space, true, false));
        assert!(screen.closed());
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click_on_done() {
        let mut screen = bar();
        let button = |pressed| Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: DONE_BUTTON.center(),
        };
        screen.input(&button(true));
        screen.cancel_pointer();
        screen.input(&button(false));
        assert!(!screen.closed());
        assert!(format!("{screen:?}").contains("Bar"));
    }
}
