//! The game in progress the main menu holds: New Pilot and Open Pilot
//! hold their pilot, Enter Ship flies it, New Pilot over it asks `STR#`
//! 140 #8, and a return from play keeps it.

use super::menu::rewrite_save;
use super::*;
use nova_data::records::nebula::Nebula;
use nova_sim::mission::aux_ships::AuxShips;
use nova_view::ui::note::VERSION_NOTE;
use nova_view::ui::yes_no::{NEW_PILOT_QUESTION, NO_ITEM};

/// The router with the main menu over `store`, and dialogs.
fn holding_menu(store: &MemoryPilots) -> AppScreen {
    with_dialogs(data())
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

/// The game held, which must be there.
pub(super) fn game(screen: &AppScreen) -> &FlightView<Rc<GameData>> {
    screen.game_in_progress().expect("a game held")
}

/// The held game's pilot's name.
pub(super) fn held_name(screen: &AppScreen) -> String {
    game(screen).pilot().expect("a pilot").name().to_owned()
}

pub(super) fn bit(n: u16) -> nova_sim::Bit {
    nova_sim::Bit::new(n).expect("a bit")
}

/// The interface sounds `screen` reports.
fn ui_sounds(screen: &mut AppScreen) -> Vec<UiSound> {
    screen
        .take_sounds()
        .into_iter()
        .filter_map(|sound| match sound {
            Sound::Ui(sound) => Some(sound),
            _ => None,
        })
        .collect()
}

// New Pilot.

#[test]
fn a_new_pilot_returns_to_the_main_menu_loaded_and_not_flying() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.flight_view().is_none(), "not flying");
    assert_eq!(held_name(&screen), "Ada");
    assert_eq!(store.keys(), ["Ada"], "saved at once");
    assert_eq!(store.writes(), 1, "once: settling made no save due");
    assert_eq!(
        saved(&store, "Ada"),
        *game(&screen).pilot().expect("a pilot")
    );
    for _ in 0..30 {
        screen.tick(TICK);
    }
    assert_eq!(
        game(&screen).message(),
        None,
        "never ticked: a tick would show the land hint"
    );
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn a_new_pilots_failed_save_is_one_warning_and_the_menu_still_holds_it() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    store.fail_writes(true);
    create_held(&mut screen, "Ada");
    assert_eq!(
        screen.take_warnings(),
        ["nova: cannot save the pilot Ada in memory (the disk is full)"],
        "one save, its failure warned"
    );
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(held_name(&screen), "Ada");
    assert!(store.keys().is_empty(), "nothing saved");
}

// Open Pilot.

/// [`data`], with nebula 128 covering system 128 (at (0, 0)), its
/// `OnExplore` `^b10`, and mission 130 of two auxiliary ships of `düde`
/// 200 into any system.
pub(super) fn nebula_data() -> Rc<GameData> {
    let mut nebula = vec![0; Nebula::SIZE.expect("fixed")];
    put_i16s(&mut nebula, 0, &[-20, -20, 60, 60]);
    nebula[0x107..0x10B].copy_from_slice(b"^b10");
    let mut mission = venue_mission(0);
    put_i16s(&mut mission, 0x48, &[2, 200, -1]);
    loaded(
        &game_data_fork(false, false, false, 100, b"", b"", None)
            .resource(Nebula::TYPE, 128, Some(b"Holpa"), &nebula)
            .resource(Mission::TYPE, 130, Some(b"Aux job"), &mission),
    )
}

/// The main menu over [`nebula_data`] and `store`, with dialogs.
fn nebula_menu(store: &MemoryPilots) -> AppScreen {
    with_dialogs(nebula_data())
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

/// Ada over [`nebula_data`], saved having explored system 128 and holding
/// mission 130, its auxiliary ships counting down from 5 with 2 arrived
/// and 1 left to come.
pub(super) fn save_explorer(store: &MemoryPilots) {
    let mut screen = nebula_menu(store);
    create_held(&mut screen, "Ada");
    assert!(
        !game(&screen).pilot().expect("a pilot").control_bit(bit(10)),
        "a new pilot explores no nebula"
    );
    screen.quit();
    rewrite_save(store, "Ada", |fields| {
        fields.insert("explored".into(), serde_json::json!([128]));
        fields.insert(
            "missions".into(),
            serde_json::json!([{
                "slot": 0,
                "mission": 130,
                "destination": null,
                "return": null,
                "cargo_type": -1,
                "cargo_tons": 0,
                "pay": 0,
                "days_left": null,
                "deadline": null,
                "reached": false,
                "goals_met": true,
                "aboard": false,
                "failed": null,
                "ships": {"system": null, "left": 0, "arrived": 0, "appear": -1, "name": "", "subtitle": "", "destroyed": 0, "boarded": 0, "disabled": 0, "chased_off": 0, "uniform_type": null, "ship_done": false},
                "aux": {"countdown": 5, "arrived": 2, "left": 1},
            }]),
        );
    });
}

/// The held game's mission 130's auxiliary ships.
pub(super) fn held_aux(screen: &AppScreen) -> AuxShips {
    game(screen)
        .pilot()
        .expect("a pilot")
        .missions()
        .get(0)
        .expect("held")
        .aux
}

pub(super) fn nebula_explored(flight: &FlightView<Rc<GameData>>) -> bool {
    flight
        .session()
        .expect("a session")
        .galaxy()
        .nebula_explored(nova_sim::NebulaId(128))
}

#[test]
fn an_opened_pilot_returns_to_the_main_menu_loaded_with_its_nebulae_swept_and_aux_restarted() {
    let store = MemoryPilots::new();
    save_explorer(&store);
    let mut screen = nebula_menu(&store);
    let writes = store.writes();
    open_held(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.pilot_list().is_none(), "the list closed");
    assert!(screen.flight_view().is_none(), "not flying");
    assert_eq!(held_name(&screen), "Ada");
    let held = game(&screen);
    assert!(
        held.pilot().expect("a pilot").control_bit(bit(10)),
        "OnExplore ran"
    );
    assert!(nebula_explored(held));
    let aux = held_aux(&screen);
    assert!((70..140).contains(&aux.countdown), "{aux:?}");
    assert_eq!((aux.arrived, aux.left), (0, 1));
    assert_eq!(store.writes(), writes, "an open saves nothing");
}

#[test]
fn a_wrong_version_save_closes_the_list_and_shows_str_140_34_over_the_held_game() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    let newer = nova_sim::save::CURRENT + 1;
    store.put("Bea", &format!(r#"{{"version": {newer}}}"#));
    screen.take_warnings();
    choose(&mut screen, MenuChoice::OpenPilot);
    assert_eq!(screen.pilot_list().expect("open").keys(), ["Ada", "Bea"]);
    screen.input(&key(Key::Down, true));
    screen.input(&key(Key::Enter, true));
    assert!(screen.pilot_list().is_none(), "the list closed");
    assert_eq!(screen.showing(), Showing::Note);
    assert_eq!(screen.load_note().map(NoteDialog::text), Some(VERSION_NOTE));
    let warnings = screen.take_warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("nova: the pilot Bea in memory: "),
        "{warnings:?}"
    );
    assert_eq!(held_name(&screen), "Ada", "the game held as it was");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::MainMenu, "the note dismissed");
    assert_eq!(held_name(&screen), "Ada");
}

#[test]
fn without_dialogs_a_wrong_version_save_only_warns() {
    let store = MemoryPilots::new();
    let newer = nova_sim::save::CURRENT + 1;
    store.put("Bea", &format!(r#"{{"version": {newer}}}"#));
    let mut screen = menu(&store);
    screen.take_warnings();
    open_held(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    let warnings = screen.take_warnings();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert_eq!(warnings[1], VERSION_NOTE);
}

#[test]
fn an_unreadable_pilot_closes_the_list_silently_with_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    store.put("Broken", "not a save");
    screen.take_warnings();
    screen.take_sounds();
    choose(&mut screen, MenuChoice::OpenPilot);
    screen.input(&key(Key::Down, true));
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::MainMenu, "nothing shown");
    assert!(screen.pilot_list().is_none());
    let warnings = screen.take_warnings();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("nova: the pilot Broken in memory: "),
        "{warnings:?}"
    );
    assert_eq!(held_name(&screen), "Ada", "the game held as it was");
}

// Enter Ship.

#[test]
fn e_after_its_click_enters_play_with_the_loaded_pilot() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    ui_sounds(&mut screen);
    screen.input(&Input::Text('e'));
    assert_eq!(ui_sounds(&mut screen), [UiSound::ButtonKeyed]);
    assert_eq!(screen.showing(), Showing::MainMenu, "the click plays first");
    screen.keyed_click_ended();
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).name(), "Ada");
    assert!(screen.game_in_progress().is_none(), "moved into flight");
    assert_eq!(ui_sounds(&mut screen), [], "no refusal");
    screen.tick(TICK);
    assert!(flight(&screen).message().is_some(), "it ticks now");
}

#[test]
fn a_click_on_enter_ship_enters_at_once() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::EnterShip);
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).name(), "Ada");
}

#[test]
fn with_no_pilot_or_a_dying_one_enter_ship_plays_153_and_stays() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    ui_sounds(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(
        ui_sounds(&mut screen),
        [
            UiSound::ButtonDown,
            UiSound::ButtonUp,
            UiSound::EnterRefused
        ]
    );
    // A pilot whose ship breaks up, put away by Escape.
    let mut screen = super::death::landed_took_off_and_breaking_up(&store);
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    returned(&mut screen);
    assert!(game(&screen).is_dying());
    ui_sounds(&mut screen);
    choose(&mut screen, MenuChoice::EnterShip);
    assert_eq!(screen.showing(), Showing::MainMenu, "refused");
    assert!(screen.flight_view().is_none());
    assert_eq!(
        ui_sounds(&mut screen),
        [
            UiSound::ButtonDown,
            UiSound::ButtonUp,
            UiSound::EnterRefused
        ]
    );
}

#[test]
fn entering_replaces_a_developers_flight() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    screen.input(&key(Key::Tab, true));
    fly(&mut screen);
    assert_eq!(pilot(&screen).name(), "", "the developer's");
    assert_eq!(held_name(&screen), "Ada", "held through it");
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    enter_ship(&mut screen);
    assert_eq!(pilot(&screen).name(), "Ada");
    assert!(screen.game_in_progress().is_none());
    // Escape goes back to the menu, not to the developer's side.
    escape_to_menu(&mut screen);
    assert_eq!(held_name(&screen), "Ada");
}

#[test]
fn double_time_prefs_and_the_widened_size_reach_a_held_game() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    assert!(!game(&screen).double_time());
    screen.input(&key(Key::CapsLock, true));
    assert!(game(&screen).double_time());
    screen.set_widened_size((1366.0, 768.0));
    assert_eq!(game(&screen).view_size(), (1366.0, 768.0));
    let before = game(&screen).hyperspace_effects();
    open_prefs(&mut screen);
    let at = prefs_dialog(&screen)
        .dialog()
        .item_bounds(nova_view::ui::prefs::HYPERSPACE_EFFECTS_ITEM)
        .expect("the box")
        .center();
    click_at(&mut screen, at);
    assert_ne!(game(&screen).hyperspace_effects(), before);
}

#[test]
fn a_held_games_diagnostics_come_out_on_the_menu() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    screen.take_diagnostics();
    create_held(&mut screen, "Ada");
    assert!(
        !screen.take_diagnostics().is_empty(),
        "the fixture has no Red Alert icon"
    );
    assert_eq!(screen.take_diagnostics(), [], "taken");
}

// New Pilot over a game in progress.

#[test]
fn new_pilot_with_a_pilot_loaded_asks_str_140_8_first() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::NewPilot);
    assert_eq!(screen.showing(), Showing::Question);
    assert_eq!(
        screen.question().expect("asked").question(),
        NEW_PILOT_QUESTION
    );
    assert!(screen.new_pilot().is_none(), "not yet");
}

#[test]
fn no_leaves_the_loaded_pilot_as_it_was() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    let writes = store.writes();
    choose(&mut screen, MenuChoice::NewPilot);
    let at = screen
        .question()
        .expect("asked")
        .dialog()
        .item_bounds(NO_ITEM)
        .expect("No")
        .center();
    click_at(&mut screen, at);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(screen.new_pilot().is_none());
    assert_eq!(held_name(&screen), "Ada");
    assert_eq!(store.writes(), writes, "nothing written");
    assert!(!screen.take_quit());
}

#[test]
fn yes_opens_the_new_pilot_dialog() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::NewPilot);
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert!(!screen.take_quit(), "no quit");
    type_text(&mut screen, "Bea");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(held_name(&screen), "Bea", "in place of Ada");
    assert_eq!(store.keys(), ["Ada", "Bea"]);
}

#[test]
fn without_the_yes_no_dialog_new_pilot_opens_at_once() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create_held(&mut screen, "Ada");
    choose(&mut screen, MenuChoice::NewPilot);
    assert_eq!(screen.showing(), Showing::NewPilot);
}

// A return from play.

#[test]
fn a_return_and_enter_runs_no_on_explore_and_keeps_the_nebula_explored() {
    let store = MemoryPilots::new();
    save_explorer(&store);
    let mut screen = nebula_menu(&store);
    open_held(&mut screen);
    enter_ship(&mut screen);
    // Bit 10 cleared in play: a second OnExplore would set it again.
    edit(&mut screen, PilotEdit::Bit(bit(10), false));
    assert!(!pilot(&screen).control_bit(bit(10)));
    escape_to_menu(&mut screen);
    assert!(!game(&screen).pilot().expect("a pilot").control_bit(bit(10)));
    assert!(nebula_explored(game(&screen)));
    enter_ship(&mut screen);
    screen.tick(TICK);
    assert!(!pilot(&screen).control_bit(bit(10)), "no OnExplore ran");
    assert!(nebula_explored(flight(&screen)));
}

#[test]
fn quitting_on_the_menu_with_a_held_pilot_writes_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    escape_to_menu(&mut screen);
    let writes = store.writes();
    screen.quit();
    assert_eq!(store.writes(), writes);
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

// Escape in flight, by the rule.

#[test]
fn by_the_engine_escape_saves_and_returns_with_the_pilot_held_and_enter_resumes_it() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store);
    create(&mut screen, "Ada");
    screen.input(&key(Key::Up, true));
    screen.tick(TICK * 10);
    screen.input(&key(Key::Up, false));
    let left = ship(&screen).position;
    let writes = store.writes();
    escape_to_menu(&mut screen);
    assert_eq!(store.writes(), writes + 1, "saved once");
    assert!(screen.flight_view().is_none());
    assert_eq!(held_name(&screen), "Ada");
    enter_ship(&mut screen);
    assert_eq!(ship(&screen).position, left, "where it left off");
    assert_eq!(store.writes(), writes + 1);
}

#[test]
fn by_the_other_reading_escape_in_flight_does_nothing() {
    let store = MemoryPilots::new();
    let mut screen = menu(&store).with_rules(
        Rulebook::default().with_override(RuleKey::EscapeToMainScreen, RuleSource::Bible),
    );
    create(&mut screen, "Ada");
    let writes = store.writes();
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(store.writes(), writes, "nothing saved");
    screen.input(&key(Key::Char('m'), true));
    screen.input(&key(Key::Char('m'), false));
    assert_eq!(screen.showing(), Showing::FlightMap);
    screen.input(&key(Key::Escape, true));
    assert_eq!(
        screen.showing(),
        Showing::Flight,
        "Escape still closes the map"
    );
}

// X and About read the pilot held.

#[test]
fn x_expands_the_loaded_pilots_tags() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    create_held(&mut screen, "Ada");
    screen.input(&key(Key::Char('x'), true));
    screen.keyed_click_ended();
    assert_eq!(
        about(&screen).lines(),
        ["x 0", "x Ada1 First [Error]"],
        "the pilot's name and ship class, no mission"
    );
}

#[test]
fn the_about_text_resolves_its_conditional_text_for_the_loaded_pilot() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    screen.input(&key(Key::Char('a'), true));
    screen.keyed_click_ended();
    assert_eq!(about(&screen).lines()[3], "line 3", "before any pilot");
    screen.input(&key(Key::Enter, true));
    create_held(&mut screen, "Ada");
    screen.input(&key(Key::Char('a'), true));
    screen.keyed_click_ended();
    assert_eq!(about(&screen).lines()[2], "line 2 Ada [Error]");
    assert_eq!(about(&screen).lines()[3], "line 3 him", "Ada is male");
}

// P, A and X repeat on the main menu.

/// Closes the dialog open over the menu (the About or X text, or the
/// Preferences) with a click on its item 1 (Done, or OK).
fn close_by_click(screen: &mut AppScreen) {
    let dialog = match (screen.about(), screen.preferences()) {
        (Some(about), _) => about.dialog(),
        (None, Some(prefs)) => prefs.dialog(),
        (None, None) => panic!("nothing open"),
    };
    let at = dialog.item_bounds(1).expect("item 1").center();
    click_at(screen, at);
    assert_eq!(screen.showing(), Showing::MainMenu, "closed");
}

#[test]
fn held_p_a_and_x_repeat_once_their_dialog_closes() {
    let store = MemoryPilots::new();
    let mut screen = holding_menu(&store);
    for (k, shown) in [
        (Key::Char('a'), Showing::About),
        (Key::Char('x'), Showing::Note),
        (Key::Char('p'), Showing::Preferences),
    ] {
        ui_sounds(&mut screen);
        screen.input(&key(k, true));
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), shown, "{k:?}");
        screen.input(&held(k));
        assert_eq!(screen.showing(), shown, "the dialog drops it");
        close_by_click(&mut screen);
        ui_sounds(&mut screen);
        screen.input(&held(k));
        assert_eq!(
            ui_sounds(&mut screen),
            [UiSound::ButtonKeyed],
            "the repeat clicks ({k:?})"
        );
        screen.keyed_click_ended();
        assert_eq!(screen.showing(), shown, "and opens it again ({k:?})");
        close_by_click(&mut screen);
        screen.input(&key(k, false));
    }
}
