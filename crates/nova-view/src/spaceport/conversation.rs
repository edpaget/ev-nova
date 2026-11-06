//! The conversation screen: a landed stellar's conversation, drawn over
//! the spaceport a page at a time.
//!
//! [`ConversationScreen`] takes a page as plain data
//! ([`ConversationPage`](nova_sim::ConversationPage): the lines, the
//! choices, whether the story has ended, and its problem if it failed) and
//! reports what the player did with it ([`ConversationChoice`]). It talks
//! to no flight: the router sends the choice
//! (`FlightView::choose_in_conversation`) and hands the next page back
//! ([`ConversationScreen::set_page`]).
//!
//! It draws a black panel centred in the screen: the page's lines wrapped
//! in white, a problem in [`Color::ERROR`], then the choices numbered from
//! 1 in [`CHOICE_COLOR`], or, once the story has ended, the close hint.
//!
//! - A digit key `1`-`9` (its press, not a repeat) or a click on a
//!   choice's text takes that choice. More than nine choices are reached by
//!   click.
//! - While choices are pending, Return and Escape do nothing: a
//!   conversation isn't dropped half-way.
//! - An ended page closes on Return, Escape or a click anywhere.
//! - It takes every input while it is open, so the screen under it gets
//!   none.
//!
//! Text the panel can't hold runs past its foot: the conversations so far
//! are short.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::ConversationPage;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect, frame_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::text::{TextMetrics, wrap};

/// The panel, centred in the 1024 by 768 screen.
pub const PANEL: Bounds = Bounds {
    min: Point::new(212.0, 184.0),
    max: Point::new(812.0, 584.0),
};
/// The text's inset from the panel's edges.
pub const INSET: f32 = 20.0;
/// The text's font.
pub const FONT: Font = Font::Geneva;
/// The text's size.
pub const SIZE: f32 = 12.0;
/// The gap between the lines and the choices, or the close hint.
pub const GAP: f32 = 12.0;
/// The choices' colour: the warm gold of a button's text.
pub const CHOICE_COLOR: Color = Color::from_rgb24(0x00FF_CC66);
/// The hint an ended page shows.
pub const CLOSE_HINT: &str = "Return to close";

/// What the player did with the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationChoice {
    /// Took the choice at this index.
    Choose(usize),
    /// Closed the ended page.
    Close,
}

/// A line of text placed on the panel.
#[derive(Clone, Debug, PartialEq)]
struct Placed {
    text: String,
    origin: Point,
    color: Color,
}

/// The page laid out: every line placed, and the box each choice's text
/// fills.
struct Layout {
    lines: Vec<Placed>,
    choices: Vec<Bounds>,
}

/// The conversation's page, drawn over the spaceport.
#[derive(Clone)]
pub struct ConversationScreen {
    page: ConversationPage,
    metrics: Rc<dyn TextMetrics>,
    choice: Option<ConversationChoice>,
}

impl std::fmt::Debug for ConversationScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversationScreen")
            .field("page", &self.page)
            .field("choice", &self.choice)
            .finish_non_exhaustive()
    }
}

impl ConversationScreen {
    /// The screen showing `page`, its text measured by `metrics`.
    #[must_use]
    pub fn new(page: ConversationPage, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            page,
            metrics,
            choice: None,
        }
    }

    /// The page it shows.
    #[must_use]
    pub fn page(&self) -> &ConversationPage {
        &self.page
    }

    /// Shows `page` instead.
    pub fn set_page(&mut self, page: ConversationPage) {
        self.page = page;
    }

    /// What the player did since this was last taken, if anything.
    pub fn take_choice(&mut self) -> Option<ConversationChoice> {
        self.choice.take()
    }

    /// The page laid out down the panel, each line at its row: the rows
    /// before it, and the gap once the choices or the hint begin.
    fn layout(&self) -> Layout {
        let width = PANEL.width() - 2.0 * INSET;
        let left = PANEL.min.x + INSET;
        let top = PANEL.min.y + INSET;
        let step = self.metrics.line_height(FONT, SIZE);
        let mut rows = 0_u16;
        let mut gap = 0.0;
        let mut lines = Vec::new();
        let mut place = |text: &str, color: Color, gap: f32| {
            let first = rows;
            for line in wrap(&self.metrics, FONT, SIZE, text, width) {
                lines.push(Placed {
                    text: line,
                    origin: Point::new(left, top + f32::from(rows) * step + gap),
                    color,
                });
                rows += 1;
            }
            Bounds {
                min: Point::new(left, top + f32::from(first) * step + gap),
                max: Point::new(left + width, top + f32::from(rows) * step + gap),
            }
        };
        for line in &self.page.lines {
            place(line, Color::WHITE, gap);
        }
        if let Some(problem) = &self.page.problem {
            place(problem, Color::ERROR, gap);
        }
        gap += GAP;
        let choices = self
            .page
            .choices
            .iter()
            .enumerate()
            .map(|(n, choice)| place(&format!("{}. {choice}", n + 1), CHOICE_COLOR, gap))
            .collect();
        if self.page.ended {
            place(CLOSE_HINT, Color::DIM, gap);
        }
        Layout { lines, choices }
    }

    /// The choice `input` takes or the close it asks, if any.
    fn read(&self, input: &Input) -> Option<ConversationChoice> {
        match *input {
            Input::Key {
                key: Key::Enter | Key::Escape,
                pressed: true,
                repeat: false,
            } if self.page.ended => Some(ConversationChoice::Close),
            Input::Key {
                key: Key::Char(digit @ '1'..='9'),
                pressed: true,
                repeat: false,
            } => {
                let index = digit.to_digit(10)? as usize - 1;
                (index < self.page.choices.len()).then_some(ConversationChoice::Choose(index))
            }
            Input::PointerButton {
                pressed: true, at, ..
            } => {
                if self.page.ended {
                    return Some(ConversationChoice::Close);
                }
                self.layout()
                    .choices
                    .iter()
                    .position(|area| area.contains(at))
                    .map(ConversationChoice::Choose)
            }
            _ => None,
        }
    }
}

impl Screen for ConversationScreen {
    /// Notes the choice or the close the input makes (see the module
    /// docs), for [`ConversationScreen::take_choice`]. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if let Some(choice) = self.read(input) {
            self.choice = Some(choice);
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        fill_rect(list, PANEL, Color::BLACK);
        frame_rect(list, PANEL, Color::DIM);
        for line in self.layout().lines {
            list.text_in(FONT, line.text, line.origin, SIZE, None, line.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::text::fixture::MonoMetrics;

    fn page(lines: &[&str], choices: &[&str], ended: bool) -> ConversationPage {
        ConversationPage {
            lines: lines.iter().map(|&l| l.to_owned()).collect(),
            choices: choices.iter().map(|&c| c.to_owned()).collect(),
            ended,
            problem: None,
        }
    }

    fn offer() -> ConversationScreen {
        ConversationScreen::new(
            page(&["A job?"], &["Take it", "Ask", "Not now"], false),
            Rc::new(MonoMetrics),
        )
    }

    fn ended() -> ConversationScreen {
        ConversationScreen::new(page(&["Bay two."], &[], true), Rc::new(MonoMetrics))
    }

    /// Each text drawn, with its origin and colour, in order.
    fn texts(screen: &ConversationScreen) -> Vec<(String, Point, Color)> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    color,
                    font,
                    size,
                    ..
                } => {
                    assert_eq!((*font, *size), (FONT, SIZE), "{text}");
                    Some((text.clone(), *origin, *color))
                }
                _ => None,
            })
            .collect()
    }

    fn key(screen: &mut ConversationScreen, key: Key, repeat: bool) -> Option<ConversationChoice> {
        let action = screen.input(&Input::Key {
            key,
            pressed: true,
            repeat,
        });
        assert_eq!(action, ScreenAction::None);
        screen.take_choice()
    }

    fn click(screen: &mut ConversationScreen, at: Point) -> Option<ConversationChoice> {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at,
        });
        screen.take_choice()
    }

    /// Line `n`'s top.
    fn row(n: f32) -> f32 {
        PANEL.min.y + INSET + n * MonoMetrics.line_height(FONT, SIZE)
    }

    const LEFT: f32 = 232.0;

    #[test]
    fn it_draws_a_black_panel_then_the_lines_then_the_numbered_choices() {
        let screen = offer();
        let mut list = DrawList::new();
        screen.draw(&mut list);
        let mut panel = DrawList::new();
        fill_rect(&mut panel, PANEL, Color::BLACK);
        frame_rect(&mut panel, PANEL, Color::DIM);
        let head: Vec<_> = list.iter().take(panel.len()).cloned().collect();
        assert_eq!(head, panel.iter().cloned().collect::<Vec<_>>());
        assert_eq!(
            texts(&screen),
            [
                (
                    "A job?".to_owned(),
                    Point::new(LEFT, row(0.0)),
                    Color::WHITE
                ),
                (
                    "1. Take it".to_owned(),
                    Point::new(LEFT, row(1.0) + GAP),
                    CHOICE_COLOR
                ),
                (
                    "2. Ask".to_owned(),
                    Point::new(LEFT, row(2.0) + GAP),
                    CHOICE_COLOR
                ),
                (
                    "3. Not now".to_owned(),
                    Point::new(LEFT, row(3.0) + GAP),
                    CHOICE_COLOR
                ),
            ]
        );
    }

    #[test]
    fn long_lines_and_choices_wrap_within_the_panel() {
        // 560 wide at 6 a character: 93 characters to a line.
        let long = "word ".repeat(30);
        let screen = ConversationScreen::new(
            page(&[long.trim_end()], &[long.trim_end()], false),
            Rc::new(MonoMetrics),
        );
        let drawn = texts(&screen);
        assert_eq!(drawn.len(), 4, "{drawn:?}");
        assert_eq!(drawn[1].1, Point::new(LEFT, row(1.0)));
        assert!(drawn[2].0.starts_with("1. word"), "{drawn:?}");
        assert_eq!(drawn[3].1, Point::new(LEFT, row(3.0) + GAP));
        for (text, ..) in &drawn {
            assert!(MonoMetrics.width(FONT, SIZE, text) <= 560.0, "{text}");
        }
    }

    #[test]
    fn a_digit_takes_its_choice_and_one_past_them_or_a_repeat_takes_none() {
        let mut screen = offer();
        assert_eq!(
            key(&mut screen, Key::Char('2'), false),
            Some(ConversationChoice::Choose(1))
        );
        assert_eq!(
            key(&mut screen, Key::Char('1'), false),
            Some(ConversationChoice::Choose(0))
        );
        assert_eq!(
            key(&mut screen, Key::Char('3'), false),
            Some(ConversationChoice::Choose(2))
        );
        assert_eq!(key(&mut screen, Key::Char('4'), false), None);
        assert_eq!(key(&mut screen, Key::Char('9'), false), None);
        assert_eq!(key(&mut screen, Key::Char('0'), false), None);
        assert_eq!(key(&mut screen, Key::Char('2'), true), None);
        let released = screen.input(&Input::Key {
            key: Key::Char('2'),
            pressed: false,
            repeat: false,
        });
        assert_eq!(released, ScreenAction::None);
        assert_eq!(screen.take_choice(), None);
    }

    #[test]
    fn the_ninth_choice_has_a_key_and_the_tenth_only_a_click() {
        let ten: Vec<String> = (1..=10).map(|n| format!("c{n}")).collect();
        let ten: Vec<&str> = ten.iter().map(String::as_str).collect();
        let mut screen = ConversationScreen::new(page(&[], &ten, false), Rc::new(MonoMetrics));
        assert_eq!(
            key(&mut screen, Key::Char('9'), false),
            Some(ConversationChoice::Choose(8))
        );
        assert_eq!(
            click(&mut screen, Point::new(LEFT + 1.0, row(9.0) + GAP + 1.0)),
            Some(ConversationChoice::Choose(9))
        );
    }

    #[test]
    fn a_click_on_a_choice_s_text_takes_it_and_elsewhere_nothing() {
        let mut screen = offer();
        let second = Point::new(LEFT + 1.0, row(2.0) + GAP + 1.0);
        assert_eq!(
            click(&mut screen, second),
            Some(ConversationChoice::Choose(1))
        );
        let line = Point::new(LEFT + 1.0, row(0.0) + 1.0);
        assert_eq!(click(&mut screen, line), None);
        assert_eq!(click(&mut screen, Point::new(10.0, 10.0)), None);
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: second,
        });
        assert_eq!(screen.take_choice(), None, "a release takes nothing");
    }

    #[test]
    fn return_and_escape_do_nothing_while_choices_are_pending() {
        let mut screen = offer();
        assert_eq!(key(&mut screen, Key::Enter, false), None);
        assert_eq!(key(&mut screen, Key::Escape, false), None);
    }

    #[test]
    fn an_ended_page_draws_its_hint_and_closes_on_return_escape_or_a_click() {
        let mut screen = ended();
        assert_eq!(
            texts(&screen),
            [
                (
                    "Bay two.".to_owned(),
                    Point::new(LEFT, row(0.0)),
                    Color::WHITE
                ),
                (
                    CLOSE_HINT.to_owned(),
                    Point::new(LEFT, row(1.0) + GAP),
                    Color::DIM
                ),
            ]
        );
        for close in [Key::Enter, Key::Escape] {
            assert_eq!(
                key(&mut screen, close, false),
                Some(ConversationChoice::Close)
            );
            assert_eq!(key(&mut screen, close, true), None, "not a repeat");
        }
        assert_eq!(
            click(&mut screen, Point::new(10.0, 10.0)),
            Some(ConversationChoice::Close)
        );
        assert_eq!(key(&mut screen, Key::Char('1'), false), None);
    }

    #[test]
    fn a_problem_is_drawn_in_the_error_colour_after_the_lines() {
        let screen = ConversationScreen::new(
            ConversationPage {
                problem: Some("the story failed".to_owned()),
                ..page(&["Hm."], &[], true)
            },
            Rc::new(MonoMetrics),
        );
        let drawn = texts(&screen);
        assert_eq!(
            drawn[1],
            (
                "the story failed".to_owned(),
                Point::new(LEFT, row(1.0)),
                Color::ERROR
            )
        );
        assert_eq!(drawn[2].0, CLOSE_HINT);
    }

    #[test]
    fn set_page_replaces_what_is_drawn_and_what_input_reads() {
        let mut screen = offer();
        screen.set_page(page(&["Bay two."], &[], true));
        assert_eq!(screen.page(), &page(&["Bay two."], &[], true));
        assert_eq!(texts(&screen)[0].0, "Bay two.");
        assert_eq!(
            key(&mut screen, Key::Enter, false),
            Some(ConversationChoice::Close)
        );
    }

    #[test]
    fn it_debugs_its_page_and_choice() {
        let shown = format!("{:?}", offer());
        assert!(shown.starts_with("ConversationScreen {"), "{shown}");
        assert!(shown.contains("A job?"), "{shown}");
    }

    #[test]
    fn nothing_moves_on_a_tick() {
        let mut screen = offer();
        let before = texts(&screen);
        screen.tick(Duration::from_secs(1));
        assert_eq!(texts(&screen), before);
    }
}
