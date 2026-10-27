use super::*;
use crate::flight::PortScreen;

// The map opened from the spaceport, a shop, the shipyard or the hire
// screen, or the ship description (`_DoPortDialog`, `_DoTradeDialog`,
// `_DoOutfitDialog`, `_DoShipyardDialog`, `_DoShipyardDescription`).

const SCREENS: [PortScreen; 5] = [
    PortScreen::Spaceport,
    PortScreen::Exchange,
    PortScreen::Outfitter,
    PortScreen::Shipyard,
    PortScreen::ShipDescription,
];

/// The screens that put back the nav as their map opened, saved just
/// before the call.
const SAVING_BEFORE: [PortScreen; 3] = [
    PortScreen::Spaceport,
    PortScreen::Shipyard,
    PortScreen::ShipDescription,
];

/// Closes the map as M does on it.
fn press_map(view: &mut View) {
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open());
}

/// Turns the nav off docked, a nav the port dialog's opening didn't
/// save.
fn turn_nav_off(view: &mut View) {
    view.session.as_mut().expect("docked").nav_off();
    assert_eq!(nav_mode(view), NavMode::Off);
}

#[test]
fn each_port_screens_map_opens_as_the_course_map_with_no_marks() {
    for screen in SCREENS {
        let mut view = in_the_bar(None);
        view.open_port_screen_map(screen);
        assert!(view.map_open(), "{screen:?}");
        assert!(!view.dialog_map_open(), "{screen:?}: no mission dialog's");
        assert_eq!(view.course_map().mode(), MapMode::Course, "{screen:?}");
        assert!(
            !view
                .course_map()
                .picture()
                .expect("drawn")
                .systems
                .iter()
                .any(|system| system.green_arrow),
            "{screen:?}: no green system"
        );
    }
}

#[test]
fn each_port_screens_map_opens_and_closes_in_silence() {
    for screen in SCREENS {
        let mut view = in_the_bar(None);
        view.take_sounds();
        let _ = ui_sounds(&mut view);
        view.open_port_screen_map(screen);
        assert_eq!(view.take_sounds(), [], "{screen:?}: opened");
        assert_eq!(ui_sounds(&mut view), [], "{screen:?}: opened");
        press_map(&mut view);
        assert_eq!(view.take_sounds(), [], "{screen:?}: closed");
        assert_eq!(ui_sounds(&mut view), [], "{screen:?}: closed");
    }
}

#[test]
fn each_port_screens_map_closing_cues_no_course() {
    for screen in SCREENS {
        let mut view = in_the_bar(Some(131));
        view.open_port_dialog();
        view.open_port_screen_map(screen);
        for close in [key(MAP, true), key(Key::Enter, true)] {
            if !view.map_open() {
                view.open_port_screen_map(screen);
            }
            view.port_map_input(&close);
            assert!(!view.map_open(), "{screen:?}: {close:?} closes it");
            assert_eq!(port_nav(&view), naming("Earth"), "{screen:?}: no cue drawn");
            assert_eq!(nav_mode(&view), NavMode::Stellar(Some(StellarId(128))));
            assert_eq!(
                view.session().map(|session| session.course().to_vec()),
                Ok(vec![SystemId(131)]),
                "{screen:?}: the course kept"
            );
        }
    }
}

#[test]
fn the_spaceports_shipyards_and_descriptions_maps_put_back_the_nav_as_they_opened() {
    for screen in SAVING_BEFORE {
        let mut view = planner(in_the_bar(None));
        view.open_port_dialog();
        turn_nav_off(&mut view);
        view.open_port_screen_map(screen);
        port_click(&mut view, 131);
        assert_eq!(
            nav_mode(&view),
            NavMode::Hyperspace(Some(SystemId(131))),
            "{screen:?}: the planner's click cues the route"
        );
        press_map(&mut view);
        assert_eq!(
            port_nav(&view),
            cueing_alpha(),
            "{screen:?}: the map's selection drawn, before the restore"
        );
        assert_eq!(
            nav_mode(&view),
            NavMode::Off,
            "{screen:?}: the nav as the map opened"
        );
        assert_eq!(
            view.session().map(|session| session.course().to_vec()),
            Ok(vec![SystemId(131)]),
            "{screen:?}: the course kept"
        );
    }
}

#[test]
fn the_exchanges_and_outfitters_maps_put_back_the_nav_the_dialog_opened_with_and_keep_it() {
    for screen in [PortScreen::Exchange, PortScreen::Outfitter] {
        let mut view = planner(in_the_bar(None));
        view.open_port_dialog();
        turn_nav_off(&mut view);
        view.open_port_screen_map(screen);
        port_click(&mut view, 131);
        press_map(&mut view);
        assert_eq!(port_nav(&view), cueing_alpha(), "{screen:?}: drawn");
        assert_eq!(
            nav_mode(&view),
            NavMode::Stellar(Some(StellarId(128))),
            "{screen:?}: the dialog's save put back"
        );
        turn_nav_off(&mut view);
        view.close_port_dialog(PortDialog::Outfitter);
        assert_eq!(
            nav_mode(&view),
            NavMode::Stellar(Some(StellarId(128))),
            "{screen:?}: the save kept for the dialog's close"
        );
    }
}

#[test]
fn a_clear_route_on_a_port_screens_map_is_undone() {
    for screen in SCREENS {
        let mut view = in_the_bar(Some(131));
        view.open_port_dialog();
        view.open_port_screen_map(screen);
        view.port_map_input(&key(Key::Char('c'), true));
        assert_eq!(nav_mode(&view), NavMode::Stellar(None), "{screen:?}");
        press_map(&mut view);
        assert_eq!(
            nav_mode(&view),
            NavMode::Stellar(Some(StellarId(128))),
            "{screen:?}: put back"
        );
    }
}
