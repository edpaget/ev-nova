//! Use As My Ship's rename, as `_DoPlunderDialog` asks it
//! (@0x943a3-0x94596).

use nova_view::ui::text_input::{CANCEL_ITEM, OK_ITEM, PROMPT_ITEM};

use super::encounters::{assignment_click, boarding_data, captured, captured_over, drawn_at};
use super::*;

/// [`captured`], Use As My Ship clicked: the rename open.
fn naming(store: &MemoryPilots) -> AppScreen {
    let mut screen = captured(store);
    assignment_click(&mut screen, MY_SHIP_ITEM);
    assert_eq!(screen.showing(), Showing::CaptureNaming);
    screen
}

/// Clicks item `item` of the rename.
fn click_rename(screen: &mut AppScreen, item: usize) {
    let at = screen
        .capture_naming()
        .expect("the rename is open")
        .dialog()
        .item_bounds(item)
        .expect("an item")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at,
        });
    }
}

/// The rename's prompt, as its lines show it.
fn prompt_shown(screen: &AppScreen) -> String {
    screen
        .capture_naming()
        .expect("the rename is open")
        .dialog()
        .text(PROMPT_ITEM)
        .expect("the prompt")
        .lines()
        .join(" ")
}

/// The rename's text.
fn typed(screen: &AppScreen) -> String {
    screen
        .capture_naming()
        .expect("the rename is open")
        .field()
        .text()
        .to_owned()
}

#[test]
fn use_as_my_ship_opens_the_rename_with_119_and_the_captured_class_name() {
    let store = MemoryPilots::new();
    let screen = naming(&store);
    assert!(screen.assignment().is_none(), "the assignment gone");
    assert!(screen.plunder().is_some(), "the plunder dialog kept");
    assert_eq!(prompt_shown(&screen), "m119");
    let default = typed(&screen);
    let digits = default.strip_prefix("Trader ").expect("the class's Name");
    assert_eq!(digits.len(), 3, "{default:?}");
    assert!(
        digits.chars().all(|c| ('1'..='9').contains(&c)),
        "{default:?}"
    );
    assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(128), "not yet");
}

#[test]
fn the_rename_is_drawn_over_the_plunder_dialog_and_flight_pauses() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    let plunder = drawn_at(&screen, "Select what to plunder").expect("the plunder drawn");
    let prompt = drawn_at(&screen, "m119").expect("the prompt drawn");
    assert!(plunder < prompt, "the rename over it");
    let alpha = flight(&screen).alpha();
    screen.tick(TICK / 2);
    assert_eq!(
        flight(&screen).alpha().to_bits(),
        alpha.to_bits(),
        "flight is paused"
    );
}

#[test]
fn return_confirms_the_default_and_flies_the_ship_so_named() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    let default = typed(&screen);
    screen.input(&key(Key::Enter, true));
    assert!(screen.capture_naming().is_none(), "closed");
    assert!(screen.plunder().is_none(), "the plunder dialog closed");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(129));
    assert_eq!(pilot(&screen).ship_name(), Some(default.as_str()));
    assert_eq!(
        flight(&screen).message(),
        Some("You retained your old ship as an escort.")
    );
    assert_eq!(saved(&store, "Ada").ship_name(), Some(default.as_str()));
}

#[test]
fn a_name_typed_and_ok_is_kept_unculled_and_saved() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    type_text(&mut screen, "The Prize");
    click_rename(&mut screen, OK_ITEM);
    assert_eq!(screen.showing(), Showing::Flight);
    assert!(screen.plunder().is_none(), "the plunder dialog closed");
    assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(129));
    assert_eq!(pilot(&screen).ship_name(), Some("The Prize"));
    let kept = saved(&store, "Ada");
    assert_eq!(kept.ship(), nova_sim::ShipId(129));
    assert_eq!(kept.ship_name(), Some("The Prize"));
}

#[test]
fn cancel_loses_the_prize_says_122_and_saves_nothing() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    let name = pilot(&screen).ship_name().map(str::to_owned);
    let writes = store.writes();
    click_rename(&mut screen, CANCEL_ITEM);
    assert!(screen.capture_naming().is_none(), "closed");
    assert!(screen.plunder().is_none(), "the plunder dialog closed");
    assert_eq!(screen.showing(), Showing::Flight);
    assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(128));
    assert_eq!(pilot(&screen).ship_name(), name.as_deref());
    assert!(pilot(&screen).escorts().is_empty());
    assert_eq!(
        flight(&screen).message(),
        Some("You decided not to capture this ship after all.")
    );
    assert_eq!(store.writes(), writes, "nothing saved");
}

#[test]
fn escape_does_nothing_in_the_rename() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    screen.input(&key(Key::Escape, true));
    screen.input(&key(Key::Escape, false));
    assert_eq!(screen.showing(), Showing::CaptureNaming);
    assert_eq!(pilot(&screen).ship(), nova_sim::ShipId(128));
}

#[test]
fn use_as_my_ship_by_its_key_types_nothing_into_the_rename() {
    let store = MemoryPilots::new();
    let mut screen = captured(&store);
    screen.input(&key(Key::Char('m'), true));
    screen.input(&Input::Text('m'));
    assert_eq!(screen.showing(), Showing::CaptureNaming);
    assert!(typed(&screen).starts_with("Trader "), "the M typed nothing");
    screen.input(&Input::Text('x'));
    assert_eq!(typed(&screen), "x", "the next text replaces the default");
}

#[test]
fn use_as_my_ship_by_a_click_keeps_the_first_text_typed() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    screen.input(&Input::Text('m'));
    assert_eq!(typed(&screen), "m");
}

#[test]
fn the_renames_beep_and_clicks_are_kept() {
    let too_long = "a".repeat(nova_sim::shipyard::SHIP_NAME_CHARS + 1);
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    screen.take_sounds();
    type_text(&mut screen, &too_long);
    screen.input(&key(Key::Enter, true));
    assert!(screen.capture_naming().is_some(), "refused: at most 64");
    type_text(&mut screen, "b");
    screen.input(&key(Key::Enter, true));
    assert!(screen.capture_naming().is_none(), "closed");
    assert!(screen.take_sounds().contains(&Sound::Ui(UiSound::Alert)));
}

/// [`Dialogs`], without "Text Input".
struct WithoutTextInput;

impl DialogResources for WithoutTextInput {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == nova_view::ui::text_input::TEXT_INPUT_DIALOG {
            return Err(format!("no DLOG {id}"));
        }
        Dialogs.dialog_template(id)
    }
}

#[test]
fn without_a_text_input_template_the_built_in_rename_opens_with_a_warning() {
    let store = MemoryPilots::new();
    let base = AppScreen::new(boarding_data())
        .with_dialogs(Rc::new(WithoutTextInput), Rc::new(MonoMetrics));
    let mut screen = captured_over(&store, base);
    screen.take_warnings();
    assignment_click(&mut screen, MY_SHIP_ITEM);
    assert_eq!(screen.showing(), Showing::CaptureNaming);
    assert_eq!(prompt_shown(&screen), "m119");
    assert_eq!(
        screen.take_warnings(),
        ["nova: using the built-in Text Input dialog: no DLOG 3001"]
    );
}

#[test]
fn dropping_flight_drops_the_rename() {
    let store = MemoryPilots::new();
    let mut screen = naming(&store);
    screen.drop_flight();
    assert!(screen.capture_naming().is_none());
    assert!(screen.plunder().is_none());
}
