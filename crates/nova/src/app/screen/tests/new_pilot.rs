//! The New Pilot dialog's name and nickname, as `_DoNewPilot` opens and
//! keeps them (@0x18b44-0x18ba7, @0x18bd6-0x18c03), and its question
//! over a pilot already saved under the name (@0x18c08-0x18c63).

use std::cell::RefCell;

use nova_sim::{RuleKey, RuleSource, Rulebook};
use nova_view::flight::SharedChance;
use nova_view::ui::new_pilot::NAME_TAKEN;
use nova_view::ui::text_input::CANCEL_ITEM;
use nova_view::ui::yes_no::{NO_ITEM, REPLACE_PILOT_QUESTION};

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

/// [`data`], with `STR#` 128 holding the stock names: #1-#3 the pilot's,
/// #4-#6 the nickname's, #7-#9 the ship's.
fn named_data() -> Rc<GameData> {
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
    let built = game_data_fork(false, false, false, 100, b"", b"", None)
        .resource(StrList::TYPE, 128, None, &str_list(&names))
        .build();
    loaded_bytes(built.bytes)
}

/// The router over `data` keeping pilots in `store`, with dialogs,
/// rolling `roll` every time, past its appear.
fn new_pilot_menu(data: Rc<GameData>, store: &MemoryPilots, roll: u16) -> AppScreen {
    with_dialogs(data)
        .with_chance(SharedChance::new(Rc::new(RefCell::new(Rolls(roll)))))
        .with_pilots(Some(keeper(store)), Rc::new(MonoMetrics))
        .appeared()
}

/// New Pilot from the main menu (answering yes to `STR#` 140 #8 over a
/// game held), its name replaced by `name` and its nickname by
/// `nickname`, then Return with its typed carriage return.
fn confirm_new_pilot(screen: &mut AppScreen, name: &str, nickname: &str) {
    assert_eq!(choose(screen, MenuChoice::NewPilot), ScreenAction::None);
    if screen.showing() == Showing::Question {
        screen.input(&key(Key::Enter, true));
        screen.input(&Input::Text('\r'));
    }
    assert_eq!(screen.showing(), Showing::NewPilot);
    // The name opens selected whole: typing replaces it.
    type_text(screen, name);
    screen.input(&key(Key::Tab, true));
    type_text(screen, nickname);
    screen.input(&key(Key::Enter, true));
    screen.input(&Input::Text('\r'));
}

/// Saves a pilot named `name` with nickname `nickname` in `store`, as
/// an earlier New Pilot would have.
fn save_pilot(store: &MemoryPilots, data: &GameData, name: &str, nickname: &str) {
    let pilot = nova_sim::Pilot::new(data, name)
        .expect("starts")
        .with_nickname(nickname.to_owned());
    keeper(store).save(&pilot).expect("saves");
}

/// The pilot of the game the main menu holds.
fn held(screen: &AppScreen) -> &nova_sim::Pilot {
    screen
        .game
        .as_ref()
        .expect("a game held")
        .pilot()
        .expect("a pilot")
}

/// Clicks item `item` of the open question.
fn answer(screen: &mut AppScreen, item: usize) {
    let at = screen
        .question()
        .expect("asked")
        .dialog()
        .item_bounds(item)
        .expect("the item")
        .center();
    click_at(screen, at);
}

#[test]
fn new_pilot_opens_with_the_str_128_defaults_the_chance_picks() {
    let store = MemoryPilots::new();
    for (roll, name, nickname) in [
        (0, "Shane Merrol", "Hunter"),
        (2, "Goroth Obarskyr", "Maverick"),
    ] {
        let mut screen = new_pilot_menu(named_data(), &store, roll);
        choose(&mut screen, MenuChoice::NewPilot);
        let dialog = screen.new_pilot().expect("open");
        assert_eq!(dialog.name(), name);
        assert_eq!(dialog.nickname(), nickname);
        assert!(dialog.field().selected(), "the name selected whole");
    }
    let mut screen = new_pilot_menu(data(), &store, 1);
    choose(&mut screen, MenuChoice::NewPilot);
    let dialog = screen.new_pilot().expect("open");
    assert_eq!((dialog.name(), dialog.nickname()), ("", ""), "no STR# 128");
}

#[test]
fn the_name_and_nickname_are_kept_culled_then_trimmed() {
    let store = MemoryPilots::new();
    let mut screen = new_pilot_menu(named_data(), &store, 0);
    confirm_new_pilot(&mut screen, "The Ace;x", "THE Hunter ;; ;");
    christen(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(held(&screen).name(), "Ace");
    assert_eq!(held(&screen).nickname(), "Hunter");
    assert_eq!(store.keys(), ["Ace"]);
}

#[test]
fn a_name_culled_or_trimmed_to_nothing_is_refused() {
    let store = MemoryPilots::new();
    let mut screen = new_pilot_menu(named_data(), &store, 0);
    confirm_new_pilot(&mut screen, "the ;x", "");
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert_eq!(
        screen.new_pilot().expect("open").refusal(),
        Some(UNUSABLE_NAME)
    );
    assert_eq!(store.writes(), 0);
}

#[test]
fn the_nickname_typed_survives_a_save_and_a_reopen() {
    let store = MemoryPilots::new();
    let mut screen = new_pilot_menu(named_data(), &store, 0);
    confirm_new_pilot(&mut screen, "Ada", "Ace");
    christen(&mut screen);
    assert_eq!(held(&screen).nickname(), "Ace");
    assert_eq!(saved(&store, "Ada").nickname(), "Ace");
    let mut reopened = new_pilot_menu(named_data(), &store, 0);
    open_held(&mut reopened);
    assert_eq!(held(&reopened).nickname(), "Ace");
}

#[test]
fn a_name_already_saved_asks_str_140_9_with_the_new_pilot_dialog_closed() {
    let store = MemoryPilots::new();
    let data = named_data();
    save_pilot(&store, &data, "Ada", "Old");
    let mut screen = new_pilot_menu(data, &store, 0);
    confirm_new_pilot(&mut screen, "ada", "New");
    assert_eq!(screen.showing(), Showing::Question);
    assert!(screen.new_pilot().is_none(), "closed");
    assert_eq!(
        screen.question().expect("asked").question(),
        REPLACE_PILOT_QUESTION
    );
}

#[test]
fn no_returns_to_the_main_menu_with_nothing_changed() {
    let store = MemoryPilots::new();
    let data = named_data();
    let mut screen = new_pilot_menu(Rc::clone(&data), &store, 0);
    confirm_new_pilot(&mut screen, "Bob", "");
    christen(&mut screen);
    save_pilot(&store, &data, "Ada", "Old");
    let ada = store.text("Ada").expect("saved");
    let writes = store.writes();
    confirm_new_pilot(&mut screen, "Ada", "New");
    assert_eq!(screen.showing(), Showing::Question);
    answer(&mut screen, NO_ITEM);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.text("Ada"), Some(ada), "byte for byte");
    assert_eq!(store.keys(), ["Ada", "Bob"]);
    assert_eq!(store.writes(), writes);
    assert_eq!(held(&screen).name(), "Bob", "the game held stays held");
}

#[test]
fn yes_christens_the_new_pilot_and_replaces_the_saved_one() {
    let store = MemoryPilots::new();
    let data = named_data();
    save_pilot(&store, &data, "Ada", "Old");
    let mut screen = new_pilot_menu(data, &store, 0);
    confirm_new_pilot(&mut screen, "Ada", "New");
    screen.input(&key(Key::Enter, true));
    screen.input(&Input::Text('\r'));
    assert_eq!(screen.showing(), Showing::Christening);
    assert_eq!(
        screen.christening().expect("open").field().text(),
        "Ring of Glory",
        "the key that answered types nothing"
    );
    christen(&mut screen);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.keys(), ["Ada"], "one pilot listed");
    assert_eq!(saved(&store, "Ada").nickname(), "New");
    assert_eq!(held(&screen).nickname(), "New");
}

#[test]
fn yes_then_cancelling_the_christening_leaves_the_saved_pilot() {
    let store = MemoryPilots::new();
    let data = named_data();
    save_pilot(&store, &data, "Ada", "Old");
    let ada = store.text("Ada").expect("saved");
    let mut screen = new_pilot_menu(data, &store, 0);
    confirm_new_pilot(&mut screen, "Ada", "New");
    screen.input(&key(Key::Enter, true));
    assert_eq!(screen.showing(), Showing::Christening);
    let at = screen
        .christening()
        .expect("open")
        .dialog()
        .item_bounds(CANCEL_ITEM)
        .expect("Cancel")
        .center();
    click_at(&mut screen, at);
    assert_eq!(screen.showing(), Showing::MainMenu);
    assert_eq!(store.text("Ada"), Some(ada));
    assert_eq!(store.keys(), ["Ada"]);
    assert!(screen.game.is_none(), "the new pilot is abandoned");
}

#[test]
fn replacing_a_pilot_saved_in_another_case_leaves_one_listed() {
    let store = MemoryPilots::new();
    let data = named_data();
    save_pilot(&store, &data, "shane", "Old");
    let mut screen = new_pilot_menu(data, &store, 0);
    confirm_new_pilot(&mut screen, "Shane", "New");
    screen.input(&key(Key::Enter, true));
    christen(&mut screen);
    assert_eq!(store.keys(), ["Shane"]);
    assert_eq!(saved(&store, "Shane").nickname(), "New");
    assert_eq!(screen.take_warnings(), Vec::<String>::new());
}

#[test]
fn by_the_other_reading_a_name_already_saved_is_refused_in_the_dialog() {
    let store = MemoryPilots::new();
    let data = named_data();
    save_pilot(&store, &data, "Ada", "Old");
    let mut screen = new_pilot_menu(data, &store, 0).with_rules(
        Rulebook::default().with_override(RuleKey::NewPilotNameTaken, RuleSource::Bible),
    );
    confirm_new_pilot(&mut screen, "ada", "New");
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert!(screen.question().is_none(), "nothing asked");
    assert_eq!(
        screen.new_pilot().expect("open").refusal(),
        Some(NAME_TAKEN)
    );
    assert_eq!(store.keys(), ["Ada"]);
}

/// [`Dialogs`], without the Yes/No dialog.
struct WithoutYesNo;

impl DialogResources for WithoutYesNo {
    fn dialog_template(&self, id: i16) -> Result<DialogTemplate, String> {
        if id == YES_NO_DIALOG {
            return Err(format!("no DLOG {id}"));
        }
        Dialogs.dialog_template(id)
    }
}

#[test]
fn without_the_yes_no_dialog_a_name_already_saved_is_refused() {
    let store = MemoryPilots::new();
    let data = named_data();
    save_pilot(&store, &data, "Ada", "Old");
    let mut screen = AppScreen::new(data)
        .with_dialogs(Rc::new(WithoutYesNo), Rc::new(MonoMetrics))
        .with_pilots(Some(keeper(&store)), Rc::new(MonoMetrics))
        .appeared();
    confirm_new_pilot(&mut screen, "Ada", "New");
    assert_eq!(screen.showing(), Showing::NewPilot);
    assert_eq!(
        screen.new_pilot().expect("open").refusal(),
        Some(NAME_TAKEN)
    );
}
