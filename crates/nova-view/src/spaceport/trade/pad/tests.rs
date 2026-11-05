use std::rc::Rc;

use nova_sim::Direction;

use super::super::tests::{
    alt_click, click, click_item, key, market, quantity, quantity_item, screen, template,
};
use super::super::{BUY_ITEM, DONE_ITEM, FIRST_ROW_ITEM, Lot, SELL_ITEM, TradeScreen, positions};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, Key, PadButton, Surroundings};
use crate::screen::Screen;
use crate::text::fixture::MonoMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::{Dialog, FocusRing};
use crate::ui::pad_focus::every_screen::PadProbe;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

struct Probe(TradeScreen);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        const SLOTS: [&str; 8] = [
            "slot 0", "slot 1", "slot 2", "slot 3", "slot 4", "slot 5", "slot 6", "slot 7",
        ];
        let dialog = self.0.dialog().expect("laid out");
        let bounds = |item| dialog.item_bounds(item).expect("an item");
        let mut controls = vec![
            ("Done", bounds(DONE_ITEM)),
            ("Buy", bounds(BUY_ITEM)),
            ("Sell", bounds(SELL_ITEM)),
        ];
        let filled = positions(&self.0.market)
            .into_iter()
            .take(SLOTS.len())
            .enumerate()
            .filter(|(_, row)| row.is_some())
            .map(|(slot, _)| (SLOTS[slot], bounds(FIRST_ROW_ITEM + slot)));
        controls.extend(filled);
        controls
    }

    fn outcome(&mut self) -> String {
        format!(
            "closed {} selected {:?} order {:?} asking {}",
            self.0.closed(),
            self.0.selected(),
            self.0.take_order(),
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

/// The trade center's row in the pad's every-screen table.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(screen()))
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn pad(button: PadButton, pressed: bool) -> Input {
    Input::PadButton { button, pressed }
}

fn tap(screen: &mut TradeScreen, button: PadButton) {
    screen.input(&pad(button, true));
    screen.input(&pad(button, false));
}

fn highlight(screen: &TradeScreen) -> Option<usize> {
    screen.dialog().and_then(Dialog::pad_highlight)
}

/// Taps the right shoulder until the pad's highlight is on `item`.
fn shoulder_to(screen: &mut TradeScreen, item: usize) {
    for _ in 0..20 {
        if highlight(screen) == Some(item) {
            return;
        }
        tap(screen, PadButton::RightShoulder);
    }
    panic!("the shoulder never reaches item {item}");
}

#[test]
fn the_quantity_dialog_opens_told() {
    let mut screen = screen();
    screen.input(&told(PAD));
    alt_click(&mut screen, BUY_ITEM);
    let asking = screen.quantity().expect("asking");
    assert_eq!(asking.dialog().device(), PAD);
    assert!(asking.dialog().pad_highlight().is_some(), "on OK at once");
}

#[test]
fn a_pad_alone_opens_the_quantity_dialog_with_west_held_and_buys_its_count() {
    let mut screen = screen();
    screen.input(&told(PAD));
    shoulder_to(&mut screen, BUY_ITEM);
    screen.input(&pad(PadButton::West, true));
    tap(&mut screen, PadButton::South);
    screen.input(&pad(PadButton::West, false));
    let asking = screen.quantity().expect("West held asks, as Option does");
    assert_eq!(asking.field().text(), "7", "min(1000 / 93, 7 free)");
    assert_eq!(screen.take_order(), None, "nothing yet");
    tap(&mut screen, PadButton::South);
    let mut keyed = self::screen();
    alt_click(&mut keyed, BUY_ITEM);
    let ok = quantity_item(&keyed, 1);
    click(&mut keyed, ok);
    let bought = keyed.take_order();
    assert_eq!(
        bought.map(|order| (order.direction, order.lot)),
        Some((Direction::Buy, Lot::Count(7)))
    );
    assert_eq!(screen.take_order(), bought, "OK, as a click on it");
    assert!(screen.quantity().is_none(), "closed");
}

#[test]
fn without_west_south_on_buy_trades_as_a_plain_click_does() {
    let mut screen = screen();
    screen.input(&told(PAD));
    shoulder_to(&mut screen, SELL_ITEM);
    tap(&mut screen, PadButton::South);
    let mut clicked = self::screen();
    click_item(&mut clicked, SELL_ITEM);
    assert!(screen.quantity().is_none());
    assert_eq!(screen.take_order(), clicked.take_order());
}

#[test]
fn under_focus_ring_off_the_right_shoulder_steps_the_selection_as_tab_does() {
    let off = || screen().with_focus_ring(FocusRing::Off);
    let mut padded = off();
    padded.input(&told(PAD));
    let mut keyed = off();
    for _ in 0..2 {
        tap(&mut padded, PadButton::RightShoulder);
        keyed.input(&key(Key::Tab, true, false));
        keyed.input(&key(Key::Tab, false, false));
        assert_eq!(padded.selected(), keyed.selected());
    }
    assert_ne!(padded.selected(), screen().selected(), "it stepped");
    assert_eq!(highlight(&padded), Some(DONE_ITEM), "the focus stays");
}

#[test]
fn the_left_shoulder_steps_back_as_tab_back_does_under_off() {
    let off = || screen().with_focus_ring(FocusRing::Off);
    let mut padded = off();
    padded.input(&told(PAD));
    tap(&mut padded, PadButton::LeftShoulder);
    let mut keyed = off();
    keyed.input(&key(Key::ShiftLeft, true, false));
    keyed.input(&key(Key::Tab, true, false));
    assert_eq!(padded.selected(), keyed.selected());
    assert_ne!(padded.selected(), screen().selected(), "it stepped");
}

#[test]
fn the_header_summary_and_event_line_are_no_targets() {
    let mut enabled = template();
    for item in [3, 12, 15] {
        enabled.items[item - 1].enabled = true;
    }
    let screen = TradeScreen::new(
        Ok((enabled, Rc::new(MonoMetrics))),
        Ok(quantity()),
        market(),
        ButtonStyle::STOCK,
    );
    let ids: Vec<usize> = screen
        .dialog()
        .expect("laid out")
        .pad_targets()
        .iter()
        .map(|target| target.id)
        .collect();
    let filled = positions(&market())
        .into_iter()
        .enumerate()
        .filter(|(_, row)| row.is_some())
        .map(|(slot, _)| FIRST_ROW_ITEM + slot);
    let mut expected = vec![DONE_ITEM];
    expected.extend(filled);
    expected.extend([BUY_ITEM, SELL_ITEM]);
    assert_eq!(ids, expected, "nor the empty slots");
}

#[test]
fn the_left_stick_steps_the_selection_as_the_arrows_do() {
    let mut padded = screen();
    padded.input(&told(PAD));
    let mut keyed = screen();
    for (stick, arrow) in [
        (PadButton::LeftStickDown, Key::ArrowDown),
        (PadButton::LeftStickDown, Key::ArrowDown),
        (PadButton::LeftStickUp, Key::ArrowUp),
    ] {
        tap(&mut padded, stick);
        keyed.input(&key(arrow, true, false));
        keyed.input(&key(arrow, false, false));
        assert_eq!(padded.selected(), keyed.selected(), "{stick:?}");
    }
}
