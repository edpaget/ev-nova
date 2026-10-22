//! The bar's news: the visit's commercial line and story, shown over the
//! bar by its News button, laid out by the interface file's news dialog
//! (`DLOG` 1014; `_DoNewsDialog` @0x47f0a in the `EV Nova` executable).
//!
//! The stock dialog is 300 x 230, centred, and its `DITL` has no items:
//! `_NewsDialogUpdate` (0x47c57) draws everything, in this order:
//!
//! 1. Black over the whole dialog (`PaintRect` @0x47caf).
//! 2. The docked stellar's government's news picture, stretched to the
//!    dialog (`DrawPicHandle` @0x47cd2): the one asked for, else 9000,
//!    else none ([`shown_news_picture`], `_DoNewsDialog` @0x47f68-0x47f9d).
//! 3. The commercial in the band from 140 to 180 px below the dialog's
//!    top, 10 px in from each side (@0x47ce2-0x47d55), then the story
//!    from 170 px to 4 px above the bottom (@0x47d93-0x47e06). Each is a
//!    `TETextBox` (@0x47d7b, @0x47e2c) inverted (@0x47d83, @0x47e34):
//!    white Geneva text on an opaque black box, flush left, as written,
//!    its tags not filled. The story's box covers the commercial's last
//!    10 px, as it is drawn second.
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
use nova_sim::news::DEFAULT_NEWS_PICTURE;

use super::view::{PROBLEM_AT, PROBLEM_SIZE};
use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::image::ImageKey;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::{Sound, UiSound};
use crate::text::TextMetrics;
use crate::ui::desc::{BODY_COLOR, BODY_SIZE};
use crate::ui::dialog::DialogTemplate;
use crate::ui::scroll_text::ScrollText;

/// The news dialog's `DLOG` ID.
pub const NEWS_DIALOG: i16 = 1014;

/// The text boxes' inset from the dialog's left and right (@0x47ce2,
/// @0x47d93).
const SIDE_INSET: f32 = 10.0;
/// The commercial's box top, below the dialog's top (`+0x8C` @0x47cf6).
const COMMERCIAL_TOP: f32 = 140.0;
/// The commercial's box bottom, below the dialog's top (`+0xB4`
/// @0x47d1e).
const COMMERCIAL_BOTTOM: f32 = 180.0;
/// The story's box top, below the dialog's top (`+0xAA` @0x47dd8).
const STORY_TOP: f32 = 170.0;
/// The story's box bottom's inset above the dialog's bottom (@0x47e06).
const STORY_BOTTOM_INSET: f32 = 4.0;

/// The picture the news is drawn over, given the one `asked` for and
/// which pictures `exists`: `asked` if it exists, else
/// [`DEFAULT_NEWS_PICTURE`] if that does, else none, and the dialog
/// stays black (`_DoNewsDialog` @0x47f68-0x47f9d).
pub fn shown_news_picture(asked: i16, exists: impl Fn(i16) -> bool) -> Option<i16> {
    [asked, DEFAULT_NEWS_PICTURE]
        .into_iter()
        .find(|&id| exists(id))
}

/// The news, laid out.
#[derive(Clone, Debug)]
struct Laid {
    bounds: Bounds,
    picture: Option<i16>,
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
    /// metrics its text is measured by, or why there are none; drawn
    /// over `picture`, if any. It beeps as it opens.
    #[must_use]
    pub fn new(
        layout: Result<(DialogTemplate, Rc<dyn TextMetrics>), String>,
        news: &News,
        picture: Option<i16>,
    ) -> Self {
        let laid = layout.map(|(template, metrics)| {
            let origin = template.placement.origin(template.bounds);
            let bounds = Bounds::at(origin, template.bounds.width(), template.bounds.height());
            let left = bounds.min.x + SIDE_INSET;
            let right = bounds.max.x - SIDE_INSET;
            let boxed = |text: &str, top: f32, bottom: f32| {
                ScrollText::new(
                    text,
                    Font::Geneva,
                    BODY_SIZE,
                    BODY_COLOR,
                    Bounds::at(
                        Point::new(left, top),
                        (right - left).max(0.0),
                        (bottom - top).max(0.0),
                    ),
                    &metrics,
                )
            };
            Laid {
                bounds,
                picture,
                commercial: boxed(
                    &news.commercial,
                    bounds.min.y + COMMERCIAL_TOP,
                    bounds.min.y + COMMERCIAL_BOTTOM,
                ),
                story: boxed(
                    &news.story,
                    bounds.min.y + STORY_TOP,
                    bounds.max.y - STORY_BOTTOM_INSET,
                ),
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
                fill_rect(list, laid.bounds, Color::BLACK);
                if let Some(id) = laid.picture {
                    list.stretched_picture(
                        ImageKey::picture(id),
                        laid.bounds.min,
                        laid.bounds.width(),
                        laid.bounds.height(),
                    );
                }
                for text in [&laid.commercial, &laid.story] {
                    fill_rect(list, text.rect(), Color::BLACK);
                    text.draw(list);
                }
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
        NewsScreen::new(Ok((template(), Rc::new(MonoMetrics))), &news(), None)
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
    fn the_governments_picture_is_shown_when_it_exists() {
        let exists = |id| id == 9001 || id == 9000;
        assert_eq!(shown_news_picture(9001, exists), Some(9001));
    }

    #[test]
    fn a_missing_picture_falls_back_to_9000() {
        let only_9000 = |id| id == 9000;
        assert_eq!(shown_news_picture(0, only_9000), Some(9000));
        assert_eq!(shown_news_picture(9003, only_9000), Some(9000));
        assert_eq!(shown_news_picture(9000, only_9000), Some(9000));
    }

    #[test]
    fn with_9000_missing_too_there_is_no_picture() {
        assert_eq!(shown_news_picture(9001, |_| false), None);
        assert_eq!(shown_news_picture(9000, |_| false), None);
        assert_eq!(
            shown_news_picture(9001, |id| id == 9001),
            Some(9001),
            "9000 is not needed"
        );
    }

    #[test]
    fn the_commercial_and_story_sit_in_the_originals_bands() {
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
            Bounds::at(Point::new(ORIGIN.x + 10.0, ORIGIN.y + 140.0), 280.0, 40.0),
            "(10,140)-(290,180)"
        );
        assert_eq!(
            story.rect(),
            Bounds::at(Point::new(ORIGIN.x + 10.0, ORIGIN.y + 170.0), 280.0, 56.0),
            "(10,170)-(290,226), over the commercial's last 10 px"
        );
        assert_eq!(screen.problem(), None);
    }

    #[test]
    fn the_bands_follow_the_dialogs_right_and_bottom() {
        let template = DialogTemplate {
            bounds: Bounds::at(Point::new(0.0, 0.0), 400.0, 300.0),
            placement: Placement::Center,
            items: Vec::new(),
        };
        let screen = NewsScreen::new(Ok((template, Rc::new(MonoMetrics))), &news(), None);
        let origin = Point::new(312.0, 234.0);
        assert_eq!(screen.bounds(), Some(Bounds::at(origin, 400.0, 300.0)));
        assert_eq!(
            screen.commercial().expect("laid out").rect(),
            Bounds::at(Point::new(origin.x + 10.0, origin.y + 140.0), 380.0, 40.0)
        );
        assert_eq!(
            screen.story().expect("laid out").rect(),
            Bounds::at(Point::new(origin.x + 10.0, origin.y + 170.0), 380.0, 126.0)
        );
    }

    /// What `screen` should draw: black over its frame, `picture`
    /// stretched to it, then each band blacked out under its text.
    fn expected(screen: &NewsScreen, picture: Option<i16>) -> DrawList {
        let bounds = Bounds::at(ORIGIN, 300.0, 230.0);
        let mut list = DrawList::new();
        fill_rect(&mut list, bounds, Color::BLACK);
        if let Some(id) = picture {
            list.stretched_picture(ImageKey::picture(id), ORIGIN, 300.0, 230.0);
        }
        let commercial = screen.commercial().expect("laid out");
        fill_rect(&mut list, commercial.rect(), Color::BLACK);
        commercial.draw(&mut list);
        let story = screen.story().expect("laid out");
        fill_rect(&mut list, story.rect(), Color::BLACK);
        story.draw(&mut list);
        list
    }

    #[test]
    fn it_draws_the_picture_then_each_band_in_white_on_black() {
        let screen = NewsScreen::new(Ok((template(), Rc::new(MonoMetrics))), &news(), Some(9001));
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list, expected(&screen, Some(9001)));
        let first_text = list
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    ..
                } => Some((text.clone(), *origin, *color)),
                _ => None,
            })
            .expect("text");
        assert_eq!(first_text.0, "Buy Bibble");
        assert_eq!(first_text.2, Color::WHITE);
        assert_eq!(first_text.1, Point::new(ORIGIN.x + 10.0, ORIGIN.y + 140.0));
    }

    #[test]
    fn with_no_picture_it_draws_only_black_under_the_text() {
        let screen = open();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        assert_eq!(list, expected(&screen, None));
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
        let mut screen = NewsScreen::new(Err("no DLOG 1014".to_owned()), &news(), Some(9000));
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
        let mut screen = NewsScreen::new(Err("no DLOG 1014".to_owned()), &news(), Some(9000));
        screen.input(&key(Key::Enter, false));
        assert!(screen.closed());
    }

    /// `_NewsDialogFilter` @0x48160 maps Return and Escape to item 1 with no
    /// click; `_DoNewsDialog` plays `_beepSnd[2]` as it closes.
    #[test]
    fn keyboard_activation_is_silent_as_the_original() {
        for k in [Key::Enter, Key::Escape] {
            let mut screen = open();
            screen.take_sounds();
            screen.input(&key(k, false));
            screen.input(&Input::Key {
                key: k,
                pressed: false,
                repeat: false,
            });
            assert_eq!(
                screen.take_sounds(),
                [Sound::Ui(UiSound::NewsClosed)],
                "{k:?}"
            );
            assert!(screen.closed());
        }
    }
}
