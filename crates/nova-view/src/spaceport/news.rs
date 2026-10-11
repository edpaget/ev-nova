//! The bar's news: the visit's commercial line and story, shown over the
//! bar by its News button, laid out by the interface file's news dialog
//! (`DLOG` 1014; `_DoNewsDialog` @0x47f0a in the `EV Nova` executable).
//!
//! The stock dialog is 300 x 230, centred, and its `DITL` has no items:
//! `_NewsDialogUpdate` (0x47c57) draws everything. Here the commercial
//! and the story are two plain Geneva text boxes, each wrapped to the
//! dialog's width less a margin, the commercial above the story, both as
//! written, their tags not filled (`TETextBox` @0x47d7b, @0x47e2c).
//! crons-news-ranks phase 9 places them in the original's bands over the
//! government's picture.
//!
//! The screen beeps as it opens and as it closes (`_beepSnd[1]` @0x47f36
//! and `_beepSnd[2]` @0x4814e, raised as [`UiSound::NewsOpened`] and
//! [`UiSound::NewsClosed`]). It closes on Return, Enter or Escape, or on
//! a press inside its frame (`_NewsDialogFilter` @0x48160, key codes
//! @0x48183-0x4819c, `PtInRect` @0x481df); a press outside it does
//! nothing. Without its dialog it says why, and Return or Escape still
//! closes it.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::News;

use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::DialogTemplate;
use crate::ui::scroll_text::ScrollText;

/// The news dialog's `DLOG` ID.
pub const NEWS_DIALOG: i16 = 1014;

/// The gap between the dialog's edge and its text boxes, and between
/// the two boxes.
const MARGIN: f32 = 10.0;
/// The commercial line's box height: three lines of the body text.
const COMMERCIAL_HEIGHT: f32 = 40.0;

/// The news, laid out.
#[derive(Clone, Debug)]
struct Laid {
    bounds: Bounds,
    commercial: ScrollText,
    story: ScrollText,
}

/// The news dialog, open over the bar.
#[derive(Clone, Debug)]
pub struct NewsScreen {
    /// The dialog, or why it cannot be shown.
    laid: Result<Laid, String>,
    closed: bool,
    sounds: Vec<Sound>,
}

impl NewsScreen {
    /// `news`, laid out by `layout`: the news dialog's template and the
    /// metrics its text is measured by, or why there are none. It beeps
    /// as it opens.
    #[must_use]
    pub fn new(layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>, news: &News) -> Self {
        let laid = layout.map(|(template, metrics)| {
            let origin = template.placement.origin(template.bounds);
            let bounds = Bounds::at(origin, template.bounds.width(), template.bounds.height());
            let width = (bounds.width() - 2.0 * MARGIN).max(0.0);
            let commercial_at = Point::new(origin.x + MARGIN, origin.y + MARGIN);
            let story_at = Point::new(
                commercial_at.x,
                commercial_at.y + COMMERCIAL_HEIGHT + MARGIN,
            );
            let story_height = (bounds.max.y - MARGIN - story_at.y).max(0.0);
            let boxed = |text: &str, at: Point, height: f32| {
                ScrollText::new(
                    text,
                    Font::Geneva,
                    BODY_SIZE,
                    BODY_COLOR,
                    Bounds::at(at, width, height),
                    &metrics,
                )
            };
            Laid {
                bounds,
                commercial: boxed(&news.commercial, commercial_at, COMMERCIAL_HEIGHT),
                story: boxed(&news.story, story_at, story_height),
            }
        });
        Self {
            laid,
            closed: false,
            sounds: vec![Sound::Ui(UiSound::NewsOpened)],
        }
    }

    /// The dialog's frame on the screen, if it could be laid out.
    #[must_use]
    pub fn bounds(&self) -> Option<Bounds> {
        self.laid.as_ref().ok().map(|laid| laid.bounds)
    }

    /// The commercial line's box, if it could be laid out.
    #[must_use]
    pub fn commercial(&self) -> Option<&ScrollText> {
        self.laid.as_ref().ok().map(|laid| &laid.commercial)
    }

    /// The story's box, if it could be laid out.
    #[must_use]
    pub fn story(&self) -> Option<&ScrollText> {
        self.laid.as_ref().ok().map(|laid| &laid.story)
    }

    /// Why the news cannot be shown, if it cannot.
    #[must_use]
    pub fn problem(&self) -> Option<&str> {
        self.laid.as_ref().err().map(String::as_str)
    }

    /// Whether the player has closed it.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.closed
    }

    /// Closes it, beeping, once.
    fn close(&mut self) {
        if !self.closed {
            self.closed = true;
            self.sounds.push(Sound::Ui(UiSound::NewsClosed));
        }
    }
}

impl Screen for NewsScreen {
    /// Return, Enter or Escape closes it, and so does a press inside its
    /// frame; nothing else does anything. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } => self.close(),
            Input::PointerButton {
                pressed: true, at, ..
            } if self.bounds().is_some_and(|bounds| bounds.contains(at)) => self.close(),
            _ => {}
        }
        ScreenAction::None
    }

    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        match &self.laid {
            Ok(laid) => {
                laid.commercial.draw(list);
                laid.story.draw(list);
            }
            Err(reason) => {
                list.text(
                    format!("Cannot show the news: {reason}. Press Return or Escape to close it."),
                    PROBLEM_AT,
                    PROBLEM_SIZE,
                    None,
                    Color::ERROR,
                );
            }
        }
    }

    /// Its beeps, opening and closing, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::dialog::Placement;

    /// Stock "News": `DLOG` 1014, 300 x 230 and centred, with no items.
    fn template() -> DialogTemplate {
        DialogTemplate {
            bounds: Bounds::at(Point::new(40.0, 40.0), 300.0, 230.0),
            placement: Placement::Center,
            items: Vec::new(),
        }
    }

    /// Where the dialog goes: (1024 - 300) / 2 and (768 - 230) / 2.
    const ORIGIN: Point = Point::new(362.0, 269.0);

    fn news() -> News {
        News {
            commercial: "Buy Bibble".to_owned(),
            story: "<PN> wins the race".to_owned(),
        }
    }

    fn open() -> NewsScreen {
        NewsScreen::new(Ok((template(), Rc::new(MonoMetrics))), &news())
    }

    fn texts(screen: &NewsScreen) -> Vec<String> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn press(at: Point) -> Input {
        Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        }
    }

    #[test]
    fn the_named_values() {
        assert_eq!(NEWS_DIALOG, 1014);
    }

    #[test]
    fn it_shows_the_commercial_above_the_story_as_written() {
        let screen = open();
        assert_eq!(
            screen.bounds(),
            Some(Bounds::at(ORIGIN, 300.0, 230.0)),
            "centred"
        );
        let commercial = screen.commercial().expect("laid out");
        let story = screen.story().expect("laid out");
        assert_eq!(commercial.lines().join(" "), "Buy Bibble");
        assert_eq!(story.lines().join(" "), "<PN> wins the race", "tag kept");
        assert_eq!(
            commercial.rect(),
            Bounds::at(Point::new(ORIGIN.x + 10.0, ORIGIN.y + 10.0), 280.0, 40.0)
        );
        assert_eq!(
            story.rect(),
            Bounds::at(Point::new(ORIGIN.x + 10.0, ORIGIN.y + 60.0), 280.0, 160.0)
        );
        let shown = texts(&screen).join(" ");
        assert!(shown.contains("Buy Bibble"), "{shown}");
        assert!(shown.contains("<PN> wins"), "{shown}");
        assert!(
            shown.find("Buy").expect("commercial") < shown.find("<PN>").expect("story"),
            "the commercial first"
        );
        assert_eq!(screen.problem(), None);
    }

    #[test]
    fn it_beeps_as_it_opens_and_as_it_closes() {
        let mut screen = open();
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::NewsOpened)]);
        assert_eq!(screen.take_sounds(), [], "once");
        screen.input(&key(Key::Escape, false));
        assert_eq!(screen.take_sounds(), [Sound::Ui(UiSound::NewsClosed)]);
        screen.input(&key(Key::Escape, false));
        assert_eq!(screen.take_sounds(), [], "closing once");
    }

    #[test]
    fn return_enter_or_escape_closes_it_on_a_press() {
        for close in [Key::Enter, Key::Escape] {
            let mut screen = open();
            screen.input(&key(close, true));
            assert!(!screen.closed(), "{close:?} repeating");
            screen.input(&Input::Key {
                key: close,
                pressed: false,
                repeat: false,
            });
            assert!(!screen.closed(), "{close:?} let go");
            screen.input(&key(close, false));
            assert!(screen.closed(), "{close:?}");
        }
        let mut screen = open();
        screen.input(&key(Key::Char('n'), false));
        screen.input(&key(Key::Space, false));
        assert!(!screen.closed(), "other keys do nothing");
    }

    #[test]
    fn a_press_inside_closes_it_and_one_outside_does_nothing() {
        let mut screen = open();
        for outside in [
            Point::new(ORIGIN.x - 1.0, ORIGIN.y + 100.0),
            Point::new(ORIGIN.x + 100.0, ORIGIN.y + 231.0),
            Point::new(10.0, 10.0),
        ] {
            screen.input(&press(outside));
            assert!(!screen.closed(), "{outside:?}");
        }
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: Point::new(ORIGIN.x + 5.0, ORIGIN.y + 5.0),
        });
        assert!(!screen.closed(), "a release does nothing");
        screen.input(&press(Point::new(ORIGIN.x + 299.0, ORIGIN.y + 229.0)));
        assert!(screen.closed());
        let mut screen = open();
        screen.input(&press(Point::new(ORIGIN.x + 5.0, ORIGIN.y + 5.0)));
        assert!(screen.closed());
    }

    #[test]
    fn without_its_dialog_it_says_why_and_escape_still_closes_it() {
        let mut screen = NewsScreen::new(Err("no DLOG 1014".to_owned()), &news());
        assert_eq!(screen.problem(), Some("no DLOG 1014"));
        assert_eq!(screen.bounds(), None);
        assert!(screen.commercial().is_none() && screen.story().is_none());
        let shown = texts(&screen);
        assert_eq!(shown.len(), 1);
        assert!(shown[0].contains("no DLOG 1014"), "{shown:?}");
        screen.input(&press(Point::new(512.0, 384.0)));
        assert!(!screen.closed(), "no frame to press in");
        screen.input(&key(Key::Escape, false));
        assert!(screen.closed());
        let mut screen = NewsScreen::new(Err("no DLOG 1014".to_owned()), &news());
        screen.input(&key(Key::Enter, false));
        assert!(screen.closed());
    }
}
