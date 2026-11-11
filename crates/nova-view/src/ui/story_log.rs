//! The story log: the new game's own read-only panel for its news and its
//! active missions.
//!
//! The new game's placeholder dialogs (`nova::interface::wire`) lay out
//! the stock mission info screen (`DLOG` 1012) and news screen
//! (`DLOG` 1014), but those list the stock records: the mission slots and
//! the `nëws` strings. The new game's news and storyline missions aren't
//! those records, so they show on its own `DLOG`-free panel, drawn on the
//! geometry and font of its landed screen
//! ([`ConversationScreen`](crate::spaceport::conversation::ConversationScreen):
//! [`PANEL`], [`INSET`], [`FONT`], [`SIZE`]). It isn't a stock reading,
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
//! **Scrolling.** The heading, the entries and the hint are one page in
//! the panel's text area ([`PanelScroll`]). When they don't all fit, the
//! page scrolls a row at a time, by Up and Down (a press each, through
//! the bindings given, [`StoryLogScreen::with_bindings`]), by a wheel
//! over the text, and by the arrows in the panel's right inset, which
//! show while it overflows: every entry, the oldest news last, is
//! reachable. A log that fits draws as it is laid out, with no arrows.
//!
//! **Input.** Return, Enter, Escape or a press off the arrows closes it
//! ([`StoryLogScreen::take_closed`]). It is silent, unless given the
//! sounds it plays as it opens and closes
//! ([`StoryLogScreen::with_sounds`]): the bar's Holovid log beeps as the
//! stock news does. It takes every input while it is
//! open, so the screen under it gets none.
//!
//! Where and when it opens, and what its close marks read, is the
//! router's (the `nova` crate's `app/screen/story_log.rs`).

use std::rc::Rc;
use std::time::Duration;

use nova_sim::flight_frame::{Names, StoryObjective, StoryRow};
use nova_sim::storylines::news::HeardNews;
use nova_sim::{StellarId, SystemId};

use crate::color::Color;
use crate::draw::DrawList;
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::keymap::SharedBindings;
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::spaceport::conversation::{CHOICE_COLOR, CLOSE_HINT, FONT, GAP, INSET, PANEL, SIZE};
use crate::spaceport::credits::{TON, TONS};
use crate::text::{TextMetrics, wrap};
use crate::ui::panel::PanelBacking;
use crate::ui::panel_scroll::{PageLine, PanelScroll};
use crate::ui::scroll_arrows::ScrollArrow;

/// The news log's heading.
pub const NEWS_HEADING: &str = "News";
/// The missions log's heading.
pub const MISSIONS_HEADING: &str = "Missions";
/// What a source names a system the flight doesn't name.
pub const UNKNOWN_SYSTEM: &str = "an unknown system";
/// What an objective names a stellar the flight doesn't name.
pub const UNKNOWN_STELLAR: &str = "an unknown place";
/// The news log's one line when nothing has been heard: only the bar's
/// Holovid opens it so, as the landing's news opens only with news.
pub const NOTHING_HEARD: &str = "Nothing heard here yet.";

/// A log to show: a heading, then its entries in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoryLog {
    /// The heading, drawn first.
    pub heading: String,
    /// The entries, in the order drawn.
    pub entries: Vec<LogEntry>,
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
/// release day then id), listed newest first; with none, one untitled
/// entry, [`NOTHING_HEARD`].
#[must_use]
pub fn news_log(news: &[HeardNews], names: &Names) -> StoryLog {
    let mut entries: Vec<LogEntry> = news
        .iter()
        .rev()
        .map(|item| LogEntry {
            title: item.headline.clone(),
            lines: vec![news_source(item.from, item.age, names), item.text.clone()],
        })
        .collect();
    if entries.is_empty() {
        entries.push(LogEntry {
            title: String::new(),
            lines: vec![NOTHING_HEARD.to_owned()],
        });
    }
    StoryLog {
        heading: NEWS_HEADING.to_owned(),
        entries,
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
    }
}

/// The story log, drawn over the spaceport or the paused flight.
#[derive(Clone)]
pub struct StoryLogScreen {
    log: StoryLog,
    /// The log laid out as one page, scrolled in the panel.
    page: PanelScroll,
    closed: bool,
    /// The sound it plays as it closes, if given
    /// ([`StoryLogScreen::with_sounds`]).
    close_sound: Option<Sound>,
    /// Its sounds not taken yet.
    sounds: Vec<Sound>,
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
    #[allow(clippy::needless_pass_by_value)] // the router's shared metrics, handed over as `ConversationScreen::new` takes them
    pub fn new(log: StoryLog, metrics: Rc<dyn TextMetrics>) -> Self {
        let lines = layout(&log, &metrics);
        Self {
            log,
            page: PanelScroll::new(PANEL, INSET, FONT, SIZE, lines, &metrics),
            closed: false,
            close_sound: None,
            sounds: Vec::new(),
        }
    }

    /// The screen reading its Up and Down through `bindings`, the
    /// router's, so a rebound key scrolls it.
    #[must_use]
    pub fn with_bindings(self, bindings: SharedBindings) -> Self {
        Self {
            page: self.page.with_bindings(bindings),
            ..self
        }
    }

    /// Whether the log holds more than the panel shows, so it scrolls.
    #[must_use]
    pub fn overflows(&self) -> bool {
        self.page.overflows()
    }

    /// Where `arrow` is drawn, while the log overflows.
    #[must_use]
    pub fn arrow_bounds(&self, arrow: ScrollArrow) -> Option<Bounds> {
        self.page.arrow_bounds(arrow)
    }

    /// The screen playing `open` as it opens and `close` as it closes, as
    /// the bar's Holovid log beeps as the stock news does (`_beepSnd[1]`
    /// and `[2]`, @0x4814e; see the `spaceport::news` module). Without
    /// them it is silent, as the landing's news is.
    #[must_use]
    pub fn with_sounds(mut self, open: Sound, close: Sound) -> Self {
        self.sounds.push(open);
        self.close_sound = Some(close);
        self
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
}

/// `log` laid out down the panel as one page (see the module docs): the
/// heading, each entry after its gap, and the hint after its gap, each
/// line at its row below the gaps before it.
fn layout(log: &StoryLog, metrics: &impl TextMetrics) -> Vec<PageLine> {
    let wrapped = |text: &str| wrap(metrics, FONT, SIZE, text, PANEL.width() - 2.0 * INSET);
    let left = PANEL.min.x + INSET;
    let top = PANEL.min.y + INSET;
    let step = metrics.line_height(FONT, SIZE);
    let at = |rows: usize, gaps: usize| top + rows as f32 * step + gaps as f32 * GAP;
    let (mut rows, mut gaps) = (0, 0);
    let mut placed = Vec::new();
    let mut put = |rows: &mut usize, gaps: usize, lines: Vec<String>, color| {
        for text in lines {
            placed.push(PageLine {
                text,
                origin: Point::new(left, at(*rows, gaps)),
                color,
            });
            *rows += 1;
        }
    };
    put(&mut rows, gaps, wrapped(&log.heading), Color::WHITE);
    for entry in &log.entries {
        let title = if entry.title.is_empty() {
            Vec::new()
        } else {
            wrapped(&entry.title)
        };
        let lines: Vec<String> = entry.lines.iter().flat_map(|line| wrapped(line)).collect();
        gaps += 1;
        put(&mut rows, gaps, title, CHOICE_COLOR);
        put(&mut rows, gaps, lines, Color::WHITE);
    }
    gaps += 1;
    put(&mut rows, gaps, vec![CLOSE_HINT.to_owned()], Color::DIM);
    placed
}

impl Screen for StoryLogScreen {
    /// Scrolls its page, or notes a close (see the module docs), for
    /// [`StoryLogScreen::take_closed`]. It never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        if self.page.input(input) {
            return ScreenAction::None;
        }
        match *input {
            Input::Key {
                key: Key::Enter | Key::NumpadEnter | Key::Escape,
                pressed: true,
                repeat: false,
            }
            | Input::PointerButton { pressed: true, .. } => {
                self.closed = true;
                self.sounds.extend(self.close_sound);
            }
            _ => {}
        }
        ScreenAction::None
    }

    /// Its opening and closing sounds, when given, in order.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    fn draw(&self, list: &mut DrawList) {
        PanelBacking::NEW_GAME.draw(list, PANEL);
        self.page.draw(list);
    }
}

#[cfg(test)]
mod tests;
