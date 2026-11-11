use super::*;
use crate::draw::{DrawCommand, fill_rect, frame_rect};
use crate::input::{MouseButton, ScrollDelta, letter_key};
use crate::text::fixture::MonoMetrics;
use crate::ui::scroll_arrows::ScrollArrow;

/// Hearth (130), Drift (131) and Verge (132); Hearth Dock (128), Verge
/// Camp (129) and the Hearth Road gate (140).
fn names() -> Names {
    let mut names = Names::default();
    for (id, name) in [(130, "Hearth"), (131, "Drift"), (132, "Verge")] {
        names.systems.insert(SystemId(id), name.to_owned());
    }
    for (id, name) in [
        (128, "Hearth Dock"),
        (129, "Verge Camp"),
        (140, "Hearth Road"),
    ] {
        names.stellars.insert(StellarId(id), name.to_owned());
    }
    names
}

fn heard(headline: &str, text: &str, from: i16, age: u32) -> HeardNews {
    HeardNews {
        id: headline.to_lowercase(),
        headline: headline.to_owned(),
        text: text.to_owned(),
        from: SystemId(from),
        age,
        read: false,
    }
}

/// The fixture's two news items, as the session orders them: the side
/// road's (released first), then the plate's.
fn fixture_news() -> Vec<HeardNews> {
    vec![
        heard(
            "The side road has failed",
            "The side road's gates went dark this morning; the wardens' road is the only \
             way to the Verge.",
            130,
            6,
        ),
        heard(
            "Hull plate reaches Verge Camp",
            "Ten tons of Guild plate came in to Verge Camp today.",
            132,
            0,
        ),
    ]
}

/// The fixture's dock job, 10 tons of Hull plate to Verge Camp, aboard.
fn dock_job(cargo: u32, met: bool) -> StoryRow {
    StoryRow {
        mission: "dock-job".to_owned(),
        title: "Hull plate for Verge Camp".to_owned(),
        objective: StoryObjective::Deliver {
            cargo: "Hull plate".to_owned(),
            tons: 10,
            to: StellarId(129),
        },
        cargo,
        met,
        complete_at: Some(StellarId(129)),
    }
}

/// The fixture's road, taken from the wardens.
fn the_road() -> StoryRow {
    StoryRow {
        mission: "clear-the-road".to_owned(),
        title: "Clear the hearth road".to_owned(),
        objective: StoryObjective::TakePoint {
            gate: Some(StellarId(140)),
            holder: "The Wardens".to_owned(),
        },
        cargo: 0,
        met: false,
        complete_at: None,
    }
}

fn screen(log: StoryLog) -> StoryLogScreen {
    StoryLogScreen::new(log, Rc::new(MonoMetrics))
}

/// Each text drawn, with its origin and colour, in order.
fn texts(screen: &StoryLogScreen) -> Vec<(String, Point, Color)> {
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

fn just_text(screen: &StoryLogScreen) -> Vec<String> {
    texts(screen).into_iter().map(|(text, ..)| text).collect()
}

/// The panel's text area: [`PANEL`] less [`INSET`] on every side.
fn text_area() -> crate::geometry::Bounds {
    crate::geometry::Bounds {
        min: Point::new(PANEL.min.x + INSET, PANEL.min.y + INSET),
        max: Point::new(PANEL.max.x - INSET, PANEL.max.y - INSET),
    }
}

/// Asserts every line drawn on `screen` lies within the panel's text
/// area, its bottom included.
fn assert_inside(screen: &StoryLogScreen) {
    let area = text_area();
    let step = MonoMetrics.line_height(FONT, SIZE);
    for (text, origin, _) in texts(screen) {
        let width = MonoMetrics.width(FONT, SIZE, &text);
        assert!(
            origin.x >= area.min.x && origin.x + width <= area.max.x,
            "{text}"
        );
        assert!(
            origin.y >= area.min.y && origin.y + step <= area.max.y,
            "{text} at {origin:?}"
        );
    }
}

#[test]
fn a_source_says_today_one_day_or_its_days() {
    let names = names();
    assert_eq!(news_source(SystemId(130), 0, &names), "From Hearth, today.");
    assert_eq!(
        news_source(SystemId(130), 1, &names),
        "From Hearth, 1 day old."
    );
    assert_eq!(
        news_source(SystemId(131), 9, &names),
        "From Drift, 9 days old."
    );
    assert_eq!(
        news_source(SystemId(999), 2, &names),
        "From an unknown system, 2 days old."
    );
}

#[test]
fn the_news_lists_the_newest_first_with_its_source_then_its_text() {
    let log = news_log(&fixture_news(), &names());
    assert_eq!(log.heading, NEWS_HEADING);
    assert_eq!(
        log.entries,
        [
            LogEntry {
                title: "Hull plate reaches Verge Camp".to_owned(),
                lines: vec![
                    "From Verge, today.".to_owned(),
                    "Ten tons of Guild plate came in to Verge Camp today.".to_owned(),
                ],
            },
            LogEntry {
                title: "The side road has failed".to_owned(),
                lines: vec![
                    "From Hearth, 6 days old.".to_owned(),
                    fixture_news()[0].text.clone(),
                ],
            },
        ]
    );
}

#[test]
fn a_delivery_s_lines_say_the_job_the_cargo_aboard_and_where_once_met() {
    let names = names();
    let log = missions_log(&[dock_job(10, false)], &names);
    assert_eq!(log.heading, MISSIONS_HEADING);
    assert_eq!(
        log.entries,
        [LogEntry {
            title: "Hull plate for Verge Camp".to_owned(),
            lines: vec![
                "Deliver 10 tons of Hull plate to Verge Camp.".to_owned(),
                "Aboard: 10 tons.".to_owned(),
            ],
        }]
    );
    let met = missions_log(&[dock_job(0, true)], &names);
    assert_eq!(
        met.entries[0].lines,
        [
            "Deliver 10 tons of Hull plate to Verge Camp.",
            "Done: see Verge Camp."
        ]
    );
    let one = StoryRow {
        objective: StoryObjective::Deliver {
            cargo: "Hull plate".to_owned(),
            tons: 1,
            to: StellarId(129),
        },
        ..dock_job(1, false)
    };
    assert_eq!(
        missions_log(&[one], &names).entries[0].lines,
        [
            "Deliver 1 ton of Hull plate to Verge Camp.",
            "Aboard: 1 ton."
        ]
    );
}

#[test]
fn a_met_mission_with_no_place_says_done() {
    let row = StoryRow {
        met: true,
        ..the_road()
    };
    assert_eq!(
        missions_log(&[row], &names()).entries[0].lines,
        [
            "Take the jump point at Hearth Road from The Wardens.",
            "Done."
        ]
    );
}

#[test]
fn a_take_point_s_line_names_its_gate_and_holder() {
    assert_eq!(
        missions_log(&[the_road()], &names()).entries[0].lines,
        ["Take the jump point at Hearth Road from The Wardens."]
    );
    let unnamed = StoryRow {
        objective: StoryObjective::TakePoint {
            gate: None,
            holder: "rebels".to_owned(),
        },
        ..the_road()
    };
    assert_eq!(
        missions_log(&[unnamed], &names()).entries[0].lines,
        ["Take the jump point at an unknown place from rebels."]
    );
}

#[test]
fn an_untitled_entry_draws_its_lines_alone() {
    let shown = screen(StoryLog {
        heading: "Log".to_owned(),
        entries: vec![LogEntry {
            title: String::new(),
            lines: vec!["Found.".to_owned()],
        }],
    });
    assert_eq!(just_text(&shown), ["Log", "Found.", CLOSE_HINT]);
}

/// Line `n`'s top, `gaps` gaps down.
fn row(n: f32, gaps: f32) -> f32 {
    PANEL.min.y + INSET + n * MonoMetrics.line_height(FONT, SIZE) + gaps * GAP
}

const LEFT: f32 = 232.0;

#[test]
fn it_draws_a_black_panel_the_heading_each_title_and_its_lines_then_the_hint() {
    let shown = screen(missions_log(&[dock_job(10, false)], &names()));
    let mut list = DrawList::new();
    shown.draw(&mut list);
    let mut panel = DrawList::new();
    fill_rect(&mut panel, PANEL, Color::BLACK);
    frame_rect(&mut panel, PANEL, Color::DIM);
    let head: Vec<_> = list.iter().take(panel.len()).cloned().collect();
    assert_eq!(head, panel.iter().cloned().collect::<Vec<_>>());
    assert_eq!(
        texts(&shown),
        [
            (
                MISSIONS_HEADING.to_owned(),
                Point::new(LEFT, row(0.0, 0.0)),
                Color::WHITE
            ),
            (
                "Hull plate for Verge Camp".to_owned(),
                Point::new(LEFT, row(1.0, 1.0)),
                CHOICE_COLOR
            ),
            (
                "Deliver 10 tons of Hull plate to Verge Camp.".to_owned(),
                Point::new(LEFT, row(2.0, 1.0)),
                Color::WHITE
            ),
            (
                "Aboard: 10 tons.".to_owned(),
                Point::new(LEFT, row(3.0, 1.0)),
                Color::WHITE
            ),
            (
                CLOSE_HINT.to_owned(),
                Point::new(LEFT, row(4.0, 2.0)),
                Color::DIM
            ),
        ]
    );
}

#[test]
fn long_lines_wrap_within_the_panel() {
    let long = "word ".repeat(30);
    let shown = screen(news_log(
        &[heard("Long", long.trim_end(), 130, 0)],
        &names(),
    ));
    let drawn = just_text(&shown);
    assert_eq!(drawn.len(), 6, "{drawn:?}");
    assert_inside(&shown);
}

fn key(shown: &mut StoryLogScreen, key: Key, repeat: bool) -> bool {
    let action = shown.input(&Input::Key {
        key,
        pressed: true,
        repeat,
    });
    assert_eq!(action, ScreenAction::None);
    shown.take_closed()
}

#[test]
fn return_enter_escape_or_a_click_closes_it_and_digits_do_nothing() {
    let mut shown = screen(news_log(&fixture_news(), &names()));
    for close in [Key::Enter, Key::NumpadEnter, Key::Escape] {
        assert!(key(&mut shown, close, false), "{close:?}");
        assert!(!shown.take_closed(), "taken");
        assert!(!key(&mut shown, close, true), "not a repeat");
    }
    for digit in ['1', '2', '9'] {
        assert!(
            !key(&mut shown, letter_key(digit).unwrap(), false),
            "{digit}"
        );
    }
    let released = shown.input(&Input::Key {
        key: Key::Enter,
        pressed: false,
        repeat: false,
    });
    assert_eq!(released, ScreenAction::None);
    assert!(!shown.take_closed(), "a release");
    shown.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: Point::new(10.0, 10.0),
    });
    assert!(shown.take_closed(), "a click anywhere");
    shown.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: false,
        at: Point::new(10.0, 10.0),
    });
    assert!(!shown.take_closed(), "a release");
}

/// Twelve news items, `Item 0` the oldest, so listed last: more than the
/// panel holds.
fn twelve() -> StoryLogScreen {
    let many: Vec<HeardNews> = (0..12)
        .map(|n| heard(&format!("Item {n}"), "Some news.", 130, n))
        .collect();
    screen(news_log(&many, &names()))
}

/// Gives `shown` `input` until its oldest entry, `Item 0`, and the hint
/// are drawn, at most 100 times: how many it took. Never closes it.
fn scroll_to_the_oldest(shown: &mut StoryLogScreen, input: &Input) -> usize {
    let reached = |shown: &StoryLogScreen| {
        let drawn = just_text(shown);
        drawn.ends_with(&[
            "Item 0".to_owned(),
            "From Hearth, today.".to_owned(),
            "Some news.".to_owned(),
            CLOSE_HINT.to_owned(),
        ])
    };
    for n in 0..100 {
        if reached(shown) {
            return n;
        }
        assert_eq!(shown.input(input), ScreenAction::None);
        assert!(!shown.take_closed(), "{input:?} closed it");
    }
    panic!(
        "never reached the oldest by {input:?}: {:?}",
        just_text(shown)
    );
}

/// A press of the primary button at `at`.
fn press_at(at: Point) -> Input {
    Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at,
    }
}

#[test]
fn a_long_log_shows_its_newest_first_and_no_oldest() {
    let shown = twelve();
    let drawn = just_text(&shown);
    assert_eq!(drawn[1], "Item 11", "the newest first");
    assert!(!drawn.contains(&"Item 0".to_owned()), "{drawn:?}");
    assert!(!drawn.contains(&CLOSE_HINT.to_owned()), "{drawn:?}");
    assert!(shown.overflows());
    assert_inside(&shown);
}

#[test]
fn a_long_log_reaches_its_oldest_entry_by_down_presses() {
    let mut shown = twelve();
    let down = Input::Key {
        key: Key::ArrowDown,
        pressed: true,
        repeat: false,
    };
    assert!(scroll_to_the_oldest(&mut shown, &down) > 0);
    assert_inside(&shown);
}

#[test]
fn a_long_log_reaches_its_oldest_entry_by_the_wheel() {
    let mut shown = twelve();
    let notch = Input::Scroll {
        delta: ScrollDelta::Lines(1.0),
        at: PANEL.center(),
    };
    assert!(scroll_to_the_oldest(&mut shown, &notch) > 0);
    assert_inside(&shown);
}

#[test]
fn a_long_log_reaches_its_oldest_entry_by_its_down_arrow() {
    let mut shown = twelve();
    let arrow = shown
        .arrow_bounds(ScrollArrow::Down)
        .expect("a down arrow")
        .center();
    assert!(scroll_to_the_oldest(&mut shown, &press_at(arrow)) > 0);
    assert_inside(&shown);
}

#[test]
fn a_press_off_the_arrows_closes_a_long_log_and_one_on_them_does_not() {
    let mut shown = twelve();
    for arrow in [ScrollArrow::Up, ScrollArrow::Down] {
        let at = shown.arrow_bounds(arrow).expect("drawn").center();
        shown.input(&press_at(at));
        assert!(!shown.take_closed(), "{arrow:?}");
    }
    shown.input(&press_at(PANEL.center()));
    assert!(shown.take_closed(), "a press off the arrows");
}

#[test]
fn the_fixture_s_news_and_missions_fit_the_panel() {
    for log in [
        news_log(&fixture_news(), &names()),
        missions_log(&[dock_job(10, false), the_road()], &names()),
    ] {
        let entries = log.entries.len();
        let shown = screen(log);
        let drawn = just_text(&shown);
        assert_inside(&shown);
        assert!(!shown.overflows(), "nothing overflows: {drawn:?}");
        assert_eq!(shown.arrow_bounds(ScrollArrow::Down), None);
        assert_eq!(drawn.last().map(String::as_str), Some(CLOSE_HINT));
        assert!(entries > 0);
    }
}

#[test]
fn nothing_moves_on_a_tick_and_it_debugs_its_log() {
    let mut shown = screen(news_log(&[heard("Found.", "Here.", 130, 0)], &names()));
    let before = texts(&shown);
    shown.tick(Duration::from_secs(1));
    assert_eq!(texts(&shown), before);
    let debug = format!("{shown:?}");
    assert!(debug.starts_with("StoryLogScreen {"), "{debug}");
    assert!(debug.contains("Found."), "{debug}");
    assert_eq!(shown.log().heading, NEWS_HEADING);
}

/// Metrics with whole-number rows: 6 wide a character, 12 a line, so the
/// panel's 360 of text area falls on whole lines and gaps.
#[derive(Clone, Copy, Debug)]
struct Grid;

impl TextMetrics for Grid {
    fn width(&self, _font: crate::font::Font, _size: f32, text: &str) -> f32 {
        text.chars().count() as f32 * 6.0
    }

    fn line_height(&self, _font: crate::font::Font, _size: f32) -> f32 {
        12.0
    }
}

/// An entry of a title and `lines` more short lines.
fn rows(lines: usize) -> LogEntry {
    LogEntry {
        title: "T".to_owned(),
        lines: vec!["l".to_owned(); lines],
    }
}

/// A log of `entries`, on [`Grid`].
fn on_grid(entries: Vec<LogEntry>) -> StoryLogScreen {
    StoryLogScreen::new(
        StoryLog {
            heading: "Log".to_owned(),
            entries,
        },
        Rc::new(Grid),
    )
}

/// What a log of `entries` draws, on [`Grid`].
fn laid_out(entries: Vec<LogEntry>) -> Vec<String> {
    just_text(&on_grid(entries))
}

// On Grid the heading takes a row, an entry its gap and its rows, and the
// hint its gap and a row: 360 = 12 * rows + 12 * gaps.

#[test]
fn an_entry_ending_exactly_at_the_foot_with_the_hint_fits() {
    // Heading 1 + entry 26 + hint 1 rows and 2 gaps: 28 * 12 + 24 = 360.
    let drawn = laid_out(vec![rows(25)]);
    assert_eq!(drawn.len(), 1 + 26 + 1, "whole: {drawn:?}");
    assert_eq!(drawn.last().map(String::as_str), Some(CLOSE_HINT));
}

#[test]
fn the_exact_foot_entry_draws_no_arrows() {
    let shown = on_grid(vec![rows(25)]);
    assert!(!shown.overflows());
    assert_eq!(shown.arrow_bounds(ScrollArrow::Up), None);
    let mut list = DrawList::new();
    shown.draw(&mut list);
    let lines = list
        .iter()
        .filter(|command| matches!(command, DrawCommand::Line { .. }))
        .count();
    let mut panel = DrawList::new();
    PanelBacking::NEW_GAME.draw(&mut panel, PANEL);
    let backing = panel
        .iter()
        .filter(|command| matches!(command, DrawCommand::Line { .. }))
        .count();
    assert_eq!(lines, backing, "the panel's own fills alone");
}

#[test]
fn an_entry_one_row_past_the_foot_overflows_and_one_down_shows_the_hint() {
    let mut shown = on_grid(vec![rows(26)]);
    assert!(shown.overflows());
    let drawn = just_text(&shown);
    assert_eq!(drawn.len(), 1 + 27, "all but the hint: {drawn:?}");
    assert_eq!(drawn[0], "Log");
    shown.input(&Input::Key {
        key: Key::ArrowDown,
        pressed: true,
        repeat: false,
    });
    let drawn = just_text(&shown);
    assert_eq!(drawn.len(), 27 + 1, "all but the heading: {drawn:?}");
    assert_eq!(drawn.last().map(String::as_str), Some(CLOSE_HINT));
}

#[test]
fn a_repeated_down_moves_nothing_and_a_rebound_one_scrolls() {
    let mut shown = on_grid(vec![rows(26)]);
    shown.input(&Input::Key {
        key: Key::ArrowDown,
        pressed: true,
        repeat: true,
    });
    assert_eq!(just_text(&shown)[0], "Log", "a repeat moves nothing");
    let bindings = crate::keymap::SharedBindings::new(crate::keymap::Bindings::default().rebound(
        crate::keymap::UiCommand::Down,
        crate::keymap::KeyCombo::key(Key::KeyJ),
    ));
    let mut shown = on_grid(vec![rows(26)]).with_bindings(bindings);
    shown.input(&Input::Key {
        key: Key::KeyJ,
        pressed: true,
        repeat: false,
    });
    assert_eq!(
        just_text(&shown).last().map(String::as_str),
        Some(CLOSE_HINT)
    );
}

#[test]
fn with_no_news_the_log_says_nothing_has_been_heard() {
    let log = news_log(&[], &names());
    assert_eq!(log.heading, NEWS_HEADING);
    assert_eq!(
        log.entries,
        [LogEntry {
            title: String::new(),
            lines: vec![NOTHING_HEARD.to_owned()],
        }]
    );
    assert_eq!(NOTHING_HEARD, "Nothing heard here yet.");
}

#[test]
fn a_log_is_silent_unless_given_its_sounds_which_play_as_it_opens_and_closes() {
    let mut silent = screen(news_log(&fixture_news(), &names()));
    assert_eq!(silent.take_sounds(), []);
    key(&mut silent, Key::Escape, false);
    assert_eq!(silent.take_sounds(), [], "the landing's pop stays silent");

    let opened = Sound::Ui(crate::sound::UiSound::NewsOpened);
    let closed = Sound::Ui(crate::sound::UiSound::NewsClosed);
    let mut sounding = screen(news_log(&fixture_news(), &names())).with_sounds(opened, closed);
    assert_eq!(sounding.take_sounds(), [opened], "as it opens");
    assert_eq!(sounding.take_sounds(), [], "once");
    assert!(!key(&mut sounding, Key::KeyA, false));
    assert_eq!(sounding.take_sounds(), [], "nothing closed");
    assert!(key(&mut sounding, Key::Escape, false));
    assert_eq!(sounding.take_sounds(), [closed], "as it closes");
}
