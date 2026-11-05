use std::time::Duration;

use super::super::tests::{click, click_item, screen};
use super::super::{BUY_ITEM, DONE_ITEM, HireScreen, INFO_ITEM, SCROLL_DOWN_ITEM, SCROLL_UP_ITEM};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, Key, PadButton, Surroundings};
use crate::screen::Screen;
use crate::spaceport::shipyard::PANEL_DONE_ITEM;
use crate::ui::dialog::{Dialog, FocusRing};
use crate::ui::pad_focus::every_screen::PadProbe;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

const CELLS: [&str; 2] = ["cell 0", "cell 1"];

struct Probe(HireScreen);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        let dialog = self.0.dialog().expect("laid out");
        let bounds = |item| dialog.item_bounds(item).expect("an item");
        let mut controls: Vec<_> = self
            .0
            .pad_cells()
            .into_iter()
            .map(|(index, rect)| (CELLS[index], rect))
            .collect();
        controls.extend([
            ("Hire Escort", bounds(BUY_ITEM)),
            ("Info", bounds(INFO_ITEM)),
            ("Done", bounds(DONE_ITEM)),
            ("up", bounds(SCROLL_UP_ITEM)),
            ("down", bounds(SCROLL_DOWN_ITEM)),
        ]);
        controls
    }

    fn outcome(&mut self) -> String {
        format!(
            "selected {:?} top {} order {:?} closed {} info {}",
            self.0.selected(),
            self.0.top_row(),
            self.0.take_order(),
            self.0.closed(),
            self.0.info_panel().is_some()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Done")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.0 = screen().with_focus_ring(ring);
        true
    }
}

/// The hire screen's row: two listed cells, the rest of the grid empty.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(screen()))
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

fn tap(screen: &mut HireScreen, button: PadButton) {
    screen.input(&pad(button, true));
    screen.input(&pad(button, false));
}

/// Taps the right shoulder until the highlight is on cell `cell`.
fn shoulder_to_cell(screen: &mut HireScreen, cell: usize) {
    for _ in 0..20 {
        if screen.dialog().and_then(Dialog::pad_highlight_cell) == Some(cell) {
            return;
        }
        tap(screen, PadButton::RightShoulder);
    }
    panic!("the shoulder never reaches cell {cell}");
}

/// The hire screen with its info panel open on the selected escort.
fn with_info() -> HireScreen {
    let mut screen = screen();
    click_item(&mut screen, INFO_ITEM);
    assert!(screen.info_panel().is_some(), "Info opens the panel");
    screen
}

struct InfoProbe(HireScreen);

impl PadProbe for InfoProbe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        let panel = self.0.info_panel().expect("open");
        vec![(
            "Done",
            panel.item_bounds(PANEL_DONE_ITEM).expect("its Done"),
        )]
    }

    fn outcome(&mut self) -> String {
        format!(
            "info {} order {:?} closed {}",
            self.0.info_panel().is_some(),
            self.0.take_order(),
            self.0.closed()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Done")
    }
}

/// The info panel's row.
pub(crate) fn info_pad_probe() -> Box<dyn PadProbe> {
    Box::new(InfoProbe(with_info()))
}

#[test]
fn two_souths_on_one_cell_in_time_open_info_as_a_double_click_does() {
    let mut padded = screen();
    padded.input(&told(PAD));
    shoulder_to_cell(&mut padded, 1);
    tap(&mut padded, PadButton::South);
    assert_eq!(padded.selected(), Some(1));
    assert!(padded.info_panel().is_none(), "one press");
    padded.tick(Duration::from_millis(100));
    tap(&mut padded, PadButton::South);
    assert!(padded.info_panel().is_some(), "two in time");
    let mut clicked = screen();
    let cell = clicked.pad_cells()[1].1;
    click(&mut clicked, cell.center());
    clicked.tick(Duration::from_millis(100));
    click(&mut clicked, cell.center());
    assert!(clicked.info_panel().is_some(), "as a double-click");
}

#[test]
fn on_the_info_panel_a_pad_south_presses_the_focused_control_and_keyboard_space_is_still_done() {
    let mut padded = with_info();
    padded.input(&told(PAD));
    let panel = padded.info_panel().expect("open");
    assert_eq!(panel.pad_highlight(), Some(PANEL_DONE_ITEM));
    padded.input(&pad(PadButton::South, true));
    assert!(padded.info_panel().is_some(), "armed");
    padded.input(&pad(PadButton::South, false));
    assert!(padded.info_panel().is_none(), "Done");
    let mut keyed = with_info();
    keyed.input(&Input::Key {
        key: Key::Space,
        pressed: true,
        repeat: false,
    });
    assert!(keyed.info_panel().is_none(), "Space is Done");
}

#[test]
fn the_info_panel_opens_told() {
    let mut screen = screen();
    screen.input(&told(PAD));
    click_item(&mut screen, INFO_ITEM);
    assert_eq!(screen.info_panel().map(Dialog::device), Some(PAD));
    let mut open = with_info();
    open.input(&told(PAD));
    assert_eq!(
        open.info_panel().map(Dialog::device),
        Some(PAD),
        "an open panel follows"
    );
}
