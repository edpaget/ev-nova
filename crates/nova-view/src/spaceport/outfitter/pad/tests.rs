use super::super::tests::{
    alt_click, click, counting_screen, field_text, long, quantity_item, rack_alert, screen,
    screen_of,
};
use super::super::{
    BUY_ITEM, DONE_ITEM, GRID_ITEM, OutfitterScreen, SCROLL_DOWN_ITEM, SCROLL_UP_ITEM, SELL_ITEM,
};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, PadButton, Surroundings};
use crate::screen::Screen;
use crate::ui::dialog::{Dialog, FocusRing};
use crate::ui::pad_focus::every_screen::PadProbe;
use nova_sim::OutfitRefusal;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

struct Probe(OutfitterScreen);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        const CELLS: [&str; 4] = ["cell 0", "cell 1", "cell 2", "cell 3"];
        let dialog = self.0.dialog().expect("laid out");
        let bounds = |item| dialog.item_bounds(item).expect("an item");
        let mut controls: Vec<_> = self
            .0
            .pad_cells()
            .into_iter()
            .map(|(index, rect)| (CELLS[index], rect))
            .collect();
        controls.extend([
            ("Done", bounds(DONE_ITEM)),
            ("Buy", bounds(BUY_ITEM)),
            ("Sell", bounds(SELL_ITEM)),
            ("up", bounds(SCROLL_UP_ITEM)),
            ("down", bounds(SCROLL_DOWN_ITEM)),
        ]);
        controls
    }

    fn outcome(&mut self) -> String {
        format!(
            "selected {:?} top {} order {:?} closed {} asking {}",
            self.0.selected(),
            self.0.top_row(),
            self.0.take_order(),
            self.0.closed(),
            self.0.quantity().is_some()
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

/// The outfitter's row: four listed cells, the rest of the grid empty.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(screen()))
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

fn tap(screen: &mut OutfitterScreen, button: PadButton) {
    screen.input(&pad(button, true));
    screen.input(&pad(button, false));
}

fn dialog(screen: &OutfitterScreen) -> &Dialog {
    screen.dialog().expect("laid out")
}

/// Taps the right shoulder until the highlight is on cell `cell`.
fn shoulder_to_cell(screen: &mut OutfitterScreen, cell: usize) {
    for _ in 0..20 {
        if dialog(screen).pad_highlight_cell() == Some(cell) {
            return;
        }
        tap(screen, PadButton::RightShoulder);
    }
    panic!("the shoulder never reaches cell {cell}");
}

/// Taps the right shoulder until the highlight is on item `item`.
fn shoulder_to(screen: &mut OutfitterScreen, item: usize) {
    for _ in 0..20 {
        if dialog(screen).pad_highlight() == Some(item) {
            return;
        }
        tap(screen, PadButton::RightShoulder);
    }
    panic!("the shoulder never reaches item {item}");
}

#[test]
fn south_on_a_cell_selects_it_as_a_click_does() {
    let mut padded = screen();
    padded.input(&told(PAD));
    shoulder_to_cell(&mut padded, 2);
    padded.input(&pad(PadButton::South, true));
    assert_eq!(padded.selected(), Some(2), "on the press, as a click's");
    padded.input(&pad(PadButton::South, false));
    let mut clicked = screen();
    let cell = clicked.pad_cells()[2].1;
    click(&mut clicked, cell.center());
    assert_eq!(padded.selected(), clicked.selected());
    assert_eq!(padded.take_order(), clicked.take_order());
}

#[test]
fn empty_cells_are_no_targets() {
    let screen = screen();
    let cells = dialog(&screen)
        .pad_spots()
        .iter()
        .filter(|target| target.id.item().is_none())
        .count();
    assert_eq!(cells, 4, "the four listed, none of the empty");
    assert!(
        !dialog(&screen)
            .pad_spots()
            .iter()
            .any(|target| target.id.item() == Some(GRID_ITEM)),
        "nor the grid's item"
    );
}

#[test]
fn a_scroll_by_the_left_stick_moves_the_cells() {
    let mut screen = screen_of(long(30));
    screen.input(&told(PAD));
    let shown = |screen: &OutfitterScreen| -> Vec<(usize, Bounds)> { screen.pad_cells() };
    let before = shown(&screen);
    for _ in 0..30 {
        tap(&mut screen, PadButton::LeftStickDown);
    }
    assert!(screen.top_row() > 0, "scrolled");
    let after = shown(&screen);
    assert_ne!(after[0].0, before[0].0, "other outfits shown");
    let given: Vec<Bounds> = dialog(&screen)
        .pad_spots()
        .iter()
        .filter(|target| target.id.item().is_none())
        .map(|target| target.bounds)
        .collect();
    let cells: Vec<Bounds> = after.into_iter().map(|(_, rect)| rect).collect();
    assert_eq!(given, cells, "the dialog's cells are the grid's now");
}

#[test]
fn the_alert_and_the_quantity_dialog_open_told() {
    let mut screen = counting_screen();
    screen.input(&told(PAD));
    alt_click(&mut screen, BUY_ITEM);
    let asking = screen.quantity().expect("asking");
    assert_eq!(asking.dialog().device(), PAD);
    let cancel = quantity_item(&screen, 4);
    click(&mut screen, cancel);
    screen.alert(&rack_alert(OutfitRefusal::NegativeFreeMass, false));
    let alert = screen.alert_dialog().expect("the alert");
    assert_eq!(alert.dialog().device(), PAD);
}

#[test]
fn a_pad_alone_opens_the_quantity_dialog_with_west_held_and_buys_its_count() {
    let mut padded = counting_screen();
    padded.input(&told(PAD));
    shoulder_to(&mut padded, BUY_ITEM);
    padded.input(&pad(PadButton::West, true));
    tap(&mut padded, PadButton::South);
    padded.input(&pad(PadButton::West, false));
    let mut keyed = counting_screen();
    alt_click(&mut keyed, BUY_ITEM);
    assert!(
        padded.quantity().is_some(),
        "West held asks, as Option does"
    );
    assert_eq!(field_text(&padded), field_text(&keyed), "the same count");
    tap(&mut padded, PadButton::South);
    let ok = quantity_item(&keyed, 1);
    click(&mut keyed, ok);
    let bought = keyed.take_order();
    assert!(bought.is_some_and(|(_, count)| count > 1), "{bought:?}");
    assert_eq!(padded.take_order(), bought, "OK, as a click on it");
}
