use super::*;
use crate::key_names::fixture::FakeKeyNames;
use crate::keymap::{Bindings, FlightCommand as Fc, KeyCombo, PlayCommand, SharedBindings};
use nova_sim::flight_frame::FlightEvent;

// Hyperspace.

/// The land hint as the fixture words it, for the Moon, the nearest
/// stellar to the centre.
const MOON_HINT: &str = "Welcome to Nova - it would be a good idea to start by landing on Moon and having a look around. Hit 'L' to request landing clearance, then hit it again to land.";

#[test]
fn the_land_hint_words_a_station_as_docking() {
    assert_eq!(
        land_hint(true, "Earth Station", "L"),
        "Welcome to Nova - it would be a good idea to start by docking at Earth Station and having a look around. Hit 'L' to request landing clearance, then hit it again to land."
    );
    assert_eq!(land_hint(false, "Moon", "L"), MOON_HINT);
}

/// The text of the tutorial's land hint (for the Moon) and its
/// hyperspace hint, as a view reading `bindings` (with their layout)
/// under `rules` words them, for the device its keys read. Phase 12
/// reruns it with the pad.
fn hints(bindings: SharedBindings, rules: &Rulebook) -> (String, String) {
    let mut fake = FakeFlight::new();
    fake.set_frame(FlightFrame {
        stellars: vec![record::stellar(StellarId(129))],
        ..in_sol()
    });
    let mut view = scripted_over(catalog(), fake, rules);
    view.set_bindings(bindings);
    let mut said = |hint| {
        view.sim
            .queue_event(FlightEvent::Message(SimMessage::Tutorial(hint)));
        view.relay_messages();
        view.message().unwrap_or_default().to_owned()
    };
    let land = said(nova_sim::TutorialHint::Land {
        stellar: StellarId(129),
        station: false,
    });
    (land, said(nova_sim::TutorialHint::Hyperspace))
}

/// [`hints`] over `bindings`, naming keys through `names`, by the
/// engine's reading or, for `bible`, `hint_key_names`' other one.
fn hints_named(bindings: Bindings, names: FakeKeyNames, bible: bool) -> (String, String) {
    let mut rules = Rulebook::default();
    if bible {
        rules = rules.with_override(RuleKey::HintKeyNames, RuleSource::Bible);
    }
    hints(
        SharedBindings::new(bindings).with_key_names(Rc::new(names)),
        &rules,
    )
}

/// The default hyperspace hint.
const HYPERSPACE_HINT: &str = "Now you should probably hyperspace to another system. Hit 'M' to access the map, select a nearby system, then move outward and hit 'J' to begin your jump.";

#[test]
fn under_the_defaults_the_hints_read_as_before() {
    for (names, bible) in [
        (FakeKeyNames::new(), false),
        (FakeKeyNames::new(), true),
        (FakeKeyNames::azerty(), false),
    ] {
        let (land, hyperspace) = hints_named(Bindings::default(), names, bible);
        assert_eq!(land, MOON_HINT);
        assert_eq!(hyperspace, HYPERSPACE_HINT);
    }
    let (_, azerty) = hints_named(Bindings::default(), FakeKeyNames::azerty(), true);
    assert_eq!(
        azerty,
        hyperspace_hint(",", "J"),
        "AZERTY prints , at the US M"
    );
}

#[test]
fn the_hints_name_the_rebound_keys() {
    let bindings = Bindings::default()
        .rebound(Fc::LandDock, KeyCombo::key(Key::KeyK))
        .rebound(PlayCommand::GalaxyMap, KeyCombo::key(Key::KeyN))
        .rebound(Fc::HyperJump, KeyCombo::key(Key::KeyU));
    let (land, hyperspace) = hints_named(bindings, FakeKeyNames::new(), false);
    assert_eq!(land, land_hint(false, "Moon", "K"));
    assert!(land.contains("Hit 'K'"), "{land}");
    assert_eq!(hyperspace, hyperspace_hint("N", "U"));
}

#[test]
fn the_hints_name_the_key_from_str_129_by_default_and_through_the_layout_under_the_bible() {
    let on_w = Bindings::default().rebound(Fc::LandDock, KeyCombo::key(Key::KeyW));
    let (engine, _) = hints_named(on_w.clone(), FakeKeyNames::azerty(), false);
    assert_eq!(engine, land_hint(false, "Moon", "W"));
    let (bible, _) = hints_named(on_w, FakeKeyNames::azerty(), true);
    assert_eq!(bible, land_hint(false, "Moon", "Z"));
    let on_up = Bindings::default().rebound(Fc::LandDock, KeyCombo::key(Key::ArrowUp));
    let (engine, _) = hints_named(on_up.clone(), FakeKeyNames::new(), false);
    assert_eq!(engine, land_hint(false, "Moon", "Up"));
    let (bible, _) = hints_named(on_up, FakeKeyNames::new(), true);
    assert_eq!(bible, land_hint(false, "Moon", "Up Arrow"));
}

#[test]
fn an_unbound_key_reads_empty_in_its_hint() {
    let unbound = Bindings::default().unbound(Fc::LandDock);
    let (land, _) = hints_named(unbound, FakeKeyNames::new(), false);
    assert_eq!(land, land_hint(false, "Moon", ""));
    assert!(land.contains("Hit ''"), "{land}");
}

/// Whether [`FlightView::taken_during_jump`] takes `key` with Option
/// held or not and the escort menu open or shut, each of the four.
fn taken_with(key: Key) -> [bool; 4] {
    [(false, false), (true, false), (false, true), (true, true)]
        .map(|(option, menu_open)| Faked::taken_during_jump(key, option, menu_open))
}

#[test]
fn the_keys_a_jump_takes_are_the_originals_ungated_ones() {
    let always = [
        TARGET_KEY,
        NEAREST_KEY,
        SELECT_KEY,
        BOARD_KEY,
        JETTISON_KEY,
        MENU_KEY,
        MENU_CLOSE_KEY,
        ESCORT_ATTACK_KEY,
        ESCORT_DEFEND_KEY,
        ESCORT_HOLD_KEY,
        ESCORT_RECALL_KEY,
        ZOOM_OUT_KEY,
        ZOOM_IN_KEY,
        ZOOM_IN_KEYPAD_KEY,
    ];
    for key in always {
        assert_eq!(taken_with(key), [true; 4], "{key:?}");
    }
    assert_eq!(
        taken_with(NAV_OFF_KEY),
        [false, true, false, true],
        "with Option"
    );
    for key in GROUP_KEYS {
        assert_eq!(
            taken_with(key),
            [false, false, true, true],
            "{key:?}: menu open"
        );
    }
    let never = [
        LAND_KEY,
        MAP_KEY,
        HAIL_KEY,
        JUMP_KEY,
        MISSION_INFO_KEY,
        PLAYER_INFO_KEY,
        HYPERSPACE_MODE_KEY,
        HYPER_SELECT_KEY,
        Key::F1,
        Key::F4,
        NEAREST_STELLAR_KEY,
        Key::ArrowUp,
        FIRE_KEY,
    ];
    for key in never {
        assert_eq!(taken_with(key), [false; 4], "{key:?}");
    }
}

// Hyper Select.

// Hypergates and wormholes.

#[test]
fn the_gate_messages_read_as_the_original() {
    assert_eq!(
        arrival_message_with(EXITING_HYPERGATE, "Alpha Centauri", DATE, true),
        "Exiting hypergate in the Alpha Centauri system on June 23rd, 1177 NC."
    );
    assert_eq!(
        arrival_message_with(PASSING_WORMHOLE, "Barnard", DATE, false),
        "Passing through a wormhole into the Barnard system on June 23rd, 1177 NC. \
         No stellar objects present."
    );
    assert_eq!(
        arrival_message_with(JUMPING_INTO, "Sol", DATE, true),
        arrival_message("Sol", DATE, true)
    );
    assert_eq!(
        gate_refusal_message(GateKind::Hypergate),
        "Your ship is unable to enter this hypergate - it is offline."
    );
    assert_eq!(
        gate_refusal_message(GateKind::Wormhole),
        "Your ship is unable to enter this wormhole - the radiation levels are too extreme."
    );
    assert_eq!(HYPERGATE_CANCELLED, "Hypergate jump cancelled.");
    assert_eq!(HYPERGATE_DENIED, "Hypergate usage denied.");
}
