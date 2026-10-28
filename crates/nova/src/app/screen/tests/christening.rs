//! The new pilot's ship christening, as `_DoNewPilot` asks it
//! (@0x18cca-0x18d76, @0x19189-0x191ae).

use std::cell::RefCell;

use nova_view::flight::SharedChance;
use nova_view::ui::text_input::{CANCEL_ITEM, OK_ITEM, PROMPT_ITEM};

use super::*;

/// A chance whose every roll is `.0`, and that fires nothing.
#[derive(Default)]
struct Rolls(u16);

impl Chance for Rolls {
    fn fires(&mut self, _percent: u8) -> bool {
        false
    }

    fn below(&mut self, n: u32) -> u32 {
        n - 1
    }

    fn roll(&mut self, _sides: u16) -> u16 {
        self.0
    }
}

/// The stock prompt, `STR#` 2002 #121.
const PROMPT: &str = "Now, please christen your brand-new";

/// [`data`], with `STR#` 2002 holding the prompt at #121, `STR#` 128 the
/// stock names (#7-#9 the ship's), `STR#` 25040 "Renamed" for a `T`, ship
/// 128's `LongName` "Kestrel", and the `chär`'s `OnStart` `on_start`.
fn christening_data(on_start: &[u8]) -> Rc<GameData> {
    let mut prompts = vec![""; 120];
    prompts.push(PROMPT);
    let names = [
        "Shane Merrol",
        "Cade Connelly",
        "Goroth Obarskyr",
        "Hunter",
        "Hawkeye",
        "Maverick",
        "Ring of Glory",
        "Snowy Owl",
        "Cardinal Virtue",
    ];
    let mut built = game_data_fork(false, false, false, 100, on_start, b"", None)
        .resource(StrList::TYPE, 2002, None, &str_list(&prompts))
        .resource(StrList::TYPE, 128, None, &str_list(&names))
        .resource(StrList::TYPE, 25040, None, &str_list(&["Renamed"]))
        .build();
    let first = built.layout.data_entry(Ship::TYPE, 128) + 4;
    for (at, byte) in b"Kestrel".iter().enumerate() {
        built.put_u8(first + 0x62E + at, *byte);
    }
    loaded_bytes(built.bytes)
}

/// [`Dialogs`], without "Text Input".
struct WithoutTextInput;

impl DialogResources for WithoutTextInput {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == TEXT_INPUT_DIALOG {
            return Err(format!("no DLOG {id}"));
        }
        Dialogs.dialog_template(id)
    }
}

/// The router over `data` keeping pilots in `store`, with every dialog
/// the christening needs, rolling on `chance`, past its appear.
fn christening_menu(data: Rc<GameData>, store: &MemoryPilots, chance: Rolls) -> AppScreen {
    AppScreen::new(data)
        .with_dialogs(Rc::new(Dialogs), Rc::new(MonoMetrics))
        .with_chance(SharedChance::new(Rc::new(RefCell::new(chance))))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

/// New Pilot, `name` and Return, with its typed carriage return, as the
/// platform sends it: the christening, if it opens.
fn name_pilot(screen: &mut AppScreen, name: &str) {
    assert_eq!(choose(screen, MenuChoice::NewPilot), ScreenAction::None);
    if screen.showing() == Showing::Question {
        screen.input(&key(Key::Enter, true));
        screen.input(&Input::Text('\r'));
    }
    assert_eq!(screen.showing(), Showing::NewPilot);
    type_text(screen, name);
    screen.input(&key(Key::Enter, true));
    screen.input(&Input::Text('\r'));
}

/// Clicks item `item` of the christening.
fn click_christening(screen: &mut AppScreen, item: usize) {
    let at = screen
        .christening()
        .expect("christening")
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

/// The christening dialog's prompt, as its lines show it.
fn prompt_shown(screen: &AppScreen) -> String {
    screen
        .christening()
        .expect("christening")
        .dialog()
        .text(PROMPT_ITEM)
        .expect("the prompt")
        .lines()
        .join(" ")
}

fn click_christening_cancel(screen: &mut AppScreen) {
    click_christening(screen, CANCEL_ITEM);
}

#[test]
fn a_named_pilot_is_asked_to_christen_its_ship_with_str_128_7_by_default() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Christening);
    assert!(screen.new_pilot().is_none(), "the name is taken");
    assert_eq!(prompt_shown(&screen), format!("{PROMPT} Kestrel: "));
    let asking = screen.christening().expect("christening");
    assert_eq!(asking.field().text(), "Ring of Glory");
    assert!(asking.field().selected(), "selected whole");
    assert!(store.keys().is_empty(), "nothing saved yet");
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn the_christenings_default_is_rolled_on_the_routers_chance() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls(2));
    name_pilot(&mut screen, "Ada");
    let asking = screen.christening().expect("christening");
    assert_eq!(asking.field().text(), "Cardinal Virtue");
}

#[test]
fn the_name_confirmed_culled_is_the_ships_and_is_saved() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.take_sounds();
    type_text(&mut screen, "The Raven");
    screen.input(&key(Key::Enter, true));
    assert!(screen.christening().is_none(), "closed");
    assert_eq!(screen.showing(), Showing::MainMenu);
    let held = screen
        .game_in_progress()
        .expect("held")
        .sim()
        .pilot()
        .expect("a pilot");
    assert_eq!(held.ship_name(), Some("Raven"));
    assert_eq!(held.name(), "Ada");
    assert_eq!(saved(&store, "Ada").ship_name(), Some("Raven"));
}

#[test]
fn confirming_the_default_names_the_ship_after_it() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.input(&key(Key::Enter, true));
    assert_eq!(saved(&store, "Ada").ship_name(), Some("Ring of Glory"));
}

#[test]
fn cancelling_the_christening_abandons_the_pilot_and_saves_nothing() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.take_sounds();
    click_christening_cancel(&mut screen);
    assert!(screen.christening().is_none(), "closed");
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert!(store.keys().is_empty(), "nothing saved");
    assert!(screen.game_in_progress().is_none(), "no game held");
}

#[test]
fn cancelling_the_christening_keeps_the_game_held_before() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.input(&key(Key::Enter, true));
    let writes = store.writes();
    name_pilot(&mut screen, "Bea");
    assert_eq!(screen.showing(), Showing::Christening);
    click_christening_cancel(&mut screen);
    let held = screen
        .game_in_progress()
        .expect("held")
        .sim()
        .pilot()
        .expect("a pilot");
    assert_eq!(held.name(), "Ada", "the game held before");
    assert_eq!(store.keys(), ["Ada"]);
    assert_eq!(store.writes(), writes, "nothing saved over");
}

#[test]
fn the_christenings_beep_is_kept_when_it_is_confirmed_or_cancelled() {
    let too_long = "a".repeat(nova_sim::shipyard::SHIP_NAME_CHARS + 1);
    for confirm in [true, false] {
        let store = MemoryPilots::new();
        let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
        name_pilot(&mut screen, "Ada");
        screen.take_sounds();
        type_text(&mut screen, &too_long);
        screen.input(&key(Key::Enter, true));
        assert!(screen.christening().is_some(), "refused");
        if confirm {
            type_text(&mut screen, "b");
            screen.input(&key(Key::Enter, true));
        } else {
            click_christening_cancel(&mut screen);
        }
        assert!(screen.christening().is_none(), "closed");
        assert!(
            screen.take_sounds().contains(&Sound::Ui(UiSound::Alert)),
            "the beep is kept ({confirm})"
        );
    }
}

#[test]
fn the_chärs_on_start_runs_after_the_christening() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b"T25040"), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    type_text(&mut screen, "Raven");
    screen.input(&key(Key::Enter, true));
    let held = screen
        .game_in_progress()
        .expect("held")
        .sim()
        .pilot()
        .expect("a pilot");
    assert_eq!(held.ship_name(), Some("Renamed"), "OnStart renames it");
    assert_eq!(saved(&store, "Ada").ship_name(), Some("Renamed"));
}

#[test]
fn the_key_that_opened_the_christening_types_nothing() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.input(&Input::Text('x'));
    let asking = screen.christening().expect("christening");
    assert_eq!(asking.field().text(), "x", "the Return typed nothing");
}

#[test]
fn a_christening_opened_by_a_click_keeps_the_first_text_typed() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    choose(&mut screen, MenuChoice::NewPilot);
    type_text(&mut screen, "Ada");
    let ok = screen
        .new_pilot()
        .expect("open")
        .dialog()
        .item_bounds(1)
        .expect("OK")
        .center();
    for pressed in [true, false] {
        screen.input(&Input::PointerButton {
            button: MouseButton::Left,
            pressed,
            at: ok,
        });
    }
    assert_eq!(screen.showing(), Showing::Christening);
    type_text(&mut screen, "xy");
    let asking = screen.christening().expect("christening");
    assert_eq!(asking.field().text(), "xy", "nothing flushed");
    click_christening(&mut screen, OK_ITEM);
    assert_eq!(saved(&store, "Ada").ship_name(), Some("xy"));
}

#[test]
fn escape_does_nothing_in_the_christening() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.input(&key(Key::Escape, true));
    assert_eq!(screen.showing(), Showing::Christening);
    assert!(store.keys().is_empty());
}

#[test]
fn without_a_text_input_template_the_built_in_christening_opens_with_a_warning() {
    let store = MemoryPilots::new();
    let mut screen = AppScreen::new(christening_data(b""))
        .with_dialogs(Rc::new(WithoutTextInput), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    name_pilot(&mut screen, "Ada");
    assert_eq!(screen.showing(), Showing::Christening);
    assert_eq!(
        screen.christening().expect("open").field().text(),
        "Ring of Glory"
    );
    assert_eq!(
        screen.take_warnings(),
        ["nova: using the built-in Text Input dialog: no DLOG 3001"]
    );
    let mut without = AppScreen::new(christening_data(b""))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    name_pilot(&mut without, "Bea");
    assert_eq!(without.showing(), Showing::Christening, "no dialogs at all");
    assert_eq!(without.take_warnings(), Vec::<String>::new());
}

/// Every text `screen` draws.
fn christening_texts(screen: &AppScreen) -> Vec<String> {
    drawn(screen)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_open_christening_is_drawn_with_its_prompt_and_default_name() {
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    let texts = christening_texts(&screen);
    let lines = screen
        .christening()
        .expect("christening")
        .dialog()
        .text(PROMPT_ITEM)
        .expect("the prompt")
        .lines()
        .to_vec();
    assert_eq!(lines.join(" "), format!("{PROMPT} Kestrel: "));
    for line in &lines {
        assert!(texts.contains(line), "{line:?} drawn in {texts:?}");
    }
    assert!(
        texts.iter().any(|text| text.contains("Ring of Glory")),
        "the default name drawn in {texts:?}"
    );
}

#[test]
fn the_christenings_beep_is_collected_while_it_is_open() {
    let too_long = "a".repeat(nova_sim::shipyard::SHIP_NAME_CHARS + 1);
    let store = MemoryPilots::new();
    let mut screen = christening_menu(christening_data(b""), &store, Rolls::default());
    name_pilot(&mut screen, "Ada");
    screen.take_sounds();
    type_text(&mut screen, &too_long);
    screen.input(&key(Key::Enter, true));
    assert!(screen.christening().is_some(), "still open");
    assert!(screen.take_sounds().contains(&Sound::Ui(UiSound::Alert)));
}
