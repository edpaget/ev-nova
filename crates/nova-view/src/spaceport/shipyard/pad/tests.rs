use std::time::Duration;

use super::super::tests::{click, click_item, key, naming_by_key, screen};
use super::super::{
    BUY_ITEM, DONE_ITEM, INFO_ITEM, PANEL_DONE_ITEM, SCROLL_DOWN_ITEM, SCROLL_UP_ITEM, ShipNaming,
    ShipyardScreen,
};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, Key, PadButton, Surroundings};
use crate::screen::Screen;
use crate::ui::dialog::{Dialog, FocusRing};
use crate::ui::pad_focus::every_screen::PadProbe;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

const CELLS: [&str; 4] = ["cell 0", "cell 1", "cell 2", "cell 3"];

struct Probe(ShipyardScreen);

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
            ("Buy Ship", bounds(BUY_ITEM)),
            ("Info", bounds(INFO_ITEM)),
            ("Done", bounds(DONE_ITEM)),
            ("up", bounds(SCROLL_UP_ITEM)),
            ("down", bounds(SCROLL_DOWN_ITEM)),
        ]);
        controls
    }

    fn outcome(&mut self) -> String {
        format!(
            "selected {:?} top {} request {:?} closed {} info {}",
            self.0.selected(),
            self.0.top_row(),
            self.0.take_request(),
            self.0.closed(),
            self.0.info_panel().is_some()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Done")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.0 = buying().with_focus_ring(ring);
        true
    }
}

/// The shipyard on the Heavy Shuttle, which it can buy, so Buy Ship is
/// enabled.
fn buying() -> ShipyardScreen {
    let mut screen = screen();
    screen.input(&key(Key::ArrowRight, true, false));
    screen.input(&key(Key::ArrowRight, false, false));
    screen
}

/// The shipyard's row: four listed cells, the rest of the grid empty.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(buying()))
}

struct InfoProbe(ShipyardScreen);

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
            "info {} closed {}",
            self.0.info_panel().is_some(),
            self.0.closed()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Done")
    }
}

/// The shipyard with its info panel open on the selected ship.
fn with_info() -> ShipyardScreen {
    let mut screen = screen();
    click_item(&mut screen, INFO_ITEM);
    assert!(screen.info_panel().is_some(), "Info opens the panel");
    screen
}

/// The info panel's row.
pub(crate) fn info_pad_probe() -> Box<dyn PadProbe> {
    Box::new(InfoProbe(with_info()))
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

fn tap(screen: &mut ShipyardScreen, button: PadButton) {
    screen.input(&pad(button, true));
    screen.input(&pad(button, false));
}

fn dialog(screen: &ShipyardScreen) -> &Dialog {
    screen.dialog().expect("laid out")
}

/// Taps the right shoulder until the highlight is on cell `cell`.
fn shoulder_to_cell(screen: &mut ShipyardScreen, cell: usize) {
    for _ in 0..20 {
        if dialog(screen).pad_highlight_cell() == Some(cell) {
            return;
        }
        tap(screen, PadButton::RightShoulder);
    }
    panic!("the shoulder never reaches cell {cell}");
}

#[test]
fn two_souths_on_one_cell_in_time_open_info_as_a_double_click_does() {
    let mut padded = screen();
    padded.input(&told(PAD));
    shoulder_to_cell(&mut padded, 2);
    tap(&mut padded, PadButton::South);
    assert_eq!(padded.selected(), Some(2));
    assert!(padded.info_panel().is_none(), "one press");
    padded.tick(Duration::from_millis(100));
    tap(&mut padded, PadButton::South);
    assert!(padded.info_panel().is_some(), "two in time");
    let mut slow = screen();
    slow.input(&told(PAD));
    shoulder_to_cell(&mut slow, 2);
    tap(&mut slow, PadButton::South);
    slow.tick(Duration::from_secs(1));
    tap(&mut slow, PadButton::South);
    assert!(slow.info_panel().is_none(), "a second apart");
    let mut clicked = screen();
    let cell = clicked.pad_cells()[2].1;
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
    assert_eq!(panel.pad_highlight(), Some(PANEL_DONE_ITEM), "on its Done");
    padded.input(&pad(PadButton::South, true));
    assert!(padded.info_panel().is_some(), "Done armed, not yet pressed");
    padded.input(&pad(PadButton::South, false));
    assert!(padded.info_panel().is_none(), "Done, on South's release");
    let mut keyed = with_info();
    keyed.input(&told(PAD));
    keyed.input(&key(Key::Space, true, false));
    assert!(keyed.info_panel().is_none(), "Space is Done at once");
}

#[test]
fn a_pad_south_on_the_shipyard_presses_the_focused_control_not_info() {
    let mut padded = screen();
    padded.input(&told(PAD));
    assert_eq!(dialog(&padded).pad_highlight(), Some(DONE_ITEM));
    tap(&mut padded, PadButton::South);
    assert!(padded.closed(), "Done, the focused control");
    assert!(padded.info_panel().is_none());
    let mut keyed = screen();
    keyed.input(&key(Key::Space, true, false));
    assert!(keyed.info_panel().is_some(), "Space still opens Info");
}

#[test]
fn the_info_panel_and_the_naming_open_told() {
    let mut screen = screen();
    screen.input(&told(PAD));
    click_item(&mut screen, INFO_ITEM);
    assert_eq!(screen.info_panel().map(Dialog::device), Some(PAD));
    let mut naming = naming_by_key();
    assert_eq!(
        naming.naming().map(|prompt| prompt.dialog().device()),
        Some(Device::Keyboard)
    );
    naming.input(&told(PAD));
    assert_eq!(
        naming.naming().map(|prompt| prompt.dialog().device()),
        Some(PAD),
        "an open prompt follows"
    );
    let mut told_first = self::screen();
    told_first.input(&told(PAD));
    told_first.open_naming(&ShipNaming {
        ship: nova_sim::ShipId(129),
        prompt: "Name it: ".to_owned(),
        default: "Ship".to_owned(),
    });
    assert_eq!(
        told_first.naming().map(|prompt| prompt.dialog().device()),
        Some(PAD),
        "and one opened after"
    );
}
