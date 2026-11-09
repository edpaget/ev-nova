//! The new game's dialogs: placeholder `DLOG`s and their items, standing
//! in for the interface file the bible doesn't author, as `ïntf` 128
//! stands in for the status bar ([`super::records::default_interface`]).
//!
//! [`default_dialogs`] holds one placeholder for each dialog the new game
//! reads that the view has no built-in layout for. Each has no art: no
//! item is a picture, and nothing names a `PICT`. Each fits the 640 x 480
//! view, centred by its positioning word ([`CENTRED`]), and is named as a
//! placeholder by its title ("Default spaceport", say). Items a dialog
//! doesn't use are parked below it, wholly outside, as the stock dialogs
//! park theirs.
//!
//! Which item of a dialog shows what is the view's rule, not this
//! crate's: the defaults give each dialog the items its screen reads, and
//! the app's wiring tests pin that each screen lays itself out over them.

use std::collections::BTreeMap;

use crate::records::dialog::Dlog;
use crate::records::dialog_items::{DialogItem, ItemKind};
use crate::wire::geometry::Rect;
use crate::wire::id::DitlId;
use crate::wire::string::MacString;

/// The positioning word every default carries: centred on the main
/// screen, as every stock `DLOG`'s word is.
pub const CENTRED: u16 = 0xA80A;

/// The view every default fits: the smallest the game draws.
pub const VIEW: (i16, i16) = (640, 480);

/// One placeholder dialog: its window and its items, item 1 first.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultDialog {
    /// The window, naming its own item list.
    pub dlog: Dlog,
    /// The items.
    pub items: Vec<DialogItem>,
}

/// The new game's placeholder dialogs, by `DLOG` ID.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DefaultDialogs {
    dialogs: BTreeMap<i16, DefaultDialog>,
}

impl DefaultDialogs {
    /// `DLOG` `id`'s placeholder, if there is one.
    #[must_use]
    pub fn get(&self, id: i16) -> Option<&DefaultDialog> {
        self.dialogs.get(&id)
    }

    /// Each placeholder with its `DLOG` ID, in ID order.
    pub fn iter(&self) -> impl Iterator<Item = (i16, &DefaultDialog)> {
        self.dialogs.iter().map(|(id, dialog)| (*id, dialog))
    }
}

impl FromIterator<(i16, DefaultDialog)> for DefaultDialogs {
    fn from_iter<I: IntoIterator<Item = (i16, DefaultDialog)>>(iter: I) -> Self {
        Self {
            dialogs: iter.into_iter().collect(),
        }
    }
}

/// The new game's placeholder dialogs, every one a placeholder until the
/// bible authors an interface:
///
/// | `DLOG` | Title | Size |
/// |---|---|---|
/// | 1000 | Default spaceport | 618 x 472 |
/// | 1001 | Default trade center | 426 x 252 |
/// | 1002 | Default outfitter | 640 x 321 |
/// | 1004 | Default shipyard | 640 x 323 |
/// | 1005 | Default ship info | 250 x 285 |
/// | 1006 | Default mission computer | 510 x 201 |
/// | 1012 | Default missions log | 471 x 155 |
/// | 1013 | Default bar | 263 x 185 |
/// | 1014 | Default news | 300 x 230 |
/// | 1016 | Default mission offer | 441 x 317 |
/// | 1017 | Default player info | 413 x 227 |
/// | 1023 | Default race | 470 x 230 |
/// | 3000 | Default note | 357 x 150 |
/// | 3002 | Default question | 355 x 109 |
/// | 3003 | Default text | 441 x 313 |
/// | 4003 | Default preferences | 336 x 278 |
#[must_use]
pub fn default_dialogs() -> DefaultDialogs {
    [
        spaceport(),
        trade_center(),
        outfitter(),
        shipyard(),
        ship_info(),
        mission_computer(),
        missions_log(),
        bar(),
        news(),
        mission_offer(),
        player_info(),
        race(),
        note(),
        question(),
        text_dialog(),
        preferences(),
    ]
    .into_iter()
    .collect()
}

/// `DLOG` `id`, titled `title`, `width` x `height` centred in [`VIEW`],
/// holding `items` (built for its height).
fn dialog(
    id: i16,
    title: &str,
    (width, height): (i16, i16),
    items: Vec<DialogItem>,
) -> (i16, DefaultDialog) {
    let left = (VIEW.0 - width) / 2;
    let top = (VIEW.1 - height) / 2;
    (
        id,
        DefaultDialog {
            dlog: Dlog {
                bounds: Rect {
                    top,
                    left,
                    bottom: top + height,
                    right: left + width,
                },
                proc_id: 1,
                visible: true,
                go_away: false,
                ref_con: 0,
                items_id: DitlId(id),
                title: MacString::from(title),
                position: Some(CENTRED),
            },
            items,
        },
    )
}

/// An unused item, parked below the dialog: at the foot of the view,
/// which no dialog reaches below.
fn parked() -> DialogItem {
    item(at(0, VIEW.1, 10, 10), false, ItemKind::User)
}

/// The box at (`x`, `y`), `w` x `h`, in a dialog's own coordinates.
fn at(x: i16, y: i16, w: i16, h: i16) -> Rect {
    Rect {
        top: y,
        left: x,
        bottom: y + h,
        right: x + w,
    }
}

fn item(bounds: Rect, enabled: bool, kind: ItemKind) -> DialogItem {
    DialogItem {
        bounds,
        enabled,
        kind,
    }
}

/// An application-drawn area clicks report.
fn user(x: i16, y: i16, w: i16, h: i16) -> DialogItem {
    item(at(x, y, w, h), true, ItemKind::User)
}

/// An application-drawn area clicks don't report.
fn area(x: i16, y: i16, w: i16, h: i16) -> DialogItem {
    item(at(x, y, w, h), false, ItemKind::User)
}

/// A push button titled `title`.
fn button(x: i16, y: i16, w: i16, h: i16, title: &str) -> DialogItem {
    let title = MacString::from(title);
    item(at(x, y, w, h), true, ItemKind::Button { title })
}

/// A check box titled `title`.
fn check(x: i16, y: i16, w: i16, h: i16, title: &str) -> DialogItem {
    let title = MacString::from(title);
    item(at(x, y, w, h), true, ItemKind::CheckBox { title })
}

/// Static text, `text` until the dialog sets its own.
fn static_text(x: i16, y: i16, w: i16, h: i16, text: &str) -> DialogItem {
    let text = MacString::from(text);
    item(at(x, y, w, h), false, ItemKind::StaticText { text })
}

/// The spaceport: the landscape across the top (5), the name (3) and the
/// description (6) under it, and a column of buttons down each side: Bar
/// (11), Mission BBS (10) and Trade Center (7) on the left, Shipyard (9),
/// Outfitter (8), Recharge (4) and Leave (12) on the right.
fn spaceport() -> (i16, DefaultDialog) {
    dialog(
        1000,
        "Default spaceport",
        (618, 472),
        vec![
            parked(),
            parked(),
            area(159, 252, 303, 18),
            user(471, 370, 145, 25),
            user(3, 3, 612, 240),
            area(160, 282, 301, 185),
            user(3, 370, 145, 25),
            user(471, 329, 145, 25),
            user(471, 288, 145, 25),
            user(3, 329, 145, 25),
            user(3, 288, 145, 25),
            user(471, 411, 145, 25),
        ],
    )
}

/// The Trade Center: Done (1), the header (3), eight rows (4-11), the
/// summary (12), Buy (13), Sell (14) and the event line (15).
fn trade_center() -> (i16, DefaultDialog) {
    dialog(
        1001,
        "Default trade center",
        (426, 252),
        vec![
            user(272, 221, 99, 25),
            parked(),
            area(38, 9, 352, 16),
            user(38, 25, 352, 13),
            user(38, 38, 352, 13),
            user(38, 51, 352, 13),
            user(38, 64, 352, 13),
            user(38, 77, 352, 13),
            user(38, 90, 352, 13),
            user(38, 103, 352, 13),
            user(38, 116, 352, 13),
            area(38, 132, 352, 56),
            user(60, 221, 99, 25),
            user(166, 221, 99, 25),
            area(41, 192, 346, 24),
        ],
    )
}

/// The outfitter: the grid (5) and its arrows (10, 11) on the left, the
/// description (6) over the picture (8) and the info box (9) on the
/// right, and Buy (7), Sell (4) and Done (1) along the bottom.
fn outfitter() -> (i16, DefaultDialog) {
    dialog(
        1002,
        "Default outfitter",
        (640, 321),
        vec![
            user(528, 289, 99, 25),
            parked(),
            parked(),
            user(422, 289, 99, 25),
            user(9, 8, 333, 271),
            area(352, 8, 279, 140),
            user(316, 289, 99, 25),
            area(352, 156, 130, 123),
            area(490, 156, 141, 123),
            user(148, 288, 25, 25),
            user(178, 288, 25, 25),
        ],
    )
}

/// The shipyard: as the outfitter, with Info (10), Buy Ship (1) and Done
/// (7) along the bottom and the grid's arrows at 12 and 13.
fn shipyard() -> (i16, DefaultDialog) {
    dialog(
        1004,
        "Default shipyard",
        (640, 323),
        vec![
            user(422, 289, 99, 25),
            parked(),
            parked(),
            parked(),
            user(9, 8, 333, 271),
            area(352, 8, 279, 140),
            user(528, 289, 99, 25),
            area(352, 156, 130, 123),
            area(490, 156, 141, 123),
            user(316, 289, 99, 25),
            parked(),
            user(141, 288, 25, 25),
            user(171, 288, 25, 25),
        ],
    )
}

/// The shipyard's info panel: Done (1), the title (3) and the text (5).
fn ship_info() -> (i16, DefaultDialog) {
    dialog(
        1005,
        "Default ship info",
        (250, 285),
        vec![
            user(86, 253, 74, 25),
            parked(),
            area(3, 3, 240, 24),
            parked(),
            area(9, 32, 234, 214),
        ],
    )
}

/// The mission computer: Accept (1), the list (2) and its bar (3), the
/// text (4), the mission's name (5), Leave (7), the heading (8) and the
/// date (11).
fn mission_computer() -> (i16, DefaultDialog) {
    dialog(
        1006,
        "Default mission computer",
        (510, 201),
        vec![
            user(266, 170, 99, 25),
            user(10, 30, 195, 144),
            user(205, 30, 15, 144),
            area(233, 60, 267, 93),
            area(233, 34, 269, 21),
            parked(),
            user(368, 170, 99, 25),
            area(14, 3, 290, 15),
            parked(),
            parked(),
            area(311, 3, 187, 15),
        ],
    )
}

/// The missions log: Done (1), the list (2), the heading (3), the text
/// (4), Abort (5) and the date (7).
fn missions_log() -> (i16, DefaultDialog) {
    dialog(
        1012,
        "Default missions log",
        (471, 155),
        vec![
            user(290, 125, 99, 25),
            user(9, 24, 195, 84),
            user(13, 4, 193, 14),
            area(218, 26, 242, 91),
            user(57, 125, 99, 25),
            parked(),
            area(343, 4, 122, 14),
        ],
    )
}

/// The bar: Leave (1), Gamble (2), Holovid (3), Hire Escort (5) and the
/// text (7).
fn bar() -> (i16, DefaultDialog) {
    dialog(
        1013,
        "Default bar",
        (263, 185),
        vec![
            area(156, 154, 99, 26),
            area(156, 125, 99, 26),
            area(6, 154, 146, 26),
            parked(),
            area(6, 125, 146, 26),
            parked(),
            area(16, 10, 230, 106),
        ],
    )
}

/// The news: the window alone, which the news draws itself.
fn news() -> (i16, DefaultDialog) {
    dialog(1014, "Default news", (300, 230), Vec::new())
}

/// A mission's offer: Accept (1), Refuse (2), the text (3), OK (6) and
/// the text's arrows (9, 10).
fn mission_offer() -> (i16, DefaultDialog) {
    dialog(
        1016,
        "Default mission offer",
        (441, 317),
        vec![
            area(225, 285, 99, 25),
            area(117, 285, 99, 25),
            area(12, 9, 415, 267),
            parked(),
            parked(),
            area(173, 285, 99, 25),
            parked(),
            parked(),
            area(340, 287, 23, 23),
            area(373, 287, 23, 23),
        ],
    )
}

/// The player info: Done (1), four tabs (2-5), the content (6) and
/// Jettison (7).
fn player_info() -> (i16, DefaultDialog) {
    dialog(
        1017,
        "Default player info",
        (413, 227),
        vec![
            user(293, 195, 99, 25),
            user(7, 8, 99, 25),
            user(107, 8, 99, 25),
            user(207, 8, 99, 25),
            user(307, 8, 99, 25),
            area(4, 40, 405, 141),
            user(60, 195, 150, 25),
        ],
    )
}

/// The race: Leave (1), Help (2) and the four racers (3-6).
fn race() -> (i16, DefaultDialog) {
    dialog(
        1023,
        "Default race",
        (470, 230),
        vec![
            area(243, 196, 99, 25),
            area(129, 196, 99, 25),
            area(13, 88, 100, 100),
            area(128, 88, 100, 100),
            area(243, 88, 100, 100),
            area(358, 88, 100, 100),
        ],
    )
}

/// A note: OK (1) and the text (3), the icon's place (4) left empty.
fn note() -> (i16, DefaultDialog) {
    dialog(
        3000,
        "Default note",
        (357, 150),
        vec![
            button(282, 118, 70, 20, "OK"),
            parked(),
            static_text(52, 5, 295, 99, ""),
            area(7, 5, 32, 32),
        ],
    )
}

/// A yes-or-no question: OK (1), the question (3), the icon's place (4)
/// left empty, and Cancel (5).
fn question() -> (i16, DefaultDialog) {
    dialog(
        3002,
        "Default question",
        (355, 109),
        vec![
            button(263, 75, 70, 20, "OK"),
            parked(),
            static_text(52, 5, 295, 50, ""),
            area(7, 5, 32, 32),
            button(181, 75, 70, 20, "Cancel"),
        ],
    )
}

/// A text: Done (1), the text (3) and its arrows (5, 6).
fn text_dialog() -> (i16, DefaultDialog) {
    dialog(
        3003,
        "Default text",
        (441, 313),
        vec![
            user(173, 281, 99, 25),
            parked(),
            area(11, 10, 417, 262),
            parked(),
            area(330, 282, 23, 23),
            area(363, 282, 23, 23),
        ],
    )
}

/// The preferences: OK (1) and Key Settings (16); the effects toggles
/// down the left (12-15, 10, 18); the music (8), the Scaling list (2),
/// the display picker (11), QuickTime Movies (9), the sound (20) and
/// Hyperspace Effects (21) down the right; and the sound volume's label
/// (4), value (5) and arrows (7 up, 6 down) under them, the music
/// volume's place (3) below that.
fn preferences() -> (i16, DefaultDialog) {
    dialog(
        4003,
        "Default preferences",
        (336, 278),
        vec![
            button(225, 245, 70, 20, "OK"),
            check(171, 55, 154, 18, "Scaling"),
            area(69, 213, 245, 17),
            static_text(171, 167, 106, 16, "Sound Volume:"),
            static_text(189, 186, 122, 16, ""),
            user(172, 194, 11, 9),
            user(172, 185, 11, 9),
            check(171, 33, 99, 18, "Music"),
            check(171, 99, 136, 18, "QuickTime Movies"),
            check(11, 121, 154, 18, "Smoke Trails"),
            check(171, 77, 154, 18, "Display"),
            check(11, 33, 154, 18, "Ship Animations"),
            check(11, 55, 154, 18, "Engine Glows"),
            check(11, 77, 154, 18, "Running Lights"),
            check(11, 99, 154, 18, "Weapon Effects"),
            button(49, 245, 135, 20, "Key Settings"),
            parked(),
            check(11, 143, 154, 18, "Parallax Starfield"),
            parked(),
            check(171, 121, 136, 18, "Sound"),
            check(171, 143, 154, 18, "Hyperspace Effects"),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(item: Rect, width: i16, height: i16) -> bool {
        item.left >= 0 && item.top >= 0 && item.right <= width && item.bottom <= height
    }

    fn outside(item: Rect, width: i16, height: i16) -> bool {
        item.right <= 0 || item.bottom <= 0 || item.left >= width || item.top >= height
    }

    fn size(dlog: &Dlog) -> (i16, i16) {
        (
            dlog.bounds.right - dlog.bounds.left,
            dlog.bounds.bottom - dlog.bounds.top,
        )
    }

    #[test]
    fn the_defaults_are_exactly_the_dialogs_the_view_has_no_layout_for() {
        let ids: Vec<i16> = default_dialogs().iter().map(|(id, _)| id).collect();
        let mut expected = vec![
            1000, 1001, 1002, 1004, 1005, 3003, 1013, 1014, 1023, 1006, 1012, 1016, 1017, 3002,
            3000, 4003,
        ];
        expected.sort_unstable();
        assert_eq!(ids, expected);
    }

    #[test]
    fn every_default_fits_the_view_and_is_centred() {
        for (id, dialog) in default_dialogs().iter() {
            let bounds = dialog.dlog.bounds;
            let (width, height) = size(&dialog.dlog);
            assert!(width > 0 && height > 0, "{id}: {bounds:?}");
            assert!(inside(bounds, VIEW.0, VIEW.1), "{id}: {bounds:?}");
            assert_eq!(dialog.dlog.position, Some(CENTRED), "{id}");
        }
    }

    #[test]
    fn every_default_sits_in_the_middle_of_the_view() {
        for (id, dialog) in default_dialogs().iter() {
            let r = dialog.dlog.bounds;
            let across = (r.left - (VIEW.0 - r.right)).abs();
            let down = (r.top - (VIEW.1 - r.bottom)).abs();
            assert!(across <= 1 && down <= 1, "{id}: {r:?}");
        }
    }

    /// Items that share a place because a dialog shows one or the other:
    /// the offer's lone button (6) stands where Accept (1) and Refuse (2)
    /// do, for an offer that can't be refused.
    const ALTERNATIVES: [(i16, usize, usize); 2] = [(1016, 1, 6), (1016, 2, 6)];

    #[test]
    fn no_two_items_a_dialog_shows_overlap() {
        for (id, dialog) in default_dialogs().iter() {
            let (width, height) = size(&dialog.dlog);
            let shown: Vec<(usize, Rect)> = (1..)
                .zip(dialog.items.iter().map(|item| item.bounds))
                .filter(|(_, r)| inside(*r, width, height))
                .collect();
            for (i, (n, a)) in shown.iter().enumerate() {
                for (m, b) in &shown[i + 1..] {
                    let apart = a.bottom <= b.top
                        || b.bottom <= a.top
                        || a.right <= b.left
                        || b.right <= a.left;
                    let alternatives = ALTERNATIVES.contains(&(id, *n, *m));
                    assert!(apart || alternatives, "{id}: items {n} and {m} overlap");
                }
            }
        }
    }

    #[test]
    fn every_item_is_wholly_inside_its_dialog_or_parked_wholly_outside() {
        for (id, dialog) in default_dialogs().iter() {
            let (width, height) = size(&dialog.dlog);
            for (index, item) in dialog.items.iter().enumerate() {
                let r = item.bounds;
                assert!(r.top < r.bottom && r.left < r.right, "{id} #{}", index + 1);
                assert!(
                    inside(r, width, height) || outside(r, width, height),
                    "{id} item {} straddles the edge: {r:?}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn no_default_holds_a_picture() {
        for (id, dialog) in default_dialogs().iter() {
            for (index, item) in dialog.items.iter().enumerate() {
                assert!(
                    !matches!(item.kind, ItemKind::Picture { .. }),
                    "{id} item {}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn each_default_names_its_own_item_list() {
        for (id, dialog) in default_dialogs().iter() {
            assert_eq!(dialog.dlog.items_id, DitlId(id));
        }
    }

    #[test]
    fn each_default_is_titled_as_a_placeholder() {
        for (id, dialog) in default_dialogs().iter() {
            let title = dialog.dlog.title.as_str();
            assert!(title.starts_with("Default "), "{id}: {title:?}");
        }
        let port = default_dialogs();
        let port = port.get(1000).expect("the spaceport");
        assert_eq!(port.dlog.title, MacString::from("Default spaceport"));
    }

    #[test]
    fn a_built_set_holds_what_it_was_built_from() {
        let dialog = DefaultDialog {
            dlog: Dlog {
                bounds: Rect::default(),
                proc_id: 0,
                visible: true,
                go_away: false,
                ref_con: 0,
                items_id: DitlId(7),
                title: MacString::from("Default test"),
                position: None,
            },
            items: Vec::new(),
        };
        let set: DefaultDialogs = [(7, dialog.clone())].into_iter().collect();
        assert_eq!(set.get(7), Some(&dialog));
        assert_eq!(set.get(8), None);
        assert_eq!(set.iter().count(), 1);
    }
}
