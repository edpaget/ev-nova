//! A spaceport told the pad was the device used last
//! (rdm:phase/key-bindings/phase-9-gamepad-spaceport): its focus shows,
//! the d-pad and South work on it and every screen it holds or opens,
//! East does what Escape does, and the surroundings are no player input.
//! The untold side is in `pad_untold`.

use super::*;
use crate::input::{ControllerFamily, Device, PadButton, Surroundings};
use crate::ui::pad_focus::every_screen::PadProbe;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn tap(view: &mut SpaceportView, button: PadButton) {
    for pressed in [true, false] {
        view.input(&Input::PadButton { button, pressed });
    }
}

fn escape(view: &mut SpaceportView) {
    for pressed in [true, false] {
        view.input(&Input::Key {
            key: Key::Escape,
            pressed,
            repeat: false,
        });
    }
}

fn port_highlight(view: &SpaceportView) -> Option<usize> {
    view.dialog().and_then(Dialog::pad_highlight)
}

/// Taps the right shoulder until the port's highlight is on `item`.
fn shoulder_to(view: &mut SpaceportView, item: usize) {
    for _ in 0..20 {
        if port_highlight(view) == Some(item) {
            return;
        }
        tap(view, PadButton::RightShoulder);
    }
    panic!("the shoulder never reaches item {item}");
}

#[test]
fn told_south_presses_the_initial_target_as_return_does() {
    let mut view = earth();
    view.input(&told(PAD));
    assert_eq!(port_highlight(&view), Some(LEAVE_ITEM), "on Leave");
    tap(&mut view, PadButton::South);
    let mut returned = earth();
    returned.input(&key(Key::Enter));
    assert!(view.left(), "it takes off");
    assert_eq!(view.left(), returned.left());
}

#[test]
fn told_the_dpad_reaches_a_service_and_south_opens_it() {
    let mut view = earth();
    view.input(&told(PAD));
    let bar = 11;
    for _ in 0..12 {
        if port_highlight(&view) == Some(bar) {
            break;
        }
        tap(&mut view, PadButton::DPadUp);
        if port_highlight(&view) == Some(bar) {
            break;
        }
        tap(&mut view, PadButton::DPadLeft);
    }
    shoulder_to(&mut view, bar);
    tap(&mut view, PadButton::South);
    assert!(view.open_service().is_some(), "the bar's placeholder opens");
    assert!(!view.left());
}

#[test]
fn the_spaceport_says_the_device_it_was_told() {
    let mut view = earth();
    assert_eq!(view.device(), Device::Keyboard, "before any");
    view.input(&told(PAD));
    assert_eq!(view.device(), PAD);
    view.input(&told(Device::Mouse));
    assert_eq!(view.device(), Device::Mouse);
}

#[test]
fn told_east_leaves_as_escape_does() {
    let mut view = earth();
    view.input(&told(PAD));
    tap(&mut view, PadButton::East);
    let mut escaped = earth();
    escape(&mut escaped);
    assert!(view.left());
    assert_eq!(view.left(), escaped.left());
}

#[test]
fn told_the_dpad_and_south_work_in_the_bar_and_east_closes_it_only() {
    let mut view = drinking_with_info();
    view.input(&told(PAD));
    shoulder_to(&mut view, 11);
    tap(&mut view, PadButton::South);
    let bar = view.open_bar().expect("the bar opens");
    assert_eq!(
        bar.dialog().map(Dialog::device),
        Some(PAD),
        "told as it opens"
    );
    assert!(bar.dialog().and_then(Dialog::pad_highlight).is_some());
    tap(&mut view, PadButton::East);
    assert!(view.open_bar().is_none(), "East closes the bar");
    assert!(!view.left(), "and only the bar");
    assert_eq!(port_highlight(&view), Some(11), "the port's focus stays");
}

#[test]
fn the_surroundings_are_no_player_input() {
    let mut view = earth();
    // Return held across: a key the close keys and the wait replay
    // follow; the surroundings neither release it nor replay it.
    view.input(&Input::Key {
        key: Key::Enter,
        pressed: true,
        repeat: false,
    });
    let left = view.left();
    let mut fresh = earth();
    for device in [PAD, Device::Mouse, PAD] {
        view.input(&told(device));
        fresh.input(&told(device));
    }
    assert_eq!(view.left(), left, "no take-off on the surroundings");
    assert!(!fresh.left());
    assert!(fresh.open_service().is_none());
    assert!(!fresh.take_recharge());
    assert!(fresh.take_sounds().is_empty(), "nothing played");
}

#[test]
fn every_screen_held_is_told_not_only_the_one_in_front() {
    let mut view = shopping();
    click_item(&mut view, 7);
    assert!(view.open_trade().is_some(), "the trade center opens");
    view.show_mission_texts(vec![text("Hello.")]);
    view.input(&told(PAD));
    assert_eq!(view.dialog().map(Dialog::device), Some(PAD), "the port's");
    assert_eq!(
        view.open_trade()
            .and_then(TradeScreen::dialog)
            .map(Dialog::device),
        Some(PAD),
        "the trade center under the texts"
    );
    assert_eq!(
        view.mission_text().map(|text| text.dialog().device()),
        Some(PAD),
        "the texts"
    );
}

#[test]
fn the_landing_offer_opens_told() {
    let mut view = missioned();
    view.input(&told(PAD));
    view.open_offer(offered());
    let offer = view.offer_dialog().and_then(OfferDialog::dialog);
    assert_eq!(offer.map(Dialog::device), Some(PAD));
    let mut later = missioned();
    later.open_offer(offered());
    later.input(&told(PAD));
    let offer = later.offer_dialog().and_then(OfferDialog::dialog);
    assert_eq!(offer.map(Dialog::device), Some(PAD), "an open one follows");
}

#[test]
fn each_screen_opened_over_the_spaceport_is_told_its_device() {
    let told_view = || {
        let mut view = venues();
        view.input(&told(PAD));
        view
    };
    let mut view = told_view();
    click_item(&mut view, 7);
    let trade = view.open_trade().and_then(TradeScreen::dialog);
    assert_eq!(trade.map(Dialog::device), Some(PAD), "trade");
    let mut view = told_view();
    click_item(&mut view, 8);
    let outfitter = view.open_outfitter().and_then(OutfitterScreen::dialog);
    assert_eq!(outfitter.map(Dialog::device), Some(PAD), "outfitter");
    let mut view = told_view();
    click_item(&mut view, 9);
    let shipyard = view.open_shipyard().and_then(ShipyardScreen::dialog);
    assert_eq!(shipyard.map(Dialog::device), Some(PAD), "shipyard");
    let mut view = drinking_with_info();
    view.input(&told(PAD));
    click_item(&mut view, 11);
    let bar = view.open_bar().and_then(BarScreen::dialog);
    assert_eq!(bar.map(Dialog::device), Some(PAD), "bar");
    let mut view = earth();
    view.input(&told(PAD));
    click_item(&mut view, 11);
    assert_eq!(
        view.open_service().map(ServiceScreen::device),
        Some(PAD),
        "service"
    );
    let mut view = missioned();
    view.input(&told(PAD));
    view.open_mission_computer(Ok(offers()));
    let computer = view
        .mission_computer()
        .and_then(MissionComputerScreen::dialog);
    assert_eq!(computer.map(Dialog::device), Some(PAD), "computer");
    let mut view = missioned();
    view.input(&told(PAD));
    view.show_mission_texts(vec![text("Hello.")]);
    assert_eq!(
        view.mission_text().map(|text| text.dialog().device()),
        Some(PAD),
        "the mission texts"
    );
    let mut view = informed(earth());
    view.input(&told(PAD));
    view.open_player_info(pilot_info());
    assert_eq!(
        view.player_info().map(PlayerInfoScreen::device),
        Some(PAD),
        "player info"
    );
}

#[test]
fn south_on_the_landscape_plays_its_ambient_sound_as_a_press_does() {
    let mut view = kane();
    frame(&mut view);
    view.input(&told(PAD));
    for _ in 0..12 {
        if port_highlight(&view) == Some(LANDSCAPE_ITEM) {
            break;
        }
        tap(&mut view, PadButton::DPadUp);
    }
    assert_eq!(port_highlight(&view), Some(LANDSCAPE_ITEM));
    view.take_sounds();
    view.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    let mut pressed = kane();
    frame(&mut pressed);
    let landscape = item(&pressed, LANDSCAPE_ITEM).center();
    pressed.input(&Input::PointerButton {
        button: MouseButton::Left,
        pressed: true,
        at: landscape,
    });
    let played = pressed.take_sounds();
    assert!(!played.is_empty(), "a press plays it");
    assert_eq!(view.take_sounds(), played, "on South's press");
    view.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert_eq!(view.take_sounds(), [], "not the release");
}

struct Probe(SpaceportView);

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.0
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        let dialog = self.0.dialog().expect("laid out");
        [
            ("Recharge", RECHARGE_ITEM),
            ("landscape", LANDSCAPE_ITEM),
            ("Trade Center", 7),
            ("Bar", 11),
            ("Leave", LEAVE_ITEM),
        ]
        .into_iter()
        .filter(|&(_, item)| dialog.pad_targets().iter().any(|target| target.id == item))
        .map(|(name, item)| (name, dialog.item_bounds(item).expect("an item")))
        .collect()
    }

    fn outcome(&mut self) -> String {
        format!(
            "left {} service {:?} trade {} bar {} recharge {}",
            self.0.left(),
            self.0.open_service().map(ServiceScreen::service),
            self.0.open_trade().is_some(),
            self.0.open_bar().is_some(),
            self.0.take_recharge()
        )
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("Leave")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.0 = earth().with_focus_ring(ring);
        true
    }
}

/// The spaceport's row in the pad's every-screen table.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe(earth()))
}

#[test]
fn the_probe_lists_the_ports_buttons_and_its_landscape() {
    let probe = pad_probe();
    let names: Vec<&str> = probe.controls().into_iter().map(|(name, _)| name).collect();
    assert!(names.contains(&"Leave"), "{names:?}");
    assert!(names.contains(&"landscape"), "{names:?}");
    assert!(names.len() >= 3, "{names:?}");
}

/// South fires the primary weapon in flight. Held through the landing,
/// its release at the spaceport finishes no press the spaceport saw, so
/// it takes nothing off; only a fresh South on Leave does.
#[test]
fn a_south_held_over_from_flight_takes_off_only_on_a_fresh_press() {
    let mut view = earth();
    view.input(&told(PAD));
    view.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(!view.left(), "the held-over release does nothing");
    assert_eq!(view.dialog().expect("laid out").pressed_item(), None);
    view.input(&Input::PadButton {
        button: PadButton::South,
        pressed: true,
    });
    assert!(!view.left(), "a press arms Leave");
    view.input(&Input::PadButton {
        button: PadButton::South,
        pressed: false,
    });
    assert!(view.left(), "its release takes off");
}
