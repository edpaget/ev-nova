//! The services the frame says have nothing to list
//! ([`EmptyServices`]), greyed: by the other reading of
//! `RuleKey::EmptyServices` the session names the Mission BBS while the
//! mission computer lists nothing available and Hire Escort while the bar
//! lists no ship for hire; by the engine reading (the default) it names
//! neither, and the buttons stay as `_DrawPlanetButtons` (@0x2968d) and
//! `_DrawBarButtons` (@0x26e1d) draw them. The view decides nothing: it
//! greys what the frame names.
//!
//! A greyed BBS opens nothing by a click, which [`Dialog`] never reports,
//! or by its key, which `activate` refuses; neither plays a sound, as a
//! greyed button is silent (see the `buttons` module). Hire Escort is
//! told to the bar open and kept for the next one
//! ([`BarScreen::set_nothing_for_hire`]).
//!
//! The spaceport opens before the landing settles, and the session rolls
//! the visit's mission offers in that settle (`_DoPortDialog` shows the
//! port @0x5faff before `_OfferMissionFromPort` @0x5fc37), so a read as
//! it opens doesn't know the computer's offers yet. It asks once to be
//! told again ([`SpaceportView::take_empty_services_request`]), and
//! whoever flies the ship answers after the settle, before the first
//! frame.
//!
//! [`Dialog`]: crate::ui::dialog::Dialog
//! [`BarScreen::set_nothing_for_hire`]: crate::spaceport::bar::BarScreen::set_nothing_for_hire

use nova_sim::Service;
use nova_sim::flight_frame::EmptyServices;

use super::{Open, Port, SpaceportView};
use crate::spaceport::layout::service_item;

impl SpaceportView {
    /// Greys the services `empty` names, as whoever flies the ship reads
    /// them with the shops (`nova_sim::flight_frame::PortShops`), as the
    /// spaceport opens and at each refresh: the Mission BBS while
    /// `empty.mission_bbs`, ungreyed otherwise only where the stellar
    /// offers it; and the bar's Hire Escort, the bar open's and the next
    /// one's, while `empty.hire_escort`, the room still greying it.
    pub fn set_empty_services(&mut self, empty: EmptyServices) {
        if let Ok(port) = &mut self.port {
            port.empty = empty;
            let offered = port.offered.contains(&Service::MissionBbs);
            port.dialog.set_greyed(
                service_item(Service::MissionBbs),
                empty.mission_bbs || !offered,
            );
        }
        if let Some(barkeeping) = &mut self.barkeeping {
            barkeeping.nothing_for_hire = empty.hire_escort;
        }
        if let Some(Open::Bar(screen)) = &mut self.open {
            screen.set_nothing_for_hire(empty.hire_escort);
        }
    }

    /// Whether the spaceport asks to be told its services with nothing to
    /// list ([`SpaceportView::set_empty_services`]), once, from when it
    /// is built: whoever flies the ship answers after the landing's
    /// settle, when the visit's offers are rolled (see the module docs).
    pub fn take_empty_services_request(&mut self) -> bool {
        std::mem::take(&mut self.empty_services_request)
    }
}

impl Port {
    /// Whether the frame said `service` has nothing to list, so its
    /// button, greyed, opens nothing by its key either.
    pub(super) fn names_empty(&self, service: Service) -> bool {
        service == Service::MissionBbs && self.empty.mission_bbs
    }
}

#[cfg(test)]
mod tests {
    use nova_sim::landing::StellarFlags;

    use super::super::tests::{click_item, drawn, drinking, flagged, key, missioned, view_of};
    use super::*;
    use crate::draw::DrawCommand;
    use crate::input::Key;
    use crate::screen::Screen;
    use crate::spaceport::bar::{BarScreen, HIRE_ITEM};
    use crate::spaceport::layout::SERVICE_ITEMS;
    use crate::ui::button::ButtonSkin;

    const BBS: usize = 10;

    fn bbs_empty() -> EmptyServices {
        EmptyServices {
            mission_bbs: true,
            hire_escort: false,
        }
    }

    fn enabled(view: &SpaceportView, item: usize) -> bool {
        view.dialog().expect("laid out").item_enabled(item)
    }

    /// Whether `view` draws the disabled skin's left cap at `item`'s
    /// corner.
    fn drawn_disabled(view: &SpaceportView, item: usize) -> bool {
        let at = view
            .dialog()
            .and_then(|dialog| dialog.item_bounds(item))
            .expect("an item")
            .min;
        drawn(view).iter().any(|command| {
            matches!(
                command,
                DrawCommand::StretchedPicture { image, top_left, .. }
                    if *image == ButtonSkin::NOVA.disabled.left && *top_left == at
            )
        })
    }

    /// Clicks the BBS and presses its key, and says whether either asked
    /// for the mission computer.
    fn bbs_asks(view: &mut SpaceportView) -> bool {
        click_item(view, BBS);
        let clicked = view.take_mission_computer_request();
        view.input(&key(Key::KeyN));
        clicked | view.take_mission_computer_request()
    }

    #[test]
    fn the_spaceport_asks_once_to_be_told_its_empty_services() {
        let mut view = missioned();
        assert!(view.take_empty_services_request());
        assert!(!view.take_empty_services_request(), "once");
    }

    #[test]
    fn the_bbs_item_is_the_mission_bbs() {
        assert_eq!(service_item(Service::MissionBbs), BBS);
    }

    #[test]
    fn an_empty_bbs_is_drawn_greyed_and_opens_nothing_by_click_or_key() {
        let mut view = missioned();
        assert!(enabled(&view, BBS), "precondition: offered");
        assert!(
            bbs_asks(&mut view),
            "precondition: the click and key reach it"
        );
        view.take_sounds();
        view.set_empty_services(bbs_empty());
        assert!(!enabled(&view, BBS));
        assert!(drawn_disabled(&view, BBS));
        click_item(&mut view, BBS);
        assert!(!view.take_mission_computer_request(), "a click");
        view.input(&key(Key::KeyN));
        assert!(!view.take_mission_computer_request(), "its key");
        assert_eq!(view.take_sounds(), [], "silent");
        view.set_empty_services(EmptyServices::default());
        assert!(enabled(&view, BBS), "ungreyed");
        assert!(!drawn_disabled(&view, BBS));
        assert!(bbs_asks(&mut view), "opens again");
    }

    #[test]
    fn an_uninhabited_ports_bbs_stays_greyed_either_way() {
        let mut view = view_of(&flagged(StellarFlags::CAN_LAND | StellarFlags::UNINHABITED));
        for empty in [bbs_empty(), EmptyServices::default()] {
            view.set_empty_services(empty);
            assert!(!enabled(&view, BBS), "{empty:?}");
        }
    }

    #[test]
    fn the_other_services_are_left_as_they_were() {
        let mut view = missioned();
        let before: Vec<bool> = SERVICE_ITEMS
            .iter()
            .map(|&(_, item)| enabled(&view, item))
            .collect();
        view.set_empty_services(EmptyServices {
            mission_bbs: true,
            hire_escort: true,
        });
        for (&(service, item), was) in SERVICE_ITEMS.iter().zip(before) {
            if service != Service::MissionBbs {
                assert_eq!(enabled(&view, item), was, "{service:?}");
            }
        }
    }

    fn hire_enabled(view: &SpaceportView) -> bool {
        view.open_bar()
            .and_then(BarScreen::dialog)
            .expect("the bar laid out")
            .item_enabled(HIRE_ITEM)
    }

    #[test]
    fn hire_escort_is_greyed_in_the_next_bar_and_the_bar_open() {
        let hire_empty = EmptyServices {
            mission_bbs: false,
            hire_escort: true,
        };
        let mut view = drinking(true);
        view.set_empty_services(hire_empty);
        click_item(&mut view, 11);
        assert!(!hire_enabled(&view), "the bar that opens");
        view.set_empty_services(EmptyServices::default());
        assert!(hire_enabled(&view), "the bar open, ungreyed");
        view.set_empty_services(hire_empty);
        assert!(!hire_enabled(&view), "the bar open, greyed");
        view.input(&key(Key::Escape));
        view.set_empty_services(EmptyServices::default());
        click_item(&mut view, 11);
        assert!(hire_enabled(&view), "by default, the room alone");
    }
}
