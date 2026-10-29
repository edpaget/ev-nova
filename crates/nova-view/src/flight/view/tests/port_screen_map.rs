use super::*;
use crate::flight::PortScreen;
use nova_sim::SavedNav;

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

/// Closes `view`'s map as M does on it, over the port.
fn faked_press_map(view: &mut Faked) {
    view.port_map_input(&key(MAP, true));
    view.port_map_input(&key(MAP, false));
    assert!(!view.map_open, "closed");
}

/// The frame of a flight docked at Earth with Earth selected.
fn docked_at_earth() -> FlightFrame {
    FlightFrame {
        landed: Some(StellarId(128)),
        nav_mode: EARTH,
        ..in_sol()
    }
}

/// A view over a scripted flight [`docked_at_earth`], its map's opening
/// answered with its nav, Earth selected, and drawn as [`sol_drawing`]
/// ([`faked_sol`]).
fn faked_docked() -> Faked {
    faked_sol(docked_at_earth())
}

/// Earth (128) selected.
const EARTH: NavMode = NavMode::Stellar(Some(StellarId(128)));

/// Ends the map as [`FlightView::close_map`] ends it: its opener's
/// restore of what it opened with.
fn restore_after(view: &mut Faked) {
    let opened = view.map_opened.take();
    view.restore_opener_nav(opened);
}

#[test]
fn each_port_screens_map_opens_as_the_course_map_with_no_marks() {
    for screen in SCREENS {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        assert!(view.map_open, "{screen:?}");
        assert!(!view.dialog_map_open(), "{screen:?}: no mission dialog's");
        assert_eq!(view.map.mode(), MapMode::Course, "{screen:?}");
        assert_eq!(
            view.sim.screens_asked()[..2],
            [
                ScreenAsk::MapOpening {
                    marks: MarksFrom::Given(nova_sim::MapMarks::NONE),
                },
                ScreenAsk::Map {
                    hypergate: None,
                    missions: Vec::new(),
                    marks: nova_sim::MapMarks::NONE,
                    selected: Some(SystemId(130)),
                },
            ],
            "{screen:?}: no marks given, none of a dialog's asked"
        );
    }
}

#[test]
fn each_port_screens_map_opens_in_silence() {
    for screen in SCREENS {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        assert_eq!(view.ui_sounds, [], "{screen:?}");
        assert_eq!(sent(&view), [], "{screen:?}");
    }
}

#[test]
fn each_port_screens_map_closes_in_silence() {
    for screen in SCREENS {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        faked_press_map(&mut view);
        assert_eq!(view.ui_sounds, [], "{screen:?}: closed");
        assert_eq!(view.drained_sounds(), [], "{screen:?}: closed");
    }
}
#[test]
fn each_port_screens_map_closing_cues_no_course() {
    for screen in SCREENS {
        for close in [key(MAP, true), key(Key::Enter, true)] {
            let mut view = faked_docked();
            view.open_port_screen_map(screen);
            view.port_map_input(&close);
            assert!(!view.map_open, "{screen:?}: {close:?} closes it");
            assert!(
                !sent(&view).contains(&FlightCommand::CueCourse),
                "{screen:?}: {close:?}: no cue in port"
            );
        }
    }
}
#[test]
fn the_spaceports_shipyards_and_descriptions_maps_put_back_the_nav_as_they_opened() {
    for screen in SAVING_BEFORE {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        let stale = Some((EARTH, NavDisplay::Stellar(Some("Stale".to_owned()))));
        view.nav_scene = stale.clone();
        faked_press_map(&mut view);
        assert_eq!(
            sent(&view),
            [
                FlightCommand::RestoreMapSelection(SavedNav::of(EARTH)),
                FlightCommand::RestoreMapNav(SavedNav::of(EARTH)),
            ],
            "{screen:?}: the selection, then the nav as the map opened"
        );
        assert_ne!(
            view.nav_scene, stale,
            "{screen:?}: the close's redraw first"
        );
    }
}
#[test]
fn the_exchanges_and_outfitters_maps_put_back_the_nav_the_dialog_opened_with_and_keep_it() {
    for screen in [PortScreen::Exchange, PortScreen::Outfitter] {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        faked_press_map(&mut view);
        assert_eq!(
            sent(&view),
            [
                FlightCommand::RestoreMapSelection(SavedNav::of(EARTH)),
                FlightCommand::PutBackPortNav,
            ],
            "{screen:?}: the dialog's save put back, and kept"
        );
    }
}
#[test]
fn the_spaceports_shipyards_and_descriptions_map_closes_send_back_the_nav_as_they_opened() {
    for screen in SAVING_BEFORE {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        view.sim.set_frame(FlightFrame {
            nav_mode: NavMode::Off,
            ..docked_at_earth()
        });
        restore_after(&mut view);
        assert_eq!(
            sent(&view),
            [FlightCommand::RestoreMapNav(SavedNav::of(EARTH))],
            "{screen:?}"
        );
    }
}

#[test]
fn the_exchanges_and_outfitters_map_closes_put_back_the_dialogs_save() {
    for screen in [PortScreen::Exchange, PortScreen::Outfitter] {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        restore_after(&mut view);
        assert_eq!(sent(&view), [FlightCommand::PutBackPortNav], "{screen:?}");
    }
}

#[test]
fn the_bars_map_close_puts_back_the_bars_nav() {
    let mut view = faked_docked();
    view.open_bar_map();
    restore_after(&mut view);
    assert_eq!(sent(&view), [FlightCommand::RestoreBarNav]);
}

#[test]
fn a_port_screens_map_close_retakes_no_nav_area() {
    for screen in SCREENS {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        let stale = Some((EARTH, NavDisplay::Stellar(Some("Stale".to_owned()))));
        view.nav_scene = stale.clone();
        restore_after(&mut view);
        assert_eq!(view.nav_scene, stale, "{screen:?}");
    }
}

#[test]
fn a_clear_route_on_a_port_screens_map_is_undone() {
    for screen in SCREENS {
        let mut view = faked_docked();
        view.open_port_screen_map(screen);
        view.port_map_input(&key(Key::Char('c'), true));
        assert_eq!(sent(&view), [FlightCommand::ClearRoute], "{screen:?}");
        faked_press_map(&mut view);
        assert_eq!(
            sent(&view).get(1),
            Some(&FlightCommand::RestoreMapSelection(SavedNav::of(EARTH))),
            "{screen:?}: put back"
        );
    }
}
