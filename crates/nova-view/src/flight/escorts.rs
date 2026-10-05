//! The escort menu: which of the player's escorts a command goes to, and
//! how the menu is drawn over the flight (`_ShowEscortMenu` @0x5664a and
//! `_DrawEscortMenuInFrame` @0x56268 in the `EV Nova` executable).
//!
//! - E ([`MENU_KEY`]) opens the menu, selecting All Ships; with no escorts
//!   it stays shut and the flight says so ([`NO_ESCORTS`]). E again, or
//!   Return, closes it.
//! - While it is open, keys 1-5 ([`GROUP_KEYS`]) select All Ships,
//!   Fighters, Medium Ships, Warships or Freighters; a class the player
//!   has no escort of is not selected. While it is shut they do nothing.
//! - It closes [`MENU_TIMEOUT`] after it opened, or after the last
//!   selection or command it took ([`EscortMenu::touch`]), and as soon as
//!   the fleet is empty. The original then fades it out over 32 frames;
//!   here it closes at once.
//! - A command goes to the group selected, or to every escort while the
//!   menu is shut ([`EscortMenu::group`]), and the flight says what it
//!   changed ([`escort_command_message`]): Return to Hangar
//!   (Option-C) "returning to hangar.", or "returning to formation." when
//!   an escort that is no fighter went back to formation.
//! - Fighters abandoned as the player jumps are told in brackets
//!   ([`fighters_abandoned_message`]).
//!
//! **Drawing** ([`EscortMenu::draw`]): a [`MENU_WIDTH`] x [`MENU_HEIGHT`]
//! frame at [`MENU_AT`], black inside, bordered in `cölr` `FloatingMap`,
//! titled "Escort Commands" (`STR#` 2002 #133) centred at the top; five
//! rows [`ROW_GAP`] apart, "1) All Ships" to "5) Freighters", white, or
//! dark grey for a class the player has no escort of; each class it has
//! shows its order right-aligned ("Formation" for none); and the selected
//! row sits on a bar of `cölr` `EscortHilite`. The colours come through
//! the [`EscortMenuLooks`] port, with the stock ones as a fallback.

use std::rc::Rc;
use std::time::Duration;

use nova_sim::bay::{ABANDONED_MANY, ABANDONED_ONE};
use nova_sim::escort::{ESCORT_COMMANDS, NEW_ORDERS, WILL_ATTACK, order_label};
use nova_sim::{ClassRow, Commanded, EscortCommand, EscortGroup};

use crate::color::Color;
use crate::draw::{DrawList, fill_rect};
use crate::font::Font;
use crate::geometry::{Bounds, Point};
use crate::input::Key;
use crate::text::TextMetrics;

pub use nova_sim::escort::NO_ESCORTS;

/// The escort menu key: the original's default (`Keys.nib`'s
/// `escortMenuKey`).
pub const MENU_KEY: Key = Key::Char('e');
/// The keys that select All Ships, Fighters, Medium Ships, Warships and
/// Freighters while the menu is open (hard-coded, @0xce722).
pub const GROUP_KEYS: [Key; 5] = [
    Key::Char('1'),
    Key::Char('2'),
    Key::Char('3'),
    Key::Char('4'),
    Key::Char('5'),
];
/// How long the menu stays open after it opens, or after the last
/// selection or command: 480 ticks of 60 Hz.
pub const MENU_TIMEOUT: Duration = Duration::from_secs(8);
/// The frame's top-left corner, in the window.
pub const MENU_AT: Point = Point::new(15.0, 150.0);
/// The frame's width.
pub const MENU_WIDTH: f32 = 160.0;
/// The frame's height.
pub const MENU_HEIGHT: f32 = 120.0;
/// How far apart the rows are.
pub const ROW_GAP: f32 = 18.0;
/// The menu's text size.
pub const MENU_TEXT_SIZE: f32 = 10.0;
/// The title's baseline, below the frame's top.
const TITLE_BASELINE: f32 = 15.0;
/// The first row's baseline, below the frame's top.
const ROW_BASELINE: f32 = 40.0;
/// How far in from the frame's sides the rows' text stands.
const TEXT_INSET: f32 = 7.0;
/// How far in from the frame's sides the highlight bar reaches.
const BAR_INSET: f32 = 4.0;
/// The highlight bar's top and bottom, below the frame's top for the
/// first row.
const BAR_TOP: f32 = 30.0;
const BAR_BOTTOM: f32 = 43.0;
/// A class row the player has no escort of: dark grey.
pub const ABSENT_ROW: Color = Color::from_rgb24(0x0060_6060);

/// The escort menu's colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscortMenuColors {
    /// The frame's border: `cölr` `FloatingMap`.
    pub border: Color,
    /// The selected row's bar: `cölr` `EscortHilite`.
    pub hilite: Color,
}

impl EscortMenuColors {
    /// Stock `cölr` 128's: a near-black border and a dark red bar.
    pub const STOCK: Self = Self {
        border: Color::from_rgb24(0x000C_0C0C),
        hilite: Color::from_rgb24(0x0080_0000),
    };
}

/// The escort menu's look.
pub trait EscortMenuLooks {
    /// The menu's colours, from `cölr` 128, or [`EscortMenuColors::STOCK`]
    /// when it is missing or does not decode.
    fn escort_menu_colors(&self) -> EscortMenuColors;
}

/// A borrowed catalog is a catalog.
impl<T: EscortMenuLooks + ?Sized> EscortMenuLooks for &T {
    fn escort_menu_colors(&self) -> EscortMenuColors {
        (**self).escort_menu_colors()
    }
}

/// A shared catalog is a catalog.
impl<T: EscortMenuLooks + ?Sized> EscortMenuLooks for Rc<T> {
    fn escort_menu_colors(&self) -> EscortMenuColors {
        (**self).escort_menu_colors()
    }
}

/// What E did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    /// The menu opened, All Ships selected.
    Opened,
    /// The menu closed.
    Closed,
    /// The player has no escorts: the menu stays shut.
    NoEscorts,
}

/// The escort menu: shut, or open with a group selected (see the module
/// docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EscortMenu {
    /// While open: the group selected, and when the menu last took a
    /// selection or command.
    open: Option<(EscortGroup, Duration)>,
}

/// Whether `rows` show any escort.
fn any_escort(rows: &[ClassRow; 4]) -> bool {
    rows.iter().any(|row| row.present)
}

impl EscortMenu {
    /// Whether the menu is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The group a command goes to: the one selected, or every escort
    /// while the menu is shut.
    #[must_use]
    pub fn group(&self) -> EscortGroup {
        self.open.map_or(EscortGroup::All, |(group, _)| group)
    }

    /// E, at `now`, the fleet's classes `rows`: opens the menu on All
    /// Ships, or closes it.
    pub fn toggle(&mut self, now: Duration, rows: &[ClassRow; 4]) -> Toggled {
        if self.open.take().is_some() {
            return Toggled::Closed;
        }
        if !any_escort(rows) {
            return Toggled::NoEscorts;
        }
        self.open = Some((EscortGroup::All, now));
        Toggled::Opened
    }

    /// Key `n` of [`GROUP_KEYS`] (from 0), at `now`: while the menu is
    /// open, selects its group when the player has an escort of it (All
    /// Ships always), and says whether it did.
    pub fn select(&mut self, n: usize, now: Duration, rows: &[ClassRow; 4]) -> bool {
        let Some(group) = EscortGroup::ALL.get(n).copied() else {
            return false;
        };
        let present = match group {
            EscortGroup::All => true,
            EscortGroup::Class(class) => rows.iter().any(|row| row.class == class && row.present),
        };
        match &mut self.open {
            Some(open) if present => {
                *open = (group, now);
                true
            }
            _ => false,
        }
    }

    /// A command the menu took at `now`: it stays open
    /// [`MENU_TIMEOUT`] from now.
    pub fn touch(&mut self, now: Duration) {
        if let Some((_, since)) = &mut self.open {
            *since = now;
        }
    }

    /// Closes the menu.
    pub fn close(&mut self) {
        self.open = None;
    }

    /// Closes the menu at `now` once [`MENU_TIMEOUT`] has passed since it
    /// last took a selection or command, or when `rows` show no escort.
    pub fn update(&mut self, now: Duration, rows: &[ClassRow; 4]) {
        if let Some((_, since)) = self.open
            && (now >= since + MENU_TIMEOUT || !any_escort(rows))
        {
            self.open = None;
        }
    }

    /// Draws the open menu (see the module docs) with the fleet's classes
    /// `rows`, in `colors`, its text measured by `metrics` (without them,
    /// the title starts at the frame's left and an order at the right
    /// inset).
    pub fn draw(
        &self,
        list: &mut DrawList,
        rows: &[ClassRow; 4],
        colors: EscortMenuColors,
        metrics: Option<&dyn TextMetrics>,
    ) {
        let Some((selected, _)) = self.open else {
            return;
        };
        let frame = Bounds::at(MENU_AT, MENU_WIDTH, MENU_HEIGHT);
        let (left, top, right, bottom) = (frame.min.x, frame.min.y, frame.max.x, frame.max.y);
        fill_rect(list, frame, Color::BLACK);
        let corners = [
            Point::new(left, top),
            Point::new(right, top),
            Point::new(right, bottom),
            Point::new(left, bottom),
        ];
        for (at, &from) in corners.iter().enumerate() {
            list.line(from, corners[(at + 1) % 4], 1.0, colors.border);
        }
        let width = |text: &str| {
            metrics.map_or(0.0, |metrics| {
                metrics.width(Font::Geneva, MENU_TEXT_SIZE, text)
            })
        };
        let title_x = if metrics.is_some() {
            left + (MENU_WIDTH - width(ESCORT_COMMANDS)) / 2.0
        } else {
            left
        };
        list.text(
            ESCORT_COMMANDS,
            Point::new(title_x, top + TITLE_BASELINE - MENU_TEXT_SIZE),
            MENU_TEXT_SIZE,
            None,
            Color::WHITE,
        );
        for (n, group) in EscortGroup::ALL.into_iter().enumerate() {
            let step = ROW_GAP * n as f32;
            if group == selected {
                let bar = Bounds {
                    min: Point::new(left + BAR_INSET, top + BAR_TOP + step),
                    max: Point::new(right - BAR_INSET, top + BAR_BOTTOM + step),
                };
                fill_rect(list, bar, colors.hilite);
            }
            let row = match group {
                EscortGroup::All => None,
                EscortGroup::Class(class) => rows.iter().find(|row| row.class == class),
            };
            let present = row.is_none_or(|row| row.present);
            let color = if present { Color::WHITE } else { ABSENT_ROW };
            let text_top = top + ROW_BASELINE + step - MENU_TEXT_SIZE;
            list.text(
                format!("{}) {}", n + 1, group.menu_label()),
                Point::new(left + TEXT_INSET, text_top),
                MENU_TEXT_SIZE,
                None,
                color,
            );
            if let Some(row) = row.filter(|row| row.present) {
                let label = order_label(row.order);
                list.text(
                    label,
                    Point::new(right - TEXT_INSET - width(label), text_top),
                    MENU_TEXT_SIZE,
                    None,
                    Color::WHITE,
                );
            }
        }
    }
}

/// What the flight says of a command that changed something
/// (`_IssueNewEscortCommand` @0x660d5): "New escort orders assigned:  "
/// (two spaces), the group's name, a space, and what its escorts now do;
/// Attack with no target copied says "will attack.".
#[must_use]
pub fn escort_command_message(commanded: &Commanded) -> String {
    let doing = if commanded.command == EscortCommand::Attack && !commanded.targeted {
        WILL_ATTACK
    } else {
        commanded.command.doing()
    };
    format!("{NEW_ORDERS}{} {doing}", commanded.group.message_form())
}

/// `STR#` 137 #29-#38 of `Nova-DF.rsrc`: the numbers one to ten in
/// words, which `_StrcatOrdinalNumber` @0x8ab4 writes counts in.
pub const NUMBER_WORDS: [&str; 10] = [
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
];

/// What the flight says of `count` fighters abandoned as the player
/// jumped (`_HandlePlayer` @0x6c3c4-0x6c42b): in brackets, the count in
/// words up to ten, its first letter capitalised, and in digits above
/// that, then `STR#` 2002 #164 or #165. The original adds it to its
/// arrival line, which the flight does not show yet.
#[must_use]
pub fn fighters_abandoned_message(count: u32) -> String {
    let words = usize::try_from(count)
        .ok()
        .and_then(|n| n.checked_sub(1))
        .and_then(|index| NUMBER_WORDS.get(index));
    let number = words.map_or_else(
        || count.to_string(),
        |word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        },
    );
    let noun = if count == 1 {
        ABANDONED_ONE
    } else {
        ABANDONED_MANY
    };
    format!("({number} {noun})")
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::draw::DrawCommand;
    use nova_sim::{EscortClass, EscortOrder};

    /// The fleet's rows: a warship escort holding, a freighter in
    /// formation, and no fighters or medium ships.
    fn rows() -> [ClassRow; 4] {
        [
            ClassRow {
                class: EscortClass::Fighter,
                present: false,
                order: None,
            },
            ClassRow {
                class: EscortClass::Medium,
                present: false,
                order: None,
            },
            ClassRow {
                class: EscortClass::Warship,
                present: true,
                order: Some(EscortOrder::Hold),
            },
            ClassRow {
                class: EscortClass::Freighter,
                present: true,
                order: None,
            },
        ]
    }

    /// No escorts at all.
    fn none() -> [ClassRow; 4] {
        rows().map(|row| ClassRow {
            present: false,
            ..row
        })
    }

    const AT: Duration = Duration::from_secs(100);

    #[test]
    fn e_opens_the_menu_on_all_ships_and_closes_it() {
        let mut menu = EscortMenu::default();
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        assert_eq!(menu.toggle(AT, &rows()), Toggled::Opened);
        assert!(menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        assert_eq!(menu.toggle(AT, &rows()), Toggled::Closed);
        assert!(!menu.is_open());
        assert_eq!(MENU_KEY, Key::Char('e'));
    }

    #[test]
    fn with_no_escorts_e_does_not_open_it() {
        let mut menu = EscortMenu::default();
        assert_eq!(menu.toggle(AT, &none()), Toggled::NoEscorts);
        assert!(!menu.is_open());
        assert_eq!(NO_ESCORTS, "You don't have any escorts.");
    }

    #[test]
    fn keys_1_to_5_select_a_class_present_and_ignore_one_absent() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        assert!(menu.select(3, AT, &rows()), "4: warships");
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Warship));
        assert!(!menu.select(1, AT, &rows()), "2: no fighters");
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Warship));
        assert!(menu.select(4, AT, &rows()));
        assert_eq!(menu.group(), EscortGroup::Class(EscortClass::Freighter));
        assert!(!menu.select(2, AT, &rows()), "3: no medium ships");
        assert!(menu.select(0, AT, &rows()), "1: all, always");
        assert_eq!(menu.group(), EscortGroup::All);
        assert!(!menu.select(5, AT, &rows()), "no sixth key");
        assert_eq!(
            GROUP_KEYS,
            ['1', '2', '3', '4', '5'].map(Key::Char),
            "hard-coded"
        );
    }

    #[test]
    fn while_shut_keys_1_to_5_do_nothing() {
        let mut menu = EscortMenu::default();
        for n in 0..5 {
            assert!(!menu.select(n, AT, &rows()));
        }
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
    }

    #[test]
    fn reopening_selects_all_ships_again() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        menu.select(3, AT, &rows());
        menu.close();
        assert!(!menu.is_open());
        assert_eq!(menu.group(), EscortGroup::All);
        menu.toggle(AT, &rows());
        assert_eq!(menu.group(), EscortGroup::All);
    }

    #[test]
    fn it_closes_8_seconds_after_the_last_selection_or_command() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        let just_before = AT + MENU_TIMEOUT.saturating_sub(Duration::from_millis(1));
        menu.update(just_before, &rows());
        assert!(menu.is_open());
        menu.select(3, just_before, &rows());
        menu.update(AT + MENU_TIMEOUT, &rows());
        assert!(menu.is_open(), "the selection restarted it");
        menu.touch(just_before + Duration::from_secs(5));
        menu.update(just_before + MENU_TIMEOUT, &rows());
        assert!(menu.is_open(), "so did the command");
        menu.update(just_before + Duration::from_secs(5) + MENU_TIMEOUT, &rows());
        assert!(!menu.is_open());
        assert_eq!(MENU_TIMEOUT, Duration::from_secs(8));
        let mut shut = EscortMenu::default();
        shut.touch(AT);
        assert!(!shut.is_open(), "a command does not open it");
    }

    #[test]
    fn an_absent_class_keeps_its_selection_until_it_closes() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        assert!(!menu.select(1, AT, &rows()));
        menu.update(AT, &rows());
        assert!(menu.is_open());
    }

    #[test]
    fn it_closes_when_the_fleet_empties() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        menu.update(AT, &none());
        assert!(!menu.is_open());
    }

    /// Five units wide a character.
    struct Fixed;

    impl TextMetrics for Fixed {
        fn width(&self, _font: Font, _size: f32, text: &str) -> f32 {
            5.0 * text.chars().count() as f32
        }

        fn line_height(&self, _font: Font, size: f32) -> f32 {
            size
        }
    }

    const COLORS: EscortMenuColors = EscortMenuColors {
        border: Color::from_rgb24(0x0011_2233),
        hilite: Color::from_rgb24(0x0044_5566),
    };

    fn text(text: &str, x: f32, y: f32, color: Color) -> DrawCommand {
        DrawCommand::Text {
            text: text.to_owned(),
            font: Font::Geneva,
            origin: Point::new(x, y),
            size: MENU_TEXT_SIZE,
            wrap_width: None,
            color,
        }
    }

    fn line(from: (f32, f32), to: (f32, f32), width: f32, color: Color) -> DrawCommand {
        DrawCommand::Line {
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
            width,
            color,
        }
    }

    #[test]
    fn the_open_menu_draws_its_frame_title_rows_orders_and_bar() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        menu.select(3, AT, &rows());
        let mut list = DrawList::new();
        menu.draw(&mut list, &rows(), COLORS, Some(&Fixed));
        let white = Color::WHITE;
        let expected = vec![
            line((15.0, 210.0), (175.0, 210.0), 120.0, Color::BLACK),
            line((15.0, 150.0), (175.0, 150.0), 1.0, COLORS.border),
            line((175.0, 150.0), (175.0, 270.0), 1.0, COLORS.border),
            line((175.0, 270.0), (15.0, 270.0), 1.0, COLORS.border),
            line((15.0, 270.0), (15.0, 150.0), 1.0, COLORS.border),
            text("Escort Commands", 57.5, 155.0, white),
            text("1) All Ships", 22.0, 180.0, white),
            text("2) Fighters", 22.0, 198.0, ABSENT_ROW),
            text("3) Medium Ships", 22.0, 216.0, ABSENT_ROW),
            line((19.0, 240.5), (171.0, 240.5), 13.0, COLORS.hilite),
            text("4) Warships", 22.0, 234.0, white),
            text("Hold Position", 103.0, 234.0, white),
            text("5) Freighters", 22.0, 252.0, white),
            text("Formation", 123.0, 252.0, white),
        ];
        assert_eq!(list.iter().cloned().collect::<Vec<_>>(), expected);
        assert_eq!(
            (MENU_AT, MENU_WIDTH, MENU_HEIGHT, ROW_GAP),
            (Point::new(15.0, 150.0), 160.0, 120.0, 18.0)
        );
    }

    #[test]
    fn without_metrics_the_title_starts_at_the_left_and_the_orders_at_the_inset() {
        let mut menu = EscortMenu::default();
        menu.toggle(AT, &rows());
        let mut list = DrawList::new();
        menu.draw(&mut list, &rows(), COLORS, None);
        let texts: Vec<_> = list
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, origin, .. } => Some((text.clone(), origin.x)),
                _ => None,
            })
            .collect();
        assert_eq!(texts[0], ("Escort Commands".to_owned(), 15.0));
        assert_eq!(texts[5], ("Hold Position".to_owned(), 168.0));
        assert!(
            list.iter().any(|command| matches!(
                command,
                DrawCommand::Line { color, width, .. } if *color == COLORS.hilite && *width == 13.0
            )),
            "All Ships' bar"
        );
    }

    #[test]
    fn the_shut_menu_draws_nothing() {
        let mut list = DrawList::new();
        EscortMenu::default().draw(&mut list, &rows(), COLORS, Some(&Fixed));
        assert!(list.is_empty());
    }

    #[test]
    fn the_stock_colours_are_cölr_128s() {
        assert_eq!(EscortMenuColors::STOCK.border, Color::from_rgb24(789_516));
        assert_eq!(EscortMenuColors::STOCK.hilite, Color::from_rgb24(8_388_608));
    }

    fn commanded(group: EscortGroup, command: EscortCommand, targeted: bool) -> String {
        escort_command_message(&Commanded {
            group,
            command,
            targeted,
        })
    }

    #[test]
    fn a_command_is_told_as_the_original_tells_it() {
        assert_eq!(
            commanded(EscortGroup::All, EscortCommand::Attack, true),
            "New escort orders assigned:  All ships attacking target."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Warship),
                EscortCommand::Hold,
                false
            ),
            "New escort orders assigned:  Warships holding position."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Freighter),
                EscortCommand::Recall,
                false
            ),
            "New escort orders assigned:  Freighters returning to formation."
        );
        assert_eq!(
            commanded(EscortGroup::All, EscortCommand::Attack, false),
            "New escort orders assigned:  All ships will attack."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Medium),
                EscortCommand::Defend,
                false
            ),
            "New escort orders assigned:  Medium ships defending."
        );
        assert_eq!(
            commanded(EscortGroup::All, EscortCommand::Dock, false),
            "New escort orders assigned:  All ships returning to hangar."
        );
        assert_eq!(
            commanded(
                EscortGroup::Class(EscortClass::Fighter),
                EscortCommand::Dock,
                false
            ),
            "New escort orders assigned:  Fighters returning to hangar."
        );
    }

    #[test]
    fn fighters_abandoned_are_counted_in_words_up_to_ten() {
        assert_eq!(fighters_abandoned_message(1), "(One fighter abandoned)");
        assert_eq!(fighters_abandoned_message(2), "(Two fighters abandoned)");
        assert_eq!(fighters_abandoned_message(10), "(Ten fighters abandoned)");
        assert_eq!(fighters_abandoned_message(11), "(11 fighters abandoned)");
        assert_eq!(fighters_abandoned_message(0), "(0 fighters abandoned)");
        assert_eq!(
            NUMBER_WORDS,
            [
                "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten"
            ]
        );
        for (n, word) in (1..=10).zip(NUMBER_WORDS) {
            let message = fighters_abandoned_message(n);
            assert!(
                message.to_lowercase().starts_with(&format!("({word} ")),
                "{message}"
            );
        }
    }
}
