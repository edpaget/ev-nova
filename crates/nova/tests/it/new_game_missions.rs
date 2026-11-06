//! The new game's missions played over the checked-in fixture
//! (`nova_data::new_game::fixture::folder`), opened as the app opens it
//! (`DiskData::open_new_game`, which checks every knot against the Ink
//! runtime), with `nova::dialogs::loader` wired as `main` wires it and the
//! new game's rulebook.
//!
//! The fixture's storyline, `dock-work`: the dock job (offered at Hearth
//! Dock, ten tons of hull plate to Verge Camp, completed there), the side
//! road closing on day 3 (an event, with its news), the wardens' toll at
//! Verge Camp (every condition group) and the road's `take_point` offer.
//! The headless tests drive the session through its commands, putting a
//! pilot's state in through the save; one walks the app. The rules are
//! pinned in `nova_sim_story::storylines` and the session's forwarding in
//! `nova_sim`; this pins that the data, the real Ink runtime and the
//! session meet.

use nova::chance::SplitMix;
use nova::startup::{DataSource, DiskData, NewGameOpened};
use nova_data::GameData;
use nova_data::new_game::NewGame;
use nova_sim::flight_frame::{Answer, Converse, FlightCommand};
use nova_sim::storylines::state::MissionState;
use nova_sim::{
    ApplyCollaborators, HailOptions, NovaBoarding, NovaLaw, Pilot, Session, StellarId, SystemId,
};

use crate::new_game_travel::{fixture, new_game_rules, nova_of, pilot_for};

/// The fixture as the app opens it, its knots checked, with no problems,
/// and its ids.
fn opened() -> (GameData, NewGame) {
    let NewGameOpened::Loaded(loaded) =
        DiskData.open_new_game(&nova_data::new_game::fixture::folder())
    else {
        panic!("the fixture opens");
    };
    assert_eq!(loaded.problems, Vec::<String>::new());
    (loaded.data, fixture())
}

/// A mission game: the data, its ids and a session flying a pilot.
struct Play {
    data: GameData,
    game: NewGame,
    session: Session,
    chance: SplitMix,
}

impl Play {
    /// The fixture's start pilot, changed by `edit` and put through a
    /// save, flying in system `system`.
    fn new(system: &str, edit: impl FnOnce(&mut Pilot)) -> Self {
        let (data, game) = opened();
        let mut pilot = pilot_for(&game);
        pilot.system = SystemId(nova_of(&game.ids.systems, system));
        pilot.stellar = None;
        edit(&mut pilot);
        let pilot = nova_sim::save::decode(&nova_sim::save::encode(&pilot)).expect("loads");
        let session = Session::load(&data, pilot, &new_game_rules())
            .expect("flies")
            .with_dialog_loader(nova::dialogs::loader(1));
        Self {
            data,
            game,
            session,
            chance: SplitMix::new(3),
        }
    }

    fn stellar(&self, id: &str) -> StellarId {
        StellarId(nova_of(&self.game.ids.stellars, id))
    }

    fn system(&self, id: &str) -> SystemId {
        SystemId(nova_of(&self.game.ids.systems, id))
    }

    /// Lands at the stellar `id`, in the system the pilot is in, and
    /// settles the landing's story.
    fn land(&mut self, id: &str) {
        let stellar = self.stellar(id);
        let system = self.session.pilot().system;
        self.session
            .land_at(&self.data, system, stellar)
            .expect("lands");
        self.settle();
        assert_eq!(self.session.landed(), Some(stellar));
    }

    /// Takes off, then lands at the stellar `id` again: a new visit.
    fn visit_again(&mut self, id: &str) {
        assert!(
            self.session
                .take_off(&self.data, &mut self.chance)
                .is_some()
        );
        self.settle();
        self.land(id);
    }

    /// Lets `days` days go by and settles them.
    fn pass(&mut self, days: u32) {
        self.session.let_days_pass(days, &mut self.chance);
        self.settle();
    }

    fn settle(&mut self) {
        let _ = self.session.settle_script(&self.data, &mut self.chance);
    }

    fn converse(&mut self, converse: Converse) -> Answer {
        self.session.apply(
            FlightCommand::Converse(converse),
            &mut ApplyCollaborators {
                catalog: Some(&self.data),
                chance: &mut self.chance,
                law: &NovaLaw::default(),
                boarding_rule: &NovaBoarding::default(),
                hail_options: &HailOptions::default(),
            },
        )
    }

    /// Opens the visit's next conversation: its lines and choices.
    fn open(&mut self) -> Vec<String> {
        page(&self.converse(Converse::Open))
    }

    /// Takes the choice at `index`: the next page's lines and choices.
    fn choose(&mut self, index: usize) -> Vec<String> {
        page(&self.converse(Converse::Choose(index)))
    }

    fn close(&mut self) {
        assert_eq!(self.converse(Converse::Close), Answer::Done);
    }

    fn state(&self, id: &str) -> Option<MissionState> {
        self.session.pilot().story.mission(id)
    }

    fn flag(&self, name: &str) -> bool {
        self.session.pilot().flags.is_set(name)
    }

    /// What system `id` has heard, as (news id, age).
    fn heard(&self, id: &str) -> Vec<(String, u32)> {
        self.session
            .heard_news(self.system(id))
            .into_iter()
            .map(|news| (news.id, news.age))
            .collect()
    }
}

/// The page `answer` shows: its lines, then its choices.
fn page(answer: &Answer) -> Vec<String> {
    let Answer::Conversation(page) = answer else {
        panic!("not a page: {answer:?}");
    };
    assert_eq!(page.problem, None);
    page.lines.iter().chain(&page.choices).cloned().collect()
}

fn texts(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|&line| line.to_owned()).collect()
}

/// The dock job's offer, its first page.
const OFFER: [&str; 5] = [
    "The dockmaster looks up from a slate.",
    "\"Ten tons of hull plate for Verge Camp. It pays on delivery.\"",
    "Take the job",
    "Ask about the route",
    "Not now",
];

/// Verge Camp's greeting, with no plate on the way.
const GREETING: [&str; 1] = ["Wind snaps at the tents."];

#[test]
fn the_dock_job_is_offered_taken_and_delivered() {
    let mut play = Play::new("hearth", |_| {});
    play.land("hearth-dock");
    assert_eq!(play.open(), texts(&OFFER));
    assert_eq!(play.state("dock-job"), Some(MissionState::Offered));
    assert_eq!(
        play.choose(0),
        texts(&["\"Bay two. Mind the dead gate past Verge.\""])
    );
    assert_eq!(
        play.state("dock-job"),
        Some(MissionState::Active {
            cargo: 10,
            met: false
        })
    );
    assert_eq!(
        play.session.pilot().total_cargo(),
        10,
        "the plate is aboard"
    );
    assert!(play.flag("hauling_plate"));
    play.close();
    assert_eq!(
        play.converse(Converse::Open),
        Answer::Refused,
        "Hearth Dock names no conversation of its own"
    );

    // A saved pilot in the Verge with the job active.
    let taken = play.session.pilot().clone();
    let mut play = Play::new("verge", |pilot| {
        pilot.story = taken.story.clone();
        pilot.flags = taken.flags.clone();
        pilot.dialogs = taken.dialogs.clone();
    });
    play.land("verge-camp");
    assert_eq!(play.session.pilot().total_cargo(), 0, "dropped off");
    assert_eq!(
        play.open(),
        texts(&["The camp boss signs for the plate.", "\"Right on time.\""])
    );
    assert_eq!(play.state("dock-job"), Some(MissionState::Done));
    assert!(play.flag("dock_regular"));
    assert!(!play.flag("hauling_plate"));
    play.close();
    assert_eq!(
        play.open(),
        texts(&GREETING),
        "on_success cleared hauling_plate before the greeting"
    );
    play.close();
    assert_eq!(play.converse(Converse::Open), Answer::Refused);
}

#[test]
fn the_side_road_closes_on_day_3_and_its_news_spreads() {
    let mut play = Play::new("hearth", |_| {});
    play.pass(2);
    assert_eq!(play.state("side-road-closes"), None);
    assert!(play.session.pilot().story.points.is_empty());
    for system in ["hearth", "verge", "drift"] {
        assert_eq!(play.heard(system), [], "{system} on day 2");
    }
    play.pass(1);
    assert_eq!(play.session.pilot().story.day, 3);
    assert_eq!(play.state("side-road-closes"), Some(MissionState::Done));
    let dead = |point: &str| {
        play.session.pilot().story.points[point].stability == Some(nova_data::GateStability::Dead)
    };
    assert!(dead("hearth/side-road"));
    assert!(dead("verge/side-road"));
    let news = |age: u32| vec![("side-road-closes".to_owned(), age)];
    assert_eq!(play.heard("hearth"), news(0));
    assert_eq!(play.heard("verge"), []);
    assert_eq!(play.heard("drift"), []);
    play.pass(1);
    assert_eq!(play.heard("hearth"), news(1));
    assert_eq!(play.heard("verge"), news(0));
    assert_eq!(play.heard("drift"), news(0));
}

/// The toll is offered once the side road is dead, the dock job done and
/// the wardens hold their road. Refused, it makes the road's offer hold:
/// the visit picks its next scene again at every `Open`, so the road's
/// offer opens in the same visit, after the toll's and before the camp's
/// own greeting, which a visit shows last.
#[test]
fn the_toll_is_offered_after_the_side_road_closes_and_the_dock_job() {
    let mut play = Play::new("verge", |pilot| {
        pilot
            .story
            .missions
            .insert("dock-job".to_owned(), MissionState::Done);
        pilot.flags.set("dock_regular");
        // Day 3 has come; the event happens at the next check.
        pilot.story.day = 3;
    });
    play.land("verge-camp");
    assert_eq!(play.state("side-road-closes"), Some(MissionState::Done));
    assert_eq!(
        play.open(),
        texts(&[
            "A warden leans on the camp's gate. \"The side road's shut. Our road costs.\"",
            "Pay the toll",
            "Refuse",
            "Later",
        ])
    );
    assert!(play.flag("met_the_wardens"));
    assert_eq!(
        play.choose(1),
        texts(&["\"Then stay off the hearth road.\""])
    );
    assert_eq!(play.state("warden-toll"), Some(MissionState::Declined));
    assert!(play.flag("refused_the_toll"));
    play.close();
    assert_eq!(
        play.open(),
        texts(&[
            "The camp boss spreads a map. \"Take the hearth road off the wardens.\"",
            "I'll do it",
            "Not my fight",
        ]),
        "the road's offer, in the same visit"
    );
    assert_eq!(play.choose(0), texts(&["\"Good hunting.\""]));
    play.close();
    assert_eq!(play.open(), texts(&GREETING), "the camp's own, last");
    play.close();
    assert_eq!(play.converse(Converse::Open), Answer::Refused);
    let active = Some(MissionState::Active {
        cargo: 0,
        met: false,
    });
    assert_eq!(play.state("clear-the-road"), active);
    play.visit_again("verge-camp");
    play.pass(5);
    assert_eq!(
        play.state("clear-the-road"),
        active,
        "a take_point stays active"
    );
}

#[test]
fn a_declined_dock_job_never_comes_back() {
    let mut play = Play::new("hearth", |_| {});
    play.land("hearth-dock");
    play.open();
    play.choose(1);
    assert_eq!(play.choose(1), texts(&["\"Suit yourself.\""]));
    assert_eq!(play.state("dock-job"), Some(MissionState::Declined));
    assert!(play.flag("turned_down_the_dock"));
    play.close();
    play.visit_again("hearth-dock");
    assert_eq!(play.converse(Converse::Open), Answer::Refused);
}

#[test]
fn a_deferred_offer_comes_back_at_the_next_landing() {
    let mut play = Play::new("hearth", |_| {});
    play.land("hearth-dock");
    play.open();
    assert_eq!(play.choose(2), texts(&["\"It'll keep a day or two.\""]));
    assert_eq!(play.state("dock-job"), Some(MissionState::Offered));
    play.close();
    assert_eq!(
        play.converse(Converse::Open),
        Answer::Refused,
        "once a visit"
    );
    play.visit_again("hearth-dock");
    assert_eq!(play.open(), texts(&OFFER));
}

#[test]
fn the_plate_news_skips_the_dead_lane_and_reaches_drift_through_hearth() {
    let mut play = Play::new("hearth", |pilot| {
        pilot.story.day = 5;
        pilot.story.news.insert("plate-arrives".to_owned(), 5);
        // The side road's event is past, so only the plate is news.
        pilot
            .story
            .missions
            .insert("side-road-closes".to_owned(), MissionState::Done);
    });
    let plate = |age: u32| vec![("plate-arrives".to_owned(), age)];
    assert_eq!(play.heard("verge"), plate(0));
    assert_eq!(play.heard("hearth"), []);
    assert_eq!(play.heard("drift"), [], "the verge-drift lane is dead");
    play.pass(1);
    assert_eq!(play.heard("hearth"), plate(0));
    assert_eq!(play.heard("drift"), []);
    play.pass(1);
    assert_eq!(play.heard("drift"), plate(0), "by way of Hearth");
    assert_eq!(play.heard("verge"), plate(2));
}

#[test]
fn the_app_offers_the_dock_job_and_closes_back_to_the_spaceport() {
    use crate::new_game_conversation::{drawn, flying, land_at};
    use nova::app::Showing;
    use nova_view::spaceport::conversation::CLOSE_HINT;
    use winit::keyboard::KeyCode;

    let mut game = flying();
    land_at(&mut game, "hearth-dock");
    let page = drawn(&mut game);
    assert!(page.contains(&OFFER[0].to_owned()), "{page:?}");
    game.press(KeyCode::Digit1);
    let page = drawn(&mut game);
    assert!(
        page.ends_with(&texts(&[
            "\"Bay two. Mind the dead gate past Verge.\"",
            CLOSE_HINT
        ])),
        "{page:?}"
    );
    game.press(KeyCode::Enter);
    assert_eq!(game.showing(), Showing::Spaceport);
    let page = drawn(&mut game);
    assert!(!page.contains(&CLOSE_HINT.to_owned()), "{page:?}");
    assert!(!page.contains(&OFFER[0].to_owned()), "{page:?}");
}

/// The app over the real router: no test call settles the landing, so the
/// first scene is picked as the app picks it, as the spaceport opens.
#[test]
fn the_app_shows_the_completion_first_at_the_landing_that_delivers() {
    use crate::new_game_conversation::{drawn, flying_in, position_of, resumed};
    use crate::pilots::saved;
    use nova::app::Showing;
    use nova_sim::PilotStore;
    use nova_sim::fixture::MemoryPilots;
    use winit::keyboard::KeyCode;

    let store = MemoryPilots::new();
    // A new pilot is saved at once; she is put in the Verge with the dock
    // job taken.
    drop(flying_in(&store));
    let (_, game) = opened();
    let mut pilot = saved(&store, "Ada");
    pilot.system = SystemId(nova_of(&game.ids.systems, "verge"));
    pilot.stellar = None;
    pilot.story.missions.insert(
        "dock-job".to_owned(),
        MissionState::Active {
            cargo: 10,
            met: false,
        },
    );
    pilot.flags.set("hauling_plate");
    store
        .write("Ada", &nova_sim::save::encode(&pilot))
        .expect("writes");

    let mut app = resumed(&store);
    // The camp is an ungoverned outpost: one L lands, with no clearance
    // to wait for.
    let camp = position_of("verge-camp");
    app.app
        .screen_mut()
        .flight_view_mut()
        .expect("flying")
        .change_sim(|sim| {
            nova_sim::fixture::put_player_at(sim.session_mut().expect("a session"), camp);
        });
    app.press(KeyCode::KeyL);
    assert_eq!(app.showing(), Showing::Spaceport, "landed at Verge Camp");
    let page = drawn(&mut app);
    assert!(
        page.contains(&"The camp boss signs for the plate.".to_owned()),
        "the completion first: {page:?}"
    );
    assert!(!page.contains(&GREETING[0].to_owned()), "{page:?}");
    app.press(KeyCode::Enter);
    let page = drawn(&mut app);
    assert!(
        page.contains(&GREETING[0].to_owned()),
        "then the camp's own: {page:?}"
    );
}
