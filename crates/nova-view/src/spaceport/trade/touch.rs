//! Trade Most on touch (rdm:phase/key-bindings/phase-16-touch-pointer-gaps):
//! the trade center hosts a [`ModifierToggle`] for
//! [`TradeCommand::MaxLot`], so a tap on it then a tap on Buy or Sell does
//! what an Option-click does (`_DoTradeDialog`'s `optionKey` test,
//! @0x5e211, @0x5e45c). It shows while touch was used last and neither
//! the quantity dialog nor an offer is open over the exchange.

use super::TradeScreen;
use crate::geometry::Bounds;
use crate::keymap::{Command, TradeCommand};
use crate::ui::touch_aids::ModifierToggle;
use crate::ui::touch_aids::host::ToggleHost;

/// The trade center's toggle, clear of its dialog at `dialog`.
pub(super) fn toggle(dialog: Option<Bounds>) -> ModifierToggle {
    ModifierToggle::classic(Command::Trade(TradeCommand::MaxLot), dialog)
}

impl ToggleHost for TradeScreen {
    fn toggle(&self) -> &ModifierToggle {
        &self.toggle
    }

    fn toggle_mut(&mut self) -> &mut ModifierToggle {
        &mut self.toggle
    }

    fn toggle_live(&self) -> bool {
        self.laid.is_ok()
            && !self.quantity.is_open()
            && self
                .offering
                .as_ref()
                .is_none_or(|offering| offering.dialog().is_none())
    }
}

impl TradeScreen {
    /// Its touch toggle, for tests.
    #[cfg(test)]
    pub(crate) const fn touch_toggle(&self) -> &ModifierToggle {
        &self.toggle
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click_item, item, key, quantity_item, screen};
    use super::super::{BUY_ITEM, FIRST_ROW_ITEM, SELL_ITEM};
    use super::*;
    use crate::draw::{DrawCommand, DrawList};
    use crate::geometry::Point;
    use crate::input::{ControllerFamily, Device, Input, Key, MouseButton, Surroundings};
    use crate::screen::Screen;
    use crate::sound::Sound;
    use crate::touch::testkit::Fingers;

    fn told(device: Device) -> Input {
        Input::Surroundings(Surroundings::of(device))
    }

    fn outcome(screen: &mut TradeScreen) -> String {
        format!(
            "closed {} selected {:?} order {:?} asking {}",
            screen.closed(),
            screen.selected(),
            screen.take_order(),
            screen.quantity().is_some()
        )
    }

    /// One finger taps `screen` at each of `taps` in turn.
    fn tapped(screen: &mut TradeScreen, taps: &[Point]) {
        let mut fingers = Fingers::on(screen);
        for &at in taps {
            fingers.down(1, at);
            fingers.up(1, at);
        }
    }

    /// Told touch, the toggle tapped, then `target`: its outcome and
    /// sounds, and the screen.
    fn toggled(target: usize) -> (String, Vec<Sound>, TradeScreen) {
        let mut screen = screen();
        screen.input(&told(Device::Touch));
        let toggle = screen.touch_toggle().rect().center();
        let at = item(&screen, target).center();
        tapped(&mut screen, &[toggle, at]);
        (outcome(&mut screen), screen.take_sounds(), screen)
    }

    /// The left Option held through a mouse click on `target`.
    fn option_clicked(target: usize) -> (String, Vec<Sound>) {
        let mut screen = screen();
        screen.input(&key(Key::AltLeft, true, false));
        let at = item(&screen, target).center();
        screen.input(&Input::PointerMoved(at));
        click_item(&mut screen, target);
        screen.input(&key(Key::AltLeft, false, false));
        (outcome(&mut screen), screen.take_sounds())
    }

    fn drawn(screen: &TradeScreen) -> Vec<DrawCommand> {
        let mut list = DrawList::new();
        screen.draw(&mut list);
        list.iter().cloned().collect()
    }

    #[test]
    fn the_toggle_then_a_tap_on_buy_does_what_an_option_click_does() {
        let (outcome, sounds, _) = toggled(BUY_ITEM);
        assert_eq!((outcome.clone(), sounds), option_clicked(BUY_ITEM));
        let mut plain = screen();
        click_item(&mut plain, BUY_ITEM);
        assert_ne!(outcome, self::outcome(&mut plain), "not a plain click");
    }

    #[test]
    fn the_toggle_then_a_tap_on_sell_does_what_an_option_click_does() {
        let (outcome, sounds, _) = toggled(SELL_ITEM);
        assert_eq!((outcome, sounds), option_clicked(SELL_ITEM));
    }

    #[test]
    fn a_tap_on_buy_after_the_wrapped_tap_is_a_plain_click() {
        let row = FIRST_ROW_ITEM + 1;
        let (_, _, mut screen) = toggled(row);
        let buy = item(&screen, BUY_ITEM).center();
        tapped(&mut screen, &[buy]);
        let mut keyed = screen_after_option_click(row);
        click_item(&mut keyed, BUY_ITEM);
        assert_eq!(outcome(&mut screen), outcome(&mut keyed));
    }

    fn screen_after_option_click(target: usize) -> TradeScreen {
        let mut screen = super::super::tests::screen();
        screen.input(&key(Key::AltLeft, true, false));
        click_item(&mut screen, target);
        screen.input(&key(Key::AltLeft, false, false));
        let _ = outcome(&mut screen);
        screen
    }

    #[test]
    fn a_tap_on_the_toggle_makes_no_sound_and_no_order() {
        let mut screen = screen();
        screen.input(&told(Device::Touch));
        let toggle = screen.touch_toggle().rect().center();
        tapped(&mut screen, &[toggle]);
        assert!(screen.touch_toggle().armed());
        assert_eq!(screen.take_sounds(), []);
        assert_eq!(outcome(&mut screen), outcome(&mut self::screen()));
    }

    #[test]
    fn the_toggle_hides_while_the_quantity_dialog_is_open() {
        let mut screen = screen();
        screen.input(&told(Device::Touch));
        assert!(screen.touch_toggle().shown());
        let (_, _, mut asked) = toggled(BUY_ITEM);
        if asked.quantity().is_none() {
            // The market answers Option with the most; open the ask by key.
            asked = super::super::tests::screen();
            asked.input(&told(Device::Touch));
            asked.input(&key(Key::AltLeft, true, false));
            click_item(&mut asked, BUY_ITEM);
        }
        assert!(asked.quantity().is_some(), "asking");
        assert!(!asked.touch_toggle().shown(), "hidden under it");
        let toggle = asked.touch_toggle().rect();
        assert!(!drawn(&asked).iter().any(|command| matches!(command,
            DrawCommand::Line { from, .. } if *from == toggle.min)));
        let cancel = quantity_item(&asked, 4);
        super::super::tests::click(&mut asked, cancel);
        asked.input(&key(Key::AltLeft, false, false));
        assert!(asked.quantity().is_none(), "cancelled");
        assert!(asked.touch_toggle().shown(), "back");
    }

    #[test]
    fn the_toggle_hides_while_an_offer_is_open() {
        use crate::spaceport::offer::fixture::{prompt, templates};
        let mut screen = screen().with_offers(templates());
        screen.input(&told(Device::Touch));
        let before = drawn(&screen);
        screen.open_offer(prompt());
        let toggle = screen.touch_toggle().rect();
        let edge = |drawn: &[DrawCommand]| {
            drawn.iter().any(
                |command| matches!(command, DrawCommand::Line { from, .. } if *from == toggle.min),
            )
        };
        assert!(edge(&before), "shown before");
        assert!(!edge(&drawn(&screen)), "not drawn over the offer");
        screen.input(&Input::PointerMoved(Point::new(1.0, 1.0)));
        assert!(!screen.touch_toggle().shown());
    }

    #[test]
    fn the_toggle_is_drawn_only_while_touch_was_used_last() {
        let mut touched = screen();
        touched.input(&told(Device::Touch));
        let mut toggle = DrawList::new();
        touched.touch_toggle().draw(&mut toggle);
        let toggle: Vec<_> = toggle.iter().cloned().collect();
        assert!(!toggle.is_empty());
        let all = drawn(&touched);
        assert_eq!(all[all.len() - toggle.len()..], toggle[..], "drawn last");
        for device in [
            Device::Keyboard,
            Device::Mouse,
            Device::Gamepad(ControllerFamily::Generic),
        ] {
            let mut screen = screen();
            screen.input(&told(Device::Touch));
            screen.input(&told(device));
            let mut bare = self::screen();
            bare.input(&told(device));
            assert_eq!(drawn(&screen), drawn(&bare), "{device:?}");
        }
    }

    #[test]
    fn a_click_where_the_toggle_would_be_does_what_it_did() {
        let mut touched = screen();
        touched.input(&told(Device::Touch));
        let at = touched.touch_toggle().rect().center();
        let click = |screen: &mut TradeScreen| {
            screen.input(&told(Device::Mouse));
            screen.input(&Input::PointerMoved(at));
            for pressed in [true, false] {
                screen.input(&Input::PointerButton {
                    button: MouseButton::Left,
                    pressed,
                    at,
                });
            }
            (outcome(screen), screen.take_sounds(), drawn(screen))
        };
        let mut never = screen();
        assert_eq!(click(&mut touched), click(&mut never));
        assert!(!touched.touch_toggle().armed());
    }
}
