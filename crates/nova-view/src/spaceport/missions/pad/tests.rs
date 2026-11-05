use super::super::tests::{info_screen, info_screen_of, many_info, many_offers, screen, screen_of};
use super::super::{
    ABORT_ITEM, ACCEPT_ITEM, INFO_DONE_ITEM, LEAVE_ITEM, MissionComputerScreen, MissionInfoScreen,
};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, Key, MouseButton, PadButton, Surroundings};
use crate::screen::Screen;
use crate::ui::dialog::{Dialog, FocusRing};
use crate::ui::pad_focus::every_screen::PadProbe;
use crate::ui::row_list::{ListCell, RowList};

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

/// A name for each cell, for the table's messages.
fn cell_name(cell: ListCell) -> &'static str {
    const ROWS: [&str; 16] = [
        "row 0", "row 1", "row 2", "row 3", "row 4", "row 5", "row 6", "row 7", "row 8", "row 9",
        "row 10", "row 11", "row 12", "row 13", "row 14", "row 15",
    ];
    match cell {
        ListCell::Row(row) => ROWS[row],
        ListCell::Up => "the bar's up arrow",
        ListCell::Down => "the bar's down arrow",
    }
}

fn controls(
    dialog: &Dialog,
    rows: &RowList,
    buttons: [(&'static str, usize); 2],
) -> Vec<(&'static str, Bounds)> {
    let mut controls: Vec<_> = rows
        .pad_cells()
        .into_iter()
        .map(|(cell, rect)| (cell_name(cell), rect))
        .collect();
    controls.extend(buttons.map(|(name, item)| (name, dialog.item_bounds(item).expect("an item"))));
    controls
}

struct Computer(MissionComputerScreen);

impl PadProbe for Computer {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        controls(
            self.0.dialog().expect("laid out"),
            self.0.rows().expect("laid out"),
            [("Accept", ACCEPT_ITEM), ("Leave", LEAVE_ITEM)],
        )
    }

    fn outcome(&mut self) -> String {
        let rows = self.0.rows().expect("laid out");
        format!(
            "selected {:?} first {} accepted {:?} closed {} text {:?}",
            rows.selected(),
            rows.first(),
            self.0.take_accept(),
            self.0.closed(),
            self.0.take_text_request()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Leave")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.0 = long_computer().with_focus_ring(ring);
        true
    }
}

struct Info(MissionInfoScreen);

impl PadProbe for Info {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        controls(
            self.0.dialog().expect("laid out"),
            self.0.rows().expect("laid out"),
            [("Done", INFO_DONE_ITEM), ("Abort", ABORT_ITEM)],
        )
    }

    fn outcome(&mut self) -> String {
        let rows = self.0.rows().expect("laid out");
        format!(
            "selected {:?} first {} abort {:?} closed {} text {:?}",
            rows.selected(),
            rows.first(),
            self.0.take_abort(),
            self.0.closed(),
            self.0.take_text_request()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Done")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.0 = long_info().with_focus_ring(ring);
        true
    }
}

/// Fourteen offers in the computer's twelve rows: its bar shows.
fn long_computer() -> MissionComputerScreen {
    screen_of(many_offers(14))
}

/// Nine missions in the info screen's seven rows: its bar shows.
fn long_info() -> MissionInfoScreen {
    info_screen_of(many_info(9))
}

/// The mission computer's row in the pad's every-screen table.
pub(crate) fn computer_pad_probe() -> Box<dyn PadProbe> {
    Box::new(Computer(long_computer()))
}

/// The mission info screen's row in the pad's every-screen table, its
/// opening sound already taken, as it is no press's.
pub(crate) fn info_pad_probe() -> Box<dyn PadProbe> {
    let mut info = long_info();
    info.take_sounds();
    Box::new(Info(info))
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn tap(screen: &mut impl Screen, button: PadButton) {
    screen.input(&Input::PadButton {
        button,
        pressed: true,
    });
    screen.input(&Input::PadButton {
        button,
        pressed: false,
    });
}

fn click(screen: &mut impl Screen, rect: Bounds) {
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: rect.center(),
        });
    }
}

fn cell_at(screen: &MissionComputerScreen) -> Option<usize> {
    screen.dialog().and_then(Dialog::pad_highlight_cell)
}

#[test]
fn south_on_a_row_selects_and_shows_it_as_a_click_does() {
    let mut padded = screen();
    padded.input(&told(PAD));
    padded.take_text_request();
    for _ in 0..10 {
        if cell_at(&padded) == Some(1) {
            break;
        }
        tap(&mut padded, PadButton::DPadUp);
        tap(&mut padded, PadButton::DPadLeft);
    }
    assert_eq!(cell_at(&padded), Some(1), "row 1");
    tap(&mut padded, PadButton::South);
    let mut clicked = screen();
    clicked.take_text_request();
    let row = clicked
        .rows()
        .and_then(|rows| rows.row_bounds(1))
        .expect("row 1");
    click(&mut clicked, row);
    assert_eq!(padded.rows().and_then(RowList::selected), Some(1));
    assert_eq!(padded.take_text_request(), clicked.take_text_request());
    assert_eq!(
        padded.dialog().and_then(Dialog::pad_highlight_cell),
        Some(1),
        "the focus stays on the row"
    );
}

#[test]
fn the_left_stick_scrolls_and_the_cells_follow() {
    let mut screen = long_computer();
    screen.input(&told(PAD));
    for _ in 0..12 {
        tap(&mut screen, PadButton::LeftStickDown);
    }
    let rows = screen.rows().expect("laid out");
    assert!(rows.first() > 0, "scrolled");
    let first = rows.pad_cells()[0];
    assert_eq!(first.0, ListCell::Row(rows.first()));
    let targets = screen.dialog().expect("laid out").pad_spots();
    let cell = targets
        .iter()
        .find(|target| target.id.item().is_none())
        .expect("a cell");
    assert_eq!(cell.bounds, first.1, "the cells moved with the list");
}

#[test]
fn the_bar_arrows_are_targets_only_while_the_bar_shows() {
    let arrows = |screen: &MissionComputerScreen| {
        screen
            .rows()
            .expect("laid out")
            .pad_cells()
            .into_iter()
            .filter(|(cell, _)| matches!(cell, ListCell::Up | ListCell::Down))
            .count()
    };
    let cells = |screen: &MissionComputerScreen| {
        screen
            .dialog()
            .expect("laid out")
            .pad_spots()
            .iter()
            .filter(|target| target.id.item().is_none())
            .count()
    };
    let short = screen();
    assert_eq!(arrows(&short), 0);
    assert_eq!(cells(&short), 2, "two rows, no arrows");
    let long = long_computer();
    assert_eq!(arrows(&long), 2);
    assert_eq!(cells(&long), 12 + 2, "the rows shown and the arrows");
    let info = long_info();
    assert_eq!(
        info.rows().expect("laid out").pad_cells().len(),
        7 + 2,
        "the info screen's bar inside its list"
    );
    assert_eq!(info_screen().rows().expect("laid out").pad_cells().len(), 2);
}

#[test]
fn the_left_shoulder_steps_the_focus_back_under_either_ring() {
    for ring in [FocusRing::Tab, FocusRing::Off] {
        let mut screen = screen().with_focus_ring(ring);
        screen.input(&told(PAD));
        let selected = screen.rows().and_then(RowList::selected);
        assert_eq!(
            screen.dialog().and_then(Dialog::pad_highlight),
            Some(LEAVE_ITEM)
        );
        tap(&mut screen, PadButton::LeftShoulder);
        let dialog = screen.dialog().expect("laid out");
        assert!(
            dialog.pad_highlight() != Some(LEAVE_ITEM) || dialog.pad_highlight_cell().is_some(),
            "{ring:?}: moved back"
        );
        assert_eq!(
            screen.rows().and_then(RowList::selected),
            selected,
            "{ring:?}: the list didn't step"
        );
    }
}

#[test]
fn under_focus_ring_off_the_right_shoulder_steps_the_list_as_tab_does() {
    let mut padded = screen().with_focus_ring(FocusRing::Off);
    padded.input(&told(PAD));
    tap(&mut padded, PadButton::RightShoulder);
    let mut keyed = screen().with_focus_ring(FocusRing::Off);
    keyed.input(&Input::Key {
        key: Key::Tab,
        pressed: true,
        repeat: false,
    });
    assert_eq!(
        padded.rows().and_then(RowList::selected),
        keyed.rows().and_then(RowList::selected)
    );
    assert_eq!(padded.rows().and_then(RowList::selected), Some(1));
}
