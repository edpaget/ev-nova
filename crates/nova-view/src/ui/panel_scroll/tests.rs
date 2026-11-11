use super::*;
use crate::draw::DrawCommand;
use crate::input::{Key, MouseButton, ScrollDelta};
use crate::keymap::{Bindings, KeyCombo, UiCommand};
use crate::text::fixture::MonoMetrics;

/// A 200 x 100 panel at (0, 0), inset 10: a column 180 x 80 at (10, 10).
const PANEL: Bounds = Bounds {
    min: Point::new(0.0, 0.0),
    max: Point::new(200.0, 100.0),
};
const INSET: f32 = 10.0;
/// Geneva 10, 12 a line on [`MonoMetrics`]: six lines fit the column.
const SIZE: f32 = 10.0;
const STEP: f32 = 12.0;

/// Lines `r0` and on, one a line height below the last, from the
/// column's top.
fn page(lines: usize) -> Vec<PageLine> {
    tops(&(0..lines).map(|n| n as f32 * STEP).collect::<Vec<_>>())
}

/// A line `r<n>` at each of `tops`, below the column's top.
fn tops(tops: &[f32]) -> Vec<PageLine> {
    tops.iter()
        .enumerate()
        .map(|(n, &top)| PageLine {
            text: format!("r{n}"),
            origin: Point::new(10.0, 10.0 + top),
            color: Color::WHITE,
        })
        .collect()
}

fn scroll(lines: Vec<PageLine>) -> PanelScroll {
    PanelScroll::new(PANEL, INSET, Font::Geneva, SIZE, lines, &MonoMetrics)
}

/// Each text drawn, with its origin.
fn texts(scroll: &PanelScroll) -> Vec<(String, Point)> {
    let mut list = DrawList::new();
    scroll.draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, origin, .. } => Some((text.clone(), *origin)),
            _ => None,
        })
        .collect()
}

/// The colours of the fills drawn inside `area`.
fn fills_in(scroll: &PanelScroll, area: Bounds) -> Vec<Color> {
    let mut list = DrawList::new();
    scroll.draw(&mut list);
    list.iter()
        .filter_map(|command| match command {
            DrawCommand::Line {
                from, to, color, ..
            } if area.contains(*from) && area.contains(*to) => Some(*color),
            _ => None,
        })
        .collect()
}

fn key(key: Key, pressed: bool, repeat: bool) -> Input {
    Input::Key {
        key,
        pressed,
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

fn wheel(delta: ScrollDelta, at: Point) -> Input {
    Input::Scroll { delta, at }
}

/// A point inside the column.
const IN_COLUMN: Point = Point::new(50.0, 50.0);

#[test]
fn a_page_that_fits_draws_its_lines_at_their_origins_and_uses_nothing() {
    let lines = page(6);
    let mut fits = scroll(lines.clone());
    assert!(!fits.overflows());
    let mut list = DrawList::new();
    fits.draw(&mut list);
    let mut want = DrawList::new();
    for line in &lines {
        want.text_in(
            Font::Geneva,
            &line.text,
            line.origin,
            SIZE,
            None,
            line.color,
        );
    }
    assert_eq!(list, want, "the lines alone, no arrows");
    for input in [
        key(Key::ArrowDown, true, false),
        key(Key::ArrowUp, true, false),
        wheel(ScrollDelta::Lines(3.0), IN_COLUMN),
        press(IN_COLUMN),
        press(Point::new(195.0, 85.0)),
    ] {
        assert!(!fits.input(&input), "{input:?}");
        assert_eq!(fits.first(), 0);
    }
    assert_eq!(fits.arrow_bounds(ScrollArrow::Down), None);
    assert_eq!(fits.shown(), lines);
}

#[test]
fn a_line_whose_height_crosses_the_foot_is_not_shown() {
    // Its top at 69 puts its baseline (79) inside the 80-high column, but
    // not its line height (81).
    let crossing = scroll(tops(&[0.0, 69.0]));
    assert!(crossing.overflows());
    assert_eq!(
        texts(&crossing),
        [("r0".to_owned(), Point::new(10.0, 10.0))]
    );
    let inside = scroll(tops(&[0.0, 68.0]));
    assert!(!inside.overflows(), "its line height ends on the foot");
    assert_eq!(texts(&inside).len(), 2);
}

#[test]
fn down_moves_one_row_a_press_and_up_moves_back() {
    let mut long = scroll(page(10));
    assert!(long.overflows());
    assert_eq!(texts(&long).len(), 6, "six lines fit");
    assert!(long.input(&key(Key::ArrowDown, true, false)));
    assert_eq!(long.first(), 1);
    assert_eq!(long.shift(), STEP);
    assert_eq!(
        texts(&long)[0],
        ("r1".to_owned(), Point::new(10.0, 10.0)),
        "drawn from the column's top"
    );
    assert!(!long.input(&key(Key::ArrowDown, true, true)), "a repeat");
    assert!(!long.input(&key(Key::ArrowDown, false, false)), "a release");
    assert_eq!(long.first(), 1);
    assert!(long.input(&key(Key::ArrowUp, true, false)));
    assert_eq!(long.first(), 0);
    for _ in 0..10 {
        long.input(&key(Key::ArrowDown, true, false));
    }
    assert_eq!(long.first(), 4, "no further than the last line's fit");
    assert_eq!(
        long.shown().last().map(|line| line.text.as_str()),
        Some("r9")
    );
}

#[test]
fn a_wheel_over_the_column_moves_and_one_outside_it_does_not() {
    let mut long = scroll(page(10));
    assert!(long.input(&wheel(ScrollDelta::Lines(1.0), IN_COLUMN)));
    assert_eq!(long.first(), 1);
    assert!(long.input(&wheel(ScrollDelta::Units(STEP * 2.0), IN_COLUMN)));
    assert_eq!(long.first(), 3);
    assert!(long.input(&wheel(ScrollDelta::Lines(-1.0), IN_COLUMN)));
    assert_eq!(long.first(), 2);
    for outside in [Point::new(195.0, 50.0), Point::new(50.0, 95.0)] {
        assert!(!long.input(&wheel(ScrollDelta::Lines(1.0), outside)));
        assert_eq!(long.first(), 2, "{outside:?}");
    }
}

#[test]
fn the_arrows_sit_in_the_right_inset_bright_while_their_way_can_move() {
    let mut long = scroll(page(10));
    let up = long.arrow_bounds(ScrollArrow::Up).expect("drawn");
    let down = long.arrow_bounds(ScrollArrow::Down).expect("drawn");
    assert_eq!(up, Bounds::at(Point::new(190.0, 10.0), INSET, INSET));
    assert_eq!(down, Bounds::at(Point::new(190.0, 80.0), INSET, INSET));
    let colours = |long: &PanelScroll, area| {
        let fills = fills_in(long, area);
        assert!(!fills.is_empty(), "a glyph in {area:?}");
        let first = fills[0];
        assert!(fills.iter().all(|&fill| fill == first), "{fills:?}");
        first
    };
    assert_eq!(colours(&long, up), Color::DIM, "at the top");
    assert_eq!(colours(&long, down), Color::WHITE);
    long.input(&key(Key::ArrowDown, true, false));
    assert_eq!(colours(&long, up), Color::WHITE);
    assert_eq!(colours(&long, down), Color::WHITE);
    for _ in 0..10 {
        long.input(&key(Key::ArrowDown, true, false));
    }
    assert_eq!(colours(&long, up), Color::WHITE);
    assert_eq!(colours(&long, down), Color::DIM, "at the end");
    // The glyphs are the stock arrows' pen strokes, as many squares.
    let glyph = button_glyph::strokes(ScrollArrow::Down.label(), INSET, INSET).expect("a glyph");
    assert_eq!(fills_in(&long, down).len(), glyph.len());
}

#[test]
fn a_press_on_an_arrow_moves_a_row_its_way_and_is_used_even_at_an_end() {
    let mut long = scroll(page(10));
    let up = long.arrow_bounds(ScrollArrow::Up).expect("drawn").center();
    let down = long
        .arrow_bounds(ScrollArrow::Down)
        .expect("drawn")
        .center();
    assert!(long.input(&press(up)), "used at the top");
    assert_eq!(long.first(), 0);
    assert!(long.input(&press(down)));
    assert_eq!(long.first(), 1);
    for _ in 0..10 {
        assert!(long.input(&press(down)));
    }
    assert_eq!(long.first(), 4, "used at the end, moving nothing");
    assert!(long.input(&press(up)));
    assert_eq!(long.first(), 3);
    let released = Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at: up,
    };
    assert!(!long.input(&released), "a release");
    assert!(!long.input(&press(IN_COLUMN)), "a press off the arrows");
    assert_eq!(long.first(), 3);
}

#[test]
fn the_span_shown_runs_from_the_first_line_drawn_to_the_last_s_foot() {
    let fits = scroll(page(3));
    assert_eq!(
        fits.shown_span(),
        Some(Bounds {
            min: Point::new(10.0, 10.0),
            max: Point::new(190.0, 10.0 + 3.0 * STEP),
        })
    );
    let mut long = scroll(page(10));
    long.input(&key(Key::ArrowDown, true, false));
    assert_eq!(
        long.shown_span(),
        Some(Bounds {
            min: Point::new(10.0, 10.0),
            max: Point::new(190.0, 10.0 + 6.0 * STEP),
        }),
        "six lines from the column's top"
    );
    assert_eq!(scroll(Vec::new()).shown_span(), None);
}

#[test]
fn a_rebound_down_scrolls_on_its_new_key_and_a_relaid_page_keeps_it() {
    let bindings =
        SharedBindings::new(Bindings::default().rebound(UiCommand::Down, KeyCombo::key(Key::KeyJ)));
    let mut long = scroll(page(10)).with_bindings(bindings);
    assert!(long.input(&key(Key::KeyJ, true, false)));
    assert_eq!(long.first(), 1);
    assert!(!long.input(&key(Key::ArrowDown, true, false)));
    assert_eq!(long.first(), 1);
    let mut relaid = long.relaid(page(12));
    assert_eq!(relaid.first(), 0, "from the top");
    assert!(relaid.input(&key(Key::KeyJ, true, false)));
    assert_eq!(relaid.first(), 1);
}

#[test]
fn a_restyled_page_keeps_its_scroll_and_draws_its_new_colours() {
    let mut long = scroll(page(10));
    assert!(long.input(&key(Key::ArrowDown, true, false)));
    assert!(long.input(&key(Key::ArrowDown, true, false)));
    assert_eq!(long.first(), 2);
    let mut lines = page(10);
    lines[3].color = Color::DIM;
    let restyled = long.restyled(lines);
    assert_eq!(restyled.first(), 2, "kept");
    assert_eq!(restyled.shift(), long.shift());
    let colours: Vec<_> = restyled.shown().iter().map(|line| line.color).collect();
    assert_eq!(colours[1], Color::DIM, "r3, the second shown");
    assert_eq!(
        texts(&restyled),
        texts(&long),
        "the same lines, where they were"
    );
}
