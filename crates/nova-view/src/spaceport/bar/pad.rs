//! The bar with the pad (rdm:phase/key-bindings/phase-9-gamepad-spaceport):
//! its row in the every-screen table, and the bar telling every screen it
//! holds, and each it opens, the device it was told.
//!
//! The bar's targets are its dialog's buttons: Leave, Gamble, Holovid
//! and Hire Escort. The pad's focus starts on Leave, the default.

use super::tests::{bar, click_item, laid_hire_bar, list, news, newsy, offering, prompt, racing};
use super::{BarScreen, GAMBLE_ITEM, HIRE_ITEM, HOLOVID_ITEM, LEAVE_ITEM};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, Key, PadButton, Surroundings};
use crate::screen::Screen;
use crate::spaceport::hire::HireScreen;
use crate::spaceport::news::NewsScreen;
use crate::spaceport::offer::OfferDialog;
use crate::spaceport::race::RaceScreen;
use crate::ui::dialog::{Dialog, FocusRing};
use crate::ui::pad_focus::every_screen::PadProbe;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

struct Probe(BarScreen);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        let dialog = self.0.dialog().expect("laid out");
        [
            ("Leave", LEAVE_ITEM),
            ("Gamble", GAMBLE_ITEM),
            ("Holovid", HOLOVID_ITEM),
            ("Hire Escort", HIRE_ITEM),
        ]
        .into_iter()
        .map(|(name, item)| (name, dialog.item_bounds(item).expect("an item")))
        .collect()
    }

    fn outcome(&mut self) -> String {
        format!("closed {} asks {:?}", self.0.closed, self.0.asks)
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Leave")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.0 = std::mem::replace(&mut self.0, bar()).with_focus_ring(ring);
        true
    }
}

/// The bar's row in the pad's every-screen table.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(bar()))
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn tap(screen: &mut BarScreen, button: PadButton) {
    screen.input(&Input::PadButton {
        button,
        pressed: true,
    });
    screen.input(&Input::PadButton {
        button,
        pressed: false,
    });
}

fn escape(screen: &mut BarScreen) {
    for pressed in [true, false] {
        screen.input(&Input::Key {
            key: Key::Escape,
            pressed,
            repeat: false,
        });
    }
}

#[test]
fn the_news_opens_told_the_device_the_bar_was_told() {
    let mut screen = newsy();
    screen.input(&told(PAD));
    screen.open_news(&news(), 9000);
    assert_eq!(screen.news_screen().map(NewsScreen::device), Some(PAD));
}

#[test]
fn the_race_opens_told() {
    let mut screen = racing();
    screen.input(&told(PAD));
    screen.open_race(4_000);
    let race = screen.race_screen().and_then(RaceScreen::dialog);
    assert_eq!(race.map(Dialog::device), Some(PAD));
}

#[test]
fn the_hire_screen_opens_told() {
    let mut screen = laid_hire_bar();
    screen.input(&told(PAD));
    screen.open_hire(list(2));
    let hire = screen.hire_screen().and_then(HireScreen::dialog);
    assert_eq!(hire.map(Dialog::device), Some(PAD));
}

#[test]
fn the_bars_offer_opens_told() {
    let mut screen = offering();
    screen.input(&told(PAD));
    screen.open_offer(prompt());
    let offer = screen.offer_dialog().and_then(OfferDialog::dialog);
    assert_eq!(offer.map(Dialog::device), Some(PAD));
}

#[test]
fn an_open_news_follows_the_surroundings() {
    let mut screen = newsy();
    screen.open_news(&news(), 9000);
    assert_eq!(
        screen.news_screen().map(NewsScreen::device),
        Some(Device::Keyboard)
    );
    screen.input(&told(PAD));
    assert_eq!(screen.news_screen().map(NewsScreen::device), Some(PAD));
    assert_eq!(
        screen.dialog().map(Dialog::device),
        Some(PAD),
        "and the bar"
    );
    assert!(screen.news_screen().is_some(), "nothing closed");
}

#[test]
fn east_on_the_news_closes_the_news_only_as_escape_does() {
    let opened = || {
        let mut screen = newsy();
        screen.input(&told(PAD));
        screen.open_news(&news(), 9000);
        screen
    };
    let mut escaped = opened();
    escape(&mut escaped);
    let mut padded = opened();
    tap(&mut padded, PadButton::East);
    for screen in [&escaped, &padded] {
        assert!(screen.news_screen().is_none(), "the news closed");
        assert!(!screen.closed, "the bar stays");
    }
}

#[test]
fn the_pad_and_south_reach_holovid_and_ask_for_the_news() {
    let mut screen = newsy();
    screen.input(&told(PAD));
    let at = |screen: &BarScreen| screen.dialog().and_then(Dialog::pad_highlight);
    assert_eq!(at(&screen), Some(LEAVE_ITEM));
    for _ in 0..8 {
        if at(&screen) == Some(HOLOVID_ITEM) {
            break;
        }
        tap(&mut screen, PadButton::RightShoulder);
    }
    assert_eq!(at(&screen), Some(HOLOVID_ITEM));
    tap(&mut screen, PadButton::South);
    let mut clicked = newsy();
    click_item(&mut clicked, HOLOVID_ITEM);
    assert_eq!(screen.take_news_request(), clicked.take_news_request());
}
