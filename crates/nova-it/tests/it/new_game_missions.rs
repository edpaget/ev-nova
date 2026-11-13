//! The new game's missions played over the checked-in fixture
//! (`nova_data::new_game::fixture::folder`), opened as the app opens it
//! (`DiskData::open_new_game`, which checks every knot against the Ink
//! runtime), with `nova::dialogs::loader` wired as `main` wires it and the
//! new game's rulebook.
//!
//! The fixture's storyline, `dock-work`: the dock job (offered at Hearth
//! Dock, ten tons of hull plate to Verge Camp, completed there, lending a
//! Dock pass on the way), the side
//! road closing on its day (an event, with its news), the wardens' toll at
//! Verge Camp (every condition group) and the road's `take_point` offer,
//! met by beating the wardens at the road's gate.
//! The headless tests drive the session through its commands, putting a
//! pilot's state in through the save; one walks the app. The rules are
//! pinned in `nova_sim_story::storylines` and the session's forwarding in
//! `nova_sim`; this pins that the data, the real Ink runtime and the
//! session meet.

use nova_data::new_game::NewGame;
use nova_data::new_game::fixture::folder;
use nova_sim::flight_frame::{Answer, Converse};
use nova_sim::storylines::state::MissionState;
use nova_sim::{OutfitId, Pilot, StellarId, SystemId};

use crate::new_game_play::{self, Play, texts};
use crate::new_game_travel::{fixture, gates, nova_of, side_road_closing_day};

/// The fixture's start pilot, changed by `edit`, flying in `system`.
fn fixture_play(system: &str, edit: impl FnOnce(&mut Pilot)) -> Play {
    Play::new(&folder(), system, edit)
}

/// How many Dock passes (`outfits/dock-pass.md`) the pilot owns.
fn passes(play: &Play) -> u16 {
    let pass = OutfitId(nova_of(&play.game.ids.outfits, "dock-pass"));
    play.session.pilot().owned(pass)
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
    let mut play = fixture_play("hearth", |_| {});
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
    assert_eq!(passes(&play), 1, "the accept lends a Dock pass");
    play.close();
    assert_eq!(
        play.converse(Converse::Open),
        Answer::Refused,
        "Hearth Dock names no conversation of its own"
    );

    // A saved pilot in the Verge with the job active and the pass.
    let taken = play.session.pilot().clone();
    let mut play = fixture_play("verge", |pilot| {
        pilot.story = taken.story.clone();
        pilot.flags = taken.flags.clone();
        pilot.dialogs = taken.dialogs.clone();
        pilot.outfits = taken.outfits.clone();
    });
    assert_eq!(passes(&play), 1, "the pass survives a save");
    play.land("verge-camp");
    assert_eq!(passes(&play), 1, "kept until the completion");
    assert_eq!(play.session.pilot().total_cargo(), 0, "dropped off");
    assert_eq!(
        play.open(),
        texts(&["The camp boss signs for the plate.", "\"Right on time.\""])
    );
    assert_eq!(play.state("dock-job"), Some(MissionState::Done));
    assert!(play.flag("dock_regular"));
    assert!(!play.flag("hauling_plate"));
    assert_eq!(passes(&play), 0, "the success takes the pass back");
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
fn the_side_road_closes_on_its_day_and_its_news_spreads() {
    let closing = side_road_closing_day();
    let mut play = fixture_play("hearth", |_| {});
    play.pass(closing - 1);
    assert_eq!(play.state("side-road-closes"), None);
    assert!(play.session.pilot().story.points.is_empty());
    for system in ["hearth", "verge", "drift"] {
        assert_eq!(play.heard(system), [], "{system} the day before");
    }
    play.pass(1);
    assert_eq!(play.session.pilot().story.day, closing);
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
    assert_eq!(play.heard("drift"), news(0));
    // The other road into verge is unstable at verge's end, so closed: no
    // ship, and no news, crosses to verge once the side road is dead.
    assert_eq!(play.heard("verge"), []);
    play.pass(5);
    assert_eq!(play.heard("verge"), []);
}

/// The side road's gate in hearth, over which the pilot flies.
fn over_the_side_road(pilot: &mut Pilot, game: &NewGame) {
    pilot.stellar = Some(StellarId(nova_of(&game.ids.gates, "hearth/side-road")));
}

#[test]
fn the_side_road_s_gate_clears_until_its_closing_day_and_is_denied_after() {
    let closing = side_road_closing_day();
    let game = fixture();
    let side_road = StellarId(nova_of(&game.ids.gates, "hearth/side-road"));
    let mut play = fixture_play("hearth", |pilot| over_the_side_road(pilot, &game));
    let (hearth, verge) = (play.system("hearth"), play.system("verge"));
    play.pass(closing - 1);
    assert!(play.session.land().is_ok(), "cleared the day before");
    assert_eq!(
        play.session.galaxy().lanes().gates(hearth, verge),
        [side_road]
    );
    play.pass(1);
    assert!(
        matches!(
            play.session.land(),
            Err(nova_sim::LandingRefusal::Denied { stellar, .. }) if stellar == side_road
        ),
        "denied on the day, its request granted or not"
    );
    assert_eq!(play.session.galaxy().lanes().gates(hearth, verge), []);
    let drawing = play
        .session
        .map_drawing(None, &[], nova_sim::galaxy_map::MapMarks::NONE);
    assert!(
        !drawing.current_links.contains(&(hearth, verge)),
        "{:?}",
        drawing.current_links
    );
    assert_eq!(
        drawing
            .closed_lanes
            .iter()
            .map(|look| (look.from, look.to))
            .collect::<Vec<_>>(),
        [(hearth, verge)]
    );
}

/// The toll is offered once the side road is dead, the dock job done and
/// the wardens hold their road. Refused, it makes the road's offer hold:
/// the visit picks its next scene again at every `Open`, so the road's
/// offer opens in the same visit, after the toll's and before the camp's
/// own greeting, which a visit shows last.
#[test]
fn the_toll_is_offered_after_the_side_road_closes_and_the_dock_job() {
    let mut play = fixture_play("verge", |pilot| {
        pilot
            .story
            .missions
            .insert("dock-job".to_owned(), MissionState::Done);
        pilot.flags.set("dock_regular");
        // The closing day has come; the event happens at the next check.
        pilot.story.day = side_road_closing_day();
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
        "a take_point stays active while its guards hold the road"
    );
}

/// The fixture pilot at Verge Camp with the toll refused and the road
/// accepted, as the toll test reaches it.
fn on_the_road() -> Play {
    let mut play = fixture_play("verge", |pilot| {
        pilot
            .story
            .missions
            .insert("dock-job".to_owned(), MissionState::Done);
        pilot.story.day = side_road_closing_day();
    });
    play.land("verge-camp");
    play.open();
    play.choose(1);
    play.close();
    play.open();
    play.choose(0);
    play.close();
    assert_eq!(
        play.state("clear-the-road"),
        Some(MissionState::Active {
            cargo: 0,
            met: false
        })
    );
    play
}

/// The road is the wardens' until their guards at its gate are beaten:
/// accepted at Verge Camp, in the point's own system, the road's blockade
/// brings warden pickets to `verge/hearth-road`'s gate at the take-off,
/// and destroying every one meets and completes the road (it has no
/// completion), giving the point to the guild. The blockade is never
/// offered itself.
#[test]
fn the_road_is_taken_by_beating_the_wardens_at_its_gate() {
    let mut play = on_the_road();
    let blockade = nova_sim::MissionId(nova_of(&play.game.ids.blockades, "clear-the-road"));
    for venue in [
        nova_sim::mission::Venue::Computer,
        nova_sim::mission::Venue::Outside,
    ] {
        assert!(
            play.session
                .mission_offers(venue)
                .iter()
                .all(|offer| offer.mission != blockade),
            "{venue:?}: the blockade isn't offered at Verge Camp"
        );
    }
    play.take_off();
    let road = play.gate("verge/hearth-road");
    let at = gates(&play.game)
        .into_iter()
        .find(|gate| gate.stellar == road)
        .expect("the road's gate")
        .at;
    let wardens = nova_sim::GovtId(nova_of(&play.game.ids.factions, "warden"));
    let guards: Vec<_> = play
        .session
        .npcs()
        .iter()
        .filter(|npc| npc.mission.is_some())
        .map(|npc| (npc.id, npc.govt, npc.state.position))
        .collect();
    assert!(!guards.is_empty(), "the wardens hold their road");
    let record = (play
        .data
        .get::<nova_data::records::mission::Mission>(blockade.0))
    .expect("present")
    .expect("decodes");
    assert_eq!(
        guards.len(),
        usize::try_from(record.record.ship_count).expect("a count"),
        "as many as its blockade holds"
    );
    for &(_, govt, position) in &guards {
        assert_eq!(govt, Some(wardens));
        assert_eq!(position, at, "at the road's gate");
    }
    let (last, rest) = guards.split_last().expect("guards");
    for &(id, _, _) in rest {
        nova_sim::fixture::destroy(&mut play.session, id);
        play.settle();
    }
    assert_eq!(
        play.state("clear-the-road"),
        Some(MissionState::Active {
            cargo: 0,
            met: false
        }),
        "one guard is left"
    );
    nova_sim::fixture::destroy(&mut play.session, last.0);
    play.settle();
    assert_eq!(play.state("clear-the-road"), Some(MissionState::Done));
    assert_eq!(
        play.session.pilot().story.points["verge/hearth-road"]
            .owner
            .as_deref(),
        Some("guild")
    );
    let guild = nova_sim::GovtId(nova_of(&play.game.ids.factions, "guild"));
    assert_eq!(play.session.galaxy().gate_govt(road), Some(guild));
}

/// Paid, the toll has no objective, so accepting it completes it: Done,
/// with `on_accept`'s `paid_the_toll` set. The road's offer needs
/// `refused_the_toll`, so the visit goes straight to the camp's greeting.
#[test]
fn the_toll_is_paid_and_done_on_accept() {
    let mut play = fixture_play("verge", |pilot| {
        pilot
            .story
            .missions
            .insert("dock-job".to_owned(), MissionState::Done);
        pilot.flags.set("dock_regular");
        // The closing day has come; the event happens at the next check.
        pilot.story.day = side_road_closing_day();
    });
    play.land("verge-camp");
    assert_eq!(
        play.open(),
        texts(&[
            "A warden leans on the camp's gate. \"The side road's shut. Our road costs.\"",
            "Pay the toll",
            "Refuse",
            "Later",
        ])
    );
    assert_eq!(play.choose(0), texts(&["\"Pleasure.\""]));
    assert_eq!(play.state("warden-toll"), Some(MissionState::Done));
    assert!(play.flag("paid_the_toll"));
    assert!(play.flag("met_the_wardens"), "on_offer's flag stays");
    assert!(!play.flag("refused_the_toll"));
    play.close();
    assert_eq!(
        play.open(),
        texts(&GREETING),
        "no road offer once the toll is paid"
    );
    play.close();
    assert_eq!(play.converse(Converse::Open), Answer::Refused);
    assert_eq!(play.state("clear-the-road"), None);
    play.visit_again("verge-camp");
    assert_eq!(
        play.open(),
        texts(&GREETING),
        "a done toll isn't offered again"
    );
}

#[test]
fn a_declined_dock_job_never_comes_back() {
    let mut play = fixture_play("hearth", |_| {});
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
    let mut play = fixture_play("hearth", |_| {});
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
    let day = side_road_closing_day();
    let mut play = fixture_play("hearth", |pilot| {
        pilot.story.day = day;
        pilot.story.news.insert("plate-arrives".to_owned(), day);
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
    let (_, game) = new_game_play::opened(&folder());
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
