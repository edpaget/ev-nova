//! Ask Bet on touch (rdm:phase/key-bindings/phase-16-touch-pointer-gaps):
//! the race hosts a [`ModifierToggle`] for [`RaceCommand::Ask`], so a tap
//! on it then a tap on a racer asks the bet as an Option-click does
//! (`_DoRaceDialog` polls the left Option, `_KeyCheck(0x3a)` @0x48be9).
//! It shows while touch was used last and the bet's dialog isn't open.

use super::RaceScreen;
use crate::geometry::Bounds;
use crate::keymap::{Command, RaceCommand};
use crate::ui::touch_aids::ModifierToggle;
use crate::ui::touch_aids::host::AidHost;

/// The race's toggle, clear of its dialog at `dialog`.
pub(super) fn toggle(dialog: Option<Bounds>) -> ModifierToggle {
    ModifierToggle::classic(Command::Race(RaceCommand::Ask), dialog)
}

impl AidHost for RaceScreen {
    type Aid = ModifierToggle;

    fn aid(&self) -> &ModifierToggle {
        &self.toggle
    }

    fn aid_mut(&mut self) -> &mut ModifierToggle {
        &mut self.toggle
    }

    fn aid_live(&self) -> bool {
        self.laid.is_ok() && !self.quantity.is_open()
    }
}

impl RaceScreen {
    /// Its touch toggle, for tests.
    #[cfg(test)]
    pub(crate) const fn touch_toggle(&self) -> &ModifierToggle {
        &self.toggle
    }
}

/// The race as the toggles' table drives it.
#[cfg(test)]
struct Probe(RaceScreen);

#[cfg(test)]
impl crate::ui::touch_aids::tests::ToggleProbe for Probe {
    fn screen(&mut self) -> &mut dyn crate::screen::Screen {
        &mut self.0
    }

    fn toggle(&self) -> &ModifierToggle {
        &self.0.toggle
    }

    fn target(&self) -> crate::geometry::Point {
        super::tests::centre(&self.0, super::tests::racer_item(1))
    }

    fn holds(&mut self) -> bool {
        self.0.quantity.alt_held()
    }
}

/// The race's row in the toggles' table.
#[cfg(test)]
pub(crate) fn toggle_probe() -> Box<dyn crate::ui::touch_aids::tests::ToggleProbe> {
    Box::new(Probe(super::tests::open()))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{centre, click, open, racer_item};
    use super::*;
    use crate::draw::DrawList;
    use crate::geometry::Point;
    use crate::input::{Device, Input, Key, Surroundings};
    use crate::screen::Screen;
    use crate::ui::touch_aids::testkit::{OTHER_DEVICES, drawn, modifier_click, toggle_then_taps};

    fn toggle_at(screen: &RaceScreen) -> Point {
        screen.touch_toggle().rect().center()
    }

    fn outcome(screen: &mut RaceScreen) -> String {
        format!(
            "bet {:?} asking {:?} sounds {:?}",
            screen.take_bet(),
            screen
                .quantity()
                .and_then(|asking| asking.dialog())
                .map(|dialog| dialog.field().text().to_owned()),
            screen.take_sounds()
        )
    }

    #[test]
    fn the_toggle_then_a_tap_on_a_racer_asks_the_bet_as_an_option_click_does() {
        let mut toggled = open();
        let racer = centre(&toggled, racer_item(2));
        toggle_then_taps(&mut toggled, toggle_at, &[racer]);
        let mut keyed = open();
        modifier_click(&mut keyed, Key::AltLeft, racer);
        let asked = outcome(&mut toggled);
        assert_eq!(asked, outcome(&mut keyed));
        assert!(asked.contains("asking Some"), "{asked}");
    }

    #[test]
    fn a_tap_on_a_racer_after_the_wrapped_tap_is_a_plain_click() {
        let mut toggled = open();
        let leave = centre(&toggled, 1);
        toggle_then_taps(&mut toggled, toggle_at, &[Point::new(2.0, 2.0)]);
        assert!(!toggled.touch_toggle().armed(), "spent on a miss");
        click(&mut toggled, racer_item(1));
        let mut plain = open();
        click(&mut plain, racer_item(1));
        assert_eq!(outcome(&mut toggled), outcome(&mut plain));
        let _ = leave;
    }

    #[test]
    fn the_toggle_hides_while_the_bet_is_asked() {
        let mut screen = open();
        let racer = centre(&screen, racer_item(0));
        toggle_then_taps(&mut screen, toggle_at, &[racer]);
        assert!(screen.quantity().is_some(), "asking");
        let mut alone = DrawList::new();
        screen.touch_toggle().draw(&mut alone);
        let drawn = drawn(&screen);
        assert!(alone.iter().all(|command| !drawn.contains(command)));
    }

    #[test]
    fn the_toggle_is_drawn_only_while_touch_was_used_last() {
        let told = |device| Input::Surroundings(Surroundings::of(device));
        let mut touched = open();
        touched.input(&told(Device::Touch));
        let mut alone = DrawList::new();
        touched.touch_toggle().draw(&mut alone);
        assert!(!alone.is_empty());
        let all = drawn(&touched);
        assert_eq!(
            all[all.len() - alone.len()..],
            alone.iter().cloned().collect::<Vec<_>>()[..]
        );
        for device in OTHER_DEVICES {
            let mut screen = open();
            screen.input(&told(Device::Touch));
            screen.input(&told(device));
            let mut bare = open();
            bare.input(&told(device));
            assert_eq!(drawn(&screen), drawn(&bare), "{device:?}");
        }
    }

    #[test]
    fn a_click_where_the_toggle_would_be_does_what_it_did() {
        let told = |device| Input::Surroundings(Surroundings::of(device));
        let mut touched = open();
        touched.input(&told(Device::Touch));
        let at = toggle_at(&touched);
        touched.input(&told(Device::Mouse));
        modifier_click(&mut touched, Key::KeyZ, at);
        let mut never = open();
        never.input(&told(Device::Mouse));
        modifier_click(&mut never, Key::KeyZ, at);
        assert_eq!(outcome(&mut touched), outcome(&mut never));
        assert_eq!(drawn(&touched), drawn(&never));
    }
}
