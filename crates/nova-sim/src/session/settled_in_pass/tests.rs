//! The tests of the `settled_in_pass` module.

#![allow(clippy::float_cmp)]

use std::cell::RefCell;

use super::*;
use crate::ai::{Behaviour, Decision, Goal, NovaAi, Reaction, Surroundings};
use crate::catalog::SystemId;
use crate::chance::NeverFires;
use crate::combat::armament::Trigger;
use crate::combat::{CombatEvent, Rules, ShipRef, Strike};
use crate::flight::Controls;
use crate::flight_frame::{StepInput, StepReport};
use crate::geometry::Vec2;
use crate::session::step::StepCollaborators;
use crate::session::tests::{FIRE, OUT, armed, bound_for_131};
use crate::sound::{SimSound, WARP_UP_HALTS};
use crate::testkit::{FakePilotCatalog, catalog, work};
use crate::traffic::npc::{Npc, NpcId};

/// `catalog`'s ship bound for 131 on a warp whose sounds last 0 ticks,
/// run two passes short of its natural end: no warp-up plays, so its
/// 29th pass, at a stage of 31, ends it.
fn two_short_of_the_end(catalog: &FakePilotCatalog) -> Session {
    let mut session = bound_for_131(catalog, OUT, Vec2::ZERO, 90.0);
    session.statics_mut().warp_sounds = [0, 0];
    session.begin_jump().expect("jumps");
    for _ in 0..27 {
        work(&mut session, catalog, Controls::default());
    }
    assert_eq!(session.ship.warp_stage(session.mode.jump()), 29.0);
    session
}

/// [`two_short_of_the_end`], a pass further on: the next pass ends it.
fn a_pass_short_of_the_end(catalog: &FakePilotCatalog) -> Session {
    let mut session = two_short_of_the_end(catalog);
    work(&mut session, catalog, Controls::default());
    session
}

/// One advance pass of `session`, Nova's AI deciding: whether a rescue
/// settled on it.
fn advance(session: &mut Session, catalog: &FakePilotCatalog) -> bool {
    session.game_work(
        Controls::default(),
        catalog,
        &NovaAi::default(),
        Rules::default(),
        &mut NeverFires,
    )
}

/// `count` steps of `session` under `input`, on `catalog`'s collaborators.
fn steps(
    session: &mut Session,
    catalog: &FakePilotCatalog,
    input: &StepInput,
    count: u32,
) -> StepReport {
    let behaviour = NovaAi::default();
    let mut chance = NeverFires;
    let mut collab = StepCollaborators {
        catalog,
        behaviour: &behaviour,
        rules: Rules::default(),
        chance: &mut chance,
    };
    let mut report = StepReport::default();
    session.step(input, count, &mut collab, &mut report);
    report
}

/// Whether `sounds` hold the arrival's: the warp-ups' halt and the
/// warp-out.
fn the_arrivals(sounds: &[SimSound]) -> bool {
    sounds.contains(&SimSound::Halt(&WARP_UP_HALTS)) && sounds.contains(&SimSound::Arrived)
}

#[test]
fn a_warp_ended_on_the_advance_pass_arrives_in_it() {
    let catalog = catalog();
    let mut session = a_pass_short_of_the_end(&catalog);
    session.take_sounds();
    assert!(!advance(&mut session, &catalog), "no rescue");
    assert_eq!(session.jumping(), None);
    assert_eq!(session.system(), SystemId(131));
    let sounds = session.take_sounds();
    assert!(the_arrivals(&sounds), "in the pass: {sounds:?}");
}

#[test]
fn a_warp_ended_on_the_second_pass_arrives_in_it() {
    let catalog = catalog();
    let mut session = a_pass_short_of_the_end(&catalog);
    session.take_sounds();
    session.tick_again(
        Controls::default(),
        &catalog,
        &NovaAi::default(),
        Rules::default(),
        &mut NeverFires,
    );
    assert_eq!(session.jumping(), None);
    assert_eq!(session.system(), SystemId(131));
    let sounds = session.take_sounds();
    assert!(the_arrivals(&sounds), "in the pass: {sounds:?}");
}

#[test]
fn an_exit_reads_its_pass_again_among_the_new_system_marked_as_returned() {
    let catalog = catalog();
    let mut session = a_pass_short_of_the_end(&catalog);
    let left = session.space.traffic.add_npc(crate::testkit::npc(
        7,
        crate::stats::ShipStats::new(crate::testkit::FAST, &[]),
    ));
    let mut start = super::super::game_work::PassStart::of(&session);
    assert!(!start.player_head().returned);
    assert!(start.condition_of(left).is_some());
    assert_eq!(
        session.settle_exit(&mut start, &catalog, &mut NeverFires),
        None,
        "no exit due before the end"
    );
    // The warp's end, as the pass's warp stage leaves it.
    work(&mut session, &catalog, Controls::default());
    let mut start = super::super::game_work::PassStart::of(&session);
    session.ship.fate.rescue_due = true;
    assert_eq!(
        session.settle_exit(&mut start, &catalog, &mut NeverFires),
        Some(Exit::Rescue)
    );
    assert!(start.player_head().returned, "left by the epilogue");
    assert_eq!(start.condition_of(left), None, "the system left's ship");
    assert_eq!(session.out.exit, None, "a pass run alone records none");
    assert_eq!(session.out.second_pass, None, "nor holds lines");
}

/// Nova's AI, recording which NPC each decision was for, and firing the
/// primary for the NPC it is told to.
#[derive(Debug, Default)]
struct Recording {
    ai: NovaAi,
    firing: Option<NpcId>,
    decided: RefCell<Vec<NpcId>>,
}

impl Behaviour for Recording {
    fn decide(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Goal {
        self.ai.decide(npc, around, chance)
    }

    fn decision(&self, npc: &Npc, around: &Surroundings, chance: &mut dyn Chance) -> Decision {
        self.decided.borrow_mut().push(npc.id);
        self.ai.decision(npc, around, chance)
    }

    fn trigger(&self, npc: &Npc, around: &Surroundings) -> Trigger {
        if self.firing == Some(npc.id) {
            FIRE
        } else {
            self.ai.trigger(npc, around)
        }
    }

    fn target(&self, npc: &Npc, around: &Surroundings) -> Option<ShipRef> {
        self.ai.target(npc, around)
    }

    fn react(&self, npc: &Npc, strike: &Strike, around: &Surroundings) -> Reaction {
        self.ai.react(npc, strike, around)
    }
}

/// The ship `event` names, if it names one.
fn named(event: &CombatEvent) -> Option<ShipRef> {
    match *event {
        CombatEvent::Fired { ship, .. }
        | CombatEvent::Disabled { ship }
        | CombatEvent::BreakingUp { ship, .. }
        | CombatEvent::Hit { ship, .. } => Some(ship),
        _ => None,
    }
}

/// [`armed`], with traffic in 131 as in 130, four ships on average.
fn two_systems() -> FakePilotCatalog {
    let mut catalog = armed();
    let mut there = catalog.traffic[0];
    there.0 = SystemId(131);
    there.1.avg_ships = 4;
    catalog.traffic.push(there);
    catalog
}

#[test]
fn on_the_pass_a_warp_ends_no_ship_of_the_old_system_thinks_flies_fires_or_hits() {
    // The arrival leaves `_HandlePlayer` (`jmp 0x71113` @0x6c625) and the
    // rest of the pass runs among the ships `_SetupShipsInSystem` set up
    // (@0x6c1f9): the AI over them (@0x4429d), then `_HandleShot`
    // (@0x44385).
    let catalog = two_systems();
    let mut session = two_short_of_the_end(&catalog);
    session.populate(
        &catalog,
        &mut crate::testkit::Draws::of(&[6, 6, 0, 0, 750, 650, 180]),
    );
    let old = session.npcs().first().expect("130's traffic").id;
    // Beside the player, facing it, firing on the pass before.
    let at = session.ship.player.position + Vec2::new(0.0, -100.0);
    let npc = session.space.npc_mut(old).expect("there");
    npc.state.position = at;
    npc.state.heading = 180.0;
    let behaviour = Recording {
        firing: Some(old),
        ..Recording::default()
    };
    let pass = |session: &mut Session| {
        session.game_work(
            Controls::default(),
            &catalog,
            &behaviour,
            Rules::default(),
            &mut NeverFires,
        )
    };
    pass(&mut session);
    let fired = session.take_combat_events();
    assert!(
        fired.iter().any(|event| matches!(
            event,
            CombatEvent::Fired { ship, .. } if *ship == ShipRef::Npc(old)
        )),
        "it fires on the pass before: {fired:?}"
    );
    assert!(!session.shots().is_empty(), "its shot in flight");
    behaviour.decided.borrow_mut().clear();
    pass(&mut session);
    assert_eq!(session.system(), SystemId(131), "the arrival's pass");
    let decided = behaviour.decided.take();
    assert!(!decided.contains(&old), "it never decides: {decided:?}");
    let new: Vec<NpcId> = session.npcs().iter().map(|npc| npc.id).collect();
    assert!(!new.contains(&old), "gone with its system");
    assert!(!decided.is_empty(), "the new system's ships think");
    assert!(
        decided.iter().all(|id| new.contains(id)),
        "every decision a new ship's: {decided:?}, {new:?}"
    );
    let events = session.take_combat_events();
    assert!(
        events
            .iter()
            .all(|event| named(event) != Some(ShipRef::Npc(old))),
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, CombatEvent::Hit { .. })),
        "its shot hits nothing: {events:?}"
    );
    assert!(
        session.shots().is_empty(),
        "nor flies on: {:?}",
        session.shots()
    );
}

// `jmp 0x71113` (@0x6c625) skips the rest of `_HandlePlayer`: the death
// timer (@0x6d7e6), the escort menu's block (@0x6df59), the klaxon
// (@0x6f1e4), the eject (@0x6f22c) and the cloak key (@0x705b7).

#[test]
fn the_arrivals_pass_runs_no_escort_menu_block() {
    let catalog = catalog();
    let mut session = a_pass_short_of_the_end(&catalog);
    let report = steps(&mut session, &catalog, &StepInput::default(), 1);
    let run = &report.steps[0];
    assert!(run.arrival.is_some(), "the arrival's step");
    assert_eq!(run.passes[0].escort_menu, None, "no escort-menu block");
}

#[test]
fn the_arrivals_pass_takes_no_eject() {
    use crate::catalog::OutfitId;
    use crate::session::eject::tests::{POD, ejecting, in_the_pod};
    let catalog = ejecting(100);
    let mut session = a_pass_short_of_the_end(&catalog);
    // A pod aboard and the chord held, the armour gone after the pass's
    // head read the player intact, so the move doesn't collapse the jump
    // and the eject check, reading `_IsDying` by the armour, would eject.
    session.pilot.outfits.insert(OutfitId(POD), 1);
    session.pilot.reserves.armor.now = 0.0;
    let input = StepInput {
        eject: true,
        ..StepInput::default()
    };
    let report = steps(&mut session, &catalog, &input, 1);
    assert!(report.steps[0].arrival.is_some(), "the arrival's step");
    assert!(!in_the_pod(&session), "no eject");
    assert_eq!(session.pod, None);
}

#[test]
fn the_arrivals_pass_takes_no_cloak_key() {
    use crate::catalog::OutfitId;
    use crate::cloak::{Cloak, MOD_CLOAK};
    let mut catalog = armed();
    catalog
        .outfits
        .push(nova_sim_catalog::fixture::outfit(300, &[(MOD_CLOAK, 0)]));
    let mut session = a_pass_short_of_the_end(&catalog);
    // A cloak owned and paid for: off the jump, the key would cloak.
    session.pilot.outfits.insert(OutfitId(300), 1);
    session.pilot.reserves.fuel.now = 100.0;
    session.pilot.reserves.shield.now = 100.0;
    session.take_sounds();
    let input = StepInput {
        cloak: true,
        ..StepInput::default()
    };
    let report = steps(&mut session, &catalog, &input, 1);
    assert!(report.steps[0].arrival.is_some(), "the arrival's step");
    assert_eq!(session.player_cloak(), Cloak::default(), "not cloaking");
    let sounds = session.take_sounds();
    assert!(
        !sounds.contains(&SimSound::Cloaked) && !sounds.contains(&SimSound::Refused),
        "the key unread: {sounds:?}"
    );
}

#[test]
fn the_arrivals_start_ticks_no_death_timer_and_sounds_no_klaxon() {
    // A player dying at the pass's head collapses its jump in the move
    // (see the `jump_collapse` module), so no whole pass arrives with its
    // timer running. The pass's own steps show the skip instead: its
    // move ends the warp, the exit settles, and a timer running then
    // keeps its ticks.
    use super::super::game_work::PassStart;
    use crate::combat::GamePass;
    use crate::combat::hull::Condition;
    let catalog = catalog();
    let mut session = a_pass_short_of_the_end(&catalog);
    let mut start = PassStart::of(&session);
    let pass = GamePass::Advance { double_time: false };
    session.player_move(Controls::default(), pass, start.player_head());
    assert_eq!(
        session.settle_exit(&mut start, &catalog, &mut NeverFires),
        Some(Exit::Arrival)
    );
    let dying = Condition::Dying {
        ticks_left: 5,
        starting: false,
    };
    session.ship.condition = dying;
    session.take_sounds();
    session.player_display_head(start.player_head(), &mut NeverFires);
    assert_eq!(session.ship.condition, dying, "the timer unticked");
    let sounds = session.take_sounds();
    assert!(!sounds.contains(&SimSound::Klaxon), "{sounds:?}");
}

#[test]
fn the_arrivals_pass_displays_the_player_once_in_the_new_system() {
    // `_DoPlayGameWork` goes on from @0x43e1e: the player's
    // `_HandleShipDisplay` (@0x43e29) runs after the arrival.
    let catalog = catalog();
    let mut session = a_pass_short_of_the_end(&catalog);
    let report = steps(&mut session, &catalog, &StepInput::default(), 1);
    let run = &report.steps[0];
    let arrival = run.arrival.as_ref().expect("arrived");
    let pass = &run.passes[0];
    assert_eq!(
        pass.player_position, arrival.place.position,
        "where it arrived"
    );
    assert_eq!(pass.ships[0].ship, ShipRef::Player);
    assert_eq!(pass.ships[0].position, arrival.place.position);
    assert_eq!(session.system(), SystemId(131));
}

#[test]
fn the_fight_events_before_the_arrival_are_heard_not_reported() {
    use crate::session::crons::tests::scripted;
    use crate::session::y_explosion::tests::{exploding, flown, heard_booms};
    let catalog = exploding(vec![scripted(128, 1, "Y140", "")]);
    let mut session = flown(&catalog, crate::rulebook::Rulebook::default());
    session.plot_course(SystemId(131)).expect("a route");
    crate::testkit::fly_out(&mut session, &catalog);
    crate::testkit::begin_jump_now(&mut session, &catalog).expect("jumps");
    let mut probe = session.clone();
    let mut before = 0;
    while steps(&mut probe, &catalog, &StepInput::default(), 1).steps[0]
        .arrival
        .is_none()
    {
        before += 1;
        assert!(before < 2000, "the warp never ends");
    }
    for _ in 0..before {
        steps(&mut session, &catalog, &StepInput::default(), 1);
    }
    // An explosion pending in the system left as the arrival's pass runs.
    let boom = crate::catalog::Explosion {
        boom: crate::catalog::BoomId(130),
        extra: false,
    };
    session.space.combat.explode(Vec2::ZERO, boom);
    let report = steps(&mut session, &catalog, &StepInput::default(), 1);
    let run = &report.steps[0];
    let arrival = run.arrival.as_ref().expect("arrived");
    let pending = |events: &[CombatEvent]| {
        events.iter().any(|event| {
            matches!(
                event,
                CombatEvent::Exploded { explosion, .. } if explosion.boom == boom.boom
            )
        })
    };
    assert!(pending(&arrival.before), "heard: {:?}", arrival.before);
    assert!(!pending(&run.passes[0].combat), "not drawn");
    assert_eq!(heard_booms(&arrival.before), [], "the days' come after it");
    assert_eq!(
        heard_booms(&run.passes[0].combat),
        [Vec2::new(100.0, 200.0)],
        "the arrival's days' `Y`, reported with its pass"
    );
}

/// [`armed`]'s catalog holding the player's class 128's sheet, a
/// 10-pixel square, and, if `ring`, a ring as graphic type 3, with a
/// deadly stellar of type 3 in 131 centred on `at`: the player there sits
/// in the ring's hole, which its mask misses and its disc meets (see the
/// `deadly_masks` module).
fn deadly_at(at: Vec2, ring: bool) -> FakePilotCatalog {
    use crate::catalog::{LandingSite, MaskSheet, ShipId};
    use crate::deadly_stellar::DEADLY_STELLAR;
    use crate::session::deadly_masks::tests::{SIDE, ringed, solid};
    let mut catalog = armed();
    catalog.rle_sheets = vec![(MaskSheet::Ship(ShipId(128)), solid(10))];
    if ring {
        catalog.rle_sheets.push((MaskSheet::Stellar(3), ringed()));
    }
    let site = LandingSite {
        flags2: DEADLY_STELLAR,
        graphic: 3,
        frame_size: Some((SIDE, SIDE)),
        ..crate::testkit::planet(140, at.x, at.y)
    };
    catalog.sites.retain(|(system, _)| *system != SystemId(131));
    catalog.sites.push((SystemId(131), vec![site]));
    catalog
}

/// Whether the player, a pass short of the warp's end on `deadly_at`'s
/// catalog, lives through the arrival's pass, run by `pass`, where the
/// arrival puts it: on the ring's hole, by the new system's masks.
///
/// The system left asks for no stellar sheet, so the ring's is read only
/// if the pass asks again after its exit: by the load after it, or by the
/// beams' phase's own ask, which its `sheets` gives it. Either one
/// suffices, since the arrival's pass holds nothing else of the new
/// system's for the scoop or the collision passes to touch (no box, no
/// shot); these tests fail when the pass asks by neither.
fn lives_the_arrival(ring: bool, pass: impl Fn(&mut Session, &FakePilotCatalog)) -> bool {
    use crate::combat::hull::Condition;
    // Where the arrival puts it, with no deadly stellar there.
    let mut probe = a_pass_short_of_the_end(&armed());
    pass(&mut probe, &armed());
    assert_eq!(probe.system(), SystemId(131));
    let at = probe.ship.player.position;
    let catalog = deadly_at(at, ring);
    let mut session = a_pass_short_of_the_end(&catalog);
    let asked = catalog.rle_sheets_asked.borrow().clone();
    assert!(
        !asked
            .iter()
            .any(|sheet| matches!(sheet, crate::catalog::MaskSheet::Stellar(_))),
        "130 lists no deadly stellar, so no stellar sheet was asked: {asked:?}"
    );
    pass(&mut session, &catalog);
    assert_eq!(session.system(), SystemId(131), "the arrival's pass");
    assert_eq!(session.ship.player.position, at);
    session.ship.condition != Condition::Destroyed
}

#[test]
fn an_advance_pass_arrival_touches_the_new_systems_deadly_stellar_by_its_masks() {
    let first = |session: &mut Session, catalog: &FakePilotCatalog| {
        advance(session, catalog);
    };
    assert!(lives_the_arrival(true, first), "in the hole by its mask");
    assert!(!lives_the_arrival(false, first), "by the discs, it dies");
}

#[test]
fn a_second_pass_arrival_touches_the_new_systems_deadly_stellar_by_its_masks() {
    let again = |session: &mut Session, catalog: &FakePilotCatalog| {
        session.tick_again(
            Controls::default(),
            catalog,
            &NovaAi::default(),
            Rules::default(),
            &mut NeverFires,
        );
    };
    assert!(lives_the_arrival(true, again), "in the hole by its mask");
    assert!(!lives_the_arrival(false, again), "by the discs, it dies");
}
