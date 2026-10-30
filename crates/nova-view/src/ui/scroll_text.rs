//! Text wrapped into a box and scrolled through it a whole line at a time,
//! or a pixel at a time as `_ScrollDialogText` @0x51839 scrolls one
//! ([`ScrollText::pixel_scrolled`]).

use crate::color::Color;
use crate::draw::{DrawCommand, DrawList};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key, ScrollDelta};
use crate::text::{TextMetrics, wrap};

use super::scroll_arrows::ScrollArrow;

/// How far an arrow's step scrolls a pixel-scrolled text, in the original's
/// pixels (the engine's logical units): `_ShowTextDialog` passes
/// `_ScrollDialogText` 10 for items 5 and 6 (@0x5b0af-0x5b130).
pub const ARROW_UNITS: f32 = 10.0;

/// Text wrapped to a box's width once, when it is made, and shown as many
/// whole lines as fit the box's height (at least one in a box as tall as
/// the text's size), from a first line the arrow keys, a wheel or trackpad
/// over the box, and an owner's page keys, Home and End (through
/// [`ScrollText::scroll_units`]) move. Nothing is drawn outside the box, so
/// the renderer needs no clip.
///
/// [`ScrollText::pixel_scrolled`] makes it scroll by pixels instead: it
/// then shows every line that reaches into the box, a partial one clipped
/// to it.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollText {
    lines: Vec<String>,
    font: Font,
    size: f32,
    color: Color,
    rect: Bounds,
    line_height: f32,
    visible: usize,
    first: usize,
    /// What wheel and trackpad scrolls have moved short of a whole line,
    /// or of a whole pixel when pixel-scrolled, in logical units, toward
    /// the end for positive.
    pending: f32,
    /// Where a pixel-scrolled text is; none for one of whole lines.
    pixels: Option<PixelScroll>,
}

/// Where a pixel-scrolled text is.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PixelScroll {
    /// How far the text is above its place, from 0 to
    /// [`ScrollText::max_offset`].
    offset: f32,
    /// The original's "can scroll down" (`*0xded18`), which is sticky: see
    /// [`ScrollText::scroll_units`].
    can_down: bool,
}

impl ScrollText {
    /// `text` in `font` at `size` and `color`, wrapped by `metrics` to the
    /// width of `rect` and shown inside it from the first line.
    #[must_use]
    pub fn new(
        text: &str,
        font: Font,
        size: f32,
        color: Color,
        rect: Bounds,
        metrics: &impl TextMetrics,
    ) -> Self {
        let line_height = metrics.line_height(font, size);
        // A NaN or negative quotient (no room, or no line height) is 0. A
        // box at least as tall as the text's size, short of a whole line
        // with its leading, still holds the first line's glyphs.
        let whole = (rect.height() / line_height).floor() as usize;
        let visible = if rect.height() >= size {
            whole.max(1)
        } else {
            whole
        };
        Self {
            lines: wrap(metrics, font, size, text, rect.width()),
            font,
            size,
            color,
            rect,
            line_height,
            visible,
            first: 0,
            pending: 0.0,
            pixels: None,
        }
    }

    /// This text scrolled by pixels, as `_ScrollDialogText` scrolls a text
    /// dialog's: from its start, with "can scroll down" set exactly when
    /// it can move, as `_ShowTextDialog` finds out with its two
    /// `TEPinScroll`s (@0x5ae6c-0x5aed7).
    #[must_use]
    pub fn pixel_scrolled(self) -> Self {
        let pixels = Some(PixelScroll {
            offset: 0.0,
            can_down: self.max_offset() > 0.0,
        });
        Self {
            pixels,
            pending: 0.0,
            ..self
        }
    }

    /// `text` in this one's font, size, colour and box, wrapped by
    /// `metrics` and shown from the first line, scrolled the way this one
    /// is.
    #[must_use]
    pub fn rewrapped(&self, text: &str, metrics: &impl TextMetrics) -> Self {
        let new = Self::new(text, self.font, self.size, self.color, self.rect, metrics);
        if self.pixels.is_some() {
            new.pixel_scrolled()
        } else {
            new
        }
    }

    /// Every wrapped line.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The first line shown: when pixel-scrolled, the one at the box's top,
    /// perhaps partly above it.
    #[must_use]
    pub fn first(&self) -> usize {
        self.pixels.map_or(self.first, |pixels| {
            (pixels.offset / self.line_height).floor() as usize
        })
    }

    /// How far the text is scrolled, in logical units.
    #[must_use]
    pub fn offset(&self) -> f32 {
        self.pixels.map_or_else(
            || self.first as f32 * self.line_height,
            |pixels| pixels.offset,
        )
    }

    /// The most a pixel-scrolled text moves: its height, less the box's.
    ///
    /// `TEPinScroll` stops when the last line is in the view rectangle
    /// (Inside Macintosh: Text), with the text's height as
    /// `_ShowTextDialog` measures it, wrapped lines by line height.
    #[must_use]
    pub fn max_offset(&self) -> f32 {
        (self.lines.len() as f32 * self.line_height - self.rect.height()).max(0.0)
    }

    /// How many whole lines fit the box.
    #[must_use]
    pub fn visible(&self) -> usize {
        self.visible
    }

    /// The box.
    #[must_use]
    pub fn rect(&self) -> Bounds {
        self.rect
    }

    /// Whether there is more text than fits the box.
    #[must_use]
    pub fn overflows(&self) -> bool {
        self.lines.len() > self.visible
    }

    /// Moves the first line shown by `lines` (down for positive), no
    /// further than the start, or than where the last line fills the
    /// bottom of the box. A pixel-scrolled text moves that many line
    /// heights.
    pub fn scroll_by(&mut self, lines: isize) {
        if self.pixels.is_some() {
            self.scroll_units(lines as f32 * self.line_height);
            return;
        }
        let last = self.lines.len().saturating_sub(self.visible);
        self.first = self.first.saturating_add_signed(lines).min(last);
    }

    /// One step of `arrow`, as `_ShowTextDialog` takes items 5 and 6
    /// (@0x5b0af-0x5b130): [`ARROW_UNITS`] its way, and nothing while its
    /// flag ([`Self::can_scroll_up`], [`Self::can_scroll_down`]) is clear.
    pub fn scroll_arrow(&mut self, arrow: ScrollArrow) {
        match arrow {
            ScrollArrow::Up if self.can_scroll_up() => self.scroll_units(-ARROW_UNITS),
            ScrollArrow::Down if self.can_scroll_down() => self.scroll_units(ARROW_UNITS),
            ScrollArrow::Up | ScrollArrow::Down => {}
        }
    }

    /// Scrolls one line on each Up or Down press, key repeats included, or
    /// one [`Self::scroll_arrow`] step when pixel-scrolled, and by a wheel
    /// or trackpad scroll over the box, a whole line for each line height
    /// it builds up to, or its pixels when pixel-scrolled. Returns whether
    /// it used `input`: a scroll over the box is used even where it can't
    /// move.
    pub fn input(&mut self, input: &Input) -> bool {
        let key = match *input {
            Input::Key {
                key, pressed: true, ..
            } => key,
            Input::Scroll { delta, at } => return self.wheel(delta, at),
            _ => return false,
        };
        let pixels = self.pixels.is_some();
        match key {
            Key::Up if pixels => self.scroll_arrow(ScrollArrow::Up),
            Key::Down if pixels => self.scroll_arrow(ScrollArrow::Down),
            Key::Up => self.scroll_by(-1),
            Key::Down => self.scroll_by(1),
            _ => return false,
        }
        true
    }

    /// Whether a line before the first shown is hidden, or part of one
    /// when pixel-scrolled: the original's "can scroll up" (`*0xded4c`,
    /// which `_ScrollDialogText` @0x51839 sets).
    #[must_use]
    pub fn can_scroll_up(&self) -> bool {
        self.pixels
            .map_or(self.first > 0, |pixels| pixels.offset > 0.0)
    }

    /// Whether a line after the last shown is hidden: the original's "can
    /// scroll down" (`*0xded18`, which `_ScrollDialogText` @0x51839 sets).
    /// When pixel-scrolled it is that flag itself, which
    /// [`Self::scroll_units`] leaves set at the end until a try fails.
    #[must_use]
    pub fn can_scroll_down(&self) -> bool {
        self.pixels.map_or(
            self.first < self.lines.len().saturating_sub(self.visible),
            |pixels| pixels.can_down,
        )
    }

    /// Clears a pixel-scrolled text's "can scroll down", as End does when
    /// it has scrolled (`_TextDialogFilter` @0x520f7).
    pub fn clear_can_scroll_down(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            pixels.can_down = false;
        }
    }

    /// Scrolls by a wheel's `delta` when `at` is over the box, through
    /// [`Self::scroll_units`].
    fn wheel(&mut self, delta: ScrollDelta, at: Point) -> bool {
        if !self.rect.contains(at) {
            return false;
        }
        self.scroll_units(match delta {
            ScrollDelta::Lines(lines) => lines * self.line_height,
            ScrollDelta::Units(units) => units,
        });
        true
    }

    /// Scrolls by `units` logical units (toward the end for positive) a
    /// whole line for each line height they build up to, keeping what falls
    /// short of a whole line for the next scroll, unless it points past the
    /// end the first line already sits at. That drops what a move cut
    /// short at an end had left over, so a scroll back moves at once.
    ///
    /// A pixel-scrolled text moves a whole pixel at a time instead, as
    /// `_ScrollDialogText`'s immediate path does (@0x518ea-0x51983), keeping
    /// what falls short of one. A scroll that moves no whole pixel leaves
    /// the flags alone. A positive one that leaves the text where it was
    /// clears "can scroll down", and any other sets it. So a page landing
    /// exactly on the end leaves it set until the next try fails. The
    /// original's gates keep a text that fits from getting here, but a
    /// wheel isn't gated, so a text that can't move never sets it.
    pub fn scroll_units(&mut self, units: f32) {
        self.pending += units;
        if let Some(pixels) = self.pixels {
            self.pixels = Some(self.moved_pixels(pixels));
            return;
        }
        let whole = (self.pending / self.line_height).trunc();
        self.pending -= whole * self.line_height;
        self.scroll_by(whole as isize);
        // The leftover has the sign of the move, so a move cut short at an
        // end leaves it pointing past that end too.
        let last = self.lines.len().saturating_sub(self.visible);
        if self.first == 0 {
            self.pending = self.pending.max(0.0);
        }
        if self.first == last {
            self.pending = self.pending.min(0.0);
        }
    }

    /// `pixels` moved by the whole pixels of what is pending, keeping the
    /// rest unless it points past the end the text then sits at.
    fn moved_pixels(&mut self, pixels: PixelScroll) -> PixelScroll {
        let whole = self.pending.trunc();
        self.pending -= whole;
        let most = self.max_offset();
        let mut moved = pixels;
        if whole != 0.0 {
            moved.offset = (pixels.offset + whole).clamp(0.0, most);
            let failed = whole.is_sign_positive() && moved.offset <= pixels.offset;
            moved.can_down = most > 0.0 && !failed;
        }
        if moved.offset <= 0.0 {
            self.pending = self.pending.max(0.0);
        }
        if moved.offset >= most {
            self.pending = self.pending.min(0.0);
        }
        moved
    }

    /// Draws the lines shown, one text run each, a line height apart from
    /// the box's top-left corner. Blank lines draw nothing but keep their
    /// row.
    ///
    /// When pixel-scrolled, every line with its top above the box's bottom
    /// shows, from [`Self::first`]: `TEPinScroll` scrolls the offscreen
    /// world and `CopyBits` copies the box to the window (@0x51a1f), so
    /// one crossing either edge is cut there, as [`list.clipped`](DrawList::clipped)
    /// cuts it. At the start, a box that isn't a whole number of lines
    /// shows a partial last line, as `TETextBox` does.
    pub fn draw(&self, list: &mut DrawList) {
        if let Some(pixels) = self.pixels {
            self.draw_pixels(pixels.offset, list);
            return;
        }
        let shown = self.lines.iter().skip(self.first).take(self.visible);
        for (row, line) in shown.enumerate() {
            if line.is_empty() {
                continue;
            }
            let origin = Point::new(
                self.rect.min.x,
                self.rect.min.y + row as f32 * self.line_height,
            );
            list.text_in(self.font, line, origin, self.size, None, self.color);
        }
    }

    fn draw_pixels(&self, offset: f32, list: &mut DrawList) {
        let (top, bottom) = (self.rect.min.y, self.rect.max.y);
        for (row, line) in self.lines.iter().enumerate().skip(self.first()) {
            let at = top + row as f32 * self.line_height - offset;
            // A NaN bottom compares as no ordering: such a box shows nothing.
            if at.partial_cmp(&bottom) != Some(std::cmp::Ordering::Less) {
                break;
            }
            if line.is_empty() {
                continue;
            }
            let run = DrawCommand::Text {
                text: line.clone(),
                font: self.font,
                origin: Point::new(self.rect.min.x, at),
                size: self.size,
                wrap_width: None,
                color: self.color,
            };
            if at < top || at + self.line_height > bottom {
                list.clipped(self.rect, run);
            } else {
                list.push(run);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::text::fixture::MonoMetrics;
    use crate::ui::scroll_arrows::ScrollArrow;

    const GREEN: Color = Color::rgba(0, 255, 0, 255);

    /// Geneva 10 (5 wide, 12 high a line) in a 20 x `height` box at
    /// (100, 50): four characters a line.
    fn scroll(text: &str, height: f32) -> ScrollText {
        let rect = Bounds::at(Point::new(100.0, 50.0), 20.0, height);
        ScrollText::new(text, Font::Geneva, 10.0, GREEN, rect, &MonoMetrics)
    }

    /// Ten lines, "l0" to "l9", in a box three lines high.
    fn ten_lines() -> ScrollText {
        let text: Vec<String> = (0..10).map(|n| format!("l{n}")).collect();
        scroll(&text.join("\r"), 36.0)
    }

    fn texts(text: &ScrollText) -> Vec<(String, Point)> {
        let mut list = DrawList::new();
        text.draw(&mut list);
        list.iter()
            .map(|command| match command {
                DrawCommand::Text {
                    text,
                    origin,
                    font,
                    size,
                    wrap_width,
                    color,
                } => {
                    assert_eq!(
                        (*font, *size, *wrap_width, *color),
                        (Font::Geneva, 10.0, None, GREEN)
                    );
                    (text.clone(), *origin)
                }
                other => panic!("not text: {other:?}"),
            })
            .collect()
    }

    fn up(repeat: bool) -> Input {
        Input::Key {
            key: Key::Up,
            pressed: true,
            repeat,
        }
    }

    fn down(repeat: bool) -> Input {
        Input::Key {
            key: Key::Down,
            pressed: true,
            repeat,
        }
    }

    /// A point inside [`ten_lines`]'s box.
    const INSIDE: Point = Point::new(110.0, 60.0);

    fn wheel(delta: ScrollDelta) -> Input {
        Input::Scroll { delta, at: INSIDE }
    }

    fn lines(n: f32) -> Input {
        wheel(ScrollDelta::Lines(n))
    }

    fn units(n: f32) -> Input {
        wheel(ScrollDelta::Units(n))
    }

    #[test]
    fn units_scroll_whole_lines_and_share_the_wheels_leftover() {
        let mut text = ten_lines();
        text.scroll_units(24.0);
        assert_eq!(text.first(), 2);
        text.scroll_units(5.0);
        text.scroll_units(5.0);
        assert_eq!(text.first(), 2);
        text.scroll_units(5.0);
        assert_eq!(text.first(), 3, "15 units are a line");
        text.input(&units(9.0));
        assert_eq!(text.first(), 4, "the 3 over and the wheel's 9 are a line");
    }

    #[test]
    fn units_reach_both_ends_and_leave_nothing_over_there() {
        let mut text = ten_lines();
        text.scroll_units(9999.0);
        assert_eq!(text.first(), 7);
        text.scroll_units(-12.0);
        assert_eq!(text.first(), 6, "exactly one line back from the end");
        text.scroll_units(-9999.0);
        assert_eq!(text.first(), 0);
        text.scroll_units(12.0);
        assert_eq!(text.first(), 1, "exactly one line on from the start");
    }

    #[test]
    fn it_can_scroll_up_past_the_first_line_and_down_short_of_the_last() {
        let mut text = ten_lines();
        assert!(!text.can_scroll_up());
        assert!(text.can_scroll_down());
        text.scroll_by(1);
        assert!(text.can_scroll_up());
        assert!(text.can_scroll_down());
        text.scroll_by(5);
        assert!(text.can_scroll_down(), "one line short of the last");
        text.scroll_by(1);
        assert!(text.can_scroll_up());
        assert!(!text.can_scroll_down());
    }

    #[test]
    fn text_that_fits_the_box_can_scroll_neither_way() {
        for text in ["ab\rcd", "a\rb\rc", ""] {
            let text = scroll(text, 36.0);
            assert!(!text.can_scroll_up(), "{text:?}");
            assert!(!text.can_scroll_down(), "{text:?}");
        }
    }

    #[test]
    fn wheel_lines_scroll_and_clamp_like_down_and_up() {
        let mut text = ten_lines();
        assert!(text.input(&lines(1.0)));
        assert_eq!(text.first(), 1);
        assert!(text.input(&lines(3.0)));
        assert_eq!(text.first(), 4);
        assert!(text.input(&lines(100.0)), "used even at the end");
        assert_eq!(text.first(), 7);
        assert!(text.input(&lines(-2.0)));
        assert_eq!(text.first(), 5);
        assert!(text.input(&lines(-100.0)));
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn a_wheel_outside_the_box_is_not_used() {
        let mut text = ten_lines();
        for at in [Point::new(120.5, 60.0), Point::new(110.0, 86.5)] {
            let input = Input::Scroll {
                delta: ScrollDelta::Lines(1.0),
                at,
            };
            assert!(!text.input(&input), "{at:?}");
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn a_wheel_over_text_shorter_than_the_box_is_used_but_never_moves() {
        let mut text = scroll("ab\rcd", 36.0);
        assert!(text.input(&lines(1.0)));
        assert_eq!(text.first(), 0);
        for _ in 0..3 {
            assert!(text.input(&units(5.0)));
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn sub_line_units_build_up_into_lines_from_the_top() {
        let mut text = ten_lines();
        text.input(&units(5.0));
        text.input(&units(5.0));
        assert_eq!(text.first(), 0);
        text.input(&units(5.0));
        assert_eq!(text.first(), 1, "15 units are a line, 3 over");
        text.input(&units(9.0));
        assert_eq!(text.first(), 2, "3 kept and 9 more are a line");
    }

    #[test]
    fn whole_lines_of_units_leave_nothing_over() {
        let mut text = ten_lines();
        text.input(&units(24.0));
        assert_eq!(text.first(), 2);
        text.input(&units(1.0));
        assert_eq!(text.first(), 2, "1 unit is no line");
    }

    #[test]
    fn half_lines_build_up_into_a_line() {
        let mut text = ten_lines();
        text.input(&lines(0.5));
        assert_eq!(text.first(), 0);
        text.input(&lines(0.5));
        assert_eq!(text.first(), 1);
    }

    #[test]
    fn negative_units_build_up_from_the_end() {
        let mut text = ten_lines();
        text.scroll_by(7);
        for _ in 0..3 {
            text.input(&units(-5.0));
        }
        assert_eq!(text.first(), 6);
    }

    #[test]
    fn a_leftover_pointing_past_the_top_is_dropped() {
        let mut text = ten_lines();
        text.input(&units(-5.0));
        text.input(&units(12.0));
        assert_eq!(text.first(), 1, "exactly one line, not 7 units");
    }

    #[test]
    fn a_move_cut_short_at_the_end_drops_its_leftover() {
        let mut text = ten_lines();
        text.input(&units(1205.0));
        assert_eq!(text.first(), 7);
        text.input(&units(-12.0));
        assert_eq!(text.first(), 6, "the 5 over swallow no line");
    }

    #[test]
    fn a_move_that_reaches_the_end_drops_a_leftover_past_it() {
        let mut text = ten_lines();
        text.scroll_by(6);
        text.input(&units(17.0));
        assert_eq!(text.first(), 7);
        text.input(&units(-12.0));
        assert_eq!(text.first(), 6, "the 5 over swallow no line");
    }

    #[test]
    fn rewrapped_text_starts_with_nothing_left_over() {
        let mut text = ten_lines();
        text.input(&units(11.0));
        let mut new = text.rewrapped(
            &(0..10)
                .map(|n| format!("m{n}"))
                .collect::<Vec<_>>()
                .join("\r"),
            &MonoMetrics,
        );
        new.input(&units(11.0));
        assert_eq!(new.first(), 0);
        new.input(&units(1.0));
        assert_eq!(new.first(), 1);
    }

    #[test]
    fn no_line_height_scrolls_nothing() {
        let rect = Bounds::at(Point::new(100.0, 50.0), 20.0, 36.0);
        let text: Vec<String> = (0..10).map(|n| format!("l{n}")).collect();
        let mut text = ScrollText::new(
            &text.join("\r"),
            Font::Geneva,
            0.0,
            GREEN,
            rect,
            &MonoMetrics,
        );
        text.input(&lines(3.0));
        text.input(&units(30.0));
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn text_wraps_to_the_box_width() {
        let text = scroll("abcd efgh ij", 100.0);
        assert_eq!(text.lines(), ["abcd", "efgh", "ij"]);
        assert_eq!(
            text.rect(),
            Bounds::at(Point::new(100.0, 50.0), 20.0, 100.0)
        );
    }

    #[test]
    fn whole_lines_that_fit_the_height_are_visible() {
        assert_eq!(scroll("a", 36.0).visible(), 3);
        assert_eq!(scroll("a", 47.9).visible(), 3);
        assert_eq!(scroll("a", 48.0).visible(), 4);
        assert_eq!(scroll("a", 9.9).visible(), 0);
        assert_eq!(scroll("a", -5.0).visible(), 0);
    }

    #[test]
    fn a_box_as_tall_as_the_text_but_short_of_a_line_shows_its_first_line() {
        // A line's glyphs are the size high, its leading below them: the
        // mission computer's offer name, Times 18 (21.6 a line) in a box
        // 21 high, shows its first line as `TETextBox` does.
        for height in [10.0, 11.9] {
            let text = scroll("a b", height);
            assert_eq!(text.visible(), 1, "{height}");
            assert_eq!(texts(&text).len(), 1, "{height}");
        }
    }

    #[test]
    fn only_visible_lines_draw_a_line_height_apart() {
        assert_eq!(
            texts(&ten_lines()),
            [
                ("l0".to_owned(), Point::new(100.0, 50.0)),
                ("l1".to_owned(), Point::new(100.0, 62.0)),
                ("l2".to_owned(), Point::new(100.0, 74.0)),
            ]
        );
    }

    #[test]
    fn down_scrolls_until_the_last_line_fills_the_bottom() {
        let mut text = ten_lines();
        assert!(text.overflows());
        for expected in 1..=7 {
            assert!(text.input(&down(expected > 3)));
            assert_eq!(text.first(), expected);
        }
        assert!(text.input(&down(true)), "used even at the end");
        assert_eq!(text.first(), 7);
        let shown: Vec<String> = texts(&text).into_iter().map(|(text, _)| text).collect();
        assert_eq!(shown, ["l7", "l8", "l9"]);
        assert_eq!(texts(&text)[0].1, Point::new(100.0, 50.0));
    }

    #[test]
    fn up_scrolls_back_to_the_start() {
        let mut text = ten_lines();
        text.scroll_by(5);
        assert!(text.input(&up(false)));
        assert_eq!(text.first(), 4);
        for _ in 0..10 {
            text.input(&up(true));
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn scrolling_by_many_lines_clamps_at_both_ends() {
        let mut text = ten_lines();
        text.scroll_by(100);
        assert_eq!(text.first(), 7);
        text.scroll_by(-3);
        assert_eq!(text.first(), 4);
        text.scroll_by(-100);
        assert_eq!(text.first(), 0);
        text.scroll_by(isize::MAX);
        assert_eq!(text.first(), 7);
        text.scroll_by(isize::MIN);
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn text_shorter_than_the_box_never_scrolls() {
        let mut text = scroll("ab\rcd", 36.0);
        assert!(!text.overflows());
        text.input(&down(false));
        text.scroll_by(3);
        assert_eq!(text.first(), 0);
        assert_eq!(texts(&text).len(), 2);
        let exact = scroll("a\rb\rc", 36.0);
        assert!(!exact.overflows(), "three lines fit three rows");
    }

    #[test]
    fn other_keys_and_releases_are_not_used() {
        let mut text = ten_lines();
        let release = Input::Key {
            key: Key::Down,
            pressed: false,
            repeat: false,
        };
        let enter = Input::Key {
            key: Key::Enter,
            pressed: true,
            repeat: false,
        };
        for input in [release, enter, Input::PointerMoved(Point::new(110.0, 60.0))] {
            assert!(!text.input(&input), "{input:?}");
        }
        assert_eq!(text.first(), 0);
    }

    #[test]
    fn empty_text_draws_nothing() {
        let text = scroll("", 36.0);
        assert!(text.lines().is_empty());
        assert!(texts(&text).is_empty());
        assert!(!text.overflows());
    }

    #[test]
    fn blank_lines_draw_nothing_but_keep_their_row() {
        assert_eq!(
            texts(&scroll("a\r\rb", 36.0)),
            [
                ("a".to_owned(), Point::new(100.0, 50.0)),
                ("b".to_owned(), Point::new(100.0, 74.0)),
            ]
        );
    }

    #[test]
    fn rewrapped_text_keeps_the_box_and_style_and_starts_at_the_top() {
        let mut text = ten_lines();
        text.scroll_by(4);
        let new = text.rewrapped("abcd efgh", &MonoMetrics);
        assert_eq!(new.lines(), ["abcd", "efgh"]);
        assert_eq!((new.first(), new.visible()), (0, 3));
        assert_eq!(new.rect(), text.rect());
        assert_eq!(
            texts(&new),
            [
                ("abcd".to_owned(), Point::new(100.0, 50.0)),
                ("efgh".to_owned(), Point::new(100.0, 62.0)),
            ]
        );
    }

    // Pixel scrolling, `_ScrollDialogText` @0x51839.

    /// [`ten_lines`] scrolled by pixels: 120 high in a 36 box, so 84 is
    /// the most it moves.
    fn pixels() -> ScrollText {
        ten_lines().pixel_scrolled()
    }

    /// Each run drawn: its text, its top and its clip, if any.
    fn runs(text: &ScrollText) -> Vec<(String, f32, Option<Bounds>)> {
        fn run(command: &DrawCommand, clip: Option<Bounds>) -> (String, f32, Option<Bounds>) {
            match command {
                DrawCommand::Text { text, origin, .. } => (text.clone(), origin.y, clip),
                DrawCommand::Clipped { clip, command } => run(command, Some(*clip)),
                other => panic!("not text: {other:?}"),
            }
        }
        let mut list = DrawList::new();
        text.draw(&mut list);
        list.iter().map(|command| run(command, None)).collect()
    }

    #[test]
    fn a_pixel_scrolled_text_starts_at_its_top_with_only_down_set() {
        let text = pixels();
        assert_eq!((text.offset(), text.first()), (0.0, 0));
        assert_eq!(text.max_offset(), 84.0);
        assert!(!text.can_scroll_up());
        assert!(text.can_scroll_down());
    }

    /// `_ShowTextDialog` sets Down exactly when the text can move
    /// (@0x5ae6c-0x5aead).
    #[test]
    fn down_starts_set_only_for_text_that_overflows_the_box() {
        for (height, max, down) in [(36.0, 0.0, false), (35.0, 1.0, true), (100.0, 0.0, false)] {
            let text = scroll("a\rb\rc", height).pixel_scrolled();
            assert_eq!(text.max_offset(), max, "{height}");
            assert_eq!(text.can_scroll_down(), down, "{height}");
        }
    }

    #[test]
    fn a_text_starts_pixel_scrolling_from_its_top_with_nothing_left_over() {
        let mut text = ten_lines();
        text.scroll_by(3);
        text.scroll_units(5.0);
        let mut text = text.pixel_scrolled();
        assert_eq!((text.offset(), text.first()), (0.0, 0));
        text.scroll_units(0.5);
        assert_eq!(text.offset(), 0.0, "the 5 units over are gone");
    }

    #[test]
    fn units_move_exact_pixels_and_first_floors_to_the_line_at_the_top() {
        let mut text = pixels();
        text.scroll_units(5.0);
        assert_eq!((text.offset(), text.first()), (5.0, 0));
        assert!(text.can_scroll_up());
        text.scroll_units(6.0);
        assert_eq!((text.offset(), text.first()), (11.0, 0));
        text.scroll_units(1.0);
        assert_eq!((text.offset(), text.first()), (12.0, 1));
        text.scroll_units(-7.0);
        assert_eq!((text.offset(), text.first()), (5.0, 0));
        text.scroll_units(-5.0);
        assert_eq!(text.offset(), 0.0);
        assert!(!text.can_scroll_up());
    }

    #[test]
    fn units_clamp_at_both_ends() {
        let mut text = pixels();
        text.scroll_units(9999.0);
        assert_eq!(text.offset(), 84.0);
        text.scroll_units(-9999.0);
        assert_eq!(text.offset(), 0.0);
    }

    /// Down is sticky: `_ScrollDialogText` clears it only when a scroll
    /// toward the end left the text where it was (@0x51958-0x51983).
    #[test]
    fn reaching_the_end_leaves_down_set_until_the_next_try_fails() {
        let mut text = pixels();
        text.scroll_units(84.0);
        assert_eq!(text.offset(), 84.0);
        assert!(text.can_scroll_down(), "it moved");
        text.scroll_units(1.0);
        assert!(!text.can_scroll_down(), "it couldn't");
        text.scroll_units(-1.0);
        assert!(text.can_scroll_down(), "a move back sets it");
    }

    #[test]
    fn a_move_past_the_end_that_still_moves_leaves_down_set() {
        let mut text = pixels();
        text.scroll_units(9999.0);
        assert_eq!(text.offset(), 84.0);
        assert!(text.can_scroll_down());
    }

    #[test]
    fn a_move_toward_the_start_that_cant_move_sets_down() {
        let mut text = pixels();
        text.scroll_units(84.0);
        text.scroll_units(1.0);
        text.clear_can_scroll_down();
        text.scroll_units(-84.0);
        text.clear_can_scroll_down();
        text.scroll_units(-1.0);
        assert_eq!(text.offset(), 0.0);
        assert!(text.can_scroll_down(), "any other move sets it");
    }

    #[test]
    fn clearing_down_is_the_end_keys_arm_and_no_more() {
        let mut text = pixels();
        text.clear_can_scroll_down();
        assert!(!text.can_scroll_down());
        assert_eq!(text.offset(), 0.0);
        let mut whole = ten_lines();
        whole.clear_can_scroll_down();
        assert!(whole.can_scroll_down(), "only a pixel-scrolled text has it");
    }

    #[test]
    fn text_that_fits_never_sets_down_whichever_way_it_is_scrolled() {
        let mut text = scroll("a\rb", 36.0).pixel_scrolled();
        for units in [-5.0, 5.0, -9999.0, 9999.0] {
            text.scroll_units(units);
            assert!(!text.can_scroll_down(), "{units}");
            assert!(!text.can_scroll_up(), "{units}");
            assert_eq!(text.offset(), 0.0);
        }
    }

    #[test]
    fn sub_pixel_units_build_up_and_leave_the_flags_alone_until_they_do() {
        let mut text = pixels();
        text.scroll_units(84.0);
        text.scroll_units(1.0);
        assert!(!text.can_scroll_down());
        text.scroll_units(-0.4);
        assert_eq!(text.offset(), 84.0);
        assert!(!text.can_scroll_down(), "no whole pixel, no scroll");
        text.scroll_units(-0.4);
        assert_eq!(text.offset(), 84.0);
        text.scroll_units(-0.4);
        assert_eq!(text.offset(), 83.0, "1.2 is a pixel");
        assert!(text.can_scroll_down());
    }

    #[test]
    fn a_sub_pixel_leftover_keeps_its_sign_through_a_whole_pixel() {
        let mut text = pixels();
        text.scroll_units(10.0);
        text.scroll_units(-1.5);
        assert_eq!(text.offset(), 9.0);
        text.scroll_units(-0.5);
        assert_eq!(text.offset(), 8.0, "the half left over and this half");
    }

    #[test]
    fn a_leftover_past_either_end_is_dropped() {
        let mut text = pixels();
        text.scroll_units(-0.5);
        text.scroll_units(0.5);
        assert_eq!(text.offset(), 0.0, "the first half past the top is gone");
        text.scroll_units(0.6);
        assert_eq!(text.offset(), 1.0);
        let mut text = pixels();
        text.scroll_units(84.0);
        text.scroll_units(0.5);
        text.scroll_units(-0.5);
        assert_eq!(text.offset(), 84.0, "the half past the end is gone");
        text.scroll_units(-0.6);
        assert_eq!(text.offset(), 83.0);
        let mut text = pixels();
        text.scroll_units(84.5);
        text.scroll_units(-1.0);
        assert_eq!(text.offset(), 83.0, "a move cut short keeps nothing");
    }

    #[test]
    fn a_leftover_inside_the_text_is_kept() {
        let mut text = pixels();
        text.scroll_units(1.5);
        assert_eq!(text.offset(), 1.0);
        text.scroll_units(0.5);
        assert_eq!(text.offset(), 2.0);
    }

    #[test]
    fn a_wheel_scrolls_a_pixel_scrolled_text_by_its_pixels_and_lines() {
        let mut text = pixels();
        assert!(text.input(&units(7.0)));
        assert_eq!(text.offset(), 7.0);
        assert!(text.input(&lines(1.0)));
        assert_eq!(text.offset(), 19.0);
    }

    #[test]
    fn scrolling_lines_moves_a_pixel_scrolled_text_by_line_heights() {
        let mut text = pixels();
        text.scroll_by(2);
        assert_eq!(text.offset(), 24.0);
        text.scroll_by(-1);
        assert_eq!(text.offset(), 12.0);
    }

    #[test]
    fn no_line_height_scrolls_a_pixel_scrolled_text_nowhere() {
        let rect = Bounds::at(Point::new(100.0, 50.0), 20.0, 36.0);
        let mut text =
            ScrollText::new("a\rb", Font::Geneva, 0.0, GREEN, rect, &MonoMetrics).pixel_scrolled();
        text.scroll_units(30.0);
        assert_eq!((text.offset(), text.first()), (0.0, 0));
        assert!(!text.can_scroll_down());
    }

    /// `_ShowTextDialog`'s items 5 and 6 (@0x5b0af-0x5b130).
    #[test]
    fn an_arrow_steps_ten_pixels_while_its_flag_is_set() {
        let mut text = pixels();
        text.scroll_arrow(ScrollArrow::Up);
        assert_eq!(text.offset(), 0.0, "up is clear at the top");
        text.scroll_arrow(ScrollArrow::Down);
        assert_eq!(text.offset(), ARROW_UNITS);
        text.scroll_arrow(ScrollArrow::Up);
        assert_eq!(text.offset(), 0.0);
    }

    #[test]
    fn an_arrow_steps_nothing_once_its_flag_is_clear() {
        let mut text = pixels();
        text.scroll_units(84.0);
        text.scroll_units(1.0);
        text.scroll_units(-1.0);
        text.clear_can_scroll_down();
        text.scroll_arrow(ScrollArrow::Down);
        assert_eq!(text.offset(), 83.0, "down is clear");
        assert!(!text.can_scroll_down(), "no try was made");
    }

    #[test]
    fn up_and_down_step_a_pixel_scrolled_text_ten_pixels_repeats_included() {
        let mut text = pixels();
        for (input, offset) in [
            (down(false), 10.0),
            (down(true), 20.0),
            (up(false), 10.0),
            (up(true), 0.0),
            (up(false), 0.0),
        ] {
            assert!(text.input(&input), "{input:?}");
            assert_eq!(text.offset(), offset, "{input:?}");
        }
    }

    /// `_ShowTextDialog` tests the flag before it scrolls
    /// (@0x5b0b5-0x5b0bd), so a step it stops makes no try, and leaves
    /// "can scroll down" alone where a scroll that can't move would set it.
    #[test]
    fn an_up_step_stopped_by_its_flag_is_no_scroll_at_all() {
        let mut text = pixels();
        text.clear_can_scroll_down();
        text.scroll_arrow(ScrollArrow::Up);
        assert!(!text.can_scroll_down());
    }

    #[test]
    fn down_at_a_failed_end_uses_the_key_and_moves_nothing() {
        let mut text = pixels();
        text.scroll_units(84.0);
        assert!(text.input(&down(false)), "the failed try");
        assert!(!text.can_scroll_down());
        assert!(text.input(&down(false)), "used even with its flag clear");
        assert_eq!(text.offset(), 84.0);
    }

    #[test]
    fn a_whole_line_text_reads_its_offset_from_its_first_line() {
        let mut text = ten_lines();
        text.scroll_by(2);
        assert_eq!(text.offset(), 24.0);
    }

    #[test]
    fn at_offset_zero_a_whole_line_box_draws_plain_runs_as_ever() {
        assert_eq!(
            runs(&pixels()),
            [
                ("l0".to_owned(), 50.0, None),
                ("l1".to_owned(), 62.0, None),
                ("l2".to_owned(), 74.0, None),
            ]
        );
    }

    #[test]
    fn a_partial_line_is_clipped_to_the_box_at_the_top_and_the_bottom() {
        let mut text = pixels();
        text.scroll_units(5.0);
        let clip = Some(text.rect());
        assert_eq!(
            runs(&text),
            [
                ("l0".to_owned(), 45.0, clip),
                ("l1".to_owned(), 57.0, None),
                ("l2".to_owned(), 69.0, None),
                ("l3".to_owned(), 81.0, clip),
            ]
        );
    }

    #[test]
    fn a_line_filling_the_box_exactly_to_either_edge_is_plain() {
        let mut text = pixels();
        text.scroll_units(12.0);
        assert_eq!(
            runs(&text),
            [
                ("l1".to_owned(), 50.0, None),
                ("l2".to_owned(), 62.0, None),
                ("l3".to_owned(), 74.0, None),
            ]
        );
    }

    #[test]
    fn a_line_just_scrolled_out_of_the_box_is_not_drawn() {
        let mut text = pixels();
        text.scroll_units(11.0);
        assert_eq!(runs(&text)[0], ("l0".to_owned(), 39.0, Some(text.rect())));
        text.scroll_units(1.0);
        assert_eq!(runs(&text)[0].0, "l1");
    }

    #[test]
    fn a_box_of_part_of_a_line_shows_it_at_the_start() {
        let text = scroll("a\rb\rc", 18.0).pixel_scrolled();
        let clip = Some(text.rect());
        assert_eq!(
            runs(&text),
            [("a".to_owned(), 50.0, None), ("b".to_owned(), 62.0, clip)]
        );
    }

    #[test]
    fn a_blank_line_keeps_its_row_while_scrolled() {
        let mut text = scroll("a\r\rb\rc\rd", 36.0).pixel_scrolled();
        text.scroll_units(6.0);
        let clip = Some(text.rect());
        assert_eq!(
            runs(&text),
            [
                ("a".to_owned(), 44.0, clip),
                ("b".to_owned(), 68.0, None),
                ("c".to_owned(), 80.0, clip),
            ]
        );
    }

    #[test]
    fn an_empty_pixel_scrolled_text_draws_nothing() {
        assert_eq!(runs(&scroll("", 36.0).pixel_scrolled()), []);
    }

    #[test]
    fn a_box_with_a_nan_bottom_draws_nothing() {
        let rect = Bounds {
            min: Point::new(100.0, 50.0),
            max: Point::new(120.0, f32::NAN),
        };
        let text =
            ScrollText::new("a\rb", Font::Geneva, 10.0, GREEN, rect, &MonoMetrics).pixel_scrolled();
        assert_eq!(runs(&text), []);
    }

    #[test]
    fn rewrapped_text_stays_pixel_scrolled_and_starts_at_the_top() {
        let mut text = pixels();
        text.scroll_units(50.0);
        text.clear_can_scroll_down();
        let mut new = text.rewrapped("a\rb\rc\rd", &MonoMetrics);
        assert_eq!(new.offset(), 0.0);
        assert!(new.can_scroll_down(), "it overflows by a line");
        new.scroll_units(5.0);
        assert_eq!(new.offset(), 5.0, "exact pixels");
        let fits = text.rewrapped("a", &MonoMetrics);
        assert!(!fits.can_scroll_down());
    }
}
