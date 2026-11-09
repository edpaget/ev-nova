//! The story log: the new game's own read-only panel for its news and its
//! active missions.
//!
//! The new game ships no interface file, so the stock screens laid out
//! from a `DLOG` (the bar's news, `DLOG` 1014; the mission info screen,
//! `DLOG` 1012) can't open in it. Its landed screen is its own
//! `DLOG`-free panel ([`ConversationScreen`](crate::spaceport::conversation::ConversationScreen)),
//! and the story log is drawn on the same geometry and font
//! ([`PANEL`], [`INSET`], [`FONT`], [`SIZE`]). It isn't a stock reading,
//! so it needs no `Rulebook` key, and stock never opens it.
//!
//! [`StoryLogScreen`] takes a [`StoryLog`] as plain data: a heading, then
//! entries, each a title drawn in [`CHOICE_COLOR`] and its lines wrapped
//! in white, then the close hint. The wording functions build the logs:
//!
//! - [`news_log`]: the news heard where the ship is, newest first, each
//!   its headline, its source and age ("From Hearth, 6 days old.") and its
//!   text. The age is the session's, worded as given.
//! - [`missions_log`]: the active storyline missions, each its title, its
//!   objective, the cargo aboard while there is some, and where to go once
//!   it is met.
//!
//! **The cut.** The log scrolls nothing. When the entries don't all fit
//! above the close hint, the last ones (the oldest news) are cut whole,
//! and a last line names how many: "And 2 older items."
//!
//! **Input.** Return, Enter, Escape or a click closes it
//! ([`StoryLogScreen::take_closed`]). It takes every input while it is
//! open, so the screen under it gets none.
//!
//! Where it opens is the router's (the `nova` crate's
//! `app/screen/story_log.rs`).

use std::rc::Rc;
use std::time::Duration;

use nova_sim::flight_frame::{Names, StoryObjective, StoryRow};
use nova_sim::storylines::news::HeardNews;
use nova_sim::{StellarId, SystemId};

use crate::color::Color;
use crate::draw::{DrawList, fill_rect, frame_rect};
use crate::geometry::Point;
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::spaceport::conversation::{CHOICE_COLOR, CLOSE_HINT, FONT, GAP, INSET, PANEL, SIZE};
use crate::spaceport::credits::{TON, TONS};
use crate::text::{TextMetrics, wrap};

/// The news log's heading.
pub const NEWS_HEADING: &str = "News";
/// The missions log's heading.
pub const MISSIONS_HEADING: &str = "Missions";
/// What a source names a system the flight doesn't name.
pub const UNKNOWN_SYSTEM: &str = "an unknown system";
/// What an objective names a stellar the flight doesn't name.
pub const UNKNOWN_STELLAR: &str = "an unknown place";

/// A log to show: a heading, then its entries in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryLog {
    /// The heading, drawn first.
    pub heading: String,
    /// The entries, in the order drawn.
    pub entries: Vec<LogEntry>,
    /// What the cut calls the entries it drops: "older" for news.
    pub cut_word: &'static str,
}

/// One entry of a [`StoryLog`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogEntry {
    /// Its title, drawn in [`CHOICE_COLOR`]; none drawn when empty.
    pub title: String,
    /// Its lines, each wrapped, in white.
    pub lines: Vec<String>,
}

/// The source line of news from `from`, heard `age` days ago: "From
/// Hearth, today.", "From Hearth, 1 day old." or "From Hearth, 9 days
/// old.".
#[must_use]
pub fn news_source(from: SystemId, age: u32, names: &Names) -> String {
    let system = names.system(from).unwrap_or(UNKNOWN_SYSTEM);
    match age {
        0 => format!("From {system}, today."),
        1 => format!("From {system}, 1 day old."),
        n => format!("From {system}, {n} days old."),
    }
}

/// The news log of `news`, given oldest first (the session's order,
/// release day then id), listed newest first.
#[must_use]
pub fn news_log(news: &[HeardNews], names: &Names) -> StoryLog {
    StoryLog {
        heading: NEWS_HEADING.to_owned(),
        entries: news
            .iter()
            .rev()
            .map(|item| LogEntry {
                title: item.headline.clone(),
                lines: vec![news_source(item.from, item.age, names), item.text.clone()],
            })
            .collect(),
        cut_word: "older",
    }
}

/// `tons` with its unit: "1 ton", "10 tons".
fn tons_of(tons: u32) -> String {
    format!("{tons} {}", if tons == 1 { TON } else { TONS })
}

/// `stellar` by its name, or [`UNKNOWN_STELLAR`].
fn stellar_named(stellar: Option<StellarId>, names: &Names) -> &str {
    stellar
        .and_then(|stellar| names.stellar(stellar))
        .unwrap_or(UNKNOWN_STELLAR)
}

/// What `objective` asks: "Deliver 10 tons of Hull plate to Verge
/// Camp." or "Take the jump point at Hearth Road from The Wardens.".
#[must_use]
pub fn objective_line(objective: &StoryObjective, names: &Names) -> String {
    match objective {
        StoryObjective::Deliver { cargo, tons, to } => format!(
            "Deliver {} of {cargo} to {}.",
            tons_of(*tons),
            stellar_named(Some(*to), names)
        ),
        StoryObjective::TakePoint { gate, holder } => format!(
            "Take the jump point at {} from {holder}.",
            stellar_named(*gate, names)
        ),
    }
}

/// The missions log of `rows`, in their order: each its title, its
/// objective, "Aboard: 10 tons." while its cargo is aboard, and "Done: see
/// Verge Camp." once it is met.
#[must_use]
pub fn missions_log(rows: &[StoryRow], names: &Names) -> StoryLog {
    StoryLog {
        heading: MISSIONS_HEADING.to_owned(),
        entries: rows
            .iter()
            .map(|row| {
                let mut lines = vec![objective_line(&row.objective, names)];
                if row.cargo > 0 {
                    lines.push(format!("Aboard: {}.", tons_of(row.cargo)));
                }
                if row.met {
                    lines.push(match row.complete_at {
                        Some(at) => format!("Done: see {}.", stellar_named(Some(at), names)),
                        None => "Done.".to_owned(),
                    });
                }
                LogEntry {
                    title: row.title.clone(),
                    lines,
                }
            })
            .collect(),
        cut_word: "more",
    }
}

/// A line of text placed on the panel.
#[derive(Clone, Debug, PartialEq)]
struct Placed {
    text: String,
    origin: Point,
    color: Color,
}

/// The story log, drawn over the spaceport or the paused flight.
#[derive(Clone)]
pub struct StoryLogScreen {
    log: StoryLog,
    metrics: Rc<dyn TextMetrics>,
    closed: bool,
}

impl std::fmt::Debug for StoryLogScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoryLogScreen")
            .field("log", &self.log)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl StoryLogScreen {
    /// The screen showing `log`, its text measured by `metrics`.
    #[must_use]
    pub fn new(log: StoryLog, metrics: Rc<dyn TextMetrics>) -> Self {
        Self {
            log,
            metrics,
            closed: false,
        }
    }

    /// The log it shows.
    #[must_use]
    pub fn log(&self) -> &StoryLog {
        &self.log
    }

    /// Whether the player closed it since this was last taken.
    pub fn take_closed(&mut self) -> bool {
        std::mem::take(&mut self.closed)
    }

    /// `text` wrapped to the panel's width.
    fn wrapped(&self, text: &str) -> Vec<String> {
        wrap(&self.metrics, FONT, SIZE, text, PANEL.width() - 2.0 * INSET)
    }

    /// The log laid out down the panel (see the module docs' cut): each
    /// line at its row, below the gaps before it.
    fn layout(&self) -> Vec<Placed> {
        let left = PANEL.min.x + INSET;
        let top = PANEL.min.y + INSET;
        let foot = PANEL.max.y - INSET;
        let step = self.metrics.line_height(FONT, SIZE);
        let at = |rows: usize, gaps: usize| top + rows as f32 * step + gaps as f32 * GAP;
        let (mut rows, mut gaps) = (0, 0);
        let mut placed = Vec::new();
        let mut put = |rows: &mut usize, gaps: usize, lines: Vec<String>, color| {
            for text in lines {
                placed.push(Placed {
                    text,
                    origin: Point::new(left, at(*rows, gaps)),
                    color,
                });
                *rows += 1;
            }
        };
        put(
            &mut rows,
            gaps,
            self.wrapped(&self.log.heading),
            Color::WHITE,
        );
        let count = self.log.entries.len();
        let mut shown = 0;
        for (n, entry) in self.log.entries.iter().enumerate() {
            let title = if entry.title.is_empty() {
                Vec::new()
            } else {
                self.wrapped(&entry.title)
            };
            let lines: Vec<String> = entry
                .lines
                .iter()
                .flat_map(|line| self.wrapped(line))
                .collect();
            let rest = count - n - 1;
            let cut = if rest > 0 {
                self.wrapped(&cut_line(rest, self.log.cut_word)).len()
            } else {
                0
            };
            // The entry after its gap, the cut line if any is left, and
            // the hint after its gap.
            let bottom = at(rows + title.len() + lines.len() + cut + 1, gaps + 2);
            if bottom > foot {
                break;
            }
            gaps += 1;
            put(&mut rows, gaps, title, CHOICE_COLOR);
            put(&mut rows, gaps, lines, Color::WHITE);
            shown += 1;
        }
        let cut = count - shown;
        if cut > 0 {
            let line = self.wrapped(&cut_line(cut, self.log.cut_word));
            put(&mut rows, gaps, line, Color::DIM);
        }
        gaps += 1;
        put(&mut rows, gaps, vec![CLOSE_HINT.to_owned()], Color::DIM);
        placed
    }
}

/// The line naming the `cut` entries left out: "And 2 older items.", "And
/// 1 older item.".
fn cut_line(cut: usize, word: &str) -> String {
    let items = if cut == 1 { "item" } else { "items" };
    format!("And {cut} {word} {items}.")
}

impl Screen for StoryLogScreen {
    /// Notes a close (see the module docs), for
    /// [`StoryLogScreen::take_closed`]. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            Input::Key {
                key: Key::Enter | Key::NumpadEnter | Key::Escape,
                pressed: true,
                repeat: false,
            }
            | Input::PointerButton { pressed: true, .. } => self.closed = true,
            _ => {}
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        fill_rect(list, PANEL, Color::BLACK);
        frame_rect(list, PANEL, Color::DIM);
        for line in self.layout() {
            list.text_in(FONT, line.text, line.origin, SIZE, None, line.color);
        }
    }
}

#[cfg(test)]
mod tests;
