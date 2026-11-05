//! A course plotted over the new game's gates and flown by hand, gate by
//! gate, over the checked-in fixture (`nova_data::new_game::fixture`),
//! wired as the app wires it: the mapped `GameData` and the rulebook
//! `ruleset_rulebook` gives `Ruleset::NewGame`.
//!
//! - **The course**: from drift, a Shift-click on hearth then verge,
//!   closed with the cue, as the course map plots it a hop at a time.
//! - **The flight**: for each hop the ship is put at the gate serving it,
//!   presses L to clear and L to enter, and picks the hop on the
//!   hypergate map; it comes out at the partner's gate with the rest of
//!   the course kept and its next hop cued. J never jumps.
//! - **The autopilot**: J engages the gate autopilot, the frame's steps
//!   fly the ship with no controls held, each L it asks for is sent, and
//!   each hypergate entered for its system, as the view wires them; it
//!   flies the course to its end and is never in a hyperspace jump.
//! - **The map**: each lane drawn per direction in its stability's look.
//! - **The stock rulebook** over the same data finds no link anywhere.
//!
//! Which readers follow the lanes, how a look is worked out and what the
//! exit does to the course are nova-sim's rules, pinned there; this pins
//! that the data, the ruleset and the session meet. [`fly_by_hand`] and
//! [`fly_on_autopilot`] are shared with `sample_routes`, which flies
//! game-bible's sample.

use nova::chance::SplitMix;
use nova_data::GameData;
use nova_data::new_game::NewGame;
use nova_sim::catalog::GateStability::{Dead, Stable, Unstable};
use nova_sim::flight_frame::{
    Answer, FlightCommand, FlightEvent, FlightFrame, MapClick, StepInput, StepReport,
};
use nova_sim::galaxy_map::MapMarks;
use nova_sim::gate::GateKind;
use nova_sim::hyperspace::{JumpRefusal, RouteError};
use nova_sim::landing::LandOutcome;
use nova_sim::map_lanes::{Back, LaneLook};
use nova_sim::navigation::NavMode;
use nova_sim::{
    ApplyCollaborators, HailOptions, LandPress, NovaAi, NovaBoarding, NovaLaw, Rulebook, Rules,
    Session, SimSound, StepCollaborators, SystemId,
};

use crate::new_game_travel::{Gate, fixture, gates, new_game_rules, nova_of, pass};

/// `command` applied to `session` over `data`.
fn apply(session: &mut Session, data: &GameData, command: FlightCommand) -> Answer {
    session.apply(
        command,
        &mut ApplyCollaborators {
            catalog: Some(data),
            chance: &mut SplitMix::new(3),
            law: &NovaLaw::default(),
            boarding_rule: &NovaBoarding::default(),
            hail_options: &HailOptions::default(),
        },
    )
}

/// A Shift-click on the course map hitting `system`.
fn shift_click(session: &mut Session, data: &GameData, system: SystemId) -> Answer {
    apply(
        session,
        data,
        FlightCommand::MapClick(MapClick {
            hits: vec![system],
            shift: true,
            hypergate: None,
            missions: Vec::new(),
            green: None,
        }),
    )
}

/// `game`'s system `name`.
pub(crate) fn system(game: &NewGame, name: &str) -> SystemId {
    SystemId(nova_of(&game.ids.systems, name))
}

/// `game`'s start pilot flying from `system`, at its centre, by `rules`.
pub(crate) fn flying_from(game: &NewGame, rules: &Rulebook, system: SystemId) -> Session {
    let mut pilot = crate::new_game_travel::pilot_for(game);
    pilot.system = system;
    pilot.stellar = None;
    Session::load(&game.data, pilot, rules).expect("flies")
}

/// J refuses with no destination and in silence, and starts no jump.
fn j_refuses_silently(session: &mut Session) {
    session.drain_events(&mut Vec::new());
    assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
    assert_eq!(session.preparing_jump(), None, "no pre-jump");
    assert_eq!(session.jumping(), None);
    let mut events = Vec::new();
    session.drain_events(&mut events);
    assert!(
        !events.contains(&FlightEvent::Sound(SimSound::Refused)),
        "{events:?}"
    );
}

/// Flies `session`'s course by hand over `game` by `rules`, gate by gate:
/// for each hop, the ship put at the first gate serving its lane, L to
/// clear, a pass, L to enter, and the hop picked on the hypergate map.
/// Gives each system it comes out in. After each exit but the last the
/// course keeps the rest and mode 3 cues its next hop, and J refuses it
/// silently; after the last the course is empty and the nav off. No pass
/// is ever in a hyperspace jump.
pub(crate) fn fly_by_hand(
    game: &NewGame,
    rules: &Rulebook,
    session: &mut Session,
) -> Vec<SystemId> {
    let gates = gates(game);
    let mut visited = vec![session.system()];
    let mut chance = SplitMix::new(5);
    while let Some(&hop) = session.course().first() {
        let here = session.system();
        let serving = session.galaxy().lanes().gates(here, hop).to_vec();
        let road: &Gate = gates
            .iter()
            .find(|road| serving.first() == Some(&road.stellar))
            .unwrap_or_else(|| panic!("a road from {here:?} to {hop:?}"));
        nova_sim::fixture::put_player_at(session, road.at);
        assert!(session.land().is_ok(), "{road:?}: cleared");
        pass(session, &game.data, rules, &mut chance);
        assert_eq!(session.jumping(), None, "no hyperspace jump");
        assert_eq!(
            session.land(),
            Ok(LandPress::AtGate {
                stellar: road.stellar,
                kind: GateKind::Hypergate,
            }),
            "{road:?}"
        );
        assert_eq!(
            apply(session, &game.data, FlightCommand::OpenHypergate),
            Answer::Offered(vec![hop]),
            "{road:?}: the one road's partner"
        );
        let entered = session.enter_hypergate(Some(hop), &game.data, &mut chance);
        assert_eq!(entered, Ok(hop), "{road:?}");
        assert_eq!(session.system(), road.partner.1);
        assert_eq!(session.player().position, road.partner.2, "at the partner");
        visited.push(hop);
        match session.course().first() {
            Some(&next) => {
                assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(next)));
                j_refuses_silently(session);
            }
            None => assert_eq!(session.nav_mode(), NavMode::Off),
        }
    }
    visited
}

/// The most frame steps the autopilot may take to reach a gate, ask for
/// clearance and enter it.
const TICKS_PER_GATE: u32 = 4_000;

/// `session`'s frame as it stands.
fn frame_of(session: &Session) -> FlightFrame {
    let mut frame = FlightFrame::default();
    session.fill_frame(&mut frame);
    frame
}

/// Flies `session`'s course on the gate autopilot over `game` by `rules`,
/// wired as the view wires it: J pressed toward the system selected in
/// mode 3 answers the autopilot engaged; then the session steps with no
/// controls held, a step a frame; each time the frame asks for L, L is
/// sent, and the press over a cleared hypergate enters it for the
/// autopilot's system. Gives each system it comes out in. Each L it asks
/// for counts as its own (it stays engaged after each), each exit comes
/// out at the partner's gate, no frame is ever in a hyperspace jump, and
/// each gate is reached within [`TICKS_PER_GATE`] steps. It ends when the
/// autopilot lets go.
pub(crate) fn fly_on_autopilot(
    game: &NewGame,
    rules: &Rulebook,
    session: &mut Session,
) -> Vec<SystemId> {
    let gates = gates(game);
    let NavMode::Hyperspace(Some(first)) = session.nav_mode() else {
        panic!("a system selected in mode 3: {:?}", session.nav_mode());
    };
    assert_eq!(
        apply(session, &game.data, FlightCommand::Jump),
        Answer::GatePilot(first)
    );
    let behaviour = NovaAi::from_rulebook(rules);
    let mut chance = SplitMix::new(7);
    let input = StepInput {
        window_width: 1024,
        ..StepInput::default()
    };
    let mut report = StepReport::default();
    let mut visited = vec![session.system()];
    let mut ticks = 0;
    loop {
        let frame = frame_of(session);
        assert!(!frame.jump_under_way, "never in a hyperspace jump");
        let Some(shown) = frame.gate_pilot else {
            return visited;
        };
        if shown.press_land {
            match apply(session, &game.data, FlightCommand::Land) {
                Answer::LandPress(LandPress::Outcome(LandOutcome::Requested { .. })) => {}
                Answer::LandPress(LandPress::AtGate {
                    stellar,
                    kind: GateKind::Hypergate,
                }) => {
                    assert!(frame_of(session).gate_pilot.is_some(), "its own press");
                    let road = gates
                        .iter()
                        .find(|road| road.stellar == stellar)
                        .unwrap_or_else(|| panic!("{stellar:?} is a jump point"));
                    assert!(
                        matches!(
                            apply(session, &game.data, FlightCommand::EnterHypergate(Some(shown.to))),
                            Answer::Arrived { system, .. } if system == shown.to
                        ),
                        "{road:?}"
                    );
                    assert_eq!(session.system(), road.partner.1);
                    assert_eq!(session.player().position, road.partner.2, "at the partner");
                    visited.push(shown.to);
                    ticks = 0;
                    continue;
                }
                other => panic!("the autopilot's press answered {other:?}"),
            }
            assert!(frame_of(session).gate_pilot.is_some(), "its own press");
        }
        let mut collab = StepCollaborators {
            catalog: &game.data,
            behaviour: &behaviour,
            rules: Rules::default(),
            chance: &mut chance,
        };
        session.step(&input, 1, &mut collab, &mut report);
        ticks += 1;
        assert!(
            ticks < TICKS_PER_GATE,
            "no gate within {TICKS_PER_GATE} steps from {:?}",
            session.system()
        );
    }
}

/// The look `session` gives the link between `a` and `b`, as the map
/// draws it from whichever end it draws it.
pub(crate) fn look_between(session: &Session, a: SystemId, b: SystemId) -> LaneLook {
    nova_sim::map_lanes::lane_look(session.galaxy(), a, b)
}

#[test]
fn a_shift_click_course_over_the_gates_is_flown_by_hand_keeping_the_course() {
    let game = fixture();
    let rules = new_game_rules();
    let (drift, hearth, verge) = (
        system(&game, "drift"),
        system(&game, "hearth"),
        system(&game, "verge"),
    );
    let mut session = flying_from(&game, &rules, drift);
    for hop in [hearth, verge] {
        assert!(
            matches!(
                shift_click(&mut session, &game.data, hop),
                Answer::Clicked(_)
            ),
            "{hop:?}"
        );
    }
    assert_eq!(session.course(), [hearth, verge], "a hop at a time");
    apply(&mut session, &game.data, FlightCommand::CueCourse);
    assert_eq!(session.nav_mode(), NavMode::Hyperspace(Some(hearth)));
    j_refuses_silently(&mut session);
    assert_eq!(
        fly_by_hand(&game, &rules, &mut session),
        [drift, hearth, verge]
    );
    assert_eq!(session.course(), []);
}

#[test]
fn a_shift_click_course_over_the_gates_is_flown_on_the_autopilot_gate_by_gate() {
    let game = fixture();
    let rules = new_game_rules();
    let (drift, hearth, verge) = (
        system(&game, "drift"),
        system(&game, "hearth"),
        system(&game, "verge"),
    );
    let mut session = flying_from(&game, &rules, drift);
    for hop in [hearth, verge] {
        shift_click(&mut session, &game.data, hop);
    }
    apply(&mut session, &game.data, FlightCommand::CueCourse);
    assert_eq!(
        fly_on_autopilot(&game, &rules, &mut session),
        [drift, hearth, verge]
    );
    assert_eq!(session.course(), []);
    assert_eq!(frame_of(&session).gate_pilot, None, "the autopilot off");
    assert_eq!(session.nav_mode(), NavMode::Off);
}

#[test]
fn the_map_draws_each_lane_per_direction_in_its_stabilitys_look() {
    let game = fixture();
    let session = flying_from(&game, &new_game_rules(), system(&game, "drift"));
    let (drift, hearth, verge) = (
        system(&game, "drift"),
        system(&game, "hearth"),
        system(&game, "verge"),
    );
    let both = |out| (Some(out), Back::Way(Some(out)));
    let look = |a, b| {
        let look = look_between(&session, a, b);
        (look.out, look.back)
    };
    assert_eq!(look(drift, hearth), both(Stable));
    assert_eq!(look(hearth, verge), both(Stable));
    assert_eq!(look(drift, verge), (Some(Unstable), Back::Way(Some(Dead))));
    assert_eq!(look(verge, drift), (Some(Dead), Back::Way(Some(Unstable))));
    let drawing = session.map_drawing(None, &[], MapMarks::NONE);
    let mut drawn = drawing.current_links.clone();
    drawn.sort_unstable();
    assert_eq!(drawn, [(drift, hearth), (drift, verge)]);
    for (at, &(from, to)) in drawing.current_links.iter().enumerate() {
        assert_eq!(
            drawing.lanes[at],
            look_between(&session, from, to),
            "{to:?}"
        );
    }
}

#[test]
fn no_system_has_a_con_link_under_either_rulebook() {
    let game = fixture();
    for rules in [new_game_rules(), Rulebook::default()] {
        let session = flying_from(&game, &rules, system(&game, "drift"));
        for name in ["drift", "hearth", "verge"] {
            let id = system(&game, name);
            assert_eq!(session.galaxy().con_slots(id), [], "{name}");
            assert_eq!(session.star_map().con_slots(id), [], "{name}");
        }
    }
}

#[test]
fn under_the_stock_rulebook_the_new_game_finds_no_link() {
    let game = fixture();
    let (drift, verge) = (system(&game, "drift"), system(&game, "verge"));
    let mut session = flying_from(&game, &Rulebook::default(), drift);
    let drawing = session.map_drawing(None, &[], MapMarks::NONE);
    assert_eq!(drawing.current_links, []);
    assert_eq!(drawing.links, []);
    assert_eq!(drawing.lanes, []);
    assert_eq!(session.plot_course(verge), Err(RouteError::Unreachable));
    let explored: Vec<SystemId> = session.pilot().explored().collect();
    let known: Vec<SystemId> = session.known().iter().copied().collect();
    assert_eq!(
        known, explored,
        "the known set is the explored systems alone"
    );
    session.mark_known([verge]);
    assert!(matches!(
        shift_click(&mut session, &game.data, verge),
        Answer::Clicked(_)
    ));
    assert_eq!(session.course(), [], "no link to extend along");
    assert_eq!(session.begin_jump(), Err(JumpRefusal::NoDestination));
}
