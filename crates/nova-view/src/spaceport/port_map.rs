//! The course map the spaceport asks for
//! ([`SpaceportView::take_map_request`](super::SpaceportView::take_map_request)),
//! on each port screen's map key: the bar's (`_BarFilter` item 8,
//! @0x47a6e-0x47a9b); a port screen's (the spaceport's `_PortFilter`,
//! the exchange's `_TradeFilter`, the outfitter's `_OutfitFilter`, the
//! shipyard's and the hire screen's `_ShipyardFilter`, and the ship
//! description's `_ShipDescFilter`); or a mission dialog's, on its own
//! (an offer over any venue, the mission computer, mission info, or the
//! accept's briefing text). Whoever flies the ship opens it over the
//! spaceport, which lets go of its pointer and keys, and the screen that
//! asked stays open under it.

use crate::flight::{MapOpener, PortScreen};

/// Which course map the spaceport asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortMap {
    /// The bar's
    /// ([`FlightView::open_bar_map`](crate::flight::FlightView::open_bar_map)),
    /// whose close puts back the nav the bar opened with.
    Bar,
    /// A port screen's
    /// ([`FlightView::open_port_screen_map`](crate::flight::FlightView::open_port_screen_map)),
    /// restored as that screen does.
    Screen(PortScreen),
    /// A mission dialog's
    /// ([`FlightView::open_dialog_map`](crate::flight::FlightView::open_dialog_map)),
    /// marked and restored as that dialog does.
    Dialog(MapOpener),
}

#[cfg(test)]
mod tests {
    use nova_sim::{MissionId, MissionText, MissionTextKind};

    use super::*;
    use crate::flight::view::MAP_KEY;
    use crate::screen::Screen;
    use crate::spaceport::view::tests::{
        actives, click_item, drinking_with_info, hiring, in_the_shipyards_panel, informed, key,
        missioned, offered, pilot_info, serve_openings, venues, without_its_dialog,
    };
    use crate::spaceport::{BarScreen, SpaceportView};

    /// The map `view` asks for on a press of the map key, taken once.
    fn asked(view: &mut SpaceportView) -> Option<PortMap> {
        view.input(&key(MAP_KEY));
        let asked = view.take_map_request();
        assert_eq!(view.take_map_request(), None, "once");
        asked
    }

    #[test]
    fn the_map_key_on_the_bare_spaceport_asks_for_its_own_map() {
        let mut view = venues();
        assert_eq!(
            asked(&mut view),
            Some(PortMap::Screen(PortScreen::Spaceport))
        );
        view.input(&crate::Input::Key {
            key: MAP_KEY,
            pressed: true,
            repeat: true,
        });
        assert_eq!(view.take_map_request(), None, "a repeat");
        assert!(!view.take_player_info_request());
        assert!(!view.left());
    }

    #[test]
    fn without_the_port_dialog_the_map_key_asks_nothing() {
        let mut view = without_its_dialog();
        assert_eq!(asked(&mut view), None);
    }

    #[test]
    fn what_covers_the_spaceport_takes_the_map_key_as_it_does() {
        let mut view = venues();
        view.open_offer(offered());
        assert_eq!(
            asked(&mut view),
            Some(PortMap::Dialog(MapOpener::Offer(MissionId(141)))),
            "the spaceport's own offer"
        );
        let mut view = missioned();
        view.open_mission_info(actives(2));
        assert_eq!(
            asked(&mut view),
            Some(PortMap::Dialog(MapOpener::MissionInfo(Some(0)))),
            "mission info"
        );
        let mut view = missioned();
        view.show_mission_texts(vec![MissionText {
            kind: MissionTextKind::Brief,
            mission: MissionId(130),
            text: "Go.".to_owned(),
            graphic: None,
            movie: None,
            map: None,
        }]);
        assert_eq!(asked(&mut view), None, "a mission text with no map");
        let mut view = informed(missioned());
        view.open_player_info(pilot_info());
        assert_eq!(asked(&mut view), None, "player info");
    }

    #[test]
    fn each_shop_asks_for_its_own_map_through_the_spaceport() {
        for (item, screen) in [
            (7, PortScreen::Exchange),
            (8, PortScreen::Outfitter),
            (9, PortScreen::Shipyard),
        ] {
            let mut view = venues();
            click_item(&mut view, item);
            serve_openings(&mut view);
            assert_eq!(
                asked(&mut view),
                Some(PortMap::Screen(screen)),
                "{screen:?}"
            );
            assert!(
                view.open_trade().is_some()
                    || view.open_outfitter().is_some()
                    || view.open_shipyard().is_some(),
                "{screen:?}: still open"
            );
        }
        let mut view = in_the_shipyards_panel();
        assert_eq!(
            asked(&mut view),
            Some(PortMap::Screen(PortScreen::ShipDescription)),
            "the shipyard's Info panel"
        );
    }

    #[test]
    fn the_hire_screen_asks_for_the_shipyards_map_through_the_spaceport() {
        let mut view = hiring(drinking_with_info());
        assert_eq!(
            asked(&mut view),
            Some(PortMap::Screen(PortScreen::Shipyard))
        );
        view.input(&key(crate::spaceport::shipyard::INFO_KEY));
        assert_eq!(
            asked(&mut view),
            Some(PortMap::Screen(PortScreen::ShipDescription))
        );
        assert!(
            view.open_bar()
                .and_then(BarScreen::hire_screen)
                .is_some_and(|hire| hire.info_panel().is_some()),
            "still open"
        );
    }
}
