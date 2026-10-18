//! The launch: the main menu opens holding the last pilot, or nothing,
//! silently.

use super::game::{bit, game, held_aux, held_name, nebula_data, nebula_explored, save_explorer};
use super::menu::rewrite_save;
use super::*;

/// The router over `data` with the main menu over `store` (none when
/// `None`) and dialogs, launched ([`AppScreen::open_last_pilot`]), past its
/// start-up appear.
fn launched_over(data: Rc<GameData>, store: Option<&MemoryPilots>) -> AppScreen {
    with_dialogs(data)
        .with_pilots(store.map(keeper), Rc::new(MonoMetrics))
        .open_last_pilot()
        .appeared()
}

/// [`launched_over`] the plain fixture data.
pub(super) fn launched(store: &MemoryPilots) -> AppScreen {
    launched_over(data(), Some(store))
}

/// Ada, created over `store` and so recorded as the last pilot.
fn save_ada(store: &MemoryPilots) {
    let mut screen = menu(store);
    create_held(&mut screen, "Ada");
    assert_eq!(store.last().as_deref(), Some("Ada"));
}

/// Asserts that `screen` launched onto the main menu holding nothing,
/// with no note open and no warning beyond those the same router gives
/// built without the launch.
fn assert_empty_and_silent(mut screen: AppScreen, store: Option<&MemoryPilots>) {
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.game_in_progress().is_none(), "no pilot held");
    assert!(screen.load_note().is_none(), "no note");
    let mut unlaunched = with_dialogs(data())
        .with_pilots(store.map(keeper), Rc::new(MonoMetrics))
        .appeared();
    assert_eq!(
        screen.take_warnings(),
        unlaunched.take_warnings(),
        "no warning"
    );
}

#[test]
fn the_last_pilot_saved_is_held_at_launch_and_enter_ship_flies_it() {
    let store = MemoryPilots::new();
    save_ada(&store);
    let mut screen = launched(&store);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.flight_view().is_none(), "not flying");
    assert_eq!(held_name(&screen), "Ada");
    assert!(screen.load_note().is_none());
    enter_ship(&mut screen);
    assert_eq!(pilot(&screen).name(), "Ada");
}

#[test]
fn the_last_pilots_panel_shows_on_the_first_frame() {
    let store = MemoryPilots::new();
    save_ada(&store);
    let screen = with_dialogs(data())
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .open_last_pilot();
    assert_eq!(super::game::panel_name(&screen).as_deref(), Some("Ada"));
    assert!(matches!(
        screen.main_menu().expect("a menu").pilot_shown(),
        nova_view::menu::PanelPilot::Alive(_)
    ));
}

#[test]
fn the_last_pilot_opened_is_held_at_launch() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::NewPilot);
    type_text(&mut screen, "Bea");
    screen.input(&key(Key::Enter, true));
    assert_eq!(store.last().as_deref(), Some("Bea"), "the last save");
    open_held(&mut screen);
    assert_eq!(held_name(&screen), "Ada", "the list's first");
    assert_eq!(store.last().as_deref(), Some("Ada"), "the last open");
    assert_eq!(held_name(&launched(&store)), "Ada");
}

#[test]
fn the_launch_writes_no_pilot_and_records_the_pilot_again() {
    let store = MemoryPilots::new();
    save_ada(&store);
    let writes = store.writes();
    store.put_last("Ada");
    let screen = launched(&store);
    assert_eq!(held_name(&screen), "Ada");
    assert_eq!(store.writes(), writes, "no pilot written");
    assert_eq!(store.last().as_deref(), Some("Ada"));
}

#[test]
fn with_no_record_the_menu_starts_empty_and_silent() {
    let store = MemoryPilots::new();
    store.put(
        "Ada",
        &nova_sim::save::encode(&nova_sim::Pilot::new(&*data(), "Ada").expect("a pilot")),
    );
    assert_eq!(store.last(), None);
    assert_empty_and_silent(launched(&store), Some(&store));
}

#[test]
fn with_no_pilot_store_the_menu_starts_empty_and_silent() {
    assert_empty_and_silent(launched_over(data(), None), None);
}

#[test]
fn a_record_naming_a_missing_pilot_starts_the_menu_empty_and_silent() {
    let store = MemoryPilots::new();
    save_ada(&store);
    store.put_last("Gone");
    assert_empty_and_silent(launched(&store), Some(&store));
}

#[test]
fn an_unreadable_or_newer_last_pilot_starts_the_menu_empty_and_silent() {
    let newer = nova_sim::save::CURRENT + 1;
    for text in [
        "not a save".to_owned(),
        format!(r#"{{"version": {newer}}}"#),
    ] {
        let store = MemoryPilots::new();
        store.put("Ada", &text);
        store.put_last("Ada");
        assert_empty_and_silent(launched(&store), Some(&store));
    }
    let store = MemoryPilots::new();
    save_ada(&store);
    store.fail_last_reads(true);
    assert_empty_and_silent(launched(&store), Some(&store));
}

#[test]
fn a_last_pilot_whose_load_falls_back_starts_the_menu_empty_and_silent() {
    // `_LoadPilotData`'s -46 (@0x7648b-0x76498) is a failure to
    // `_OpenLastPilotFile` (@0x7658a), and the note is Open Pilot's.
    let store = MemoryPilots::new();
    save_ada(&store);
    rewrite_save(&store, "Ada", |fields| {
        fields.insert("system".into(), 300.into());
        fields.insert("stellar".into(), serde_json::Value::Null);
    });
    assert_empty_and_silent(launched(&store), Some(&store));
}

#[test]
fn a_last_pilot_whose_session_cannot_start_starts_the_menu_empty_and_silent() {
    let store = MemoryPilots::new();
    save_ada(&store);
    rewrite_save(&store, "Ada", |fields| {
        fields.insert("ship".into(), 999.into());
    });
    assert_empty_and_silent(launched(&store), Some(&store));
}

#[test]
fn a_pilot_held_at_launch_is_swept_its_aux_restarted_unguarded_and_its_system_explored() {
    let store = MemoryPilots::new();
    save_explorer(&store);
    // None left to come: Open Pilot's guarded restart would leave the
    // countdown at 5 and two arrived.
    rewrite_save(&store, "Ada", |fields| {
        fields["missions"][0]["aux"]["left"] = 0.into();
    });
    let writes = store.writes();
    let screen = launched_over(nebula_data(), Some(&store));
    assert_eq!(held_name(&screen), "Ada");
    let held = game(&screen);
    assert!(nebula_explored(held));
    assert!(
        held.pilot().expect("a pilot").control_bit(bit(10)),
        "OnExplore ran"
    );
    let aux = held_aux(&screen);
    assert!((70..140).contains(&aux.countdown), "{aux:?}");
    assert_eq!((aux.arrived, aux.left), (0, 0), "{aux:?}");
    assert_eq!(store.writes(), writes, "its hooks' save dropped");
}
