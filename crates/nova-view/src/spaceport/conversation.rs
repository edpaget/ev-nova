//! The conversation screen: a landed stellar's conversation, drawn over
//! the spaceport a page at a time.
//!
//! [`ConversationScreen`] takes a page as plain data
//! ([`ConversationPage`](nova_sim_frame::ConversationPage): the lines, the
//! choices, whether the story has ended, and its problem if it failed) and
//! reports what the player did with it ([`ConversationChoice`]). It talks
//! to no flight: the router sends the choice
//! (`FlightView::choose_in_conversation`) and hands the next page back
//! ([`ConversationScreen::set_page`]).
//!
//! It draws a black panel centred in the screen: the page's lines wrapped
//! in white, a problem in [`Color::ERROR`], the router's notice for the
//! page after a gap, also in [`Color::ERROR`] (an accept refused for
//! room, [`room_refusal_text`]), then the choices numbered from 1 in
//! [`CHOICE_COLOR`], or, once the story has ended, the close hint. A new
//! page drops the notice ([`ConversationScreen::set_page`]).
//!
//! - A digit key `1`-`9` (its press, not a repeat) or a click on a
//!   choice's text takes that choice. More than nine choices are reached by
//!   click.
//! - While choices are pending, Return and Escape do nothing: a
//!   conversation isn't dropped half-way.
//! - An ended page closes on Return, Escape or a click anywhere but its
//!   scroll arrows.
//! - It takes every input while it is open, so the screen under it gets
//!   none.
//!
//! **Scrolling.** The lines, the problem, the notice, the choices and the
//! hint are one page in the panel's text area ([`PanelScroll`]). A page
//! the panel can't hold scrolls a row at a time, by Up and Down (a press
//! each, through the bindings given,
//! [`ConversationScreen::with_bindings`]), by a wheel over the text, and
//! by the arrows in the panel's right inset, which show while it
//! overflows, so a line longer than the panel is readable to its end. The
//! choices scroll with the page: a digit takes its choice whatever is
//! shown, and a click takes a choice only where it is drawn. A new page or
//! a new notice lays the page out afresh, from its top. A page that fits
//! draws as it is laid out, with no arrows.

use std::rc::Rc;
use std::time::Duration;

use nova_sim_frame::ConversationPage;
use nova_sim_frame::flight_frame::RoomRefusal;

use crate::color::Color;
use crate::draw::DrawList;
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key, key_letter};
use crate::keymap::SharedBindings;
use crate::screen::{Screen, ScreenAction};
use crate::spaceport::credits::{TON, TONS};
use crate::text::{TextMetrics, wrap};
use crate::ui::panel::PanelBacking;
use crate::ui::panel_scroll::{PageLine, PanelScroll};
use crate::ui::scroll_arrows::ScrollArrow;

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

/// What the conversation's page says when its accept was refused for want
/// of room in the hold: "No room in the hold: this job needs 10 tons of
/// Hull plate, and 5 tons are free. The offer stands for a later visit."
/// The original's own refusal (`STR#` 2002 #356) says to try again, but
/// the new game's offer is decided once a visit, so the words are the new
/// game's own.
#[must_use]
pub fn room_refusal_text(refusal: &RoomRefusal) -> String {
    let unit = |n: u64| if n == 1 { TON } else { TONS };
    let room = u64::try_from(refusal.room).unwrap_or(0);
    let verb = if room == 1 { "is" } else { "are" };
    format!(
        "No room in the hold: this job needs {} {} of {}, and {room} {} {verb} free. \
         The offer stands for a later visit.",
        refusal.tons,
        unit(u64::from(refusal.tons)),
        refusal.cargo,
        unit(room),
    )
}

/// What the player did with the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationChoice {
    /// Took the choice at this index.
    Choose(usize),
    /// Closed the ended page.
    Close,
}

/// The page laid out: every line placed, and the box each choice's text
/// fills, as laid out unscrolled.
struct Layout {
    lines: Vec<PageLine>,
    choices: Vec<Bounds>,
}

/// The conversation's page, drawn over the spaceport.
#[derive(Clone)]
pub struct ConversationScreen {
    page: ConversationPage,
    notice: Option<String>,
    metrics: Rc<dyn TextMetrics>,
    choice: Option<ConversationChoice>,
    /// The page laid out, scrolled in the panel.
    scroll: PanelScroll,
}

impl std::fmt::Debug for ConversationScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConversationScreen")
            .field("page", &self.page)
            .field("notice", &self.notice)
            .field("choice", &self.choice)
            .finish_non_exhaustive()
    }
}

impl ConversationScreen {
    /// The screen showing `page`, its text measured by `metrics`.
    #[must_use]
    pub fn new(page: ConversationPage, metrics: Rc<dyn TextMetrics>) -> Self {
        let scroll = PanelScroll::new(PANEL, INSET, FONT, SIZE, Vec::new(), &metrics);
        let mut screen = Self {
            page,
            notice: None,
            metrics,
            choice: None,
            scroll,
        };
        screen.relay();
        screen
    }

    /// The page it shows.
    #[must_use]
    pub fn page(&self) -> &ConversationPage {
        &self.page
    }

    /// Shows `page` instead, with no notice: the router sets a notice
    /// for each page it belongs to ([`ConversationScreen::set_notice`]).
    pub fn set_page(&mut self, page: ConversationPage) {
        self.page = page;
        self.notice = None;
        self.relay();
    }

    /// Shows `notice` on the page, or none: a line of the router's own
    /// below the page's lines, such as an accept refused for room
    /// ([`room_refusal_text`]). The page is laid out afresh, from its top.
    pub fn set_notice(&mut self, notice: Option<String>) {
        self.notice = notice;
        self.relay();
    }

    /// The notice shown on the page, if any.
    #[must_use]
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// What the player did since this was last taken, if anything.
    pub fn take_choice(&mut self) -> Option<ConversationChoice> {
        self.choice.take()
    }

    /// The screen reading its Up and Down through `bindings`, the
    /// router's, so a rebound key scrolls it.
    #[must_use]
    pub fn with_bindings(self, bindings: SharedBindings) -> Self {
        Self {
            scroll: self.scroll.with_bindings(bindings),
            ..self
        }
    }

    /// Whether the page holds more than the panel shows, so it scrolls.
    #[must_use]
    pub fn overflows(&self) -> bool {
        self.scroll.overflows()
    }

    /// Where `arrow` is drawn, while the page overflows.
    #[must_use]
    pub fn arrow_bounds(&self, arrow: ScrollArrow) -> Option<Bounds> {
        self.scroll.arrow_bounds(arrow)
    }

    /// Lays the page out afresh, from its top, keeping the bindings.
    fn relay(&mut self) {
        self.scroll = self.scroll.relaid(self.layout().lines);
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
                lines.push(PageLine {
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
        if let Some(notice) = &self.notice {
            gap += GAP;
            place(notice, Color::ERROR, gap);
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
                key,
                pressed: true,
                repeat: false,
            } => {
                let digit = key_letter(key).filter(|c| ('1'..='9').contains(c))?;
                let index = digit.to_digit(10)? as usize - 1;
                (index < self.page.choices.len()).then_some(ConversationChoice::Choose(index))
            }
            Input::PointerButton {
                pressed: true, at, ..
            } => {
                if self.page.ended {
                    return Some(ConversationChoice::Close);
                }
                // A choice is taken where it is drawn: its laid-out box
                // moved up by the scroll, inside what is shown.
                let shown = self.scroll.shown_span()?;
                let up = Point::new(0.0, -self.scroll.shift());
                self.layout()
                    .choices
                    .iter()
                    .position(|area| shown.contains(at) && area.offset(up).contains(at))
                    .map(ConversationChoice::Choose)
            }
            _ => None,
        }
    }
}

impl Screen for ConversationScreen {
    /// Scrolls the page, or notes the choice or the close the input makes
    /// (see the module docs), for [`ConversationScreen::take_choice`]. A
    /// press on a scroll arrow only scrolls, and a key takes its choice or
    /// close before it scrolls. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        let pointer = matches!(input, Input::PointerButton { .. } | Input::Scroll { .. });
        if pointer && self.scroll.input(input) {
            return ScreenAction::None;
        }
        if let Some(choice) = self.read(input) {
            self.choice = Some(choice);
        } else if !pointer {
            self.scroll.input(input);
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        PanelBacking::NEW_GAME.draw(list, PANEL);
        self.scroll.draw(list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw::{DrawCommand, fill_rect, frame_rect};
    use crate::input::{MouseButton, letter_key};
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
            key(&mut screen, letter_key('2').unwrap(), false),
            Some(ConversationChoice::Choose(1))
        );
        assert_eq!(
            key(&mut screen, letter_key('1').unwrap(), false),
            Some(ConversationChoice::Choose(0))
        );
        assert_eq!(
            key(&mut screen, letter_key('3').unwrap(), false),
            Some(ConversationChoice::Choose(2))
        );
        assert_eq!(key(&mut screen, letter_key('4').unwrap(), false), None);
        assert_eq!(key(&mut screen, letter_key('9').unwrap(), false), None);
        assert_eq!(key(&mut screen, letter_key('0').unwrap(), false), None);
        assert_eq!(key(&mut screen, letter_key('2').unwrap(), true), None);
        let released = screen.input(&Input::Key {
            key: letter_key('2').unwrap(),
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
            key(&mut screen, letter_key('9').unwrap(), false),
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
    fn each_choice_s_box_spans_its_rows_and_the_text_s_width() {
        let mut screen = offer();
        let width = PANEL.width() - 2.0 * INSET;
        // Above the first choice is the gap after the lines: no choice.
        let gap = Point::new(LEFT + 1.0, row(1.0) + GAP - 0.5);
        assert_eq!(click(&mut screen, gap), None);
        let first = Point::new(LEFT + 1.0, row(1.0) + GAP + 0.5);
        assert_eq!(
            click(&mut screen, first),
            Some(ConversationChoice::Choose(0))
        );
        // Either side of the edge between the second and third.
        let second = Point::new(LEFT + 1.0, row(3.0) + GAP - 0.5);
        assert_eq!(
            click(&mut screen, second),
            Some(ConversationChoice::Choose(1))
        );
        let third = Point::new(LEFT + 1.0, row(3.0) + GAP + 0.5);
        assert_eq!(
            click(&mut screen, third),
            Some(ConversationChoice::Choose(2))
        );
        // Below the last, and either side of the right edge.
        let below = Point::new(LEFT + 1.0, row(4.0) + GAP + 0.5);
        assert_eq!(click(&mut screen, below), None);
        let inside = Point::new(LEFT + width - 0.5, row(1.0) + GAP + 0.5);
        assert_eq!(
            click(&mut screen, inside),
            Some(ConversationChoice::Choose(0))
        );
        let outside = Point::new(LEFT + width + 0.5, row(1.0) + GAP + 0.5);
        assert_eq!(click(&mut screen, outside), None);
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
        assert_eq!(key(&mut screen, letter_key('1').unwrap(), false), None);
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
    fn a_notice_is_drawn_in_the_error_colour_after_the_lines_and_before_the_hint() {
        let mut screen = ended();
        screen.set_notice(Some("No room.".to_owned()));
        assert_eq!(screen.notice(), Some("No room."));
        assert_eq!(
            texts(&screen),
            [
                (
                    "Bay two.".to_owned(),
                    Point::new(LEFT, row(0.0)),
                    Color::WHITE
                ),
                (
                    "No room.".to_owned(),
                    Point::new(LEFT, row(1.0) + GAP),
                    Color::ERROR
                ),
                (
                    CLOSE_HINT.to_owned(),
                    Point::new(LEFT, row(2.0) + 2.0 * GAP),
                    Color::DIM
                ),
            ]
        );
    }

    #[test]
    fn a_notice_comes_before_the_choices_and_their_boxes_follow_it() {
        let mut screen = offer();
        screen.set_notice(Some("No room.".to_owned()));
        let drawn = texts(&screen);
        assert_eq!(drawn[1].0, "No room.");
        assert_eq!(
            drawn[2],
            (
                "1. Take it".to_owned(),
                Point::new(LEFT, row(2.0) + 2.0 * GAP),
                CHOICE_COLOR
            )
        );
        assert_eq!(
            click(
                &mut screen,
                Point::new(LEFT + 1.0, row(2.0) + 2.0 * GAP + 1.0)
            ),
            Some(ConversationChoice::Choose(0))
        );
    }

    #[test]
    fn a_new_page_keeps_no_notice() {
        let mut screen = offer();
        screen.set_notice(Some("No room.".to_owned()));
        screen.set_page(page(&["Bay two."], &[], true));
        assert_eq!(screen.notice(), None);
        assert!(!texts(&screen).iter().any(|(text, ..)| text == "No room."));
        screen.set_notice(Some("No room.".to_owned()));
        screen.set_notice(None);
        assert_eq!(texts(&screen).len(), 2, "cleared");
    }

    /// A refusal of `tons` of Hull plate with `room` free.
    fn refusal(tons: u32, room: i64) -> RoomRefusal {
        RoomRefusal {
            mission: "dock-job".to_owned(),
            title: "Hull plate for Verge Camp".to_owned(),
            cargo: "Hull plate".to_owned(),
            tons,
            room,
        }
    }

    #[test]
    fn the_refusal_names_the_tons_the_room_and_the_cargo() {
        assert_eq!(
            room_refusal_text(&refusal(10, 5)),
            "No room in the hold: this job needs 10 tons of Hull plate, and 5 tons are free. \
             The offer stands for a later visit."
        );
        assert_eq!(
            room_refusal_text(&refusal(1, 0)),
            "No room in the hold: this job needs 1 ton of Hull plate, and 0 tons are free. \
             The offer stands for a later visit."
        );
        assert_eq!(
            room_refusal_text(&refusal(10, 1)),
            "No room in the hold: this job needs 10 tons of Hull plate, and 1 ton is free. \
             The offer stands for a later visit."
        );
    }

    #[test]
    fn the_fixture_s_accept_page_with_its_refusal_fits_the_panel() {
        let step = MonoMetrics.line_height(FONT, SIZE);
        for line in [
            "\"Bay two, then.\"",
            "\"Bay two. Mind the dead gate past Verge.\"",
        ] {
            let mut screen =
                ConversationScreen::new(page(&[line], &[], true), Rc::new(MonoMetrics));
            screen.set_notice(Some(room_refusal_text(&refusal(10, 5))));
            let drawn = texts(&screen);
            assert_eq!(
                drawn.last().map(|(text, ..)| text.as_str()),
                Some(CLOSE_HINT)
            );
            for (text, origin, _) in drawn {
                assert!(
                    origin.x + MonoMetrics.width(FONT, SIZE, &text) <= PANEL.max.x - INSET,
                    "{text}"
                );
                assert!(origin.y + step <= PANEL.max.y - INSET, "{text}");
            }
        }
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

    // A line longer than the panel.

    /// One line of 800 numbered words: some 45 rows wrapped, more than the
    /// 25 the panel's text area holds.
    fn long_line() -> String {
        (0..800)
            .map(|n| format!("w{n:03}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The long line's last row, as it wraps.
    fn last_row() -> String {
        wrap(
            &MonoMetrics,
            FONT,
            SIZE,
            &long_line(),
            PANEL.width() - 2.0 * INSET,
        )
        .pop()
        .expect("rows")
    }

    /// The long line with two choices, or ended.
    fn long(ended: bool) -> ConversationScreen {
        let choices: &[&str] = if ended { &[] } else { &["Take it", "Not now"] };
        ConversationScreen::new(page(&[&long_line()], choices, ended), Rc::new(MonoMetrics))
    }

    fn drawn_text(screen: &ConversationScreen) -> Vec<String> {
        texts(screen).into_iter().map(|(text, ..)| text).collect()
    }

    /// Gives `screen` `input` until `last` is drawn, at most 100 times:
    /// how many it took. Takes no choice.
    fn scroll_until(screen: &mut ConversationScreen, input: &Input, last: &str) -> usize {
        for n in 0..100 {
            if drawn_text(screen).iter().any(|text| text == last) {
                return n;
            }
            screen.input(input);
            assert_eq!(screen.take_choice(), None, "{input:?}");
        }
        panic!("never drew {last:?} by {input:?}");
    }

    /// Asserts every line drawn on `screen` lies inside the panel's text
    /// area.
    fn assert_inside(screen: &ConversationScreen) {
        let step = MonoMetrics.line_height(FONT, SIZE);
        for (text, origin, _) in texts(screen) {
            assert!(origin.y >= PANEL.min.y + INSET, "{text}");
            assert!(origin.y + step <= PANEL.max.y - INSET, "{text}");
        }
    }

    fn down() -> Input {
        Input::Key {
            key: Key::ArrowDown,
            pressed: true,
            repeat: false,
        }
    }

    #[test]
    fn a_line_longer_than_the_panel_does_not_draw_its_last_row_or_choices_at_first() {
        let screen = long(false);
        let drawn = drawn_text(&screen);
        assert!(!drawn.contains(&last_row()), "{drawn:?}");
        assert!(!drawn.contains(&"1. Take it".to_owned()), "{drawn:?}");
        assert!(screen.overflows());
        assert_inside(&screen);
    }

    #[test]
    fn a_page_that_fits_does_not_overflow_and_shows_no_arrows() {
        use crate::ui::scroll_arrows::ScrollArrow;
        for screen in [offer(), ended()] {
            assert!(!screen.overflows());
            assert_eq!(screen.arrow_bounds(ScrollArrow::Down), None);
        }
    }

    #[test]
    fn down_presses_reach_the_line_s_end_and_the_choices() {
        let mut screen = long(false);
        assert!(scroll_until(&mut screen, &down(), "2. Not now") > 0);
        assert!(drawn_text(&screen).contains(&last_row()));
        assert_inside(&screen);
        let repeat = Input::Key {
            key: Key::ArrowUp,
            pressed: true,
            repeat: true,
        };
        screen.input(&repeat);
        assert!(
            drawn_text(&screen).contains(&"2. Not now".to_owned()),
            "a repeat moves nothing"
        );
    }

    #[test]
    fn the_wheel_reaches_the_line_s_end_and_the_choices() {
        let mut screen = long(false);
        let notch = Input::Scroll {
            delta: crate::input::ScrollDelta::Lines(1.0),
            at: PANEL.center(),
        };
        assert!(scroll_until(&mut screen, &notch, "2. Not now") > 0);
        assert!(drawn_text(&screen).contains(&last_row()));
    }

    #[test]
    fn the_down_arrow_reaches_the_line_s_end_and_the_choices() {
        let mut screen = long(false);
        let arrow = screen
            .arrow_bounds(crate::ui::scroll_arrows::ScrollArrow::Down)
            .expect("a down arrow")
            .center();
        let press = Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: arrow,
        };
        assert!(scroll_until(&mut screen, &press, "2. Not now") > 0);
        assert!(drawn_text(&screen).contains(&last_row()));
    }

    #[test]
    fn a_digit_takes_a_choice_scrolled_out_of_view() {
        let mut screen = long(false);
        assert!(!drawn_text(&screen).contains(&"2. Not now".to_owned()));
        assert_eq!(
            key(&mut screen, letter_key('2').unwrap(), false),
            Some(ConversationChoice::Choose(1))
        );
    }

    #[test]
    fn a_click_takes_a_choice_where_it_is_drawn_and_not_where_it_was_laid_out() {
        let mut screen = long(false);
        let laid_out = ConversationScreen::new(
            page(&[&long_line()], &["Take it", "Not now"], false),
            Rc::new(MonoMetrics),
        )
        .layout()
        .choices[0];
        let at_its_place = Point::new(laid_out.min.x + 1.0, laid_out.min.y + 1.0);
        assert_eq!(click(&mut screen, at_its_place), None, "scrolled out");
        scroll_until(&mut screen, &down(), "2. Not now");
        let (_, origin, _) = texts(&screen)
            .into_iter()
            .find(|(text, ..)| text == "1. Take it")
            .expect("drawn");
        assert_eq!(click(&mut screen, at_its_place), None, "not drawn there");
        assert_eq!(
            click(&mut screen, Point::new(origin.x + 1.0, origin.y + 1.0)),
            Some(ConversationChoice::Choose(0))
        );
    }

    #[test]
    fn an_ended_long_page_closes_on_a_press_off_its_arrows_and_not_on_them() {
        use crate::ui::scroll_arrows::ScrollArrow;
        let mut screen = long(true);
        for arrow in [ScrollArrow::Up, ScrollArrow::Down] {
            let at = screen.arrow_bounds(arrow).expect("drawn").center();
            assert_eq!(click(&mut screen, at), None, "{arrow:?}");
        }
        assert_eq!(
            click(&mut screen, PANEL.center()),
            Some(ConversationChoice::Close)
        );
    }

    #[test]
    fn a_new_page_or_notice_starts_from_the_top() {
        let mut screen = long(false);
        let top = drawn_text(&screen)[0].clone();
        screen.input(&down());
        assert_ne!(drawn_text(&screen)[0], top, "scrolled");
        screen.set_page(page(&[&long_line()], &["Take it", "Not now"], false));
        assert_eq!(drawn_text(&screen)[0], top, "a new page");
        screen.input(&down());
        screen.set_notice(Some("No room.".to_owned()));
        assert_eq!(drawn_text(&screen)[0], top, "a new notice");
    }

    #[test]
    fn a_rebound_down_scrolls_it() {
        let bindings =
            crate::keymap::SharedBindings::new(crate::keymap::Bindings::default().rebound(
                crate::keymap::UiCommand::Down,
                crate::keymap::KeyCombo::key(Key::KeyJ),
            ));
        let mut screen = long(false).with_bindings(bindings);
        let top = drawn_text(&screen)[0].clone();
        screen.input(&Input::Key {
            key: Key::KeyJ,
            pressed: true,
            repeat: false,
        });
        assert_ne!(drawn_text(&screen)[0], top);
        screen.set_page(page(&[&long_line()], &[], true));
        screen.input(&Input::Key {
            key: Key::KeyJ,
            pressed: true,
            repeat: false,
        });
        assert_ne!(drawn_text(&screen)[0], top, "a new page keeps the bindings");
    }
}
