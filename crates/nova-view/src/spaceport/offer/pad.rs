//! The offer dialog with the pad (rdm:phase/key-bindings/phase-9-gamepad-spaceport):
//! its rows in the every-screen table, and the venue telling each offer
//! it opens the device it was told.
//!
//! The offer's targets are its dialog's: accept and refuse (or the single
//! button), and the scroll arrows while the text overflows. The pad's
//! focus starts on accept, the default, so South at once does what Return
//! does.

use std::rc::Rc;

use super::fixture::{prompt, stock_1016, template, templates};
use super::{ACCEPT_ITEM, OfferDialog, REFUSE_ITEM, VenueOffers};
use crate::geometry::Bounds;
use crate::input::{ControllerFamily, Device, Input, PadButton, Surroundings};
use crate::screen::Screen;
use crate::text::fixture::MonoMetrics;
use crate::ui::button::ButtonStyle;
use crate::ui::dialog::FocusRing;
use crate::ui::pad_focus::every_screen::PadProbe;

const PAD: Device = Device::Gamepad(ControllerFamily::Generic);

/// The scroll arrows' items in `DITL` 1016.
const ARROWS: [usize; 2] = [9, 10];

struct Probe {
    offer: OfferDialog,
    arrows: bool,
}

impl PadProbe for Probe {
    fn screen(&mut self) -> &mut dyn Screen {
        &mut self.offer
    }

    fn controls(&self) -> Vec<(&'static str, Bounds)> {
        let dialog = self.offer.dialog().expect("laid out");
        let bounds = |item| dialog.item_bounds(item).expect("an item");
        let mut controls = vec![
            ("accept", bounds(ACCEPT_ITEM)),
            ("refuse", bounds(REFUSE_ITEM)),
        ];
        if self.arrows {
            controls.extend([("up", bounds(ARROWS[0])), ("down", bounds(ARROWS[1]))]);
        }
        controls
    }

    fn outcome(&mut self) -> String {
        let scrolled = self
            .offer
            .dialog()
            .and_then(|dialog| dialog.scroll_text())
            .map(|text| format!("{text:?}"));
        format!("{:?} {scrolled:?}", self.offer.take_choice())
    }

    fn default_control(&self) -> Option<&'static str> {
        Some("accept")
    }

    fn set_ring(&mut self, ring: FocusRing) -> bool {
        self.offer = self.offer.clone().with_focus_ring(ring);
        true
    }
}

/// The offer's row: the fixture's accept and refuse.
pub(crate) fn pad_probe() -> Box<dyn PadProbe> {
    Box::new(Probe {
        offer: OfferDialog::new(
            Ok((template(), Rc::new(MonoMetrics))),
            prompt(),
            ButtonStyle::STOCK,
        ),
        arrows: false,
    })
}

/// The offer's row with a text that overflows stock 1016, so its arrows
/// show: a press on one holds it, scrolling, until its release.
pub(crate) fn pad_probe_overflowing() -> Box<dyn PadProbe> {
    let mut long = prompt();
    long.text = "Ferry this to Mars, by the long way round. ".repeat(80);
    Box::new(Probe {
        offer: OfferDialog::new(
            Ok((stock_1016(), Rc::new(MonoMetrics))),
            long,
            ButtonStyle::STOCK,
        ),
        arrows: true,
    })
}

fn told(device: Device) -> Input {
    Input::Surroundings(Surroundings { device })
}

fn opened(offers: &mut VenueOffers) {
    offers.open(prompt(), Rc::new(MonoMetrics), ButtonStyle::STOCK);
}

fn device(offers: &VenueOffers) -> Device {
    offers
        .dialog()
        .and_then(OfferDialog::dialog)
        .expect("an offer laid out")
        .device()
}

#[test]
fn each_offer_opened_is_told_the_device_the_venue_was_told() {
    let mut offers = VenueOffers::asking(templates());
    assert!(!offers.input(&told(PAD), &mut Vec::new()), "never taken");
    opened(&mut offers);
    assert_eq!(device(&offers), PAD);
    assert!(offers.dialog().and_then(OfferDialog::dialog).is_some());
    // South at once accepts, as Return does.
    let south = |pressed| Input::PadButton {
        button: PadButton::South,
        pressed,
    };
    offers.input(&south(true), &mut Vec::new());
    offers.input(&south(false), &mut Vec::new());
    assert!(offers.take_choice().is_some(), "the first offer answered");
    opened(&mut offers);
    assert_eq!(device(&offers), PAD, "and the next in the queue");
}

#[test]
fn an_open_offer_follows_the_surroundings() {
    let mut offers = VenueOffers::asking(templates());
    opened(&mut offers);
    assert_eq!(device(&offers), Device::Keyboard);
    offers.input(&told(PAD), &mut Vec::new());
    assert_eq!(device(&offers), PAD);
    offers.input(&told(Device::Mouse), &mut Vec::new());
    assert_eq!(device(&offers), Device::Mouse);
    assert!(
        offers.take_choice().is_none(),
        "the surroundings answer nothing"
    );
}
