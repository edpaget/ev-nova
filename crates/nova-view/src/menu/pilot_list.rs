//! The saved pilots' list, over the main menu: choose one to open.
//!
//! A modal panel in the middle of the screen listing the saved pilots by
//! key, the first selected. A click on a pilot opens it; Up and Down move
//! the selection (repeats included), scrolling the list to keep it shown,
//! and Return or Open opens it. Cancel or Escape closes the list. An empty
//! list says so, and the router can show a line in the error colour, such
//! as why a pilot could not be opened.

use std::rc::Rc;
use std::time::Duration;

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::geometry::{Bounds, Point};
use crate::input::{Input, Key};
use crate::screen::{Screen, ScreenAction};
use crate::sound::Sound;
use crate::text::TextMetrics;
use crate::ui::dialog::outline;
use crate::ui::prefs::{BACKDROP, BORDER};
use crate::ui::{Button, ButtonSkin, ButtonStyle, ButtonTracker};

/// The panel's title.
pub const TITLE: &str = "Open Pilot";
/// What an empty list says.
pub const NO_PILOTS: &str = "No saved pilots.";
/// How many pilots show at once.
pub const VISIBLE_ROWS: usize = 10;
/// The selected pilot's highlight.
pub const SELECTED: Color = Color::rgba(48, 64, 112, 255);
/// The panel: 400 x 320, in the middle of the 1024 x 768 screen.
const PANEL: Bounds = Bounds {
    min: Point::new(312.0, 224.0),
    max: Point::new(712.0, 544.0),
};
/// How far in from the panel's edges its contents are.
const MARGIN: f32 = 16.0;
/// Where the first row's top is, below the panel's top.
const ROWS_TOP: f32 = 40.0;
/// Each row's height.
const ROW_HEIGHT: f32 = 20.0;
/// The text's size, in Geneva.
const TEXT_SIZE: f32 = 12.0;
/// Where the error line's top is, below the panel's top.
const ERROR_TOP: f32 = 250.0;
/// The buttons' size.
const BUTTON_SIZE: (f32, f32) = (90.0, 25.0);
/// Where the buttons' tops are, below the panel's top.
const BUTTONS_TOP: f32 = 282.0;

/// What the player chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PilotListOutcome {
    /// Open the pilot saved under this key.
    Open(String),
    /// Close the list.
    Cancel,
}

/// What a pointer press is held on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    /// A pilot, by its index in the list.
    Row(usize),
    Open,
    Cancel,
}

/// The saved pilots, to open one of.
#[derive(Clone)]
pub struct PilotList {
    keys: Vec<String>,
    selected: Option<usize>,
    /// The first pilot shown.
    first: usize,
    error: Option<String>,
    open: Button,
    cancel: Button,
    style: ButtonStyle,
    metrics: Rc<dyn TextMetrics>,
    tracked: Option<Target>,
    tracker: ButtonTracker,
    outcome: Option<PilotListOutcome>,
    sounds: Vec<Sound>,
}

impl std::fmt::Debug for PilotList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PilotList")
            .field("keys", &self.keys)
            .field("selected", &self.selected)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl PilotList {
    /// The pilots saved under `keys`, in that order, the first selected;
    /// buttons labelled in `style` and text measured by `metrics`.
    #[must_use]
    pub fn new(keys: Vec<String>, style: ButtonStyle, metrics: Rc<dyn TextMetrics>) -> Self {
        let (width, height) = BUTTON_SIZE;
        let top = PANEL.min.y + BUTTONS_TOP;
        let open_left = PANEL.max.x - MARGIN - width;
        let cancel_left = open_left - MARGIN - width;
        let mut open = Button::new(
            Bounds::at(Point::new(open_left, top), width, height),
            "Open",
        );
        open.enabled = !keys.is_empty();
        Self {
            selected: (!keys.is_empty()).then_some(0),
            keys,
            first: 0,
            error: None,
            open,
            cancel: Button::new(
                Bounds::at(Point::new(cancel_left, top), width, height),
                "Cancel",
            ),
            style,
            metrics,
            tracked: None,
            tracker: ButtonTracker::default(),
            outcome: None,
            sounds: Vec::new(),
        }
    }

    /// The pilots' keys, in order.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// The selected pilot's index, if there are any.
    #[must_use]
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Shows `error` as a line under the pilots.
    pub fn show_error(&mut self, error: &str) {
        self.error = Some(error.to_owned());
    }

    /// The error shown, if any.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The Open button.
    #[must_use]
    pub fn open_button(&self) -> &Button {
        &self.open
    }

    /// The Cancel button.
    #[must_use]
    pub fn cancel_button(&self) -> &Button {
        &self.cancel
    }

    /// Where the `n`th row of the list (from 0, at the top) is.
    #[must_use]
    pub fn row_rect(&self, n: usize) -> Bounds {
        let top = (n as f32).mul_add(ROW_HEIGHT, PANEL.min.y + ROWS_TOP);
        Bounds::at(
            Point::new(PANEL.min.x + MARGIN, top),
            PANEL.width() - 2.0 * MARGIN,
            ROW_HEIGHT,
        )
    }

    /// Where pilot `index` is drawn, while it is shown.
    #[must_use]
    pub fn row(&self, index: usize) -> Option<Bounds> {
        let shown = index >= self.first && index < self.first + VISIBLE_ROWS;
        (shown && index < self.keys.len()).then(|| self.row_rect(index - self.first))
    }

    /// What the player chose, once.
    pub fn take_outcome(&mut self) -> Option<PilotListOutcome> {
        self.outcome.take()
    }

    /// Opens the selected pilot, if any.
    fn open_selected(&mut self) {
        if let Some(index) = self.selected {
            self.outcome = Some(PilotListOutcome::Open(self.keys[index].clone()));
        }
    }

    /// Selects pilot `index`, scrolling to show it.
    fn select(&mut self, index: usize) {
        self.selected = Some(index);
        if index < self.first {
            self.first = index;
        } else if index >= self.first + VISIBLE_ROWS {
            self.first = index + 1 - VISIBLE_ROWS;
        }
    }

    fn key(&mut self, key: Key, repeat: bool) {
        let last = self.keys.len().checked_sub(1);
        match (key, self.selected, last) {
            (Key::Down, Some(at), Some(last)) => self.select((at + 1).min(last)),
            (Key::Up, Some(at), _) => self.select(at.saturating_sub(1)),
            (Key::Enter, ..) if !repeat => self.open_selected(),
            (Key::Escape, ..) if !repeat => self.outcome = Some(PilotListOutcome::Cancel),
            _ => {}
        }
    }

    /// What is at `at`, if anything takes clicks there.
    fn target(&self, at: Point) -> Option<Target> {
        if self.open.contains(at) {
            return Some(Target::Open);
        }
        if self.cancel.contains(at) {
            return Some(Target::Cancel);
        }
        (self.first..self.keys.len())
            .find(|&index| self.row(index).is_some_and(|row| row.contains(at)))
            .map(Target::Row)
    }

    fn pointer(&mut self, input: &Input) {
        if let Input::PointerButton {
            pressed: true, at, ..
        } = *input
        {
            self.tracker.cancel_pointer();
            self.tracked = self.target(at);
        }
        let Some(target) = self.tracked else {
            return;
        };
        let (area, enabled) = match target {
            Target::Row(index) => (self.row(index).unwrap_or(PANEL), true),
            Target::Open => (self.open.rect, self.open.enabled),
            Target::Cancel => (self.cancel.rect, true),
        };
        let activated = self.tracker.track(area, enabled, input);
        let sound = self.tracker.take_sound();
        if let (Some(sound), Target::Open | Target::Cancel) = (sound, target) {
            self.sounds.push(Sound::Ui(sound));
        }
        if !self.tracker.armed() {
            self.tracked = None;
        }
        if activated {
            match target {
                Target::Row(index) => {
                    self.select(index);
                    self.open_selected();
                }
                Target::Open => self.open_selected(),
                Target::Cancel => self.outcome = Some(PilotListOutcome::Cancel),
            }
        }
    }
}

impl Screen for PilotList {
    /// The list is modal: it takes every input, and never quits.
    fn input(&mut self, input: &Input) -> ScreenAction {
        match *input {
            Input::Key {
                key,
                pressed: true,
                repeat,
            } => self.key(key, repeat),
            Input::Key { .. } | Input::Text(_) => {}
            Input::PointerButton { .. } | Input::PointerMoved(_) => self.pointer(input),
        }
        ScreenAction::None
    }

    /// Nothing moves on its own.
    fn tick(&mut self, _dt: Duration) {}

    /// The backdrop and its outline, the title, the pilots shown (the
    /// selected one highlighted) or that there are none, the error line,
    /// then Cancel and Open.
    fn draw(&self, list: &mut DrawList) {
        fill_rect(list, PANEL, BACKDROP);
        outline(list, PANEL, BORDER);
        let left = PANEL.min.x + MARGIN;
        list.text(
            TITLE,
            Point::new(left, PANEL.min.y + MARGIN),
            TEXT_SIZE,
            None,
            Color::WHITE,
        );
        let inset = (ROW_HEIGHT - self.metrics.line_height(crate::Font::Geneva, TEXT_SIZE)) / 2.0;
        for (index, key) in self.keys.iter().enumerate() {
            let Some(row) = self.row(index) else {
                continue;
            };
            if self.selected == Some(index) {
                fill_rect(list, row, SELECTED);
            }
            list.text(
                key.clone(),
                Point::new(row.min.x + 4.0, row.min.y + inset),
                TEXT_SIZE,
                None,
                Color::WHITE,
            );
        }
        if self.keys.is_empty() {
            let row = self.row_rect(0);
            list.text(
                NO_PILOTS,
                Point::new(row.min.x + 4.0, row.min.y + inset),
                TEXT_SIZE,
                None,
                Color::DIM,
            );
        }
        if let Some(error) = &self.error {
            list.text(
                error.clone(),
                Point::new(left, PANEL.min.y + ERROR_TOP),
                TEXT_SIZE,
                Some(PANEL.width() - 2.0 * MARGIN),
                Color::ERROR,
            );
        }
        for (button, target) in [(&self.cancel, Target::Cancel), (&self.open, Target::Open)] {
            let pressed = self.tracked == Some(target) && self.tracker.pressed();
            button.draw(pressed, &ButtonSkin::NOVA, &self.style, &self.metrics, list);
        }
    }

    fn cancel_pointer(&mut self) {
        self.tracker.cancel_pointer();
        self.tracked = None;
    }

    /// Open's and Cancel's sounds as they are clicked; a pilot clicks
    /// silently.
    fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.sounds)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::MouseButton;
    use crate::sound::UiSound;
    use crate::text::fixture::MonoMetrics;

    fn list(keys: &[&str]) -> PilotList {
        PilotList::new(
            keys.iter().map(|&key| key.to_owned()).collect(),
            ButtonStyle::STOCK,
            Rc::new(MonoMetrics),
        )
    }

    fn key(key: Key, repeat: bool) -> Input {
        Input::Key {
            key,
            pressed: true,
            repeat,
        }
    }

    fn click(list: &mut PilotList, at: Point) -> Vec<Sound> {
        let mut sounds = Vec::new();
        for pressed in [true, false] {
            list.input(&Input::PointerButton {
                button: MouseButton::Left,
                pressed,
                at,
            });
            sounds.extend(list.take_sounds());
        }
        sounds
    }

    fn texts(list: &PilotList) -> Vec<(String, Color)> {
        let mut drawn = DrawList::new();
        list.draw(&mut drawn);
        drawn
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => Some((text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    #[allow(clippy::unnecessary_wraps)] // Compared with `take_outcome`.
    fn open(name: &str) -> Option<PilotListOutcome> {
        Some(PilotListOutcome::Open(name.to_owned()))
    }

    #[test]
    fn the_first_pilot_starts_selected_and_return_opens_it() {
        let mut pilots = list(&["Ada", "Bob"]);
        assert_eq!(pilots.keys(), ["Ada", "Bob"]);
        assert_eq!(pilots.selected(), Some(0));
        pilots.input(&key(Key::Enter, false));
        assert_eq!(pilots.take_outcome(), open("Ada"));
        assert_eq!(pilots.take_outcome(), None, "once");
    }

    #[test]
    fn up_and_down_move_the_selection_within_the_list() {
        let mut pilots = list(&["Ada", "Bob", "Cy"]);
        pilots.input(&key(Key::Down, false));
        pilots.input(&key(Key::Down, true));
        assert_eq!(pilots.selected(), Some(2));
        pilots.input(&key(Key::Down, false));
        assert_eq!(pilots.selected(), Some(2), "the last");
        pilots.input(&key(Key::Up, true));
        assert_eq!(pilots.selected(), Some(1));
        pilots.input(&key(Key::Enter, true));
        assert_eq!(pilots.take_outcome(), None, "a repeat opens nothing");
        pilots.input(&key(Key::Enter, false));
        assert_eq!(pilots.take_outcome(), open("Bob"));
        pilots.input(&key(Key::Up, false));
        pilots.input(&key(Key::Up, false));
        assert_eq!(pilots.selected(), Some(0), "the first");
    }

    #[test]
    fn a_click_on_a_pilot_opens_it_silently() {
        let mut pilots = list(&["Ada", "Bob"]);
        let bob = pilots.row(1).expect("shown").center();
        assert_eq!(click(&mut pilots, bob), []);
        assert_eq!(pilots.selected(), Some(1));
        assert_eq!(pilots.take_outcome(), open("Bob"));
    }

    #[test]
    fn a_press_on_one_pilot_let_go_of_on_another_opens_nothing() {
        let mut pilots = list(&["Ada", "Bob"]);
        let (ada, bob) = (
            pilots.row(0).expect("shown").center(),
            pilots.row(1).expect("shown").center(),
        );
        pilots.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: ada,
        });
        pilots.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: bob,
        });
        assert_eq!(pilots.take_outcome(), None);
    }

    #[test]
    fn open_opens_the_selection_and_cancel_or_escape_cancels() {
        let mut pilots = list(&["Ada", "Bob"]);
        pilots.input(&key(Key::Down, false));
        let open_at = pilots.open_button().rect.center();
        assert_eq!(
            click(&mut pilots, open_at),
            [Sound::Ui(UiSound::ButtonDown), Sound::Ui(UiSound::ButtonUp)]
        );
        assert_eq!(pilots.take_outcome(), open("Bob"));
        let cancel = pilots.cancel_button().rect.center();
        click(&mut pilots, cancel);
        assert_eq!(pilots.take_outcome(), Some(PilotListOutcome::Cancel));
        pilots.input(&key(Key::Escape, true));
        assert_eq!(pilots.take_outcome(), None, "a repeat");
        pilots.input(&key(Key::Escape, false));
        assert_eq!(pilots.take_outcome(), Some(PilotListOutcome::Cancel));
    }

    #[test]
    fn an_empty_list_says_so_and_opens_nothing() {
        let mut pilots = list(&[]);
        assert_eq!(pilots.selected(), None);
        assert!(texts(&pilots).contains(&(NO_PILOTS.to_owned(), Color::DIM)));
        assert_eq!(NO_PILOTS, "No saved pilots.");
        pilots.input(&key(Key::Enter, false));
        pilots.input(&key(Key::Down, false));
        assert!(!pilots.open_button().enabled);
        let open_at = pilots.open_button().rect.center();
        click(&mut pilots, open_at);
        assert_eq!(pilots.take_outcome(), None);
        assert!(pilots.row(0).is_none());
        assert!(
            !texts(&list(&["Ada"]))
                .iter()
                .any(|(text, _)| text == NO_PILOTS)
        );
    }

    #[test]
    fn an_error_is_a_line_in_the_error_colour() {
        let mut pilots = list(&["Ada"]);
        assert_eq!(pilots.error(), None);
        pilots.show_error("nova: the pilot Ada in memory: broken");
        assert_eq!(
            pilots.error(),
            Some("nova: the pilot Ada in memory: broken")
        );
        assert!(texts(&pilots).contains(&(
            "nova: the pilot Ada in memory: broken".to_owned(),
            Color::ERROR
        )));
    }

    #[test]
    fn it_draws_the_title_each_pilot_and_the_selection() {
        let pilots = list(&["Ada", "Bob"]);
        let texts = texts(&pilots);
        assert_eq!(texts[0], (TITLE.to_owned(), Color::WHITE));
        assert_eq!(texts[1], ("Ada".to_owned(), Color::WHITE));
        assert_eq!(texts[2], ("Bob".to_owned(), Color::WHITE));
        let mut drawn = DrawList::new();
        pilots.draw(&mut drawn);
        let row = pilots.row(0).expect("shown");
        let mut highlight = DrawList::new();
        fill_rect(&mut highlight, row, SELECTED);
        let highlight = highlight.iter().next().expect("a fill").clone();
        assert!(drawn.iter().any(|command| *command == highlight));
        assert_eq!(pilots.row(1).expect("shown").min.y, row.max.y);
    }

    #[test]
    fn a_long_list_scrolls_to_keep_the_selection_shown() {
        let names: Vec<String> = (0..15).map(|n| format!("P{n:02}")).collect();
        let keys: Vec<&str> = names.iter().map(String::as_str).collect();
        let mut pilots = list(&keys);
        assert!(pilots.row(VISIBLE_ROWS - 1).is_some());
        assert!(pilots.row(VISIBLE_ROWS).is_none(), "below the list");
        for _ in 0..12 {
            pilots.input(&key(Key::Down, true));
        }
        assert_eq!(pilots.selected(), Some(12));
        assert!(pilots.row(12).is_some());
        assert!(pilots.row(2).is_none(), "scrolled off the top");
        assert_eq!(pilots.row(3), Some(pilots.row_rect(0)));
        let shown: Vec<String> = texts(&pilots)
            .into_iter()
            .map(|(text, _)| text)
            .filter(|text| text.starts_with('P'))
            .collect();
        assert_eq!(shown.first().map(String::as_str), Some("P03"));
        assert_eq!(shown.len(), VISIBLE_ROWS);
        for _ in 0..12 {
            pilots.input(&key(Key::Up, true));
        }
        assert_eq!(pilots.row(0), Some(pilots.row_rect(0)), "back at the top");
    }

    #[test]
    fn cancelling_the_pointer_abandons_a_click() {
        let mut pilots = list(&["Ada"]);
        let ada = pilots.row(0).expect("shown").center();
        pilots.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: true,
            at: ada,
        });
        pilots.cancel_pointer();
        pilots.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed: false,
            at: ada,
        });
        assert_eq!(pilots.take_outcome(), None);
    }
}
