//! The stellars' `dësc` movies over the stock data
//! (rdm:phase/movies/phase-3-stellar-desc-movies): of the stock
//! stellars' `dësc`s only Aurora's (338) names a movie, "flyby.qt", which
//! the stock Nova Files don't ship, and no `dësc` numbered as a system a
//! stock hypergate leads to, which its entry loads by the engine, names
//! one. So landing at Aurora with movies on starts "flyby.qt", and with
//! movies off nothing; and another stock landing, a hypergate's entry and
//! a wormhole's start no movie. Skips, passing, when `NOVA_DATA` is
//! unset.

use crate::common;

use std::rc::Rc;
use std::time::Duration;

use crate::common::EntryHarness;
use nova::app::{App, Control, Showing, WindowEvent, WindowPort, start_screen};
use nova_data::records::stellar::Stellar;
use nova_data::records::system::System;
use nova_data::{GameData, Record};
use nova_render::recording::RecordingGpu;
use nova_sim::fixture::MemoryPilots;
use nova_sim::{Pilot, PilotCatalog, PilotKeeper, PilotStore, Session, SystemId, is_landable};
use nova_view::galaxy::MapMode;
use nova_view::menu::MenuChoice;
use nova_view::movie::MoviePlayer;
use nova_view::movie::fixture::{Call, MockMoviePlayer};
use nova_view::text::fixture::MonoMetrics;
use nova_view::ui::catalog::DescriptionGraphics;
use nova_view::{Key, MouseButton, Prefs};

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

/// `Flags2` of a wormhole.
const WORMHOLE: u16 = 0x2000;

/// The `dësc`s numbered as `ids` that name a movie.
fn filmed(data: &GameData, ids: &[i16]) -> Vec<i16> {
    ids.iter()
        .copied()
        .filter(|&id| data.description_movie(id).is_some())
        .collect()
}

/// `Flags2` of a hypergate.
const HYPERGATE: u16 = 0x1000;

/// Aurora, the only stock stellar whose `dësc` names a movie.
const AURORA: i16 = 338;

/// The system each stock stellar is listed in, by stellar.
fn listers(data: &GameData) -> Vec<(i16, SystemId)> {
    data.ids(System::TYPE)
        .iter()
        .flat_map(|&system| {
            data.landing_sites(SystemId(system))
                .into_iter()
                .map(move |site| (site.id.0, SystemId(system)))
        })
        .collect()
}

/// The session's own read of a `dësc` (rdm:phase/movies/phase-5-desc-load-order:
/// the port's own as the landing settles, the rescue's 13999 first in the
/// rescue) reads the stock `dësc`s as the view's reads do, so the two
/// adapters can't drift apart.
#[test]
fn the_sessions_desc_read_matches_the_views_over_the_stock_data() {
    use nova_view::ui::catalog::DescriptionSource;
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    for id in [AURORA, nova_sim::escape::RESCUE_DESC] {
        let read = PilotCatalog::desc_text(&data, id).expect("read");
        assert_eq!(Ok(read.text), data.description(id), "{id}");
        assert_eq!(read.movie, data.description_movie(id), "{id}");
    }
    let aurora = PilotCatalog::desc_text(&data, AURORA).and_then(|desc| desc.movie);
    assert_eq!(
        aurora.map(|movie| (movie.file, movie.after, movie.double_size, movie.cinematic)),
        Some(("flyby.qt".to_owned(), false, false, false)),
        "Flags 0"
    );
}

#[test]
fn only_auroras_stellar_desc_names_a_movie_and_no_hypergate_leads_to_one() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = GameData::open(&dir, None).expect("the stock data opens");
    let stellars = data.ids(Stellar::TYPE).to_vec();
    assert!(stellars.len() > 100, "precondition: the stock spöbs");
    assert_eq!(filmed(&data, &stellars), [AURORA]);
    let movie = data.description_movie(AURORA).expect("named");
    assert_eq!(movie.file, "flyby.qt");
    assert!(
        !dir.join("flyby.qt").exists(),
        "not shipped: _PlayQTMovie finds no file and plays nothing (@0x559ff)"
    );
    let listers = listers(&data);
    let system_of = |stellar: i16| {
        listers
            .iter()
            .find(|(listed, _)| *listed == stellar)
            .map(|&(_, system)| system.0)
    };
    let led_to: Vec<i16> = data
        .gate_sites()
        .into_iter()
        .filter(|gate| gate.flags2 & HYPERGATE != 0)
        .flat_map(|gate| gate.links.into_iter().flatten())
        .filter_map(|link| system_of(link.0))
        .collect();
    assert!(!led_to.is_empty(), "precondition: stock hypergates lead on");
    assert_eq!(
        filmed(&data, &led_to),
        Vec::<i16>::new(),
        "none a hypergate's entry loads by the engine (@0x639cc)"
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

    fn session(&self) -> &Session {
        self.app
            .screen()
            .flight_view()
            .expect("flying")
            .session()
            .expect("a session")
    }

    /// L, two redraws for the dock approach to clear it, and L.
    fn land_twice(&mut self) {
        self.press(Key::Char('l'));
        self.redraw();
        self.redraw();
        self.press(Key::Char('l'));
    }
}

crate::common::entry_harness!(Driver, click, redraw);

/// The app over `data`, movies played through `player` while `on`, with a
/// pilot saved over `stellar` in `system` holding `ranks`, opened from
/// the main menu: it resumes in space over the stellar.
fn resumed_over(
    data: &Rc<GameData>,
    (system, stellar): (SystemId, i16),
    ranks: &[i16],
    player: &Rc<MockMoviePlayer>,
    on: bool,
) -> Driver {
    let pilot = Pilot::new(data.as_ref(), "Filmgoer").expect("the stock first chär starts");
    let mut save: serde_json::Value =
        serde_json::from_str(&nova_sim::save::encode(&pilot)).expect("JSON");
    save["system"] = serde_json::json!(system.0);
    save["stellar"] = serde_json::json!(stellar);
    save["ranks"] = serde_json::json!(ranks);
    let saved = nova_sim::save::decode(&save.to_string()).expect("a pilot");
    let store = MemoryPilots::new();
    let keeper = PilotKeeper::new(Box::new(store) as Box<dyn PilotStore>);
    keeper.save(&saved).expect("saved");
    let screen = start_screen(Rc::clone(data))
        .with_pilots(Some(keeper), Rc::new(MonoMetrics))
        .with_prefs(Prefs {
            quicktime_movies: on,
            ..Prefs::default()
        })
        .with_movie_player(Rc::clone(player) as Rc<dyn MoviePlayer>);
    let mut driver = Driver {
        app: App::new(&Window, Rc::clone(data), crate::common::appeared(screen)),
        gpu: RecordingGpu::new(),
        clock: Duration::ZERO,
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
    assert_eq!(driver.showing(), Showing::Flight, "resumed in space");
    driver
}

/// A player whose every movie would play: none is expected to start.
fn projector() -> Rc<MockMoviePlayer> {
    MockMoviePlayer::playing((160, 120), 2)
}

#[test]
fn a_stock_landing_hypergate_and_wormhole_with_movies_on_start_no_movie() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));

    // A landing where a new pilot starts.
    let player = projector();
    let start = Pilot::new(data.as_ref(), "Probe")
        .expect("the stock first chär starts")
        .system();
    let site = data
        .landing_sites(start)
        .into_iter()
        .find(is_landable)
        .expect("a landable stellar where the pilot starts");
    let mut driver = resumed_over(&data, (start, site.id.0), &[], &player, true);
    driver.land_twice();
    assert_eq!(driver.showing(), Showing::Spaceport, "landed, no movie");
    assert!(player.calls().is_empty(), "{:?}", player.calls());
    assert_eq!(driver.session().loaded_after_movie(), None);

    // HG-Kania (1404, in Kania, 128) for Tichel (129), holding rank 147,
    // "Have Access to Hypergate System".
    let player = projector();
    let mut driver = resumed_over(&data, (SystemId(128), 1404), &[147], &player, true);
    driver.land_twice();
    assert_eq!(driver.showing(), Showing::FlightMap);
    let map = driver
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .course_map();
    assert_eq!(map.mode(), MapMode::Hypergate);
    let tichel = map.model().system(SystemId(129)).expect("Tichel");
    let at = map.transform().world_to_screen(tichel.position());
    driver.click(at);
    driver.press(Key::Char('m'));
    assert_eq!(driver.showing(), Showing::Flight, "no movie");
    assert_eq!(driver.session().system(), SystemId(129), "entered");
    assert!(player.calls().is_empty(), "{:?}", player.calls());
    assert_eq!(driver.session().loaded_after_movie(), None);

    // The first stock wormhole a system lists.
    let (system, wormhole) = data
        .ids(System::TYPE)
        .iter()
        .find_map(|&system| {
            let system = SystemId(system);
            data.landing_sites(system)
                .into_iter()
                .find(|site| site.flags2 & WORMHOLE != 0)
                .map(|site| (system, site.id.0))
        })
        .expect("a stock wormhole");
    let player = projector();
    let mut driver = resumed_over(&data, (system, wormhole), &[], &player, true);
    driver.land_twice();
    assert_eq!(driver.showing(), Showing::Flight, "no movie");
    let message = driver
        .app
        .screen()
        .flight_view()
        .expect("flying")
        .message()
        .map(str::to_owned);
    assert!(
        message
            .as_deref()
            .is_some_and(|line| line.starts_with("Passing through a wormhole")),
        "passed through: {message:?}"
    );
    assert!(player.calls().is_empty(), "{:?}", player.calls());
    assert_eq!(driver.session().loaded_after_movie(), None);
}

#[test]
fn landing_at_aurora_starts_its_movie_with_movies_on_and_nothing_with_them_off() {
    let Some(dir) = common::nova_data() else {
        return;
    };
    let data = Rc::new(GameData::open(&dir, None).expect("the stock data opens"));
    let aurora = listers(&data)
        .into_iter()
        .find(|(stellar, _)| *stellar == AURORA)
        .map(|(_, system)| system)
        .expect("Aurora is listed");
    for on in [true, false] {
        let player = projector();
        let mut driver = resumed_over(&data, (aurora, AURORA), &[], &player, on);
        driver.land_twice();
        if on {
            assert_eq!(player.calls(), [Call::Start("flyby.qt".into())]);
            assert_eq!(driver.showing(), Showing::Movie);
            for _ in 0..10 {
                driver.redraw();
            }
        } else {
            assert!(player.calls().is_empty(), "{:?}", player.calls());
        }
        assert_eq!(driver.showing(), Showing::Spaceport, "on {on}");
    }
}
