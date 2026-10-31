//! The `dësc` movies outside the missions over the stock data
//! (rdm:phase/movies/phase-4-desc-movies-outside-missions): of every
//! stock `dësc` only Aurora's (338), the four races' (32760-32763) and
//! 4778 (`mïsn` 906's) name a movie, and only 4778 has `Flags` 0x0001: its
//! `MovieFile` is the one byte 0x01, which no Nova Files ships, so a
//! `STR#` text after it (a shop's, the escorts' report) starts a movie
//! that finds no file and plays nothing; the About text (32767) and the X
//! text (32766) name none; and the stock first `chär`'s `IntroTextID` is
//! -1, so its intro shows no text. No stock `dësc` has the cinematic flag
//! (`Flags` 0x0004, rdm:phase/movies/phase-8-cinematic-movie-widened).
//!
//! Driven with movies on, About, X and an entry through the stock intro
//! start no movie. A landing at Aurora, whose `dësc` names a movie that
//! isn't an after-movie, leaves the session's after-movie, which every
//! shop's `STR#` text is handed, at none; and the item 1 texts stock data
//! reaches start no movie once they close: at Aurora the bar's nothing
//! for hire (#224), the escorts' landing report and a sale alert (#207),
//! and an empty shipyard's #223 where the stock data has one (Aurora's
//! lists ships). No stock outfitter lists nothing, so #206 is out of
//! reach. The positive cases are pinned over fixture data (the `nova`
//! crate's router tests). Skips, passing, when `NOVA_DATA` or the
//! interface file is missing.

use crate::common;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::common::EntryHarness;
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::records::desc::Desc;
use nova_data::{GameData, Record};
use nova_render::FontFaces;
use nova_render::recording::RecordingGpu;
use nova_render::wgpu::GlyphonMetrics;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{
    Pilot, PilotCatalog, PilotKeeper, PilotStore, RuleKey, RuleSource, Rulebook, SystemId,
};
use nova_view::flight::SharedChance;
use nova_view::menu::MenuChoice;
use nova_view::menu::intro::IntroSource;
use nova_view::movie::MoviePlayer;
use nova_view::movie::fixture::MockMoviePlayer;
use nova_view::ui::catalog::DescriptionGraphics;
use nova_view::{Key, MenuCommand, MouseButton, Prefs};

struct Window;

impl WindowPort for Window {
    fn size_px(&self) -> (u32, u32) {
        (1024, 768)
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }

    fn request_redraw(&mut self) {}
}

/// Aurora, whose `dësc` names "flyby.qt" with `Flags` 0.
const AURORA: i16 = 338;

/// The `dësc` of `mïsn` 906 (3872 + 906), the one stock `dësc` with
/// `Flags` 0x0001 and a `MovieFile`: the byte 0x01.
const MISSION_906: i16 = 4778;

#[test]
fn only_4778_has_an_after_movie_of_no_file_and_about_x_and_the_intro_name_none() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let ids = data.ids(Desc::TYPE).to_vec();
    assert!(ids.len() > 1000, "precondition: the stock dësc");
    let filmed: Vec<(i16, nova_sim::DescMovie)> = ids
        .iter()
        .filter_map(|&id| data.description_movie(id).map(|movie| (id, movie)))
        .collect();
    assert_eq!(
        filmed.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [AURORA, MISSION_906, 32760, 32761, 32762, 32763]
    );
    let after: Vec<_> = filmed.iter().filter(|(_, movie)| movie.after).collect();
    assert_eq!(after.len(), 1, "{filmed:?}");
    let (id, movie) = after[0];
    assert_eq!(*id, MISSION_906);
    assert_eq!(
        movie.file, "\u{1}",
        "its MovieFile is the one byte 0x01: not empty, so the after-check \
         plays it (@0x5b40b-0x5b44f)"
    );
    assert!(
        !dir.join(&movie.file).exists(),
        "no such file: _PlayQTMovie finds none and plays nothing (@0x559ff)"
    );
    assert_eq!(data.description_movie(32767), None, "About");
    assert_eq!(data.description_movie(32766), None, "X");
    let intro = data.first_character_intro().expect("the first chär");
    assert_eq!(intro.text, None, "IntroTextID -1 (@0x170ad)");
}

#[test]
fn no_stock_desc_is_cinematic() {
    // rdm:phase/movies/phase-8-cinematic-movie-widened records this: no
    // stock movie fades the screen or covers the view, so the cinematic
    // staging over a widened view is pinned over fixture data.
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let ids = data.ids(Desc::TYPE).to_vec();
    assert!(ids.len() > 1000, "precondition: the stock dësc");
    let cinematic: Vec<i16> = ids
        .iter()
        .copied()
        .filter(|&id| match data.get::<Desc>(id) {
            Some(Ok(desc)) => desc.record.flags.0 & 0x0004 != 0,
            other => panic!("dësc {id} doesn't decode: {other:?}"),
        })
        .collect();
    assert_eq!(cinematic, Vec::<i16>::new(), "dësc with Flags 0x0004");
    let filmed: Vec<i16> = ids
        .iter()
        .copied()
        .filter(|&id| {
            data.description_movie(id)
                .is_some_and(|movie| movie.cinematic)
        })
        .collect();
    assert_eq!(
        filmed,
        Vec::<i16>::new(),
        "dësc whose movie is staged as cinematic"
    );
}

/// The app driven as the window drives it, its movies played through a
/// mock player.
struct Driver {
    app: App<Rc<GameData>>,
    gpu: RecordingGpu,
    clock: Duration,
}

impl Driver {
    fn send(&mut self, event: WindowEvent) {
        let control = self.app.handle(event, &mut Window, &mut self.gpu);
        assert_eq!(control, Control::Continue, "{event:?}");
    }

    fn press(&mut self, key: Key) {
        for pressed in [true, false] {
            self.send(WindowEvent::Key {
                key,
                pressed,
                repeat: false,
            });
        }
    }

    fn click(&mut self, at: nova_view::Point) {
        self.send(WindowEvent::PointerMoved {
            px: (f64::from(at.x), f64::from(at.y)),
        });
        for pressed in [true, false] {
            self.send(WindowEvent::PointerButton {
                button: MouseButton::Left,
                pressed,
            });
        }
    }

    fn redraw(&mut self) {
        self.clock += Duration::from_millis(50);
        self.send(WindowEvent::Redraw {
            elapsed: self.clock,
        });
    }

    fn showing(&self) -> Showing {
        self.app.screen().showing()
    }
}

crate::common::entry_harness!(Driver, click, redraw);

/// The app over the stock data and interface file, movies on and played
/// through `player`, the reopened pilot's intro read as `intro`, with
/// a new stock pilot saved over `at` (its system, its stellar) when
/// given, as `kit` edits its save, and drawing on `chance` when given (by
/// default, nothing random fires); on the main menu. `None` when the
/// interface file is missing.
fn stock_app(
    player: &Rc<MockMoviePlayer>,
    intro: RuleSource,
    at: Option<(SystemId, i16)>,
    kit: impl FnOnce(&mut serde_json::Value),
    chance: Option<SharedChance>,
) -> Option<(Driver, Rc<GameData>)> {
    let dir = common::nova_data()?;
    let interface = match nova_data::open_interface(&dir) {
        Ok(interface) => interface,
        Err(error) => {
            eprintln!("skipping: {error}");
            return None;
        }
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let mut pilot = Pilot::new(data.as_ref(), "Filmgoer").expect("the stock first chär starts");
    if let Some((system, stellar)) = at {
        let mut save: serde_json::Value =
            serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
        save["system"] = serde_json::json!(system.0);
        save["stellar"] = serde_json::json!(stellar);
        kit(&mut save);
        pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    }
    let store = MemoryPilots::new();
    let keeper = PilotKeeper::new(Box::new(store) as Box<dyn PilotStore>);
    keeper.save(&pilot).expect("saved");
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let mut screen = start_screen(Rc::clone(&data))
        .with_dialogs(Rc::new(interface), Rc::clone(&metrics) as _)
        .with_pilots(Some(keeper), metrics)
        .with_rules(Rulebook::default().with_override(RuleKey::IntroForReopenedPilot, intro))
        .with_prefs(Prefs {
            quicktime_movies: true,
            ..Prefs::default()
        })
        .with_movie_player(Rc::clone(player) as Rc<dyn MoviePlayer>);
    if let Some(chance) = chance {
        screen = screen.with_chance(chance);
    }
    let driver = Driver {
        app: App::new(&Window, Rc::clone(&data), crate::common::appeared(screen)),
        gpu: RecordingGpu::new(),
        clock: Duration::ZERO,
    };
    Some((driver, data))
}

/// A player whose every movie would play: none is expected to start.
fn projector() -> Rc<MockMoviePlayer> {
    MockMoviePlayer::playing((160, 120), 2)
}

#[test]
fn about_x_and_an_entry_through_the_stock_intro_start_no_movie() {
    let player = projector();
    let Some((mut driver, _)) = stock_app(&player, RuleSource::Bible, None, |_| {}, None) else {
        return;
    };
    driver.send(WindowEvent::Menu(MenuCommand::About));
    assert_eq!(driver.showing(), Showing::About);
    assert!(driver.app.screen().about().is_some(), "the text, no movie");
    driver.press(Key::Enter);
    driver.press(Key::Char('x'));
    driver.redraw();
    assert_eq!(driver.showing(), Showing::Note);
    assert!(driver.app.screen().about().is_some());
    driver.press(Key::Enter);
    assert_eq!(driver.showing(), Showing::MainMenu);
    assert!(player.calls().is_empty(), "{:?}", player.calls());

    // Open Pilot, then Enter Ship: the first chär's intro, by the other
    // reading of `intro_for_reopened_pilot`, has no text.
    let open = driver
        .app
        .screen()
        .main_menu()
        .expect("the main menu")
        .press_point(MenuChoice::OpenPilot);
    driver.click(open);
    driver.press(Key::Enter);
    let at = common::enter_ship_at(driver.app.screen());
    driver.click(at);
    for _ in 0..2000 {
        if !driver.app.screen().entering() {
            break;
        }
        assert_ne!(driver.showing(), Showing::IntroText, "no intro text");
        if driver.showing() == Showing::Intro {
            driver.press(nova_view::menu::intro::SKIP_KEY);
        }
        driver.redraw();
    }
    assert_eq!(driver.showing(), Showing::Flight);
    assert!(player.calls().is_empty(), "{:?}", player.calls());
}

/// The system holding the stock stellar `stellar`.
fn system_of(data: &GameData, stellar: i16) -> SystemId {
    data.ids(nova_data::records::system::System::TYPE)
        .iter()
        .map(|&id| SystemId(id))
        .find(|&system| {
            data.landing_sites(system)
                .iter()
                .any(|site| site.id.0 == stellar)
        })
        .expect("the stellar is listed")
}

impl Driver {
    /// Opens the saved pilot, enters its ship and lands where it was
    /// saved, the landing's movie played out: then over the spaceport.
    fn open_and_land(&mut self) {
        let open = self
            .app
            .screen()
            .main_menu()
            .expect("the main menu")
            .press_point(MenuChoice::OpenPilot);
        self.click(open);
        self.press(Key::Enter);
        self.enter_ship();
        assert_eq!(self.showing(), Showing::Flight);
        self.press(Key::Char('l'));
        self.redraw();
        self.redraw();
        self.press(Key::Char('l'));
        for _ in 0..4 {
            self.redraw();
        }
        assert_eq!(self.showing(), Showing::Spaceport);
    }

    fn spaceport(&self) -> &nova_view::spaceport::SpaceportView {
        self.app.screen().spaceport_view().expect("landed")
    }

    /// The `STR#` text up over the spaceport, its lines joined.
    fn text_up(&self) -> Option<String> {
        self.spaceport()
            .mission_text()
            .map(|text| text.lines().join(" "))
    }

    /// The session flight flies.
    fn session(&self) -> &nova_sim::Session {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
    }
}

/// The stock app with a stock pilot saved over `stellar` as `kit` edits
/// its save, drawing on `chance` when given, landed there; its movie
/// player; and the movie calls made by then. `None` when the data is
/// missing.
fn landed_at(
    stellar: i16,
    kit: impl FnOnce(&mut serde_json::Value),
    chance: Option<SharedChance>,
) -> Option<(Driver, Rc<MockMoviePlayer>, usize)> {
    let dir = common::nova_data()?;
    let system = system_of(
        &GameData::open(&dir, None).expect("the stock data opens"),
        stellar,
    );
    let player = projector();
    let (mut driver, _) = stock_app(
        &player,
        RuleSource::Engine,
        Some((system, stellar)),
        kit,
        chance,
    )?;
    driver.open_and_land();
    let landed = player.calls().len();
    Some((driver, player, landed))
}

/// The movie calls `player` was asked for after the first `landed`.
fn since(player: &MockMoviePlayer, landed: usize) -> Vec<nova_view::movie::fixture::Call> {
    player.calls().into_iter().skip(landed).collect()
}

#[test]
fn landing_at_aurora_leaves_a_last_desc_with_a_movie_but_no_after_movie_for_the_shops_texts() {
    let Some((driver, player, _)) = landed_at(AURORA, |_| {}, None) else {
        return;
    };
    assert_eq!(
        player.calls().first(),
        Some(&nova_view::movie::fixture::Call::Start("flyby.qt".into())),
        "the landing loads and plays Aurora's dësc (@0x632a7)"
    );
    assert_eq!(
        driver.session().loaded_after_movie(),
        None,
        "the last dësc names a movie, but not after (`Flags` 0)"
    );
}

#[test]
fn nothing_for_hire_in_auroras_bar_starts_no_movie_once_it_closes() {
    // No roll fires, so no class of a `HireRandom` below 100 is for hire
    // today, as a day of the original's can roll.
    let Some((mut driver, player, landed)) = landed_at(AURORA, |_| {}, None) else {
        return;
    };
    driver.press(Key::Char('b'));
    assert!(driver.spaceport().open_bar().is_some(), "the bar");
    driver.press(Key::Char('h'));
    assert_eq!(
        driver.text_up().as_deref(),
        Some(nova_sim::hire::NONE_FOR_HIRE),
        "#224 (@0x5e8da)"
    );
    driver.press(Key::Enter);
    driver.redraw();
    assert_eq!(driver.text_up(), None, "closed");
    assert_eq!(
        driver.session().loaded_after_movie(),
        None,
        "the bar's dësc (@0x48f91) names no movie"
    );
    assert_eq!(since(&player, landed), []);
}

#[test]
fn the_escorts_landing_report_at_aurora_starts_no_movie_once_it_closes() {
    // A Shuttle in the fleet, marked for sale in the escort dialog.
    let sold = |save: &mut serde_json::Value| {
        let gauge = |max: f32| serde_json::json!({"now": max, "max": max});
        save["escorts"] = serde_json::json!([{
            "ship": 128,
            "reserves": {"shield": gauge(30.0), "armor": gauge(45.0), "fuel": gauge(300.0)},
            "order": null,
            "carried": false,
            "wage": null,
            "person": null,
            "mark": "sell"
        }]);
    };
    let Some((mut driver, player, landed)) = landed_at(AURORA, sold, None) else {
        return;
    };
    assert_eq!(
        driver.session().loaded_after_movie(),
        None,
        "Aurora's dësc was the last loaded"
    );
    driver.press(Key::Escape);
    assert_eq!(driver.showing(), Showing::Spaceport, "held");
    assert!(driver.text_up().is_some(), "the report (@0x407e8)");
    driver.press(Key::Enter);
    for _ in 0..4 {
        driver.redraw();
    }
    assert_eq!(driver.showing(), Showing::Flight, "the leave goes on");
    assert!(
        driver.session().pilot().escorts().is_empty(),
        "sold: the Shuttle's escort"
    );
    assert_eq!(since(&player, landed), []);
}

/// A chance where only rolls of 90 % or more fire, every draw the last
/// outcome: an outfit of `BuyRandom` 90, the Mass Expansion, is on sale.
struct Nineties;

impl nova_sim::Chance for Nineties {
    fn fires(&mut self, percent: u8) -> bool {
        percent >= 90
    }

    fn below(&mut self, n: u32) -> u32 {
        n.saturating_sub(1)
    }
}

/// The Mass Expansion: `Mass` -10, `BuyRandom` 90, tech level 4.
const MASS_EXPANSION: i16 = 190;
/// The Battery Pack: `Mass` 3, `BuyRandom` 100.
const BATTERY_PACK: i16 = 256;

#[test]
fn a_sale_alert_in_auroras_outfitter_starts_no_movie_once_it_closes() {
    // A Mass Expansion and three Battery Packs: selling the expansion
    // would leave the Shuttle's free mass below none (#207).
    let fitted = |save: &mut serde_json::Value| {
        let outfits = save["outfits"].as_array_mut().expect("a list");
        outfits.push(serde_json::json!({"outfit": MASS_EXPANSION, "count": 1}));
        outfits.push(serde_json::json!({"outfit": BATTERY_PACK, "count": 3}));
    };
    let chance = SharedChance::new(Rc::new(RefCell::new(Nineties)));
    let Some((mut driver, player, landed)) = landed_at(AURORA, fitted, Some(chance)) else {
        return;
    };
    driver.press(Key::Char('o'));
    let selected = |driver: &Driver| {
        driver
            .spaceport()
            .open_outfitter()
            .expect("outfitting")
            .selected_outfit()
    };
    for _ in 0..40 {
        if selected(&driver) == Some(nova_sim::OutfitId(MASS_EXPANSION)) {
            break;
        }
        driver.press(Key::Right);
    }
    assert_eq!(selected(&driver), Some(nova_sim::OutfitId(MASS_EXPANSION)));
    driver.press(Key::Char('s'));
    let open = driver.spaceport().open_outfitter().expect("outfitting");
    assert!(open.alert_dialog().is_some(), "the sale refused (@0x5c7a0)");
    assert_eq!(
        driver.session().loaded_after_movie(),
        None,
        "the selection's dësc (`_OutfitFilter` @0x58476) names no movie"
    );
    driver.press(Key::Escape);
    driver.redraw();
    let open = driver.spaceport().open_outfitter().expect("outfitting");
    assert!(open.alert_dialog().is_none(), "closed");
    assert_eq!(since(&player, landed), []);
}

/// Stellar 158, whose shipyard lists nothing for the stock pilot when no
/// roll fires; its `dësc` names no movie.
const EMPTY_SHIPYARD: i16 = 158;

#[test]
fn an_empty_shipyards_text_starts_no_movie_once_it_closes() {
    let Some((mut driver, player, landed)) = landed_at(EMPTY_SHIPYARD, |_| {}, None) else {
        return;
    };
    driver.press(Key::Char('s'));
    assert_eq!(
        driver.text_up().as_deref(),
        Some(nova_view::spaceport::shipyard::NO_SHIPS),
        "#223 (@0x5e8da)"
    );
    driver.press(Key::Enter);
    driver.redraw();
    assert_eq!(driver.text_up(), None, "closed");
    assert_eq!(since(&player, landed), []);
}
