//! The sample's missions played over its checked-in cut
//! (`nova_data::new_game::fixture::sample_folder`), opened as the app opens
//! it, with `nova::dialogs::loader` and the new game's rulebook
//! (`new_game_play`).
//!
//! The storyline "Holding the point": the supply run (offered at Luyten
//! Exchange, twelve tons to Marrow Yard), the first failure (an event on
//! day 60 that breaks the Luyten road, with its news), the crossroads
//! offer at Marrow Yard and take the point, met by beating the band's
//! guards at the far gate. The gate crossings go through the session's own commands (the
//! land press over the gate, then the hypergate entry). The rules are
//! pinned in `nova_sim_story::storylines` and the session's forwarding in
//! `nova_sim`; this pins that the sample's data, the real Ink runtime and
//! the session meet.

use nova_data::new_game::fixture::sample_folder;
use nova_sim::flight_frame::{Answer, Converse};
use nova_sim::storylines::state::MissionState;
use nova_sim::{GovtId, LandingRefusal, Pilot, StellarId, SystemId};

use crate::new_game_play::{Play, texts};
use crate::new_game_travel::{gates, nova_of};

/// The sample's start pilot, changed by `edit`, flying in `system`.
fn sample(system: &str, edit: impl FnOnce(&mut Pilot)) -> Play {
    Play::new(&sample_folder(), system, edit)
}

const ACTIVE: Option<MissionState> = Some(MissionState::Active {
    cargo: 0,
    met: false,
});

const MET: Option<MissionState> = Some(MissionState::Active {
    cargo: 0,
    met: true,
});

/// The crossroads offer's lines after its greeting, and its choices, the
/// enclave question among them or not.
fn crossroads_rest(enclave: bool) -> Vec<&'static str> {
    let mut rest = vec![
        "\"Half the Association run comes through Marrow now. That makes the far gate worth \
         taking, and the Toll took it this morning.\"",
        "Ask what the warlord wants",
        "Ask about the Crown's offer",
    ];
    if enclave {
        rest.push("Ask whether the enclave would help");
    }
    rest.extend([
        "Offer to help the commons hold the point",
        "Tell them to take the Crown's protection",
        "Say you need time",
    ]);
    rest
}

/// The government of `faction` in `play`'s data.
fn govt(play: &Play, faction: &str) -> GovtId {
    GovtId(nova_of(&play.game.ids.factions, faction))
}

#[test]
fn the_holding_the_point_chain_plays_to_the_end() {
    let mut play = sample("luyten-crossing", |_| {});
    supply_run(&mut play);
    first_failure(&mut play);
    crossroads(&mut play);
    take_the_point(&mut play);
}

/// The supply run: accepted at Luyten Exchange, flown over the Marrow
/// road and delivered at Marrow Yard, asking about the enclave.
fn supply_run(play: &mut Play) {
    play.land("luyten-exchange");
    assert_eq!(
        play.open(),
        texts(&[
            "The exchange clerk at Luyten Crossing doesn't look up from the manifest.",
            "\"Twelve tons for Marrow. Seed stock and pump parts. The commons pay on delivery, \
             and they pay late.\"",
            "Take the job",
            "Ask why nobody else will fly it",
            "Pass",
        ])
    );
    assert_eq!(
        play.choose(0),
        texts(&["\"Dock four. If the Harrow boys hail you at the far gate, don't stop.\""])
    );
    assert_eq!(
        play.state("supply-run"),
        Some(MissionState::Active {
            cargo: 12,
            met: false
        })
    );
    assert_eq!(play.session.pilot().total_cargo(), 12);
    play.close();

    assert_eq!(
        play.go_through("luyten-crossing/marrow-road"),
        play.system("marrow")
    );
    play.land("marrow-yard");
    assert_eq!(
        play.open(),
        texts(&[
            "The cargo bay opens on a station that is mostly patches. A person in a borrowed \
             pressure suit signs for the crates without counting them.",
            "\"Tally Ashgrove. I organise things here, when they let me. You came in on the far \
             gate?\"",
            "Ask who holds the far gate",
            "Ask about the lights on the Lantern road",
            "Ask what the commons need",
            "Leave",
        ])
    );
    assert_eq!(play.state("supply-run"), Some(MissionState::Done));
    assert!(play.flag("met_organizer"));
    assert_eq!(play.session.pilot().total_cargo(), 0, "dropped off");
    assert_eq!(
        play.choose(1),
        texts(&[
            "\"The enclave. Monists. They keep to themselves and take in children nobody else \
             will. Don't ask them for anything they haven't offered.\"",
            "Ask who holds the far gate",
            "Ask what the commons need",
            "Leave",
        ])
    );
    play.choose(2);
    play.close();
}

/// Days pass to day 60: the first failure breaks the Luyten road.
fn first_failure(play: &mut Play) {
    let day = play.session.pilot().story.day;
    play.pass(60 - day);
    assert_eq!(play.state("the-first-failure"), Some(MissionState::Done));
    let unstable = |play: &Play, point: &str| {
        play.session.pilot().story.points[point].stability
            == Some(nova_data::GateStability::Unstable)
    };
    assert!(unstable(play, "luyten-crossing/association-road"));
    assert!(unstable(play, "brightwater/luyten-road"));
    assert!(
        play.session
            .pilot()
            .story
            .news
            .contains_key("luyten-road-fails")
    );
}

/// The crossroads offer at Marrow Yard, to a pilot who has met the
/// organizer: committed to, then take the point's briefing accepted in
/// the same visit.
fn crossroads(play: &mut Play) {
    play.visit_again("marrow-yard");
    let mut offer = vec![
        "Tally is waiting at the dock, thinner than you remember.",
        "\"You've heard about the Luyten road.\"",
    ];
    offer.extend(crossroads_rest(true));
    assert_eq!(
        play.open(),
        texts(&offer),
        "met, and asked about the enclave"
    );
    let far_gate = play.gate("marrow/far-gate");
    assert_eq!(
        play.session.galaxy().gate_govt(far_gate),
        Some(govt(play, "harrow-band")),
        "the band took the far gate as the offer opened"
    );
    assert_eq!(
        play.choose(3),
        texts(&[
            "\"You know what that means. The Toll will want you dead, and when the Crown hears, \
             so will they.\"",
            "Commit to the commons",
            "Step back",
        ])
    );
    assert_eq!(
        play.choose(0),
        texts(&["\"Then we start with the blockade.\""])
    );
    assert!(play.flag("committed_to_the_point"));
    assert_eq!(play.state("the-crossroads-offer"), Some(MissionState::Done));
    play.close();

    assert_eq!(
        play.open(),
        texts(&[
            "\"The Toll has gunships on the far gate and a picket at the Hollow road. Break the \
             blockade and they'll call for help from Harrow Drift. Be waiting at the Hollow road \
             when it comes through.\"",
            "Fly the blockade",
            "Not yet",
        ]),
        "take the point's briefing, in the same visit"
    );
    assert_eq!(
        play.choose(0),
        texts(&["\"We'll have the Yard's guns ready, such as they are.\""])
    );
    assert_eq!(play.state("take-the-point"), ACTIVE);
    play.close();
}

/// The special ships in `play`'s system, the band's guards, as (id,
/// government, position).
fn guards(play: &Play) -> Vec<(nova_sim::NpcId, Option<GovtId>, nova_sim::Vec2)> {
    play.session
        .npcs()
        .iter()
        .filter(|npc| npc.mission.is_some())
        .map(|npc| (npc.id, npc.govt, npc.state.position))
        .collect()
}

/// Where the gate `gate` sits.
fn gate_at(play: &Play, gate: StellarId) -> nova_sim::Vec2 {
    gates(&play.game)
        .into_iter()
        .find(|at| at.stellar == gate)
        .expect("a gate")
        .at
}

/// Take the point: the band's gunships hold the far gate from the
/// take-off at Marrow Yard. Going through the gate and back meets
/// nothing, and brings them back at the gate. Destroying every one meets
/// it, and it completes at Marrow Yard, the far gate the commons'.
fn take_the_point(play: &mut Play) {
    let far_gate = play.gate("marrow/far-gate");
    play.take_off();
    let band = govt(play, "harrow-band");
    let held = guards(play);
    assert!(!held.is_empty(), "the band holds the far gate");
    for &(_, govt, position) in &held {
        assert_eq!(govt, Some(band));
        assert_eq!(position, gate_at(play, far_gate), "at the far gate");
    }
    assert_eq!(
        play.go_through("marrow/far-gate"),
        play.system("brightwater")
    );
    assert_eq!(
        play.state("take-the-point"),
        ACTIVE,
        "going through the point's gate meets nothing"
    );
    assert_eq!(
        play.go_through("brightwater/marrow-gate"),
        play.system("marrow")
    );
    assert_eq!(
        play.state("take-the-point"),
        ACTIVE,
        "arriving meets nothing"
    );
    assert_eq!(
        guards(play).len(),
        held.len(),
        "the guards are at the gate again"
    );
    for (id, _, _) in guards(play) {
        nova_sim::fixture::destroy(&mut play.session, id);
        play.settle();
    }
    assert_eq!(play.state("take-the-point"), MET, "the blockade is broken");
    play.land("marrow-yard");
    assert_eq!(
        play.open(),
        texts(&[
            "The far gate's beacon comes back on in the commons' colours. Someone on the Yard has \
             painted over the band's mark by hand.",
            "\"It's ours,\" Tally says, as if trying out the words. \"Now we find out what that \
             costs.\"",
        ])
    );
    assert_eq!(play.state("take-the-point"), Some(MissionState::Done));
    assert_eq!(
        play.session.pilot().story.points["marrow/far-gate"]
            .owner
            .as_deref(),
        Some("marrow-commons")
    );
    play.close();
    assert_eq!(
        play.session.galaxy().gate_govt(far_gate),
        Some(govt(play, "marrow-commons"))
    );
}

/// Whether `play`'s land press is denied at the gate `gate`.
fn denied(play: &mut Play, gate: StellarId) -> bool {
    matches!(
        play.session.land(),
        Err(LandingRefusal::Denied { stellar, .. }) if stellar == gate
    )
}

/// The sample's start pilot in `system` on day `day`, over the gate of
/// the jump point `point`.
fn over(system: &str, point: &str, day: u32) -> Play {
    let (_, ids) = crate::new_game_play::opened(&sample_folder());
    let point_gate = StellarId(nova_of(&ids.ids.gates, point));
    let mut play = sample(system, |pilot| pilot.stellar = Some(point_gate));
    play.pass(day);
    play
}

#[test]
fn the_first_failure_breaks_the_luyten_road() {
    let mut play = over("luyten-crossing", "luyten-crossing/association-road", 59);
    let road = play.gate("luyten-crossing/association-road");
    let (luyten, brightwater, marrow) = (
        play.system("luyten-crossing"),
        play.system("brightwater"),
        play.system("marrow"),
    );
    assert!(play.session.land().is_ok(), "cleared on day 59");
    assert_eq!(
        play.session.galaxy().lanes().gates(luyten, brightwater),
        [road]
    );
    play.pass(1);
    assert_eq!(play.state("the-first-failure"), Some(MissionState::Done));
    assert!(denied(&mut play, road), "denied on day 60");
    let lanes = play.session.galaxy().lanes();
    assert_eq!(lanes.gates(luyten, brightwater), []);
    assert_eq!(lanes.gates(brightwater, luyten), []);
    let drawing = play
        .session
        .map_drawing(None, &[], nova_sim::galaxy_map::MapMarks::NONE);
    let pair = |a: SystemId, b: SystemId| (a.min(b), a.max(b));
    assert!(
        !drawing
            .current_links
            .iter()
            .any(|&(a, b)| pair(a, b) == pair(luyten, brightwater)),
        "{:?}",
        drawing.current_links
    );
    assert_eq!(
        drawing
            .closed_lanes
            .iter()
            .map(|look| pair(look.from, look.to))
            .collect::<Vec<_>>(),
        [pair(luyten, brightwater)]
    );
    // The other roads stay open.
    let lanes = play.session.galaxy().lanes();
    assert_eq!(
        lanes.gates(luyten, marrow),
        [play.gate("luyten-crossing/marrow-road")]
    );
    assert_eq!(
        lanes.gates(marrow, luyten),
        [play.gate("marrow/luyten-road")]
    );
    assert_eq!(
        lanes.gates(marrow, brightwater),
        [play.gate("marrow/far-gate")]
    );
    assert_eq!(
        lanes.gates(brightwater, marrow),
        [play.gate("brightwater/marrow-gate")]
    );

    let mut play = over("brightwater", "brightwater/luyten-road", 60);
    let far_end = play.gate("brightwater/luyten-road");
    assert!(denied(&mut play, far_end), "the Brightwater end too");
}

#[test]
fn the_first_failures_news_reaches_the_other_systems_over_the_following_days() {
    let mut play = sample("luyten-crossing", |_| {});
    play.pass(59);
    for system in ["luyten-crossing", "marrow", "brightwater"] {
        assert_eq!(play.heard(system), [], "{system} on day 59");
    }
    let news = |age: u32| vec![("luyten-road-fails".to_owned(), age)];
    play.pass(1);
    assert_eq!(play.heard("luyten-crossing"), news(0));
    assert_eq!(play.heard("marrow"), []);
    assert_eq!(
        play.heard("brightwater"),
        [],
        "the dead road carries nothing"
    );
    play.pass(1);
    assert_eq!(play.heard("marrow"), news(0), "over the Marrow road");
    assert_eq!(play.heard("brightwater"), []);
    play.pass(1);
    assert_eq!(
        play.heard("brightwater"),
        news(0),
        "by way of Marrow's far gate"
    );
    assert_eq!(play.heard("luyten-crossing"), news(2));
}

#[test]
fn the_crossroads_offer_waits_for_the_failure_and_greets_a_stranger_without_the_supply_run() {
    let mut play = sample("marrow", |_| {});
    play.pass(59);
    play.land("marrow-yard");
    assert_eq!(
        play.converse(Converse::Open),
        Answer::Refused,
        "no crossroads before the failure"
    );
    play.pass(1);
    play.visit_again("marrow-yard");
    let mut stranger = vec![
        "A person in a borrowed pressure suit meets you at the dock.",
        "\"Tally Ashgrove. I organise the commons. You came in on the far gate, so you've heard \
         about the Luyten road.\"",
    ];
    stranger.extend(crossroads_rest(false));
    assert_eq!(play.open(), texts(&stranger), "no supply run: no enclave");
    let owner = |play: &Play| {
        play.session.pilot().story.points["marrow/far-gate"]
            .owner
            .clone()
    };
    assert_eq!(owner(&play), Some("harrow-band".to_owned()));
    assert_eq!(
        play.choose(4),
        texts(&["\"Don't take long. The Toll won't.\""]),
        "say you need time"
    );
    assert_eq!(
        play.state("the-crossroads-offer"),
        Some(MissionState::Offered)
    );
    play.close();

    play.visit_again("marrow-yard");
    assert_eq!(play.open(), texts(&stranger), "deferred: offered again");
    assert_eq!(owner(&play), Some("harrow-band".to_owned()));
    assert_eq!(
        play.choose(3),
        texts(&["\"Maybe we will. Maybe they'd even keep their word.\""])
    );
    assert_eq!(
        play.state("the-crossroads-offer"),
        Some(MissionState::Declined)
    );
    assert!(play.flag("refused_the_organizer"));
    assert!(!play.flag("committed_to_the_point"));
    play.close();
    play.visit_again("marrow-yard");
    assert_eq!(
        play.converse(Converse::Open),
        Answer::Refused,
        "declined: never again"
    );
    assert_eq!(play.state("take-the-point"), None);
}
