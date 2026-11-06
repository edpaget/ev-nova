//! Where the on-screen keyboard's keys go: five rows in a grid of
//! [`COLUMNS`] columns, inside the safe area of the host's canvas, clear
//! of the host's dialog where they fit, every size in points.
//!
//! - **The rows**: the digits 1-0; Q-P; A-L; Shift, Z-M, `-`, `'` and
//!   `.`; and Space, Clear, Delete and Done, the wide keys.
//! - **The region** is the safe area the host was told
//!   ([`Surroundings::safe_area`]) within its canvas, inset by
//!   [`MARGIN_PT`], as a touch aid's is (`touch_aids::place`).
//! - **The size**: a key is [`MIN_PT`] tall and the region's width
//!   shared across the columns, at most [`KEY_MOST_PT`] across, with
//!   [`GAP_PT`] between keys. Where the region is too small for keys of
//!   [`MIN_PT`] (an iPad's Slide Over, where only a pad would show it, an
//!   iPad having a soft keyboard), they shrink to fit it.
//! - **The place**: centred across the region, at its bottom, or at its
//!   top when the bottom one would overlap the host's dialog, or at the
//!   bottom anyway when neither is clear: the first place clear of the
//!   dialog, as `touch_aids::place` places an aid.

use crate::geometry::{Bounds, Point, intersection};
use crate::input::Surroundings;
use crate::ui::touch_aids::place::{MARGIN_PT, MIN_PT};

/// A key of the on-screen keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyId {
    /// Types its character: a digit, a lower-case letter (upper case
    /// while Shift is on), `-`, `'` or `.`.
    Char(char),
    /// Turns the capitals on for the next letter, or off.
    Shift,
    /// Types a space.
    Space,
    /// Deletes the whole field.
    Clear,
    /// Deletes the character before the caret, or the selection.
    Delete,
    /// Presses the dialog's default, as Return does.
    Done,
}

impl KeyId {
    /// The key's label: its character, upper case while `shift` is on for
    /// a letter, or its name.
    #[must_use]
    pub fn label(self, shift: bool) -> String {
        match self {
            Self::Char(c) if shift => c.to_uppercase().collect(),
            Self::Char(c) => c.to_string(),
            Self::Shift => "Shift".to_owned(),
            Self::Space => "Space".to_owned(),
            Self::Clear => "Clear".to_owned(),
            Self::Delete => "Delete".to_owned(),
            Self::Done => "Done".to_owned(),
        }
    }
}

/// The columns of the grid: the widest row's keys.
pub const COLUMNS: usize = 11;
/// The rows.
pub const ROWS: usize = 5;
/// The gap between two keys, in points.
pub const GAP_PT: f32 = 2.0;
/// The most a one-column key is across, in points, so the keyboard
/// doesn't stretch across a large window.
pub const KEY_MOST_PT: f32 = 60.0;

/// One row: how many columns in it starts, and each key with how many
/// columns it spans.
struct Row {
    indent: f32,
    keys: &'static [(KeyId, usize)],
}

const fn ch(c: char) -> (KeyId, usize) {
    (KeyId::Char(c), 1)
}

/// The rows, top to bottom.
const LAYOUT: [Row; ROWS] = [
    Row {
        indent: 0.5,
        keys: &[
            ch('1'),
            ch('2'),
            ch('3'),
            ch('4'),
            ch('5'),
            ch('6'),
            ch('7'),
            ch('8'),
            ch('9'),
            ch('0'),
        ],
    },
    Row {
        indent: 0.5,
        keys: &[
            ch('q'),
            ch('w'),
            ch('e'),
            ch('r'),
            ch('t'),
            ch('y'),
            ch('u'),
            ch('i'),
            ch('o'),
            ch('p'),
        ],
    },
    Row {
        indent: 1.0,
        keys: &[
            ch('a'),
            ch('s'),
            ch('d'),
            ch('f'),
            ch('g'),
            ch('h'),
            ch('j'),
            ch('k'),
            ch('l'),
        ],
    },
    Row {
        indent: 0.0,
        keys: &[
            (KeyId::Shift, 1),
            ch('z'),
            ch('x'),
            ch('c'),
            ch('v'),
            ch('b'),
            ch('n'),
            ch('m'),
            ch('-'),
            ch('\''),
            ch('.'),
        ],
    },
    Row {
        indent: 0.0,
        keys: &[
            (KeyId::Space, 5),
            (KeyId::Clear, 2),
            (KeyId::Delete, 2),
            (KeyId::Done, 2),
        ],
    },
];

/// The key the pad's focus starts on.
pub const INITIAL: KeyId = KeyId::Char('1');

/// The keys where they are, in reading order, and the keyboard's bounds.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    keys: Vec<(KeyId, Bounds)>,
    bounds: Bounds,
}

impl Layout {
    /// The keyboard laid out by the rules above for a host told `told`,
    /// drawing in `canvas` with its dialog at `dialog`.
    #[must_use]
    pub fn new(told: &Surroundings, canvas: Bounds, dialog: Bounds) -> Self {
        let ppp = told.points_per_pixel;
        let margin = MARGIN_PT / ppp;
        let region = intersection(told.safe_area, canvas)
            .unwrap_or(canvas)
            .inset(margin, margin);
        #[allow(clippy::cast_precision_loss)] // A handful of columns and rows.
        let (columns, rows) = (COLUMNS as f32, ROWS as f32);
        let across_pt =
            ((region.width() * ppp - (columns - 1.0) * GAP_PT) / columns).clamp(0.0, KEY_MOST_PT);
        let down_pt = ((region.height() * ppp - (rows - 1.0) * GAP_PT) / rows).clamp(0.0, MIN_PT);
        let (unit, tall, gap) = (across_pt / ppp, down_pt / ppp, GAP_PT / ppp);
        let width = columns * unit + (columns - 1.0) * gap;
        let height = rows * tall + (rows - 1.0) * gap;
        let left = region.center().x - width / 2.0;
        let at_top = |top: f32| Bounds {
            min: Point::new(left, top),
            max: Point::new(left + width, top + height),
        };
        let bottom = Bounds {
            min: Point::new(left, region.max.y - height),
            max: Point::new(left + width, region.max.y),
        };
        let clear = |bounds: Bounds| intersection(bounds, dialog).is_none();
        let bounds = [bottom, at_top(region.min.y)]
            .into_iter()
            .find(|&bounds| clear(bounds))
            .unwrap_or(bottom);
        let mut keys = Vec::new();
        let mut top = bounds.min.y;
        for row in &LAYOUT {
            let mut column = row.indent;
            for &(id, span) in row.keys {
                #[allow(clippy::cast_precision_loss)] // A span of a few columns.
                let span = span as f32;
                let x = left + column * (unit + gap);
                keys.push((
                    id,
                    Bounds {
                        min: Point::new(x, top),
                        max: Point::new(x + span * unit + (span - 1.0) * gap, top + tall),
                    },
                ));
                column += span;
            }
            top += tall + gap;
        }
        Self { keys, bounds }
    }

    /// Each key with its rect, in reading order.
    #[must_use]
    pub fn keys(&self) -> &[(KeyId, Bounds)] {
        &self.keys
    }

    /// The rects of the keys, in reading order.
    #[must_use]
    pub fn rects(&self) -> Vec<Bounds> {
        self.keys.iter().map(|&(_, rect)| rect).collect()
    }

    /// Where `key` is, if it is a key.
    #[must_use]
    pub fn key(&self, key: KeyId) -> Option<Bounds> {
        self.keys
            .iter()
            .find_map(|&(id, rect)| (id == key).then_some(rect))
    }

    /// The whole keyboard's bounds.
    #[must_use]
    pub const fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// The pad's inputs that type `keys` in turn, the pad's focus on
    /// `from`: for each, the shortest run of d-pad taps the keyboard's
    /// picker takes to it, then South's press and release. For tests that
    /// drive the keyboard from pad inputs alone.
    ///
    /// # Panics
    ///
    /// When a key isn't reachable, which no key of the layout is.
    #[cfg(any(test, feature = "fixture"))]
    #[must_use]
    pub fn pad_presses(&self, from: KeyId, keys: &[KeyId]) -> Vec<crate::input::Input> {
        use crate::input::{Input, PadButton};
        use crate::ui::pad_focus::{Direction, FocusMove, Target, step};
        let targets: Vec<_> = self
            .keys
            .iter()
            .map(|&(id, rect)| Target::at(id, rect))
            .collect();
        let dpad = [
            (Direction::Up, PadButton::DPadUp),
            (Direction::Down, PadButton::DPadDown),
            (Direction::Left, PadButton::DPadLeft),
            (Direction::Right, PadButton::DPadRight),
        ];
        let tap = |button| [true, false].map(|pressed| Input::PadButton { button, pressed });
        let mut inputs = Vec::new();
        let mut at = from;
        for &key in keys {
            let mut paths = std::collections::VecDeque::from([(at, Vec::new())]);
            let mut seen = vec![at];
            let path = loop {
                let (here, path) = paths.pop_front().expect("every key is reachable");
                if here == key {
                    break path;
                }
                for (direction, button) in dpad {
                    let next = step(&targets, Some(here), INITIAL, FocusMove::Toward(direction))
                        .unwrap_or(here);
                    if !seen.contains(&next) {
                        seen.push(next);
                        let mut longer = path.clone();
                        longer.push(button);
                        paths.push_back((next, longer));
                    }
                }
            };
            inputs.extend(path.into_iter().flat_map(tap));
            inputs.extend(tap(PadButton::South));
            at = key;
        }
        inputs
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::ui::touch_aids::testkit::{WINDOWS, Window};

    const EPS: f32 = 1e-3;

    fn canvas() -> Bounds {
        let (w, h) = Window::classic_view();
        Bounds::at(Point::new(0.0, 0.0), w, h)
    }

    /// New Pilot's size, centred in the canvas.
    fn centred() -> Bounds {
        let c = canvas().center();
        Bounds::at(Point::new(c.x - 163.0, c.y - 106.5), 326.0, 213.0)
    }

    fn region(told: &Surroundings) -> Bounds {
        let margin = MARGIN_PT / told.points_per_pixel;
        intersection(told.safe_area, canvas())
            .expect("a region")
            .inset(margin, margin)
    }

    fn inside(rect: Bounds, region: Bounds) -> bool {
        rect.min.x >= region.min.x - EPS
            && rect.min.y >= region.min.y - EPS
            && rect.max.x <= region.max.x + EPS
            && rect.max.y <= region.max.y + EPS
    }

    /// Slide Over's classic canvas is 240 pt each way: too small for five
    /// rows or eleven columns of 44 pt keys.
    fn has_room(window: &Window) -> bool {
        window.name != "Slide Over"
    }

    #[test]
    fn every_key_is_at_least_44_points_each_way_where_the_safe_area_has_room() {
        for window in WINDOWS.iter().filter(|window| has_room(window)) {
            let told = window.classic_surroundings();
            let ppp = told.points_per_pixel;
            for (key, rect) in Layout::new(&told, canvas(), centred()).keys() {
                assert!(
                    rect.height() * ppp >= MIN_PT - EPS,
                    "{}: {key:?} {rect:?}",
                    window.name
                );
                assert!(
                    rect.width() * ppp >= MIN_PT - EPS,
                    "{}: {key:?} {rect:?}",
                    window.name
                );
            }
        }
    }

    #[test]
    fn every_key_lies_inside_the_safe_area_and_the_canvas() {
        for window in &WINDOWS {
            let told = window.classic_surroundings();
            let region = region(&told);
            let layout = Layout::new(&told, canvas(), centred());
            assert!(inside(layout.bounds(), region), "{}", window.name);
            for (key, rect) in layout.keys() {
                assert!(
                    inside(*rect, region),
                    "{}: {key:?} {rect:?} in {region:?}",
                    window.name
                );
                assert!(inside(*rect, layout.bounds()), "{}: {key:?}", window.name);
            }
        }
    }

    #[test]
    fn no_two_keys_overlap() {
        for window in &WINDOWS {
            let told = window.classic_surroundings();
            let layout = Layout::new(&told, canvas(), centred());
            for (i, (a, ra)) in layout.keys().iter().enumerate() {
                for (b, rb) in &layout.keys()[i + 1..] {
                    assert!(
                        intersection(*ra, *rb).is_none(),
                        "{}: {a:?} {b:?}",
                        window.name
                    );
                }
            }
        }
    }

    #[test]
    fn the_keyboard_sits_below_the_dialog_or_above_it_when_below_overlaps() {
        for window in WINDOWS.iter().filter(|window| has_room(window)) {
            let told = window.classic_surroundings();
            let region = region(&told);
            let below = Layout::new(&told, canvas(), centred()).bounds();
            if intersection(below, centred()).is_none() {
                assert_eq!(below.max.y, region.max.y, "{}: at the bottom", window.name);
            }
        }
        let told = WINDOWS[0].classic_surroundings();
        let region = region(&told);
        let low = Bounds::at(Point::new(212.0, 500.0), 600.0, 200.0);
        let above = Layout::new(&told, canvas(), low).bounds();
        assert_eq!(above.min.y, region.min.y, "at the top");
        assert!(intersection(above, low).is_none());
        let everywhere = canvas();
        let anyway = Layout::new(&told, canvas(), everywhere).bounds();
        assert_eq!(anyway.max.y, region.max.y, "at the bottom anyway");
        let middle = Layout::new(&told, canvas(), centred()).bounds();
        assert!(
            (middle.center().x - region.center().x).abs() < EPS,
            "centred across"
        );
    }

    #[test]
    fn every_character_a_name_needs_has_a_key() {
        let layout = Layout::new(&Surroundings::default(), canvas(), centred());
        let mut needed: Vec<char> = ('a'..='z').chain('0'..='9').collect();
        needed.extend(['-', '\'', '.']);
        for c in needed {
            assert!(layout.key(KeyId::Char(c)).is_some(), "{c:?}");
        }
        for key in [
            KeyId::Shift,
            KeyId::Space,
            KeyId::Clear,
            KeyId::Delete,
            KeyId::Done,
        ] {
            assert!(layout.key(key).is_some(), "{key:?}");
        }
        assert_eq!(layout.key(KeyId::Char('A')), None, "upper case by Shift");
    }

    #[test]
    fn the_rows_run_top_to_bottom_in_reading_order() {
        let layout = Layout::new(&Surroundings::default(), canvas(), centred());
        let at = |c| layout.key(KeyId::Char(c)).expect("a key");
        assert!(at('1').max.y < at('q').min.y);
        assert!(at('q').max.y < at('a').min.y);
        assert!(at('a').max.y < at('z').min.y);
        let space = layout.key(KeyId::Space).expect("Space");
        assert!(at('z').max.y < space.min.y);
        assert!(at('q').max.x < at('w').min.x);
        let done = layout.key(KeyId::Done).expect("Done");
        assert!(space.width() > done.width(), "Space is the widest");
        assert_eq!(layout.keys().first().map(|&(id, _)| id), Some(INITIAL));
    }

    #[test]
    fn labels_are_the_characters_upper_case_under_shift_or_the_names() {
        assert_eq!(KeyId::Char('q').label(false), "q");
        assert_eq!(KeyId::Char('q').label(true), "Q");
        assert_eq!(KeyId::Char('-').label(true), "-");
        assert_eq!(KeyId::Done.label(true), "Done");
        assert_eq!(KeyId::Shift.label(false), "Shift");
        assert_eq!(KeyId::Space.label(false), "Space");
        assert_eq!(KeyId::Clear.label(false), "Clear");
        assert_eq!(KeyId::Delete.label(false), "Delete");
    }
}
