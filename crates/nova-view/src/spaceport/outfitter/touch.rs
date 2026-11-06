//! Ask Count on touch (rdm:phase/key-bindings/phase-16-touch-pointer-gaps):
//! the outfitter hosts a [`ModifierToggle`] for
//! [`OutfitterCommand::Count`], so a tap on it then a tap on Buy or Sell
//! does what an Option-click does (`_DoOutfitDialog`'s `optionKey` test,
//! @0x5c0c0, @0x5c5e3). It shows while touch was used last and no
//! quantity dialog, alert or offer is open over the outfitter.

use super::OutfitterScreen;
use crate::geometry::Bounds;
use crate::keymap::{Command, OutfitterCommand};
use crate::ui::touch_aids::ModifierToggle;
use crate::ui::touch_aids::host::AidHost;

/// The outfitter's toggle, clear of its dialog at `dialog`.
pub(super) fn toggle(dialog: Option<Bounds>) -> ModifierToggle {
    ModifierToggle::classic(Command::Outfitter(OutfitterCommand::Count), dialog)
}

impl AidHost for OutfitterScreen {
    type Aid = ModifierToggle;

    fn aid(&self) -> &ModifierToggle {
        &self.toggle
    }

    fn aid_mut(&mut self) -> &mut ModifierToggle {
        &mut self.toggle
    }

    fn aid_live(&self) -> bool {
        self.laid.is_ok()
            && !self.quantity.is_open()
            && !self.alert.is_open()
            && self
                .offering
                .as_ref()
                .is_none_or(|offering| offering.dialog().is_none())
    }
}

impl OutfitterScreen {
    /// Its touch toggle, for tests.
    #[cfg(test)]
    pub(crate) const fn touch_toggle(&self) -> &ModifierToggle {
        &self.toggle
    }
}

/// The outfitter as the toggles' table drives it.
#[cfg(test)]
struct Probe(OutfitterScreen);

#[cfg(test)]
impl crate::ui::touch_aids::tests::ToggleProbe for Probe {
    fn screen(&mut self) -> &mut dyn crate::screen::Screen {
        &mut self.0
    }

    fn toggle(&self) -> &ModifierToggle {
        &self.0.toggle
    }

    fn target(&self) -> crate::geometry::Point {
        self.0
            .dialog()
            .expect("laid out")
            .item_bounds(super::BUY_ITEM)
            .expect("Buy")
            .center()
    }

    fn holds(&mut self) -> bool {
        self.0.quantity.alt_held()
    }
}

/// The outfitter's row in the toggles' table.
#[cfg(test)]
pub(crate) fn toggle_probe() -> Box<dyn crate::ui::touch_aids::tests::ToggleProbe> {
    Box::new(Probe(super::tests::counting_screen()))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{click, counting_screen, quantity_item, screen};
    use super::super::{BUY_ITEM, SELL_ITEM};
    use super::*;
    use crate::draw::DrawList;
    use crate::geometry::Point;
    use crate::input::{Device, Input, Key, Surroundings};
    use crate::screen::Screen;
    use crate::sound::Sound;
    use crate::ui::touch_aids::testkit::{OTHER_DEVICES, drawn, modifier_click, toggle_then_taps};

    fn toggle_at(screen: &OutfitterScreen) -> Point {
        screen.touch_toggle().rect().center()
    }

    fn item(screen: &OutfitterScreen, number: usize) -> Point {
        screen
            .dialog()
            .expect("laid out")
            .item_bounds(number)
            .expect("an item")
            .center()
    }

    fn outcome(screen: &mut OutfitterScreen) -> (String, Vec<Sound>) {
        (
            format!(
                "selected {:?} order {:?} closed {} asking {}",
                screen.selected(),
                screen.take_order(),
                screen.closed(),
                screen.quantity().is_some()
            ),
            screen.take_sounds(),
        )
    }

    fn compare(build: fn() -> OutfitterScreen, target: usize) {
        let mut toggled = build();
        let at = item(&toggled, target);
        toggle_then_taps(&mut toggled, toggle_at, &[at]);
        let mut keyed = build();
        modifier_click(&mut keyed, Key::AltLeft, at);
        assert_eq!(outcome(&mut toggled), outcome(&mut keyed), "{target}");
        let mut plain = build();
        click(&mut plain, at);
        assert_ne!(outcome(&mut keyed).0, outcome(&mut plain).0, "{target}");
    }

    #[test]
    fn the_toggle_then_a_tap_on_buy_does_what_an_option_click_does() {
        compare(counting_screen, BUY_ITEM);
    }

    #[test]
    fn the_toggle_then_a_tap_on_sell_does_what_an_option_click_does() {
        compare(counting_screen, SELL_ITEM);
    }

    #[test]
    fn a_tap_on_buy_after_the_wrapped_tap_is_a_plain_click() {
        let mut toggled = counting_screen();
        let buy = item(&toggled, BUY_ITEM);
        toggle_then_taps(&mut toggled, toggle_at, &[buy]);
        let cancel = quantity_item(&toggled, 4);
        click(&mut toggled, cancel);
        outcome(&mut toggled);
        click(&mut toggled, buy);
        let mut keyed = counting_screen();
        modifier_click(&mut keyed, Key::AltLeft, buy);
        let cancel = quantity_item(&keyed, 4);
        click(&mut keyed, cancel);
        outcome(&mut keyed);
        click(&mut keyed, buy);
        assert_eq!(outcome(&mut toggled), outcome(&mut keyed));
        assert!(!toggled.touch_toggle().armed());
    }

    #[test]
    fn the_toggle_hides_while_the_quantity_dialog_is_open() {
        let mut screen = counting_screen();
        let buy = item(&screen, BUY_ITEM);
        toggle_then_taps(&mut screen, toggle_at, &[buy]);
        assert!(screen.quantity().is_some(), "asking");
        let toggle = DrawList::new();
        let mut alone = toggle;
        screen.touch_toggle().draw(&mut alone);
        let drawn = drawn(&screen);
        assert!(
            alone.iter().all(|command| !drawn.contains(command)),
            "not drawn over it"
        );
    }

    #[test]
    fn the_toggle_is_drawn_only_while_touch_was_used_last() {
        let told = |device| Input::Surroundings(Surroundings::of(device));
        let mut touched = screen();
        touched.input(&told(Device::Touch));
        let mut alone = DrawList::new();
        touched.touch_toggle().draw(&mut alone);
        let all = drawn(&touched);
        assert!(!alone.is_empty());
        assert_eq!(
            all[all.len() - alone.len()..],
            alone.iter().cloned().collect::<Vec<_>>()[..]
        );
        for device in OTHER_DEVICES {
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
        let told = |device| Input::Surroundings(Surroundings::of(device));
        let mut touched = screen();
        touched.input(&told(Device::Touch));
        let at = toggle_at(&touched);
        touched.input(&told(Device::Mouse));
        click(&mut touched, at);
        let mut never = screen();
        never.input(&told(Device::Mouse));
        click(&mut never, at);
        assert_eq!(outcome(&mut touched), outcome(&mut never));
        assert_eq!(drawn(&touched), drawn(&never));
    }
}
