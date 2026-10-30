//! The `dësc` movies outside the missions over the stock data
//! (rdm:phase/movies/phase-4-desc-movies-outside-missions): of every
//! stock `dësc` only Aurora's (338), the four races' (32760-32763) and
//! 4778 (`mïsn` 906's) name a movie, and only 4778 has `Flags` 0x0001: its
//! `MovieFile` is the one byte 0x01, which no Nova Files ships, so a
//! `STR#` text after it (a shop's, the escorts' report) starts a movie
//! that finds no file and plays nothing; the About text (32767) and the X text (32766) name none; and the
//! stock first `chär`'s `IntroTextID` is -1, so its intro shows no text.
//! Driven with movies on, About, X and an entry through the stock intro
//! start no movie; and a landing at Aurora, whose `dësc` names a movie
//! that isn't an after-movie, leaves the session's after-movie, which
//! every shop's `STR#` text is handed, at none. The positive cases are pinned over fixture data (the
//! `nova` crate's router tests). Skips, passing, when `NOVA_DATA` or the
//! interface file is missing.

use crate::common;

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
/// given; on the main menu. `None` when the interface file is missing.
fn stock_app(
    player: &Rc<MockMoviePlayer>,
    intro: RuleSource,
    at: Option<(SystemId, i16)>,
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
        pilot = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    }
    let store = MemoryPilots::new();
    let keeper = PilotKeeper::new(Box::new(store) as Box<dyn PilotStore>);
    keeper.save(&pilot).expect("saved");
    let metrics = Rc::new(GlyphonMetrics::new(&FontFaces::bundled()));
    let screen = start_screen(Rc::clone(&data))
        .with_dialogs(Rc::new(interface), Rc::clone(&metrics) as _)
        .with_pilots(Some(keeper), metrics)
        .with_rules(Rulebook::default().with_override(RuleKey::IntroForReopenedPilot, intro))
        .with_prefs(Prefs {
            quicktime_movies: true,
            ..Prefs::default()
        })
        .with_movie_player(Rc::clone(player) as Rc<dyn MoviePlayer>);
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
    let Some((mut driver, _)) = stock_app(&player, RuleSource::Bible, None) else {
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

#[test]
fn landing_at_aurora_leaves_a_last_desc_with_a_movie_but_no_after_movie_for_the_shops_texts() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let aurora = data
        .ids(nova_data::records::system::System::TYPE)
        .iter()
        .map(|&id| SystemId(id))
        .find(|&system| {
            data.landing_sites(system)
                .iter()
                .any(|site| site.id.0 == AURORA)
        })
        .expect("Aurora is listed");
    let player = projector();
    let Some((mut driver, _)) = stock_app(&player, RuleSource::Engine, Some((aurora, AURORA)))
    else {
        return;
    };
    let open = driver
        .app
        .screen()
        .main_menu()
        .expect("the main menu")
        .press_point(MenuChoice::OpenPilot);
    driver.click(open);
    driver.press(Key::Enter);
    driver.enter_ship();
    assert_eq!(driver.showing(), Showing::Flight);
    driver.press(Key::Char('l'));
    driver.redraw();
    driver.redraw();
    driver.press(Key::Char('l'));
    let flyby = player.calls();
    assert_eq!(
        flyby.first(),
        Some(&nova_view::movie::fixture::Call::Start("flyby.qt".into())),
        "the landing loads and plays Aurora's dësc (@0x632a7)"
    );
    for _ in 0..4 {
        driver.redraw();
    }
    let session = driver
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .session()
        .expect("a session");
    assert_eq!(
        session.loaded_after_movie(),
        None,
        "the last dësc names a movie, but not after (`Flags` 0)"
    );
    assert_eq!(driver.showing(), Showing::Spaceport);
}
